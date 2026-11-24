use sqlx::{
    PgPool,
    migrate::{Migrate, Migrator},
};

pub static MIGRATOR: Migrator = sqlx::migrate!("./migrations");

pub async fn migrate(pool: &PgPool) -> Result<(), sqlx::migrate::MigrateError> {
    let mut connection = pool.acquire().await?;
    connection.lock().await?;
    let result = async {
        connection.ensure_migrations_table().await?;
        let applied = connection.list_applied_migrations().await?;
        let coordinates_required_applied = applied.iter().any(|migration| migration.version == 14);
        let migrator = Migrator {
            migrations: std::borrow::Cow::Owned(
                MIGRATOR
                    .iter()
                    .filter(|migration| migration.version != 14 || coordinates_required_applied)
                    .cloned()
                    .collect(),
            ),
            locking: false,
            ..Migrator::DEFAULT
        };
        migrator.run_direct(&mut *connection).await
    }
    .await;
    let unlocked = connection.unlock().await;
    result?;
    unlocked
}

impl From<sqlx::Error> for crate::business::Error {
    fn from(e: sqlx::Error) -> Self {
        if e.as_database_error()
            .is_some_and(|x| x.constraint() == Some("file_asset_binding"))
        {
            return Self::Invalid("图片已失效或不可用于当前操作".into());
        }
        if e.as_database_error()
            .is_some_and(|x| x.is_unique_violation())
        {
            Self::Conflict
        } else {
            Self::Database(e.to_string())
        }
    }
}
