//! Transaction-local policy and a dedicated connection for file/backup coordination.
use crate::business::{Error, Result};
use sqlx::{PgConnection, PgPool, Postgres, Transaction};

const FILE_LOCK: i64 = 78_234_610_246;

pub async fn configure(connection: &mut PgConnection, hours: i64) -> Result<()> {
    if !(1..=168).contains(&hours) {
        return Err(Error::Invalid("图片清理宽限期无效".into()));
    }
    sqlx::query("SELECT set_config('linjian.media_orphan_grace_hours',$1,true)")
        .bind(hours.to_string())
        .execute(connection)
        .await?;
    Ok(())
}
pub async fn begin(pool: &PgPool) -> Result<Transaction<'static, Postgres>> {
    let hours =
        crate::media::config::Config::orphan_grace_hours_from_env().map_err(Error::Invalid)?;
    let mut tx = pool.begin().await?;
    configure(&mut tx, hours).await?;
    Ok(tx)
}

pub struct FileLock {
    // Detached connections are closed on cancellation/error, never returned with a session lock.
    pub connection: PgConnection,
    shared: bool,
}
impl FileLock {
    pub async fn shared(pool: &PgPool) -> Result<Self> {
        let mut connection = pool.acquire().await?.detach();
        sqlx::query("SELECT pg_advisory_lock_shared($1)")
            .bind(FILE_LOCK)
            .execute(&mut connection)
            .await?;
        Ok(Self {
            connection,
            shared: true,
        })
    }
    pub async fn try_exclusive(pool: &PgPool) -> Result<Option<Self>> {
        let mut connection = pool.acquire().await?.detach();
        let locked: bool = sqlx::query_scalar("SELECT pg_try_advisory_lock($1)")
            .bind(FILE_LOCK)
            .fetch_one(&mut connection)
            .await?;
        Ok(locked.then_some(Self {
            connection,
            shared: false,
        }))
    }
    pub async fn release(mut self) -> Result<()> {
        let sql = if self.shared {
            "SELECT pg_advisory_unlock_shared($1)"
        } else {
            "SELECT pg_advisory_unlock($1)"
        };
        sqlx::query(sql)
            .bind(FILE_LOCK)
            .execute(&mut self.connection)
            .await?;
        // Owned PgConnection drops and closes, including on errors above.
        Ok(())
    }
}
