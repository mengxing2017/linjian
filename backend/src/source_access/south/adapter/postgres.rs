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
        // Concurrent verification of the same configured caller reuses its existing connection.
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,0))")
            .bind(format!("{}:{}", config.origin, config.key_id))
            .execute(&mut *tx)
            .await?;
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
        sqlx::query("INSERT INTO source_connection_versions(id,connection_id,revision,source_id,origin,public_origin) VALUES($1,$2,$3,$4,$5,$6)").bind(version).bind(id).bind(rev).bind(source).bind(config.origin).bind(public).execute(&mut *tx).await?;
        sqlx::query("UPDATE school_source_bindings SET connection_version=$2,revision=revision+1 WHERE connection_id=$1").bind(id).bind(version).execute(&mut *tx).await?;
        sqlx::query("UPDATE forum_outbox SET blocked_reason=NULL,blocked_at=NULL,next_attempt_at=now() WHERE source_id=$1 AND schema_version=2 AND blocked_reason IN ('authentication_failed','credential_unavailable','source_disabled','route_unavailable','wekeeper_not_configured')").bind(source).execute(&mut *tx).await?;
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
            "SELECT id,route_id,active,revision FROM school_source_bindings WHERE school_id=$1",
        )
        .bind(school)
        .fetch_optional(&self.0)
        .await?;
        Ok(SchoolRouteSettings {
            binding: row.map(|row| SchoolRoute {
                id: row.get("id"),
                route_id: row.get("route_id"),
                active: row.get("active"),
                revision: row.get("revision"),
            }),
        })
    }
    pub async fn bind(
        &self,
        a: &Actor,
        school: Uuid,
        connection_id: Uuid,
        v: BindingInput,
    ) -> Result<Value> {
        ensure(a.platform_admin, "forbidden")?;
        let mut tx = self.0.begin().await?;
        sqlx::query("SELECT id FROM schools WHERE id=$1 FOR UPDATE")
            .bind(school)
            .fetch_one(&mut *tx)
            .await?;
        let c=sqlx::query("SELECT c.*,v.id AS version FROM source_connections c JOIN source_connection_versions v ON connection_id=c.id AND v.revision=c.revision WHERE c.id=$1 FOR UPDATE OF c").bind(connection_id).fetch_one(&mut *tx).await?;
        let source: Uuid = c.get("source_id");
        let base: String = c.get("origin");
        let owner:Option<Uuid>=sqlx::query_scalar("SELECT school_id FROM school_source_bindings WHERE origin=$1 AND source_id=$2 AND route_id=$3").bind(&base).bind(source).bind(&v.route_id).fetch_optional(&mut *tx).await?;
        ensure(owner.is_none_or(|s| s == school), "route_already_bound")?;
        let id:Uuid=sqlx::query_scalar("INSERT INTO school_source_bindings(id,school_id,connection_id,connection_version,source_id,origin,route_id,active) VALUES($1,$2,$3,$4,$5,$6,$7,$8) ON CONFLICT(school_id) DO UPDATE SET connection_id=$3,connection_version=$4,source_id=$5,origin=$6,route_id=$7,active=$8,revision=school_source_bindings.revision+1 RETURNING id").bind(Uuid::now_v7()).bind(school).bind(connection_id).bind(c.get::<Uuid,_>("version")).bind(source).bind(base).bind(v.route_id).bind(v.active).fetch_one(&mut *tx).await.map_err(|error| match &error { sqlx::Error::Database(db) if db.is_unique_violation() => Fault("route_already_bound"), _ => Fault("temporarily_unavailable") })?;
        crate::audit::record(
            &mut tx,
            a,
            school,
            "source.binding.save",
            id,
            json!({"active":v.active}),
        )
        .await
        .map_err(|_| Fault("temporarily_unavailable"))?;
        tx.commit().await?;
        Ok(json!({"id":id}))
    }
}

impl crate::source_access::south::port::CredentialStore for Repository {
    async fn load_material(&self, id: Uuid) -> Result<String> {
        self.material(id).await
    }
}
