use crate::business::{Actor, Error, Result};
use crate::legal_documents::south::port::Store;
use serde_json::{Value, json};
use sqlx::{PgPool, Row};
#[derive(Clone)]
pub struct Repository(pub PgPool);
impl Store for Repository {
    async fn read(&self, kind: Option<&str>, admin: bool) -> Result<Value> {
        if let Some(kind) = kind {
            let value: Option<Value> = sqlx::query_scalar("SELECT jsonb_build_object('id',d.kind,'kind',d.kind,'revision',d.revision,'content',d.content,'updated_at',d.updated_at) || CASE WHEN $2 THEN jsonb_build_object('updated_by_name',a.display_name) ELSE '{}'::jsonb END FROM legal_documents d LEFT JOIN accounts a ON a.id=d.updated_by WHERE d.kind=$1 AND ($2 OR d.content IS NOT NULL)")
                .bind(kind).bind(admin).fetch_optional(&self.0).await?;
            value.ok_or(Error::NotFound)
        } else {
            let rows: Vec<Value> = sqlx::query_scalar("SELECT jsonb_build_object('id',kind,'kind',kind,'revision',revision,'updated_at',updated_at) FROM legal_documents WHERE content IS NOT NULL ORDER BY kind DESC").fetch_all(&self.0).await?;
            Ok(json!(rows))
        }
    }
    async fn history(&self, kind: &str, page: i64) -> Result<Value> {
        let rows: Vec<Value> = sqlx::query_scalar("SELECT jsonb_build_object('id',r.kind||':'||r.revision,'revision',r.revision,'content',r.content,'updated_at',r.updated_at,'updated_by_name',a.display_name) FROM legal_document_revisions r JOIN accounts a ON a.id=r.updated_by WHERE r.kind=$1 ORDER BY r.revision DESC LIMIT 20 OFFSET $2")
            .bind(kind).bind((page-1)*20).fetch_all(&self.0).await?;
        Ok(json!(rows))
    }
    async fn save(
        &self,
        actor: &Actor,
        kind: &str,
        revision: i64,
        content: Option<String>,
    ) -> Result<Value> {
        let mut transaction = self.0.begin().await?;
        sqlx::query("SELECT a.id FROM accounts a JOIN sessions s ON s.account_id=a.id WHERE a.id=$1 AND a.active AND a.platform_admin AND s.token_hash=$2 AND s.kind='admin_browser' AND s.expires_at>now() AND s.last_seen_at>now()-interval '30 minutes' FOR SHARE OF a,s").bind(actor.id).bind(&actor.session_hash).fetch_optional(&mut *transaction).await?.ok_or(Error::Unauthorized)?;
        crate::miniapp_users::south::adapter::postgres::school_binding::platform_session(
            &mut transaction,
            actor,
        )
        .await?;
        sqlx::query("SELECT id FROM legal_document_catalog FOR UPDATE")
            .execute(&mut *transaction)
            .await?;
        let row =
            sqlx::query("SELECT revision,content FROM legal_documents WHERE kind=$1 FOR UPDATE")
                .bind(kind)
                .fetch_one(&mut *transaction)
                .await?;
        if row.get::<i64, _>("revision") != revision {
            return Err(Error::Conflict);
        }
        if row.get::<Option<String>, _>("content") != content {
            sqlx::query("UPDATE legal_documents SET revision=revision+1,content=$2,updated_at=now(),updated_by=$3 WHERE kind=$1").bind(kind).bind(&content).bind(actor.id).execute(&mut *transaction).await?;
            sqlx::query("INSERT INTO legal_document_revisions(kind,revision,content,updated_by) VALUES($1,$2,$3,$4)").bind(kind).bind(revision+1).bind(&content).bind(actor.id).execute(&mut *transaction).await?;
            sqlx::query("UPDATE legal_document_catalog SET revision=revision+1")
                .execute(&mut *transaction)
                .await?;
            sqlx::query("INSERT INTO audit_events(id,actor_id,action,detail) VALUES($1,$2,'legal_document.update',$3)").bind(uuid::Uuid::now_v7()).bind(actor.id).bind(json!({"kind":kind,"revision":revision+1,"cleared":content.is_none()})).execute(&mut *transaction).await?;
        }
        transaction.commit().await?;
        self.read(Some(kind), true).await
    }
}
