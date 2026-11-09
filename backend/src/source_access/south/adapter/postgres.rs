use crate::{
    business::Actor,
    source_access::{
        configuration::ConnectionConfig,
        domain::*,
        north::local::{BindingInput, SchoolRoute, SchoolRouteSettings},
    },
};
use serde_json::{Value, json};
use sqlx::{PgPool, Row};
use uuid::Uuid;
pub struct Repository(pub PgPool);
impl Repository {
    pub async fn publish_environment(&self) -> Result<()> {
        let config = ConnectionConfig::from_env()?;
        let (origin, key) = config
            .as_ref()
            .map(|c| (c.origin.as_str(), c.key_id.as_str()))
            .unwrap_or(("", ""));
        sqlx::query("UPDATE source_active_configuration SET connection_id=(SELECT id FROM source_connections WHERE origin=$1 AND key_id=$2 AND checked_at IS NOT NULL ORDER BY created_at,id LIMIT 1),updated_at=now() WHERE singleton AND connection_id IS DISTINCT FROM (SELECT id FROM source_connections WHERE origin=$1 AND key_id=$2 AND checked_at IS NOT NULL ORDER BY created_at,id LIMIT 1)").bind(origin).bind(key).execute(&self.0).await?;
        Ok(())
    }
    pub async fn connection(&self, id: Uuid) -> Result<Value> {
        sqlx::query_scalar(
            "SELECT to_jsonb(c)-'source_id'-'capability' FROM source_connections c WHERE id=$1",
        )
        .bind(id)
        .fetch_optional(&self.0)
        .await?
        .ok_or(Fault("not_found"))
    }
    pub async fn material(&self, id: Uuid) -> Result<String> {
        let row = sqlx::query("SELECT origin,key_id FROM source_connections WHERE id=$1")
            .bind(id)
            .fetch_optional(&self.0)
            .await?
            .ok_or(Fault("not_found"))?;
        ConnectionConfig::from_env()?
            .ok_or(Fault("wekeeper_not_configured"))?
            .material(row.get("origin"), row.get("key_id"))
    }
    pub async fn configured_connection(&self, config: &ConnectionConfig) -> Result<Option<Value>> {
        Ok(sqlx::query_scalar("SELECT to_jsonb(c)-'source_id'-'capability' FROM source_connections c WHERE origin=$1 AND key_id=$2 ORDER BY created_at,id LIMIT 1").bind(&config.origin).bind(&config.key_id).fetch_optional(&self.0).await?)
    }
    pub async fn school_connection(&self, school: Uuid) -> Result<Option<Uuid>> {
        Ok(sqlx::query_scalar(
            "SELECT connection_id FROM school_source_bindings WHERE school_id=$1",
        )
        .bind(school)
        .fetch_optional(&self.0)
        .await?)
    }
    pub async fn save(
        &self,
        a: &Actor,
        config: ConnectionConfig,
        source: Uuid,
        cap: Value,
    ) -> Result<Value> {
        ensure(a.platform_admin, "forbidden")?;
        let mut tx = self.0.begin().await?;
        sqlx::query("SELECT set_config('linjian.source_actor_id',$1,true)")
            .bind(a.id.to_string())
            .execute(&mut *tx)
            .await?;
        // Concurrent verification of the same configured caller reuses its existing connection.
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,0))")
            .bind(format!("{}:{}", config.origin, config.key_id))
            .execute(&mut *tx)
            .await?;
        sqlx::query("SELECT s.id FROM schools s JOIN school_source_bindings b ON b.school_id=s.id ORDER BY s.id FOR UPDATE OF s").fetch_all(&mut *tx).await?;
        let old = sqlx::query("SELECT id,source_id FROM source_connections WHERE origin=$1 AND key_id=$2 ORDER BY created_at,id LIMIT 1 FOR UPDATE").bind(&config.origin).bind(&config.key_id).fetch_optional(&mut *tx).await?;
        let id = if let Some(old) = old {
            ensure(source == old.get::<Uuid, _>("source_id"), "invalid_request")?;
            old.get("id")
        } else {
            Uuid::now_v7()
        };
        let rev:i64=sqlx::query_scalar("INSERT INTO source_connections(id,name,source_id,origin,key_id,checked_at,expires_at,capability) VALUES($1,'WeKeeper',$2,$3,$4,now(),now()+interval '60 seconds',$5) ON CONFLICT(id) DO UPDATE SET checked_at=now(),expires_at=now()+interval '60 seconds',capability=$5,revision=source_connections.revision+1,updated_at=now() RETURNING revision").bind(id).bind(source).bind(&config.origin).bind(&config.key_id).bind(cap).fetch_one(&mut *tx).await?;
        let public =
            std::env::var("LINJIAN_PUBLIC_ORIGIN").map_err(|_| Fault("public_origin_invalid"))?;
        let public = origin(&public)
            .map_err(|_| Fault("public_origin_invalid"))?
            .origin()
            .ascii_serialization();
        let version = Uuid::now_v7();
        sqlx::query("INSERT INTO source_connection_versions(id,connection_id,revision,source_id,origin,public_origin,key_id) VALUES($1,$2,$3,$4,$5,$6,$7)").bind(version).bind(id).bind(rev).bind(source).bind(&config.origin).bind(public).bind(&config.key_id).execute(&mut *tx).await?;
        sqlx::query("UPDATE school_source_bindings SET connection_version=$2,revision=revision+1,updated_at=now() WHERE connection_id=$1").bind(id).bind(version).execute(&mut *tx).await?;
        sqlx::query("UPDATE source_active_configuration SET connection_id=$1,updated_at=now() WHERE singleton").bind(id).execute(&mut *tx).await?;
        sqlx::query("UPDATE forum_outbox o SET blocked_reason=NULL,blocked_at=NULL,next_attempt_at=now(),delivery_state=CASE WHEN receipt_id IS NULL THEN 'ready' ELSE 'tracking_receipt' END FROM source_connection_versions v WHERE o.connection_version=v.id AND v.origin=$1 AND v.key_id=$2 AND v.source_id=$3 AND o.schema_version=2 AND o.lease_token IS NULL AND o.blocked_reason IN ('authentication_failed','credential_unavailable','source_disabled','wekeeper_not_configured')").bind(&config.origin).bind(&config.key_id).bind(source).execute(&mut *tx).await?;
        crate::audit::record(
            &mut tx,
            a,
            Uuid::nil(),
            "source.connection.save",
            id,
            json!({"revision":rev}),
        )
        .await
        .map_err(|_| Fault("temporarily_unavailable"))?;
        tx.commit().await?;
        Ok(json!({"id":id,"revision":rev}))
    }
    pub async fn bindings(&self, a: &Actor, school: Uuid) -> Result<SchoolRouteSettings> {
        ensure(a.can(school, "admin"), "forbidden")?;
        let row = sqlx::query(
            "SELECT id,route_id,active,revision,checked_at,updated_at FROM school_source_bindings WHERE school_id=$1",
        )
        .bind(school)
        .fetch_optional(&self.0)
        .await?;
        let mut backlog: std::collections::BTreeMap<String,i64> = sqlx::query("SELECT CASE WHEN o.delivery_state IN ('ready','retry_wait') AND o.receipt_id IS NULL AND o.payload->>'event_type'='content.upsert' AND b.active=false THEN 'paused' ELSE o.delivery_state END AS delivery_state,count(*) AS count FROM forum_outbox o LEFT JOIN school_source_bindings b ON b.school_id=o.school_id WHERE o.school_id=$1 AND o.delivery_state NOT IN ('settled','superseded') GROUP BY 1").bind(school).fetch_all(&self.0).await?.iter().map(|r| (r.get("delivery_state"),r.get("count"))).collect();
        let jobs:i64=sqlx::query_scalar("SELECT count(*) FROM source_reconciliation_jobs WHERE school_id=$1 AND completed_at IS NULL").bind(school).fetch_one(&self.0).await?;
        if jobs > 0 {
            backlog.insert("withdrawal_jobs".into(), jobs);
        }
        Ok(SchoolRouteSettings {
            backlog,
            binding: row.map(|row| SchoolRoute {
                id: row.get("id"),
                route_id: row.get("route_id"),
                active: row.get("active"),
                revision: row.get("revision"),
                checked_at: row.get("checked_at"),
                updated_at: row.get("updated_at"),
            }),
        })
    }
    pub async fn bind(
        &self,
        a: &Actor,
        school: Uuid,
        connection_id: Uuid,
        v: BindingInput,
        expected: i64,
        connection_revision: i64,
    ) -> Result<Value> {
        ensure(a.platform_admin, "forbidden")?;
        let mut tx = self.0.begin().await?;
        sqlx::query("SELECT set_config('linjian.source_actor_id',$1,true)")
            .bind(a.id.to_string())
            .execute(&mut *tx)
            .await?;
        sqlx::query("SELECT id FROM schools WHERE id=$1 FOR UPDATE")
            .bind(school)
            .fetch_one(&mut *tx)
            .await?;
        let revision: Option<i64> =
            sqlx::query_scalar("SELECT revision FROM school_source_bindings WHERE school_id=$1")
                .bind(school)
                .fetch_optional(&mut *tx)
                .await?;
        ensure(revision.unwrap_or(0) == expected, "revision_conflict")?;
        let c=sqlx::query("SELECT c.*,v.id AS version FROM source_connections c JOIN source_connection_versions v ON connection_id=c.id AND v.revision=c.revision WHERE c.id=$1 FOR UPDATE OF c").bind(connection_id).fetch_one(&mut *tx).await?;
        ensure(
            c.get::<i64, _>("revision") == connection_revision,
            "revision_conflict",
        )?;
        if v.active {
            let current: Option<Uuid> = sqlx::query_scalar(
                "SELECT connection_id FROM source_active_configuration WHERE singleton",
            )
            .fetch_one(&mut *tx)
            .await?;
            ensure(current == Some(connection_id), "revision_conflict")?;
        }
        let source: Uuid = c.get("source_id");
        let base: String = c.get("origin");
        let owner:Option<Uuid>=sqlx::query_scalar("SELECT school_id FROM school_source_bindings WHERE origin=$1 AND source_id=$2 AND route_id=$3").bind(&base).bind(source).bind(&v.route_id).fetch_optional(&mut *tx).await?;
        ensure(owner.is_none_or(|s| s == school), "route_already_bound")?;
        let id:Uuid=sqlx::query_scalar("INSERT INTO school_source_bindings(id,school_id,connection_id,connection_version,source_id,origin,route_id,active,checked_at) VALUES($1,$2,$3,$4,$5,$6,$7,$8,CASE WHEN $8 THEN now() END) ON CONFLICT(school_id) DO UPDATE SET connection_id=$3,connection_version=$4,source_id=$5,origin=$6,route_id=$7,active=$8,checked_at=CASE WHEN $8 THEN now() END,updated_at=now(),revision=school_source_bindings.revision+1 RETURNING id").bind(Uuid::now_v7()).bind(school).bind(connection_id).bind(c.get::<Uuid,_>("version")).bind(source).bind(base).bind(&v.route_id).bind(v.active).fetch_one(&mut *tx).await.map_err(|error| match &error { sqlx::Error::Database(db) if db.is_unique_violation() => Fault("route_already_bound"), _ => Fault("temporarily_unavailable") })?;
        crate::audit::record(
            &mut tx,
            a,
            school,
            "source.binding.save",
            id,
            json!({"active":v.active,"revision":expected+1,"route_id":v.route_id}),
        )
        .await
        .map_err(|_| Fault("temporarily_unavailable"))?;
        tx.commit().await?;
        Ok(json!({"id":id,"revision":expected+1}))
    }
}

impl crate::source_access::south::port::CredentialStore for Repository {
    async fn load_material(&self, id: Uuid) -> Result<String> {
        self.material(id).await
    }
}
