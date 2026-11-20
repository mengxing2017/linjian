use crate::{
    business::{Actor, Error, Result, require},
    statistics::{domain::*, south::port::StatisticsStore},
};
use chrono::{DateTime, Duration, Utc};
use serde_json::{Value, json};
use sqlx::{PgPool, Postgres, Row, Transaction};
use std::collections::BTreeMap;
use uuid::Uuid;
#[derive(Clone)]
pub struct Repository(pub PgPool, pub crate::statistics::config::Config);
async fn authorize(
    tx: &mut Transaction<'_, Postgres>,
    a: &Actor,
    school: Option<Uuid>,
    permission: &str,
) -> Result<()> {
    if !a.is_admin_browser() {
        return Err(Error::Forbidden);
    }
    let admin: bool =
        sqlx::query_scalar("SELECT platform_admin FROM accounts WHERE id=$1 AND active")
            .bind(a.id)
            .fetch_optional(&mut **tx)
            .await?
            .ok_or(Error::Unauthorized)?;
    if admin {
        return Ok(());
    }
    let s = school.ok_or(Error::Forbidden)?;
    let permitted: bool = sqlx::query_scalar("SELECT forum_has_permission($1,$2,$3)")
        .bind(a.id)
        .bind(s)
        .bind(permission)
        .fetch_one(&mut **tx)
        .await?;
    if permitted {
        Ok(())
    } else {
        Err(Error::Forbidden)
    }
}
#[derive(Clone)]
struct CoverageData {
    content: Option<DateTime<Utc>>,
    registration: Option<DateTime<Utc>>,
    unknown_time: bool,
    activity_start: Option<DateTime<Utc>>,
    activity_end: Option<DateTime<Utc>>,
    gaps: Vec<(DateTime<Utc>, DateTime<Utc>)>,
    content_gaps: Vec<(DateTime<Utc>, DateTime<Utc>)>,
    registration_gaps: Vec<(DateTime<Utc>, DateTime<Utc>)>,
}
impl CoverageData {
    fn at(&self, w: &Window, full: bool, unknown: bool, unknown_base: bool) -> Coverage {
        let content = self.content.is_some_and(|x| x <= w.start_at)
            && !self
                .content_gaps
                .iter()
                .any(|(a, b)| *a < w.actual_end_at && *b > w.start_at);
        let registration = self.registration.is_some_and(|x| x <= w.start_at)
            && !self.unknown_time
            && !self
                .registration_gaps
                .iter()
                .any(|(a, b)| *a < w.actual_end_at && *b > w.start_at);
        let assignment = full || !unknown;
        let base_assignment = full || !unknown_base;
        let activity = self.activity_start.is_some_and(|x| x <= w.start_at)
            && self.activity_end.is_some_and(|x| x >= w.actual_end_at)
            && !self
                .gaps
                .iter()
                .any(|(a, b)| *a < w.actual_end_at && *b > w.start_at);
        let mut reasons = Vec::new();
        if !content {
            reasons.push("内容数据存在缺口".into());
        }
        if !registration {
            reasons.push("注册数据存在缺口或时间异常".into());
        }
        if !assignment || !base_assignment {
            reasons.push("注册学校归属数据缺失，学校注册统计不可用".into());
        }
        if !activity {
            reasons.push("活跃采集未覆盖、延迟或存在缺口".into());
        }
        Coverage {
            content,
            registration,
            registration_assignment: assignment,
            registration_base: !self.unknown_time
                && base_assignment
                && !self
                    .registration_gaps
                    .iter()
                    .any(|(start, _)| *start < w.actual_end_at),
            content_from: self.content,
            registration_from: self.registration,
            activity_from: self.activity_start,
            activity_through: self.activity_end,
            activity,
            reasons,
        }
    }
}
async fn coverage(
    tx: &mut Transaction<'_, Postgres>,
    now: DateTime<Utc>,
    retention: i32,
) -> Result<CoverageData> {
    let content=sqlx::query_scalar("SELECT min(start_at) FROM statistics_coverage WHERE family='content' AND status='complete' AND end_at IS NULL").fetch_one(&mut **tx).await?;
    let registration=sqlx::query_scalar("SELECT min(start_at) FROM statistics_coverage WHERE family='registration' AND status='complete' AND end_at IS NULL").fetch_one(&mut **tx).await?;
    let unknown_time: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM statistics_registrations WHERE time_quality='unknown')",
    )
    .fetch_one(&mut **tx)
    .await?;
    // Explicitly verified client deployment is necessary before claiming complete access coverage.
    let activity_start:Option<DateTime<Utc>>=sqlx::query_scalar("SELECT min(start_at) FROM statistics_coverage WHERE family='activity' AND status='complete' AND end_at IS NULL").fetch_one(&mut **tx).await?;
    let retained_from:DateTime<Utc>=sqlx::query_scalar("SELECT date_trunc('month',now() AT TIME ZONE 'Asia/Shanghai') AT TIME ZONE 'Asia/Shanghai' - make_interval(months=>$1)").bind(retention).fetch_one(&mut **tx).await?;
    let activity_start = activity_start.map(|start| start.max(retained_from));
    let runs=sqlx::query("SELECT started_at,sealed_at,heartbeat_at,closed_at,failed FROM statistics_activity_runs ORDER BY started_at").fetch_all(&mut **tx).await?;
    let mut activity_end = None;
    let mut gaps = Vec::new();
    let mut previous_end = activity_start;
    for r in runs {
        let start: DateTime<Utc> = r.get("started_at");
        let sealed: DateTime<Utc> = r.get("sealed_at");
        let heartbeat: DateTime<Utc> = r.get("heartbeat_at");
        let closed: Option<DateTime<Utc>> = r.get("closed_at");
        let failed: bool = r.get("failed");
        if let Some(end) = previous_end
            && start > end
        {
            gaps.push((end, start));
        }
        let end = closed.unwrap_or(if heartbeat < now - Duration::seconds(30) {
            heartbeat + Duration::seconds(30)
        } else {
            now
        });
        if failed {
            gaps.push((start, end));
        } else if sealed < end {
            gaps.push((sealed, end));
        }
        if closed.is_none() && heartbeat < now - Duration::seconds(30) {
            gaps.push((sealed, end));
        }
        previous_end = Some(previous_end.unwrap_or(end).max(end));
        activity_end = Some(activity_end.unwrap_or(sealed).max(sealed));
    }
    let mut content_gaps = Vec::new();
    let mut registration_gaps = Vec::new();
    for r in sqlx::query("SELECT family,start_at,coalesce(end_at,$1) AS end_at FROM statistics_coverage WHERE status<>'complete'").bind(now).fetch_all(&mut **tx).await?{let interval=(r.get("start_at"),r.get("end_at"));match r.get::<String,_>("family").as_str(){"content"=>content_gaps.push(interval),"registration"=>registration_gaps.push(interval),_=>gaps.push(interval)}}
    Ok(CoverageData {
        content,
        registration,
        unknown_time,
        activity_start,
        activity_end,
        gaps,
        content_gaps,
        registration_gaps,
    })
}
async fn aggregate(
    tx: &mut Transaction<'_, Postgres>,
    windows: &[Window],
    ids: &[Uuid],
    full: bool,
    data: &CoverageData,
    scopes: &BTreeMap<String, Vec<Uuid>>,
    view: &str,
) -> Result<Vec<Bucket>> {
    let inputs: Vec<Value> = windows
        .iter()
        .map(|w| {
            let mut value = json!(w);
            value["school_ids"] = json!(scopes.get(&w.key).map(Vec::as_slice).unwrap_or(ids));
            value["full_site"] = json!(full && !scopes.contains_key(&w.key));
            value["content"] = json!(matches!(view, "overview" | "content" | "schools"));
            value["registration"] = json!(matches!(view, "overview" | "registration" | "schools"));
            value["activity"] = json!(matches!(view, "overview" | "activity" | "schools"));
            value["cohort"] = json!(view == "registration");
            value["detail"] = json!(view != "overview");
            value
        })
        .collect();
    let rows: Vec<Value> = sqlx::query_scalar(include_str!("metrics.sql"))
        .bind(json!(inputs))
        .fetch_all(&mut **tx)
        .await?;
    let mut map: BTreeMap<String, (Metrics, bool, bool)> = rows
        .into_iter()
        .map(|v| {
            Ok((
                v["key"].as_str().ok_or(Error::Unavailable)?.into(),
                (
                    serde_json::from_value(v["metrics"].clone()).map_err(|_| Error::Unavailable)?,
                    v["unknown_assignments"].as_bool().unwrap_or(true),
                    v["unknown_base"].as_bool().unwrap_or(true),
                ),
            ))
        })
        .collect::<Result<_>>()?;
    windows
        .iter()
        .map(|w| {
            let (mut m, unknown, unknown_base) = map.remove(&w.key).ok_or(Error::Unavailable)?;
            let coverage = data.at(
                w,
                full && !scopes.contains_key(&w.key),
                unknown,
                unknown_base,
            );
            // Preserve recorded counts during explicit data gaps, but mask derived
            // metrics and comparisons whose dependencies are incomplete.
            if !coverage.registration_assignment {
                m.registrations = None;
            }
            if !coverage.registration_base {
                m.cumulative_registrations = None;
            }
            if !coverage.registration || !coverage.registration_assignment || !coverage.content {
                m.new_participants = None;
            }
            if !coverage.activity {
                m.active_users = None;
                m.registered_participants = None;
            }
            if w.state == "future" {
                m = Metrics::default();
            }
            let days = ((w.actual_end_at - w.start_at).num_seconds() as f64 / 86400.)
                .ceil()
                .max(1.);
            m.daily_posts = m
                .posts
                .filter(|_| coverage.content)
                .map(|x| (x as f64 / days * 10.).round() / 10.);
            m.daily_comments = m
                .comments
                .filter(|_| coverage.content)
                .map(|x| (x as f64 / days * 10.).round() / 10.);
            m.ratios();
            if view == "overview" {
                m = Metrics {
                    posts: m.posts,
                    miniapp_posts: m.miniapp_posts,
                    web_posts: m.web_posts,
                    seed_posts: m.seed_posts,
                    unknown_posts: m.unknown_posts,
                    comments: m.comments,
                    registrations: m.registrations,
                    active_users: m.active_users,
                    ..Default::default()
                };
            }
            Ok(Bucket {
                window: w.clone(),
                metrics: m,
                coverage,
            })
        })
        .collect()
}
fn comparable_metrics(bucket: &Bucket) -> Metrics {
    let mut metrics = bucket.metrics.clone();
    if !bucket.coverage.content {
        metrics.posts = None;
        metrics.miniapp_posts = None;
        metrics.web_posts = None;
        metrics.seed_posts = None;
        metrics.unknown_posts = None;
        metrics.comments = None;
        metrics.post_authors = None;
        metrics.comment_authors = None;
        metrics.participants = None;
    }
    if !bucket.coverage.registration {
        metrics.registrations = None;
    }
    metrics
}
pub fn changes(m: &Metrics, previous: &Metrics) -> BTreeMap<String, Change> {
    [
        ("posts", m.posts, previous.posts),
        ("miniapp_posts", m.miniapp_posts, previous.miniapp_posts),
        ("web_posts", m.web_posts, previous.web_posts),
        ("seed_posts", m.seed_posts, previous.seed_posts),
        ("unknown_posts", m.unknown_posts, previous.unknown_posts),
        ("comments", m.comments, previous.comments),
        ("participants", m.participants, previous.participants),
        ("post_authors", m.post_authors, previous.post_authors),
        (
            "comment_authors",
            m.comment_authors,
            previous.comment_authors,
        ),
        ("registrations", m.registrations, previous.registrations),
        ("active_users", m.active_users, previous.active_users),
        (
            "registered_participants",
            m.registered_participants,
            previous.registered_participants,
        ),
        (
            "new_participants",
            m.new_participants,
            previous.new_participants,
        ),
    ]
    .into_iter()
    .map(|(k, a, b)| (k.into(), change(a, b)))
    .collect()
}
impl StatisticsStore for Repository {
    async fn validate_activity(&self, a: &Actor, v: &ActivityInput) -> Result<()> {
        if a.is_admin_browser() {
            return Err(Error::Forbidden);
        }
        v.validate()?;
        a.authorize(v.school_id, "member")?;
        crate::schools::south::adapter::postgres::Repository(self.0.clone())
            .require_active(v.school_id)
            .await?;
        if let Some(id) = v.post_id {
            let visible:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM posts WHERE id=$1 AND school_id=$2 AND visibility<>'deleted' AND (author_id=$3 OR (audit_status='approved' AND visibility='visible')))").bind(id).bind(v.school_id).bind(a.id).fetch_one(&self.0).await?;
            if !visible {
                return Err(Error::NotFound);
            }
        }
        let conflict:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM statistics_activity_events WHERE event_id=$1 AND (account_id<>$2 OR school_id IS DISTINCT FROM $3 OR kind<>$4 OR post_id IS DISTINCT FROM $5))").bind(v.event_id).bind(a.id).bind(v.school_id).bind(&v.kind).bind(v.post_id).fetch_one(&self.0).await?;
        if conflict {
            return Err(Error::Conflict);
        }
        Ok(())
    }

    async fn report(&self, a: &Actor, school: Option<Uuid>, q: Query) -> Result<Report> {
        let mut tx = self.0.begin().await?;
        sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ READ ONLY")
            .execute(&mut *tx)
            .await?;
        sqlx::query("SELECT set_config('statement_timeout',$1,true)")
            .bind(format!("{}ms", self.1.timeout_ms))
            .execute(&mut *tx)
            .await?;
        authorize(&mut tx, a, school, "statistics.school.read").await?;
        let now: DateTime<Utc> = sqlx::query_scalar("SELECT transaction_timestamp()")
            .fetch_one(&mut *tx)
            .await?;
        let view = q.view()?.to_string();
        require(
            view != "schools" || school.is_none(),
            "学校对比仅支持平台范围",
        )?;
        let mut plan = Plan::new(&q, now)?;
        if matches!(view.as_str(), "overview" | "schools") {
            plan.buckets.clear();
        }
        require(
            school.is_none() || q.school_ids.is_none() && q.school_status.is_none(),
            "单校范围不接受学校筛选",
        )?;
        require(
            q.school_status
                .as_deref()
                .is_none_or(|x| matches!(x, "normal" | "hidden")),
            "学校状态无效",
        )?;
        let requested = q
            .school_ids
            .as_deref()
            .map(|s| {
                s.split(',')
                    .map(|x| {
                        x.parse::<Uuid>()
                            .map_err(|_| Error::Invalid("学校筛选无效".into()))
                    })
                    .collect::<Result<Vec<_>>>()
            })
            .transpose()?;
        if let Some(ids) = &requested {
            require(!ids.is_empty() && ids.len() <= 100, "学校筛选最多 100 所")?;
        }
        let schools=sqlx::query("SELECT id,name,active FROM schools WHERE ($1::uuid IS NULL OR id=$1) AND ($2::uuid[] IS NULL OR id=ANY($2)) AND ($3::text IS NULL OR active=($3='normal')) ORDER BY id").bind(school).bind(&requested).bind(&q.school_status).fetch_all(&mut *tx).await?;
        let ids: Vec<Uuid> = schools.iter().map(|r| r.get("id")).collect();
        if let Some(s) = school
            && !ids.contains(&s)
        {
            return Err(Error::NotFound);
        }
        if let Some(wanted) = requested {
            require(
                wanted.iter().all(|x| ids.contains(x)),
                "所选学校不存在或不符合状态筛选",
            )?;
        }
        let full = school.is_none() && q.school_ids.is_none() && q.school_status.is_none();
        let data = coverage(&mut tx, now, self.1.retention_months).await?;
        let mut windows = vec![plan.window.clone()];
        windows.extend(plan.buckets.clone());
        for (kind, w) in &plan.comparisons {
            if let Some(w) = w {
                windows.push(w.clone());
                let mut bs = if plan.buckets.is_empty() {
                    Vec::new()
                } else {
                    buckets(w.start_at, w.end_at, &plan.granularity, now)?
                };
                for b in &mut bs {
                    b.key = format!("{kind}-{}", b.key);
                }
                windows.extend(bs.clone());
            }
        }
        let mut scopes = BTreeMap::new();
        if view == "schools" {
            for r in &schools {
                let id: Uuid = r.get("id");
                let mut ws = vec![plan.window.clone()];
                ws.extend(plan.comparisons.iter().filter_map(|(_, w)| w.clone()));
                for mut w in ws {
                    w.key = format!("school-{id}-{}", w.key);
                    scopes.insert(w.key.clone(), vec![id]);
                    windows.push(w);
                }
            }
        }
        let all = aggregate(&mut tx, &windows, &ids, full, &data, &scopes, &view).await?;
        let summary = all[0].metrics.clone();
        let comparable_summary = comparable_metrics(&all[0]);
        let cov = all[0].coverage.clone();
        let buckets = all[1..1 + plan.buckets.len()].to_vec();
        let comparisons = plan
            .comparisons
            .iter()
            .map(|(kind, w)| {
                let m = all
                    .iter()
                    .find(|b| b.window.key == *kind)
                    .map(comparable_metrics);
                let changes = changes(&comparable_summary, &m.clone().unwrap_or_default());
                Comparison {
                    kind: kind.clone(),
                    window: w.clone(),
                    reason: w.is_none().then(|| "无可比同期".into()),
                    metrics: m,
                    changes,
                    buckets: all
                        .iter()
                        .filter(|b| b.window.key.starts_with(&format!("{kind}-bucket-")))
                        .map(|bucket| {
                            let mut bucket = bucket.clone();
                            bucket.metrics = comparable_metrics(&bucket);
                            bucket
                        })
                        .collect(),
                }
            })
            .collect();
        let current_state: Value = if view == "content" {
            sqlx::query_scalar(include_str!("state.sql"))
                .bind(&ids)
                .bind(plan.window.start_at)
                .bind(plan.window.actual_end_at)
                .fetch_one(&mut *tx)
                .await?
        } else {
            json!({})
        };
        let groups: Value = if full && view == "registration" {
            sqlx::query_scalar("SELECT coalesce(jsonb_object_agg(school_assignment,n),'{}') FROM (SELECT school_assignment,count(*) AS n FROM statistics_registrations WHERE registered_at>=$1 AND registered_at<$2 AND time_quality='verified' GROUP BY school_assignment) g").bind(plan.window.start_at).bind(plan.window.actual_end_at).fetch_one(&mut *tx).await?
        } else {
            json!({})
        };
        let mut school_rows = Vec::new();
        if view == "schools" {
            for r in &schools {
                let id: Uuid = r.get("id");
                let current = all
                    .iter()
                    .find(|b| b.window.key == format!("school-{id}-summary"))
                    .ok_or(Error::Unavailable)?;
                let previous = all
                    .iter()
                    .find(|b| {
                        b.window.key
                            == format!(
                                "school-{id}-{}",
                                if plan.period == "year" {
                                    "year"
                                } else {
                                    "previous"
                                }
                            )
                    })
                    .map(comparable_metrics)
                    .unwrap_or_default();
                school_rows.push(SchoolRow {
                    id,
                    name: r.get("name"),
                    status: if r.get("active") { "normal" } else { "hidden" }.into(),
                    metrics: current.metrics.clone(),
                    coverage: current.coverage.clone(),
                    changes: changes(&comparable_metrics(current), &previous),
                });
            }
        }
        let report = Report {
            view,
            school_count: schools.len(),
            id: Uuid::new_v4(),
            school_id: school,
            school_ids: if full { Vec::new() } else { ids },
            school_names: if full {
                Vec::new()
            } else {
                schools.iter().map(|r| r.get("name")).collect()
            },
            full_site: full,
            timezone: "Asia/Shanghai".into(),
            period: plan.period,
            granularity: plan.granularity,
            window: plan.window,
            as_of: now,
            expires_at: now + Duration::seconds(self.1.snapshot_seconds),
            definition_version: 3,
            summary,
            coverage: cov,
            buckets,
            comparisons,
            current_state,
            registration_groups: groups,
            schools: school_rows,
        };
        tx.commit().await?;
        sqlx::query("INSERT INTO statistics_snapshots(id,owner_id,school_id,as_of,expires_at,definition_version,result) VALUES($1,$2,$3,$4,$5,3,$6)").bind(report.id).bind(a.id).bind(school).bind(now).bind(report.expires_at).bind(json!(report)).execute(&self.0).await?;
        Ok(report)
    }
    async fn snapshot(&self, a: &Actor, school: Option<Uuid>, id: Uuid) -> Result<Report> {
        let mut tx = self.0.begin().await?;
        authorize(&mut tx, a, school, "statistics.school.export").await?;
        let result:Option<Value>=sqlx::query_scalar("SELECT result FROM statistics_snapshots WHERE id=$1 AND owner_id=$2 AND school_id IS NOT DISTINCT FROM $3 AND expires_at>now()").bind(id).bind(a.id).bind(school).fetch_optional(&mut *tx).await?;
        tx.commit().await?;
        serde_json::from_value(result.ok_or(Error::Invalid("快照不可用，请刷新后重新导出".into()))?)
            .map_err(|_| Error::Unavailable)
    }
}
