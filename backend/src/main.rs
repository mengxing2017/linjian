use clap::Parser;
use linjian_api::{config::Config, observability, runtime};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    observability::init();
    let config = Config::parse();
    let pool = config.connect().await?;
    if let Some(username) = config.create_admin {
        use std::io::Read;
        let mut password = String::new();
        std::io::stdin().read_to_string(&mut password)?;
        linjian_api::database::migrate(&pool).await?;
        let a = linjian_api::bootstrap::build_application(pool.clone());
        let id = a
            .identity
            .bootstrap(&username, &username, password.trim_end(), true)
            .await?;
        println!("Administrator created: {id}");
        pool.close().await;
        return Ok(());
    }
    let result = runtime::serve(pool.clone(), config.bind_addr).await;
    pool.close().await;
    result?;
    Ok(())
}
