//! Offline snapshot tooling: source is repeatable-read/read-only; target must be
//! an empty Linjian database. Sessions, machine credentials and sending data never move.
use crate::business::{Error, Result, require};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::{PgPool, Row};
use std::collections::BTreeMap;

pub const TABLES: &[&str] = &[
    "schools",
    "wecom_departments",
    "wecom_members",
    "accounts",
    "school_memberships",
    "posts",
    "comments",
    "post_reactions",
    "comment_reactions",
    "follows",
    "notifications",
    "moderation_events",
    "cases",
    "school_resources",
    "media_assets",
    "operational_media",
    "resource_publications",
    "audit_events",
    "legacy_id_map",
    "legacy_aliases",
];
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Snapshot {
    pub schema_version: u32,
    pub tables: BTreeMap<String, Vec<Value>>,
    pub digest: String,
}
fn normalize(rows: &mut [Value]) {
    rows.sort_by_cached_key(Value::to_string);
}
fn digest(tables: &BTreeMap<String, Vec<Value>>) -> String {
    crate::business::hash(&serde_json::to_string(tables).expect("serializable JSON"))
}
fn filter(table: &str) -> &'static str {
    match table {
        "audit_events" => {
            " WHERE action LIKE 'post.%' OR action LIKE 'comment.%' OR action LIKE 'case.%' OR action LIKE 'school.%' OR action LIKE 'account.%' OR action LIKE 'resource.%' OR action LIKE 'wecom.binding%'"
        }
        "legacy_id_map" => {
            " WHERE source_table IN ('accounts','schools','posts','comments','cases','school_resources')"
        }
        "legacy_aliases" => {
            " WHERE kind IN ('accounts','schools','posts','comments','cases','school_resources')"
        }
        _ => "",
    }
}
pub async fn export(pool: &PgPool) -> Result<Snapshot> {
    let mut tx = pool.begin().await?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ READ ONLY")
        .execute(&mut *tx)
        .await?;
    let mut tables = BTreeMap::new();
    for table in TABLES {
        let query = format!("SELECT to_jsonb(t) FROM {table} t{}", filter(table));
        let mut rows: Vec<Value> = sqlx::query_scalar(&query).fetch_all(&mut *tx).await?;
        // Identity mappings survive, but a copied directory snapshot is not fresh verification.
        if *table == "wecom_members" {
            for row in &mut rows {
                row["verification"] = json!("unverified");
                row["checked_at"] = Value::Null;
                row["snapshot_id"] = Value::Null;
            }
        }
        normalize(&mut rows);
        tables.insert((*table).into(), rows);
    }
    tx.commit().await?;
    Ok(Snapshot {
        schema_version: 1,
        digest: digest(&tables),
        tables,
    })
}
pub async fn import(pool: &PgPool, snapshot: &Snapshot, same_wechat_app: bool) -> Result<Value> {
    require(
        snapshot.schema_version == 1 && snapshot.digest == digest(&snapshot.tables),
        "快照版本或摘要不匹配",
    )?;
    require(
        snapshot.tables.len() == TABLES.len()
            && TABLES.iter().all(|t| snapshot.tables.contains_key(*t)),
        "快照表清单不完整",
    )?;
    require(
        same_wechat_app
            || !snapshot.tables["accounts"]
                .iter()
                .any(|v| !v["wx_openid"].is_null()),
        "含微信身份的快照必须确认迁移同一小程序 AppID；不同 AppID 需先完成显式身份映射",
    )?;
    let mut tx = pool.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock(78234610245)")
        .execute(&mut *tx)
        .await?;
    let target: Option<String> =
        sqlx::query_scalar("SELECT application FROM application_metadata WHERE singleton")
            .fetch_optional(&mut *tx)
            .await?;
    require(
        target.as_deref() == Some("linjian"),
        "目标必须是 Linjian 独立数据库",
    )?;
    for table in TABLES {
        sqlx::query(&format!("LOCK TABLE {table} IN EXCLUSIVE MODE"))
            .execute(&mut *tx)
            .await?;
        let exists: bool = sqlx::query_scalar(&format!("SELECT EXISTS(SELECT 1 FROM {table})"))
            .fetch_one(&mut *tx)
            .await?;
        require(!exists, "目标已有业务数据，拒绝覆盖；请使用新的隔离库")?;
    }
    sqlx::query("SET CONSTRAINTS ALL DEFERRED")
        .execute(&mut *tx)
        .await?;
    sqlx::query("ALTER TABLE posts DISABLE TRIGGER forum_post_outbox")
        .execute(&mut *tx)
        .await?;
    for table in TABLES {
        let columns:Vec<String>=sqlx::query("SELECT column_name FROM information_schema.columns WHERE table_schema=current_schema() AND table_name=$1 AND is_generated='NEVER' ORDER BY ordinal_position").bind(table).fetch_all(&mut *tx).await?.iter().map(|r|r.get("column_name")).collect();
        // Table/column names are introspected, never taken from the snapshot.
        let quoted = columns
            .iter()
            .map(|c| format!("\"{}\"", c.replace('"', "\"\"")))
            .collect::<Vec<_>>()
            .join(",");
        for row in &snapshot.tables[*table] {
            sqlx::query(&format!("INSERT INTO {table}({quoted}) SELECT {quoted} FROM jsonb_populate_record(NULL::{table},$1)")).bind(row).execute(&mut *tx).await?;
        }
    }
    sqlx::query("ALTER TABLE posts ENABLE TRIGGER forum_post_outbox")
        .execute(&mut *tx)
        .await?;
    // Importing history does not enqueue it. Future edits/withdrawals continue at a stable version.
    sqlx::query("INSERT INTO forum_source_versions(school_id,post_id,sequence,available) SELECT p.school_id,p.id,greatest(p.revision,1),p.audit_status='approved' AND p.visibility='visible' AND s.active FROM posts p JOIN schools s ON s.id=p.school_id").execute(&mut *tx).await?;
    sqlx::query("SELECT setval('legacy_public_id',greatest(1000000000000,coalesce((SELECT max(public_id) FROM legacy_aliases),0)),true)").execute(&mut *tx).await?;
    let mut actual = BTreeMap::new();
    for table in TABLES {
        let mut rows: Vec<Value> = sqlx::query_scalar(&format!(
            "SELECT to_jsonb(t) FROM {table} t{} ",
            filter(table)
        ))
        .fetch_all(&mut *tx)
        .await?;
        normalize(&mut rows);
        actual.insert((*table).to_owned(), rows);
    }
    // New/removed/generated columns must not silently hide a data loss.
    for table in TABLES {
        let mut expected = snapshot.tables[*table].clone();
        let cols:Vec<String>=sqlx::query_scalar("SELECT column_name::text FROM information_schema.columns WHERE table_schema=current_schema() AND table_name=$1 ORDER BY ordinal_position").bind(table).fetch_all(&mut *tx).await?;
        for row in &mut expected {
            require(
                row.as_object()
                    .ok_or(Error::Conflict)?
                    .keys()
                    .all(|key| cols.contains(key)),
                "快照包含目标不支持的列，拒绝静默丢弃数据",
            )?;
        }
        normalize(&mut expected);
        require(actual[*table] == expected, "导入后数据核对失败，事务已回滚")?;
    }
    let counts: BTreeMap<_, _> = actual.iter().map(|(t, r)| (t.clone(), r.len())).collect();
    let report = json!({"source_digest":snapshot.digest,"target_digest":digest(&actual),"counts":counts,"sessions_imported":false,"history_enqueued":false});
    sqlx::query("INSERT INTO migration_runs(id,source_hash,summary) VALUES($1,$2,$3)")
        .bind(uuid::Uuid::now_v7())
        .bind(&snapshot.digest)
        .bind(&report)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(report)
}
