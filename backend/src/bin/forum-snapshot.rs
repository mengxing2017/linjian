use clap::{Parser, Subcommand};
use linjian_api::{database, migration};
use sqlx::PgPool;

#[derive(Parser)]
#[command(about = "Read-only forum export / atomic import into an empty Linjian database")]
struct Args {
    #[command(subcommand)]
    command: Command,
}
#[derive(Subcommand)]
enum Command {
    Export {
        file: std::path::PathBuf,
    },
    Import {
        file: std::path::PathBuf,
        #[arg(long)]
        same_wechat_app: bool,
    },
}
async fn execute(pool: &PgPool, command: Command) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        Command::Export { file } => {
            let snapshot = migration::export(pool).await?;
            let mut options = std::fs::OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600);
            }
            serde_json::to_writer(options.open(file)?, &snapshot)?;
            println!(
                "{}",
                serde_json::json!({"digest":snapshot.digest,"table_count":snapshot.tables.len()})
            );
        }
        Command::Import {
            file,
            same_wechat_app,
        } => {
            let snapshot = serde_json::from_reader(std::fs::File::open(file)?)?;
            let existing: Option<String> =
                sqlx::query_scalar("SELECT to_regclass('application_metadata')::text")
                    .fetch_one(pool)
                    .await?;
            if existing.is_some() {
                let name: Option<String> = sqlx::query_scalar("SELECT to_jsonb(m)->>'application' FROM application_metadata m WHERE singleton")
                    .fetch_optional(pool).await?;
                if name.as_deref() != Some("linjian") {
                    return Err("Refusing to migrate a non-Linjian database".into());
                }
            }
            database::migrate(pool).await?;
            println!(
                "{}",
                migration::import(pool, &snapshot, same_wechat_app).await?
            );
        }
    }
    Ok(())
}
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let name = match args.command {
        Command::Export { .. } => "LINJIAN_MIGRATION_SOURCE_URL",
        Command::Import { .. } => "LINJIAN_MIGRATION_TARGET_URL",
    };
    let url = std::env::var(name)
        .map_err(|_| format!("Set {name} explicitly; no application database fallback"))?;
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect(&url)
        .await?;
    let result = execute(&pool, args.command).await;
    pool.close().await;
    result
}
