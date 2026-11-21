//! Offline snapshot tooling: source is repeatable-read/read-only; target must be
//! an empty Linjian database. Sessions, machine credentials never move; source event identities and deduplication evidence are preserved.
use crate::business::{Error, Result, require};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::{Connection, PgPool, Row};
use std::collections::BTreeMap;

pub const TABLES: &[&str] = &[
    "schools",
    "accounts",
    "school_memberships",
    "school_member_authorizations",
    "school_member_permission_overrides",
    "file_assets",
    "miniapp_avatar_assets",
    "miniapp_users",
    "miniapp_identities",
    "post_seed_batches",
    "posts",
    "post_seed_items",
    "comments",
    "post_reactions",
    "comment_reactions",
    "follows",
    "notifications",
    "moderation_events",
    "forum_publication_snapshots",
    "source_connections",
    "source_connection_versions",
    "school_source_bindings",
    "school_source_binding_versions",
    "forum_source_versions",
    "miniapp_link_assets",
    "forum_outbox",
    "forum_publication_preparations",
    "forum_post_recommendations",
    "forum_source_terminations",
    "cases",
    "operation_service_categories",
    "operation_partners",
    "operation_cooperations",
    "operation_service_targets",
    "operation_target_versions",
    "operation_target_reviews",
    "school_resources",
    "media_assets",
    "operational_media",
    "resource_publications",
    "customer_config",
    "school_shortcut_overrides",
    "operation_config_revisions",
    "marketing_slots",
    "marketing_events",
    "school_marketing_settings",
    "operation_metric_events",
    "operation_action_events",
    "operation_daily_metrics",
    "operation_report_snapshots",
    "audit_events",
    "legacy_id_map",
    "legacy_aliases",
    "file_asset_references",
];
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Snapshot {
    pub schema_version: u32,
    pub tables: BTreeMap<String, Vec<Value>>,
    pub digest: String,
    pub files: BTreeMap<uuid::Uuid, Vec<u8>>,
}
fn normalize(rows: &mut [Value]) {
    for row in rows.iter_mut() {
        if row.get("store_id").is_some() {
            row["store_id"] = json!(uuid::Uuid::nil());
        }
    }
    rows.sort_by_cached_key(Value::to_string);
}
fn digest(tables: &BTreeMap<String, Vec<Value>>) -> String {
    crate::business::hash(&serde_json::to_string(tables).expect("serializable JSON"))
}
fn filter(table: &str) -> &'static str {
    match table {
        "file_asset_references" => " WHERE object_type <> 'product_previews'",
        "audit_events" => {
            " WHERE action LIKE 'post.%' OR action LIKE 'comment.%' OR action LIKE 'case.%' OR action LIKE 'school.%' OR action LIKE 'account.%' OR action LIKE 'resource.%' OR action LIKE 'operation.%'"
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
pub async fn export(
    pool: &PgPool,
    files: &crate::media::south::adapter::filesystem::Files,
) -> Result<Snapshot> {
    let mut lock = crate::media::south::adapter::lifecycle::FileLock::shared(pool).await?;
    let mut tx = lock.connection.begin().await?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ READ ONLY")
        .execute(&mut *tx)
        .await?;
    let mut payloads = BTreeMap::new();
    let rows =
        sqlx::query("SELECT id,store_id,size_bytes,sha256 FROM file_assets WHERE state='ready'")
            .fetch_all(&mut *tx)
            .await?;
    for row in rows {
        let id: uuid::Uuid = row.get("id");
        payloads.insert(
            id,
            files
                .read(
                    row.get("store_id"),
                    id,
                    row.get("size_bytes"),
                    row.get("sha256"),
                )
                .await?,
        );
    }
    let mut tables = BTreeMap::new();
    for table in TABLES {
        let query = format!("SELECT to_jsonb(t) FROM {table} t{}", filter(table));
        let mut rows: Vec<Value> = sqlx::query_scalar(&query).fetch_all(&mut *tx).await?;
        normalize(&mut rows);
        tables.insert((*table).into(), rows);
    }
    tx.commit().await?;
    lock.release().await?;
    Ok(Snapshot {
        schema_version: 6,
        digest: digest(&tables),
        tables,
        files: payloads,
    })
}
pub async fn import(
    pool: &PgPool,
    snapshot: &Snapshot,
    same_wechat_app: bool,
    files: &crate::media::south::adapter::filesystem::Files,
) -> Result<Value> {
    require(
        snapshot.schema_version == 6 && snapshot.digest == digest(&snapshot.tables),
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
    let expected_files: Vec<&Value> = snapshot.tables["file_assets"]
        .iter()
        .filter(|v| v["state"] == "ready")
        .collect();
    require(
        expected_files.len() == snapshot.files.len(),
        "文件包清单不完整",
    )?;
    for asset in expected_files {
        let id = crate::business::uuid(asset, "id")?;
        let bytes = snapshot
            .files
            .get(&id)
            .ok_or(Error::Invalid("文件缺失".into()))?;
        require(
            Some(bytes.len() as u64) == asset["size_bytes"].as_u64()
                && Some(crate::media::south::adapter::filesystem::digest(bytes).as_str())
                    == asset["sha256"].as_str(),
            "文件摘要或大小不匹配",
        )?;
        files.write(id, bytes.clone()).await?;
    }
    for table in [
        "schools",
        "miniapp_users",
        "posts",
        "cases",
        "school_resources",
        "school_shortcut_overrides",
        "operation_target_versions",
        "resource_publications",
        "forum_publication_snapshots",
    ] {
        sqlx::query(&format!(
            "ALTER TABLE {table} DISABLE TRIGGER file_references"
        ))
        .execute(&mut *tx)
        .await?;
    }
    // Restore immutable review evidence without applying today's live authorization policy.
    // This offline operation already requires an empty target and an exact snapshot digest.
    for (table, trigger) in [
        ("school_source_bindings", "forum_binding_history"),
        ("posts", "seed_post_guard"),
        ("posts", "forum_post_outbox"),
        ("posts", "forum_post_publication_guard"),
        ("moderation_events", "forum_decision_guard"),
        ("forum_publication_snapshots", "forum_new_snapshot_guard"),
    ] {
        sqlx::query(&format!("ALTER TABLE {table} DISABLE TRIGGER {trigger}"))
            .execute(&mut *tx)
            .await?;
    }
    for table in TABLES {
        let columns:Vec<String>=sqlx::query("SELECT column_name FROM information_schema.columns WHERE table_schema=current_schema() AND table_name=$1 AND is_generated='NEVER' ORDER BY ordinal_position").bind(table).fetch_all(&mut *tx).await?.iter().map(|r|r.get("column_name")).collect();
        // Table/column names are introspected, never taken from the snapshot.
        let quoted = columns
            .iter()
            .map(|c| format!("\"{}\"", c.replace('"', "\"\"")))
            .collect::<Vec<_>>()
            .join(",");
        for row in &snapshot.tables[*table] {
            let mut row = row.clone();
            if *table == "file_assets" {
                row["store_id"] = json!(files.store_id);
            }
            sqlx::query(&format!("INSERT INTO {table}({quoted}) SELECT {quoted} FROM jsonb_populate_record(NULL::{table},$1)")).bind(&row).execute(&mut *tx).await?;
        }
    }
    // Validate the restored cyclic references before altering trigger configuration.
    sqlx::query("SET CONSTRAINTS ALL IMMEDIATE")
        .execute(&mut *tx)
        .await?;
    for (table, trigger) in [
        ("school_source_bindings", "forum_binding_history"),
        ("posts", "seed_post_guard"),
        ("posts", "forum_post_outbox"),
        ("posts", "forum_post_publication_guard"),
        ("moderation_events", "forum_decision_guard"),
        ("forum_publication_snapshots", "forum_new_snapshot_guard"),
    ] {
        sqlx::query(&format!("ALTER TABLE {table} ENABLE TRIGGER {trigger}"))
            .execute(&mut *tx)
            .await?;
    }
    for table in [
        "schools",
        "miniapp_users",
        "posts",
        "cases",
        "school_resources",
        "school_shortcut_overrides",
        "operation_target_versions",
        "resource_publications",
        "forum_publication_snapshots",
    ] {
        sqlx::query(&format!(
            "ALTER TABLE {table} ENABLE TRIGGER file_references"
        ))
        .execute(&mut *tx)
        .await?;
    }
    let invalid: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM file_asset_references r JOIN file_assets f ON f.id=r.asset_id LEFT JOIN posts p ON r.object_type='posts' AND p.id=r.object_id WHERE f.state<>'ready' OR NOT f.ever_bound OR f.gc_after IS NOT NULL OR (r.object_type='posts' AND (p.id IS NULL OR p.visibility='deleted')))")
        .fetch_one(&mut *tx).await?;
    require(!invalid, "快照图片引用或清理状态不一致")?;
    // Exact restoration never creates replacement events or drops request idempotency.
    // Active source configuration and credentials are not imported; an administrator configures the destination explicitly.
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
    let batch_id = uuid::Uuid::now_v7();
    sqlx::query("INSERT INTO migration_runs(id,source_hash,summary) VALUES($1,$2,$3)")
        .bind(batch_id)
        .bind(&snapshot.digest)
        .bind(&report)
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO source_import_batches(id) VALUES($1)")
        .bind(batch_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO source_imported_posts(post_id,batch_id) SELECT id,$1 FROM posts")
        .bind(batch_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(report)
}
