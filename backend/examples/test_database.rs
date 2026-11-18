//! Run the normal integration target against a new, isolated temporary PostgreSQL.
use postgresql_embedded::{BOOTSTRAP_SUPERUSER, PostgreSQL, SettingsBuilder, V17};
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut pg = PostgreSQL::new(
        SettingsBuilder::new()
            .host("127.0.0.1")
            .port(0)
            .username(BOOTSTRAP_SUPERUSER)
            .temporary(true)
            .version(V17.clone())
            .build(),
    );
    pg.setup().await?;
    pg.start().await?;
    let result = async {
        pg.create_database("linjian_statistics_test").await?;
        let url = pg.settings().url("linjian_statistics_test");
        let status = std::process::Command::new("make")
            .arg("test-integration")
            .env("LINJIAN_TEST_DATABASE_URL", url)
            .status()?;
        if !status.success() {
            return Err("integration checks failed".into());
        }
        Ok::<_, Box<dyn std::error::Error>>(())
    }
    .await;
    let stopped = pg.stop().await;
    result?;
    stopped?;
    Ok(())
}
