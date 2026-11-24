use crate::{
    business::{Actor, Error, Result, require},
    statistics::{
        domain::*,
        south::{adapter::postgres::Repository, port::StatisticsStore},
    },
};
use uuid::Uuid;
#[derive(Clone)]
pub struct Service<R: StatisticsStore = Repository>(pub R, std::sync::Arc<tokio::sync::Semaphore>);
impl<R: StatisticsStore> Service<R> {
    pub fn new(store: R, concurrency: usize) -> Self {
        Self(
            store,
            std::sync::Arc::new(tokio::sync::Semaphore::new(concurrency)),
        )
    }
    pub async fn validate_activity(&self, a: &Actor, v: &ActivityInput) -> Result<()> {
        self.0.validate_activity(a, v).await
    }
    pub async fn report(&self, a: &Actor, s: Option<Uuid>, q: Query) -> Result<Report> {
        if !a.is_admin_browser() {
            return Err(Error::Forbidden);
        }
        if let Some(s) = s {
            a.permission(s, "statistics.school.export")?;
        } else {
            a.platform()?;
        }
        let _permit = self.1.try_acquire().map_err(|_| Error::Unavailable)?;
        tokio::time::timeout(std::time::Duration::from_secs(60), self.0.report(a, s, q))
            .await
            .map_err(|_| Error::Unavailable)?
    }
    pub async fn snapshot(&self, a: &Actor, s: Option<Uuid>, id: Uuid) -> Result<Report> {
        if !a.is_admin_browser() {
            return Err(Error::Forbidden);
        }
        if let Some(s) = s {
            a.permission(s, "statistics.school.read")?;
        } else {
            a.platform()?;
        }
        self.0.snapshot(a, s, id).await
    }
}
pub fn csv(report: &Report, kind: &str) -> Result<Vec<u8>> {
    require(matches!(kind, "periods" | "schools"), "导出类型无效")?;
    require(
        kind != "schools" || report.school_id.is_none(),
        "单校不支持学校对比导出",
    )?;
    require(
        if kind == "schools" {
            report.view == "schools"
        } else {
            !report.buckets.is_empty()
        },
        "请进入对应统计模块后导出",
    )?;
    let mut writer = csv::Writer::from_writer(Vec::new());
    let header = [
        "范围",
        "周期开始",
        "周期结束",
        "时区",
        "数据截至",
        "统计规则版本",
        "数据完整性",
        "新增帖子",
        "新增评论",
        "发帖人数",
        "评论人数",
        "内容参与人数",
        "新增注册",
        "期末累计注册",
        "新注册参与人数",
        "活跃注册用户数",
        "参与内容互动的注册用户数",
        "日均发帖",
        "日均评论",
        "评论/发帖量比",
        "新注册参与率(%)",
        "注册用户内容参与率(%)",
        "小程序投稿",
        "后台直接发布",
        "模拟投稿",
        "未知来源投稿",
    ];
    let metrics: &[usize] = match report.view.as_str() {
        "content" => &[0, 1, 2, 3, 4, 10, 11, 12, 15, 16, 17, 18],
        "registration" => &[5, 6, 7, 13],
        "activity" => &[8, 9, 14],
        "schools" => &[0, 1, 4, 5, 6, 8, 15, 16, 17, 18],
        _ => &[],
    };
    let include = |index: usize| index < 7 || metrics.contains(&(index - 7));
    writer
        .write_record(
            header
                .iter()
                .enumerate()
                .filter(|(index, _)| include(*index))
                .map(|(_, value)| value),
        )
        .map_err(|_| Error::Unavailable)?;
    let safe = |s: &str| {
        if s.trim_start()
            .starts_with(['=', '+', '-', '@', '\t', '\r', '\n'])
        {
            format!("'{s}")
        } else {
            s.into()
        }
    };
    let mut row = |name: &str, w: &Window, m: &Metrics, c: &Coverage| -> Result<()> {
        let mut values = vec![
            safe(name),
            w.start_at
                .with_timezone(&chrono::FixedOffset::east_opt(8 * 3600).unwrap())
                .to_rfc3339(),
            w.actual_end_at
                .with_timezone(&chrono::FixedOffset::east_opt(8 * 3600).unwrap())
                .to_rfc3339(),
            report.timezone.clone(),
            report
                .as_of
                .with_timezone(&chrono::FixedOffset::east_opt(8 * 3600).unwrap())
                .to_rfc3339(),
            report.definition_version.to_string(),
            if w.state == "future" {
                "尚未发生".into()
            } else if c.reasons.is_empty() {
                match w.state.as_str() {
                    "complete" => "已结束",
                    "partial" => "部分周期",
                    "in_progress" => "统计中",
                    state => state,
                }
                .into()
            } else {
                c.reasons.join("；")
            },
        ];
        for n in [
            m.posts,
            m.comments,
            m.post_authors,
            m.comment_authors,
            m.participants,
            m.registrations,
            m.cumulative_registrations,
            m.new_participants,
            m.active_users,
            m.registered_participants,
        ] {
            values.push(n.map(|x| x.to_string()).unwrap_or_default());
        }
        for n in [
            m.daily_posts,
            m.daily_comments,
            m.comment_post_ratio,
            m.new_participation_rate,
            m.participation_rate,
        ] {
            values.push(n.map(|x| x.to_string()).unwrap_or_default());
        }
        for count in [m.miniapp_posts, m.web_posts, m.seed_posts, m.unknown_posts] {
            values.push(count.map(|value| value.to_string()).unwrap_or_default());
        }
        writer
            .write_record(
                values
                    .into_iter()
                    .enumerate()
                    .filter(|(index, _)| include(*index))
                    .map(|(_, value)| value),
            )
            .map_err(|_| Error::Unavailable)
    };
    if kind == "schools" {
        for school in &report.schools {
            row(
                &school.name,
                &report.window,
                &school.metrics,
                &school.coverage,
            )?;
        }
    } else {
        let name = if report.full_site {
            "全站".into()
        } else {
            report.school_names.join("、")
        };
        for b in &report.buckets {
            row(&name, &b.window, &b.metrics, &b.coverage)?;
        }
        row(
            &format!("{name} 合计（人数去重，累计取期末）"),
            &report.window,
            &report.summary,
            &report.coverage,
        )?;
    }
    let mut bytes = vec![0xef, 0xbb, 0xbf];
    bytes.extend(writer.into_inner().map_err(|_| Error::Unavailable)?);
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn csv_uses_shanghai_time_and_escapes_formula_text() {
        let doc: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../api-contract/fixtures/platform-statistics.json"
        ))
        .unwrap();
        let mut attributes = doc["data"]["attributes"].clone();
        attributes["id"] = doc["data"]["id"].clone();
        let mut report: Report = serde_json::from_value(attributes).unwrap();
        let rows: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../api-contract/fixtures/school-statistics-rows.json"
        ))
        .unwrap();
        let mut row = rows["data"][0]["attributes"].clone();
        row["id"] = rows["data"][0]["id"].clone();
        report.schools.push(serde_json::from_value(row).unwrap());
        report.schools[0].name = "=1+1".into();
        let bytes = csv(&report, "schools").unwrap();
        assert!(bytes.starts_with(&[0xef, 0xbb, 0xbf]));
        let text = String::from_utf8(bytes).unwrap();
        assert!(text.contains("'=1+1"));
        assert!(text.contains("2026-11-16T00:00:00+08:00"));
        assert!(text.contains("活跃注册用户数"));
        assert!(!text.contains("新注册参与率"));
        assert!(!text.contains("日均发帖"));
        assert!(text.contains("统计规则版本,数据完整性"));

        report.view = "activity".into();
        report.coverage.reasons.clear();
        report.buckets = ["complete", "in_progress", "partial", "future"]
            .into_iter()
            .map(|state| Bucket {
                window: Window {
                    state: state.into(),
                    ..report.window.clone()
                },
                metrics: report.summary.clone(),
                coverage: report.coverage.clone(),
            })
            .collect();
        let bytes = csv(&report, "periods").unwrap();
        let mut reader = csv::Reader::from_reader(&bytes[3..]);
        let header = reader.headers().unwrap().clone();
        assert!(header.iter().any(|value| value == "活跃注册用户数"));
        assert!(
            header
                .iter()
                .any(|value| value == "参与内容互动的注册用户数")
        );
        assert!(header.iter().any(|value| value == "注册用户内容参与率(%)"));
        let rows: Vec<_> = reader.records().map(|row| row.unwrap()).collect();
        for (row, status) in rows
            .iter()
            .zip(["已结束", "统计中", "部分周期", "尚未发生"])
        {
            assert_eq!(&row[6], status);
        }
    }
}
