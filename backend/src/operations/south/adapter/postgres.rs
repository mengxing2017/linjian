use crate::{
    business::{Actor, Error, Result, require},
    operations::south::port::{Command, Store},
    schools::{
        domain::{ImageRef, ResourceAction},
        south::adapter::postgres::preview,
    },
};
use serde_json::{Value, json};
use sqlx::{PgPool, Postgres, Row, Transaction};
use uuid::Uuid;
#[derive(Clone)]
pub struct Repository(pub PgPool);
fn school(s: Option<Uuid>) -> Result<Uuid> {
    s.ok_or(Error::Forbidden)
}
async fn auth(
    tx: &mut Transaction<'_, Postgres>,
    a: &Actor,
    s: Option<Uuid>,
    role: &str,
) -> Result<()> {
    if role != "member" {
        crate::miniapp_users::south::adapter::postgres::authorize(a, s, role == "admin")?;
    } else {
        a.authorize(school(s)?, "member")?;
    }
    preview::authorize(tx, a, s, role).await?;
    if let Some(s) = s {
        crate::miniapp_users::south::adapter::postgres::guard_write(tx, a, s).await?;
    }
    Ok(())
}
async fn audit(
    tx: &mut Transaction<'_, Postgres>,
    a: &Actor,
    s: Option<Uuid>,
    action: &str,
    id: Uuid,
    detail: Value,
) -> Result<()> {
    crate::audit::record(tx, a, s.unwrap_or(Uuid::nil()), action, id, detail).await
}
fn needle(s: Option<&str>) -> String {
    s.unwrap_or("")
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}
const RESOURCE_VIEW: &str = "SELECT to_jsonb(r)||jsonb_build_object('school_name',coalesce(s.name,'平台默认'),'needs_repair',(char_length(btrim(r.title)) NOT BETWEEN 1 AND 100 OR (r.kind='qrcode' AND char_length(r.content)>255) OR NOT EXISTS(SELECT 1 FROM operational_media m WHERE m.id::text=r.image_ref->>'id' AND m.school_id IS NOT DISTINCT FROM r.school_id)),'image_url',CASE WHEN r.image_ref IS NULL THEN NULL ELSE CASE WHEN r.school_id IS NULL THEN '/api/v1/platform/resource-artwork/'||(r.image_ref->>'id') ELSE '/api/v1/schools/'||r.school_id||'/operations-images/'||r.id END END) FROM school_resources r LEFT JOIN schools s ON s.id=r.school_id";
async fn resource_get(
    tx: &mut Transaction<'_, Postgres>,
    s: Option<Uuid>,
    kind: &str,
    id: Uuid,
) -> Result<Value> {
    sqlx::query_scalar(&format!("{RESOURCE_VIEW} WHERE r.school_id IS NOT DISTINCT FROM $1 AND r.kind=$2 AND r.id=$3 AND r.deleted_at IS NULL")).bind(s).bind(kind).bind(id).fetch_optional(&mut **tx).await?.ok_or(Error::NotFound)
}
async fn check_image(tx: &mut Transaction<'_, Postgres>, s: Option<Uuid>, id: Uuid) -> Result<()> {
    sqlx::query("SELECT id FROM operational_media WHERE id=$1 AND school_id IS NOT DISTINCT FROM $2 FOR SHARE").bind(id).bind(s).fetch_optional(&mut **tx).await?.ok_or(Error::Forbidden)?;
    Ok(())
}
async fn check_action(
    tx: &mut Transaction<'_, Postgres>,
    s: Option<Uuid>,
    action: &ResourceAction,
) -> Result<()> {
    if let ResourceAction::InternalPage { page, object_id } = action
        && page == "post"
    {
        let s = s.ok_or_else(|| Error::Invalid("平台广告不能引用学校帖子".into()))?;
        sqlx::query("SELECT id FROM posts WHERE id=$1 AND school_id=$2 AND audit_status='approved' AND visibility='visible' FOR SHARE").bind(object_id).bind(s).fetch_optional(&mut **tx).await?.ok_or(Error::NotFound)?;
    }
    Ok(())
}
/// Shared selection for both legacy resources and operations display. GET never counts views.
pub async fn effective_resources(pool: &PgPool, s: Uuid) -> Result<Vec<Value>> {
    let mut rows:Vec<Value>=sqlx::query_scalar("SELECT to_jsonb(r)||jsonb_build_object('image_url',CASE WHEN r.image_ref->>'kind'='asset' THEN '/api/v1/schools/'||($1::uuid)::text||'/resource-media/'||r.id ELSE NULL END,'action_available',CASE WHEN r.action->>'kind'='internal_page' AND r.action->>'page'='post' THEN EXISTS(SELECT 1 FROM posts p WHERE p.id::text=r.action->>'object_id' AND p.school_id=$1 AND p.visibility='visible' AND p.audit_status='approved') ELSE true END) FROM school_resources r WHERE r.deleted_at IS NULL AND r.kind IN ('banner','qrcode','service','legal') AND (r.kind NOT IN ('banner','qrcode') OR (char_length(btrim(r.title)) BETWEEN 1 AND 100 AND (r.kind<>'qrcode' OR char_length(r.content)<=255) AND EXISTS(SELECT 1 FROM operational_media m WHERE m.id::text=r.image_ref->>'id' AND m.school_id IS NOT DISTINCT FROM r.school_id))) AND r.active AND (r.starts_at IS NULL OR r.starts_at<=now()) AND (r.ends_at IS NULL OR r.ends_at>now()) AND (r.school_id=$1 OR (r.school_id IS NULL AND (r.kind='banner' OR NOT EXISTS(SELECT 1 FROM school_resources c WHERE c.school_id=$1 AND c.kind=r.kind AND c.deleted_at IS NULL AND c.active AND (c.kind NOT IN ('banner','qrcode') OR (char_length(btrim(c.title)) BETWEEN 1 AND 100 AND (c.kind<>'qrcode' OR char_length(c.content)<=255) AND EXISTS(SELECT 1 FROM operational_media cm WHERE cm.id::text=c.image_ref->>'id' AND cm.school_id IS NOT DISTINCT FROM c.school_id))) AND (c.starts_at IS NULL OR c.starts_at<=now()) AND (c.ends_at IS NULL OR c.ends_at>now()))))) ORDER BY (r.school_id IS NULL),r.sort_order DESC,r.id ASC").bind(s).fetch_all(pool).await?;
    let mut qr = false;
    let mut banners = 0;
    rows.retain(|r| {
        if r["kind"] == "qrcode" {
            if qr {
                return false;
            }
            qr = true;
        }
        if r["kind"] == "banner" {
            banners += 1;
            return banners <= 5;
        }
        true
    });
    Ok(rows)
}
async fn customer(tx: &mut Transaction<'_, Postgres>) -> Result<Value> {
    Ok(sqlx::query_scalar("SELECT to_jsonb(c)-'singleton'||jsonb_build_object('id','current') FROM customer_config c WHERE singleton").fetch_optional(&mut **tx).await?.unwrap_or(json!({"id":"current","revision":0,"enabled":false,"business_asset_id":null,"join_asset_id":null,"service_asset_id":null,"fallback_asset_id":null,"note":"添加客服微信后，请备注来意"})))
}
const FEEDBACK_VIEW: &str = "SELECT (to_jsonb(c)-'submitted_ip'-'submission_key')||jsonb_build_object('submitted_ip',host(c.submitted_ip),'reporter_name',coalesce(nullif(u.nickname,''),nullif(a.display_name,''),'未知用户'),'school_name',s.name) FROM cases c JOIN schools s ON s.id=c.school_id LEFT JOIN accounts a ON a.id=c.account_id LEFT JOIN miniapp_users u ON u.account_id=c.account_id";
async fn feedback_get(tx: &mut Transaction<'_, Postgres>, s: Uuid, id: Uuid) -> Result<Value> {
    let mut v:Value=sqlx::query_scalar(&format!("{FEEDBACK_VIEW} WHERE c.school_id=$1 AND c.id=$2 AND c.kind='feedback' AND c.deleted_at IS NULL")).bind(s).bind(id).fetch_optional(&mut **tx).await?.ok_or(Error::NotFound)?;
    if let Some(images) = v["images"].as_array_mut() {
        for image in images {
            if let Some(media) = image
                .as_str()
                .and_then(|u| u.rsplit('/').next())
                .and_then(|u| u.parse::<Uuid>().ok())
            {
                *image = json!(format!("/api/v1/schools/{s}/feedback/{id}/images/{media}"));
            }
        }
    }
    Ok(v)
}
impl Store for Repository {
    async fn execute(&self, a: &Actor, s: Option<Uuid>, c: Command) -> Result<Value> {
        let mut tx = self.0.begin().await?;
        let result = match c {
            Command::List { kind, q } => {
                auth(&mut tx, a, s, "operator").await?;
                let scope = if q.defaults { None } else { s };
                let (limit, offset) = q.page().fetch_bounds()?;
                let mut rows:Vec<Value>=sqlx::query_scalar(&format!("{RESOURCE_VIEW} WHERE r.school_id IS NOT DISTINCT FROM $1 AND r.kind=$2 AND r.deleted_at IS NULL AND r.title ILIKE '%'||$3||'%' AND ($4='' OR r.active=($4='normal')) AND ($5='' OR r.popup_strategy=$5) AND ($6='' OR r.action->>'kind'=$6) AND ($7::timestamptz IS NULL OR r.starts_at>=$7) AND ($8::timestamptz IS NULL OR r.ends_at<=$8) ORDER BY r.sort_order DESC,r.id ASC LIMIT $9 OFFSET $10")).bind(scope).bind(kind).bind(needle(q.search.as_deref())).bind(q.status.unwrap_or_default()).bind(q.strategy.unwrap_or_default()).bind(q.action.unwrap_or_default()).bind(q.starts_at).bind(q.ends_at).bind(limit).bind(offset).fetch_all(&mut *tx).await?;
                for row in &mut rows {
                    if row["image_ref"].is_object()
                        && let Some(s) = s
                    {
                        row["image_url"] = json!(format!(
                            "/api/v1/schools/{s}/operations-images/{}",
                            row["id"].as_str().unwrap_or("")
                        ));
                    }
                }
                json!(rows)
            }
            Command::Get { kind, id } => {
                auth(&mut tx, a, s, "operator").await?;
                resource_get(&mut tx, s, &kind, id).await?
            }
            Command::Save { kind, id, input: v } => {
                auth(&mut tx, a, s, "admin").await?;
                sqlx::query("SELECT pg_advisory_xact_lock(78234610244)")
                    .execute(&mut *tx)
                    .await?;
                let prior = if let Some(id) = id {
                    Some(resource_get(&mut tx, s, &kind, id).await?)
                } else {
                    None
                };
                if let Some(p) = &prior {
                    if p["revision"].as_i64() != Some(v.revision) {
                        return Err(Error::Conflict);
                    }
                } else {
                    require(v.revision == 0, "新增不能指定旧版本")?;
                }
                let ImageRef::Asset { id: asset } = v.image_ref;
                check_image(&mut tx, s, asset).await?;
                check_action(&mut tx, s, &v.action).await?;
                if !matches!(
                    v.action,
                    ResourceAction::None {} | ResourceAction::InternalPage { .. }
                ) {
                    require(
                        prior
                            .as_ref()
                            .is_some_and(|p| p["action"] == json!(v.action)),
                        "新广告请选择本站页面或不跳转",
                    )?;
                }
                let id = id.unwrap_or_else(Uuid::now_v7);
                if prior.is_some() {
                    let n=sqlx::query("UPDATE school_resources SET title=$3,content=$4,image_ref=$5,action=$6,active=$7,sort_order=$8,starts_at=$9,ends_at=$10,popup_strategy=$11,popup_interval_days=$12,revision=revision+1 WHERE id=$1 AND school_id IS NOT DISTINCT FROM $2 AND revision=$13 AND deleted_at IS NULL").bind(id).bind(s).bind(&v.title).bind(&v.content).bind(json!({"kind":"asset","id":asset})).bind(json!(v.action)).bind(v.active).bind(v.sort_order).bind(v.starts_at).bind(v.ends_at).bind(&v.popup_strategy).bind(v.popup_interval_days).bind(v.revision).execute(&mut *tx).await?.rows_affected();
                    if n != 1 {
                        return Err(Error::Conflict);
                    }
                } else {
                    sqlx::query("INSERT INTO school_resources(id,school_id,kind,title,content,image_ref,action,active,sort_order,starts_at,ends_at,popup_strategy,popup_interval_days) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13)").bind(id).bind(s).bind(&kind).bind(&v.title).bind(&v.content).bind(json!({"kind":"asset","id":asset})).bind(json!(v.action)).bind(v.active).bind(v.sort_order).bind(v.starts_at).bind(v.ends_at).bind(&v.popup_strategy).bind(v.popup_interval_days).execute(&mut *tx).await?;
                }
                let current = resource_get(&mut tx, s, &kind, id).await?;
                sqlx::query("INSERT INTO resource_publications(id,resource_id,school_id,actor_id,snapshot) VALUES($1,$2,$3,$4,$5)").bind(Uuid::now_v7()).bind(id).bind(s).bind(a.id).bind(&current).execute(&mut *tx).await?;
                audit(
                    &mut tx,
                    a,
                    s,
                    "resource.operations.save",
                    id,
                    json!({"kind":kind,"revision":current["revision"]}),
                )
                .await?;
                current
            }
            Command::Delete { kind, id, revision } => {
                auth(&mut tx, a, s, "admin").await?;
                let n=sqlx::query("UPDATE school_resources SET deleted_at=now(),revision=revision+1 WHERE id=$1 AND school_id IS NOT DISTINCT FROM $2 AND kind=$3 AND revision=$4 AND deleted_at IS NULL").bind(id).bind(s).bind(kind).bind(revision).execute(&mut *tx).await?.rows_affected();
                if n != 1 {
                    return Err(Error::Conflict);
                }
                audit(
                    &mut tx,
                    a,
                    s,
                    "resource.operations.delete",
                    id,
                    json!({"revision":revision}),
                )
                .await?;
                Value::Null
            }
            Command::Customer => {
                auth(&mut tx, a, None, "admin").await?;
                customer(&mut tx).await?
            }
            Command::SaveCustomer(v) => {
                auth(&mut tx, a, None, "admin").await?;
                sqlx::query("SELECT pg_advisory_xact_lock(78234610244)")
                    .execute(&mut *tx)
                    .await?;
                let old = customer(&mut tx).await?;
                if old["revision"].as_i64() != Some(v.revision) {
                    return Err(Error::Conflict);
                }
                for id in [
                    v.business_asset_id,
                    v.join_asset_id,
                    v.service_asset_id,
                    v.fallback_asset_id,
                ]
                .into_iter()
                .flatten()
                {
                    check_image(&mut tx, None, id).await?;
                }
                sqlx::query("INSERT INTO customer_config(singleton,enabled,business_asset_id,join_asset_id,service_asset_id,fallback_asset_id,note,revision,updated_by) VALUES(true,$1,$2,$3,$4,$5,$6,1,$7) ON CONFLICT(singleton) DO UPDATE SET enabled=$1,business_asset_id=$2,join_asset_id=$3,service_asset_id=$4,fallback_asset_id=$5,note=$6,revision=customer_config.revision+1,updated_at=now(),updated_by=$7").bind(v.enabled).bind(v.business_asset_id).bind(v.join_asset_id).bind(v.service_asset_id).bind(v.fallback_asset_id).bind(v.note).bind(a.id).execute(&mut *tx).await?;
                let current = customer(&mut tx).await?;
                audit(
                    &mut tx,
                    a,
                    None,
                    "resource.customer.save",
                    Uuid::nil(),
                    json!({"revision":current["revision"],"enabled":current["enabled"]}),
                )
                .await?;
                current
            }
            Command::FeedbackList(q) => {
                auth(&mut tx, a, s, "operator").await?;
                let s = school(s)?;
                let (limit, offset) = q.page().fetch_bounds()?;
                let ids:Vec<Uuid>=sqlx::query_scalar("SELECT c.id FROM cases c LEFT JOIN accounts a ON a.id=c.account_id LEFT JOIN miniapp_users u ON u.account_id=c.account_id WHERE c.school_id=$1 AND c.kind='feedback' AND c.deleted_at IS NULL AND c.content ILIKE '%'||$2||'%' AND c.contact ILIKE '%'||$3||'%' AND ($4='' OR c.feedback_type=$4 OR ($4='legacy' AND c.feedback_type IS NULL)) AND ($5='' OR c.state=$5) AND (coalesce(u.nickname,a.display_name,'') ILIKE '%'||$6||'%' OR c.account_id::text=$6) AND ($7::timestamptz IS NULL OR c.created_at>=$7) AND ($8::timestamptz IS NULL OR c.created_at<$8) ORDER BY c.created_at DESC,c.id DESC LIMIT $9 OFFSET $10").bind(s).bind(needle(q.search.as_deref())).bind(needle(q.contact.as_deref())).bind(q.feedback_type.unwrap_or_default()).bind(q.status.unwrap_or_default()).bind(needle(q.user.as_deref())).bind(q.starts_at).bind(q.ends_at).bind(limit).bind(offset).fetch_all(&mut *tx).await?;
                let mut rows = Vec::new();
                for id in ids {
                    rows.push(feedback_get(&mut tx, s, id).await?);
                }
                json!(rows)
            }
            Command::FeedbackGet(id) => {
                auth(&mut tx, a, s, "operator").await?;
                feedback_get(&mut tx, school(s)?, id).await?
            }
            Command::FeedbackDelete { id, revision } => {
                auth(&mut tx, a, s, "admin").await?;
                let n=sqlx::query("UPDATE cases SET deleted_at=now(),deleted_by=$4,updated_at=now(),revision=revision+1 WHERE school_id=$1 AND id=$2 AND kind='feedback' AND revision=$3 AND deleted_at IS NULL").bind(school(s)?).bind(id).bind(revision).bind(a.id).execute(&mut *tx).await?.rows_affected();
                if n != 1 {
                    return Err(Error::Conflict);
                }
                audit(
                    &mut tx,
                    a,
                    s,
                    "case.feedback.delete",
                    id,
                    json!({"revision":revision}),
                )
                .await?;
                Value::Null
            }
            Command::Submit { input: v, ip } => {
                auth(&mut tx, a, s, "member").await?;
                let s = school(s)?;
                // Per-account transaction lock makes retries deterministic, including mismatching payloads.
                sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,73120))")
                    .bind(a.id.to_string())
                    .execute(&mut *tx)
                    .await?;
                let prior:Option<Value>=sqlx::query_scalar("SELECT to_jsonb(c) FROM cases c WHERE account_id=$1 AND school_id=$2 AND submission_key=$3").bind(a.id).bind(s).bind(v.submission_key).fetch_optional(&mut *tx).await?;
                if let Some(p) = prior {
                    require(
                        p["content"] == v.content
                            && p["contact"] == v.contact
                            && p["feedback_type"] == v.feedback_type
                            && p["images"] == json!(v.images),
                        "提交标识已用于其他反馈",
                    )?;
                    require(p["deleted_at"].is_null(), "反馈已删除")?;
                    json!({"id":p["id"],"submitted":true})
                } else {
                    let count:i64=sqlx::query_scalar("SELECT count(*) FROM cases WHERE account_id=$1 AND created_at>now()-interval '1 minute'").bind(a.id).fetch_one(&mut *tx).await?;
                    require(count < 10, "提交过于频繁，请稍后再试")?;
                    for image in &v.images {
                        let prefix = format!("/api/v1/schools/{s}/media/");
                        let id = image
                            .strip_prefix(&prefix)
                            .and_then(|i| i.parse::<Uuid>().ok())
                            .ok_or_else(|| Error::Invalid("图片必须来自本校上传".into()))?;
                        sqlx::query("SELECT id FROM media_assets WHERE id=$1 AND school_id=$2 AND owner_id=$3 FOR SHARE").bind(id).bind(s).bind(a.id).fetch_optional(&mut *tx).await?.ok_or(Error::Forbidden)?;
                    }
                    let id = Uuid::now_v7();
                    sqlx::query("INSERT INTO cases(id,school_id,account_id,kind,feedback_type,content,contact,images,submission_key,submitted_ip) VALUES($1,$2,$3,'feedback',$4,$5,$6,$7,$8,$9::text::inet)").bind(id).bind(s).bind(a.id).bind(v.feedback_type).bind(v.content).bind(v.contact).bind(json!(v.images)).bind(v.submission_key).bind(ip).execute(&mut *tx).await?;
                    json!({"id":id,"submitted":true})
                }
            }
            Command::Display => {
                auth(&mut tx, a, s, "member").await?;
                let s = school(s)?;
                let rows = effective_resources(&self.0, s).await?;
                let cfg = customer(&mut tx).await?;
                let enabled = cfg["enabled"] == true;
                let mut display = json!({"enabled":enabled});
                if enabled {
                    display["note"] = cfg["note"].clone();
                    for (kind, key) in [
                        ("business", "business_asset_id"),
                        ("join", "join_asset_id"),
                        ("service", "service_asset_id"),
                    ] {
                        let asset = cfg[key].as_str().or(cfg["fallback_asset_id"].as_str());
                        display[kind] = json!(asset.map(|_| format!(
                            "/api/v1/schools/{s}/customer-images/{kind}?revision={}",
                            cfg["revision"]
                        )));
                    }
                }
                json!({"id":"current","server_time":chrono::Utc::now(),"qrcode":rows.iter().find(|r|r["kind"]=="qrcode"),"banners":rows.iter().filter(|r|r["kind"]=="banner").collect::<Vec<_>>(),"customer":display})
            }
            Command::Event(v) => {
                auth(&mut tx, a, s, "member").await?;
                let s = school(s)?;
                // Serialize a user's metric events to make both throttling and deduplication race-safe.
                sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,73121))")
                    .bind(a.id.to_string())
                    .execute(&mut *tx)
                    .await?;
                let visible = effective_resources(&self.0, s).await?;
                visible
                    .iter()
                    .find(|r| r["id"] == v.resource_id.to_string() && r["kind"] == "banner")
                    .ok_or(Error::NotFound)?;
                sqlx::query("SELECT id FROM school_resources WHERE id=$1 AND deleted_at IS NULL AND active AND (starts_at IS NULL OR starts_at<=now()) AND (ends_at IS NULL OR ends_at>now()) FOR SHARE").bind(v.resource_id).fetch_optional(&mut *tx).await?.ok_or(Error::NotFound)?;
                let count:i64=sqlx::query_scalar("SELECT count(*) FROM banner_metric_events WHERE account_id=$1 AND created_at>now()-interval '1 minute'").bind(a.id).fetch_one(&mut *tx).await?;
                require(count < 120, "统计请求过于频繁")?;
                let n=sqlx::query("INSERT INTO banner_metric_events(event_id,resource_id,school_id,account_id,event_type,display_cycle,occurred_at) VALUES($1,$2,$3,$4,$5,$6,$7) ON CONFLICT DO NOTHING").bind(v.event_id).bind(v.resource_id).bind(s).bind(a.id).bind(&v.event_type).bind(v.display_cycle).bind(v.occurred_at).execute(&mut *tx).await?.rows_affected();
                if n == 1 {
                    let sql = if v.event_type == "view" {
                        "UPDATE school_resources SET views=views+1 WHERE id=$1"
                    } else {
                        "UPDATE school_resources SET clicks=clicks+1 WHERE id=$1"
                    };
                    sqlx::query(sql)
                        .bind(v.resource_id)
                        .execute(&mut *tx)
                        .await?;
                }
                json!({"id":v.event_id,"accepted":true})
            }
            Command::ResourceMedia(id) => {
                auth(&mut tx, a, s, "operator").await?;
                let row=sqlx::query("SELECT m.mime_type,m.content FROM school_resources r JOIN operational_media m ON m.id::text=r.image_ref->>'id' AND m.school_id IS NOT DISTINCT FROM r.school_id WHERE r.id=$1 AND r.deleted_at IS NULL AND (r.school_id=$2 OR r.school_id IS NULL)").bind(id).bind(school(s)?).fetch_optional(&mut *tx).await?.ok_or(Error::NotFound)?;
                json!({"mime":row.get::<String,_>("mime_type"),"bytes":row.get::<Vec<u8>,_>("content")})
            }
            Command::CustomerMedia(kind) => {
                auth(&mut tx, a, s, "member").await?;
                let key = match kind.as_str() {
                    "business" => "business_asset_id",
                    "join" => "join_asset_id",
                    "service" => "service_asset_id",
                    _ => return Err(Error::NotFound),
                };
                let cfg = customer(&mut tx).await?;
                if cfg["enabled"] != true {
                    return Err(Error::NotFound);
                }
                let asset = cfg[key]
                    .as_str()
                    .or(cfg["fallback_asset_id"].as_str())
                    .and_then(|s| s.parse::<Uuid>().ok())
                    .ok_or(Error::NotFound)?;
                let row=sqlx::query("SELECT mime_type,content FROM operational_media WHERE id=$1 AND school_id IS NULL").bind(asset).fetch_optional(&mut *tx).await?.ok_or(Error::NotFound)?;
                json!({"mime":row.get::<String,_>("mime_type"),"bytes":row.get::<Vec<u8>,_>("content")})
            }
            Command::FeedbackMedia { case, media } => {
                auth(&mut tx, a, s, "operator").await?;
                let s = school(s)?;
                let path = format!("/api/v1/schools/{s}/media/{media}");
                let row=sqlx::query("SELECT m.mime_type,m.content FROM media_assets m JOIN cases c ON c.school_id=m.school_id WHERE c.id=$1 AND c.school_id=$2 AND c.kind='feedback' AND c.deleted_at IS NULL AND c.images @> $3 AND m.id=$4").bind(case).bind(s).bind(json!([path])).bind(media).fetch_optional(&mut *tx).await?.ok_or(Error::NotFound)?;
                json!({"mime":row.get::<String,_>("mime_type"),"bytes":row.get::<Vec<u8>,_>("content")})
            }
        };
        tx.commit().await?;
        Ok(result)
    }
}
