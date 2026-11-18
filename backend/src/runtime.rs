use std::net::SocketAddr;

use sqlx::PgPool;
use tracing::info;

use crate::{bootstrap, database, http};

/// Shared startup path for the production binary and local embedded-PG example.
/// The caller owns the pool and closes it, including when startup fails.
pub async fn serve(pool: PgPool, bind_addr: SocketAddr) -> Result<(), Box<dyn std::error::Error>> {
    serve_inner(pool, bind_addr, false).await
}

/// Local temporary-database entry; automatically provisions a development admin.
pub async fn serve_embedded(
    pool: PgPool,
    bind_addr: SocketAddr,
) -> Result<(), Box<dyn std::error::Error>> {
    if !bind_addr.ip().is_loopback() {
        return Err("embedded development API must bind to a loopback address".into());
    }
    serve_inner(pool, bind_addr, true).await
}

async fn serve_inner(
    pool: PgPool,
    bind_addr: SocketAddr,
    embedded: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    crate::source_access::configuration::ConnectionConfig::from_env()
        .map_err(|fault| std::io::Error::new(std::io::ErrorKind::InvalidInput, fault.0))?;
    crate::content_publication::domain::Policy::from_env()?;
    let concurrency = std::env::var("LINJIAN_MINIAPP_PREPARATION_CONCURRENCY")
        .unwrap_or_else(|_| "2".into())
        .parse::<usize>()?;
    if !(1..=16).contains(&concurrency) {
        return Err("miniapp preparation concurrency must be between 1 and 16".into());
    }
    std::env::var("LINJIAN_MINIAPP_SOURCE_ENABLED")
        .unwrap_or_else(|_| "true".into())
        .parse::<bool>()?;
    let local = embedded || std::env::var("LINJIAN_INSECURE_LOCAL_AUTH").is_ok_and(|v| v == "true");
    let files = crate::media::south::adapter::filesystem::Files::open_for_startup(
        if embedded {
            crate::media::config::Config::from_env_with_root(
                std::env::temp_dir().join("linjian-media"),
            )
        } else {
            crate::media::config::Config::from_env()
        }
        .map_err(std::io::Error::other)?,
    )?;
    info!(directory = %files.config.storage.filesystem_root.display(), "media file storage initialized");
    let options = if embedded
        && [
            "LINJIAN_ADMIN_WEB_ORIGIN",
            "LINJIAN_BROWSER_ORIGINS",
            "LINJIAN_OFFICIAL_ACCOUNT_WEB_ORIGIN",
            "LINJIAN_COMMUNITY_WEB_ORIGIN",
            "LINJIAN_API_PUBLIC_ORIGIN",
        ]
        .iter()
        .all(|key| std::env::var(key).is_err())
    {
        bootstrap::Options::local_with_files(files)
    } else {
        bootstrap::Options::from_env_with_files(local, files).map_err(std::io::Error::other)?
    };
    database::migrate(&pool).await?;
    let wrong_store:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM file_assets WHERE state IN ('ready','uploading') AND store_id<>$1)").bind(options.files.store_id).fetch_one(&pool).await?;
    if wrong_store {
        return Err("configured media volume does not match database store_id".into());
    }
    crate::source_access::south::adapter::postgres::Repository(pool.clone())
        .publish_environment()
        .await
        .map_err(|fault| std::io::Error::other(fault.0))?;
    let application = bootstrap::build_application(pool.clone(), options);
    let configured_password = std::env::var("LINJIAN_DEV_ADMIN_PASSWORD").ok();
    let generated_password = (embedded && configured_password.is_none())
        .then(|| uuid::Uuid::new_v4().simple().to_string());
    if let Some(password) = configured_password.as_ref().or(generated_password.as_ref()) {
        if !application.options.insecure_local_auth {
            return Err("development bootstrap requires LINJIAN_INSECURE_LOCAL_AUTH=true".into());
        }
        if application.identity.0.account("admin").await?.is_none() {
            application
                .identity
                .bootstrap("admin", "本地管理员", password, true)
                .await?;
        }
    }
    let listener = tokio::net::TcpListener::bind(bind_addr).await?;
    if embedded {
        if let Some(origin) = &application.options.endpoints.admin {
            eprintln!("本地管理后台：{}/admin", origin.as_str());
        }
        eprintln!("管理员用户名：admin");
        if let Some(password) = generated_password {
            eprintln!("本次临时管理员密码：{password}");
        } else {
            eprintln!("管理员密码：使用 LINJIAN_DEV_ADMIN_PASSWORD 设置的值");
        }
    }
    info!(address = %listener.local_addr()?, "Linjian API listening");
    let artwork = crate::schools::south::adapter::postgres::Repository(application.pool.clone());
    let avatar_cleanup =
        crate::miniapp_users::south::adapter::postgres::Repository(application.pool.clone());
    let import_cleanup =
        crate::community::south::adapter::postgres::Repository(application.pool.clone());
    let media_cleanup = application.media.0.clone();
    let artwork_worker = tokio::spawn(async move {
        loop {
            if media_cleanup.cleanup().await.is_err() {
                tracing::warn!("media cleanup failed; retrying later");
            }
            if import_cleanup.cleanup_post_imports().await.is_err() {
                tracing::warn!("expired post import preview cleanup failed");
            }
            if avatar_cleanup.cleanup().await.is_err() {
                tracing::warn!("unused miniapp avatar cleanup failed");
            }
            if artwork.cleanup_artwork().await.is_err() {
                tracing::warn!("abandoned artwork cleanup failed; referenced assets retained");
            }
            tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
        }
    });
    let mut publication_tasks = Vec::new();
    for _ in 0..concurrency {
        let publication =
            crate::content_publication::north::local::Worker::new(application.pool.clone())?;
        let provider = crate::integrations::south::adapter::miniapp_http::Client::from_env(
            application.pool.clone(),
        )?;
        publication_tasks.push(tokio::spawn(async move {
            loop {
                match publication.run_with(&provider).await {
                    Ok(true) => {}
                    Ok(false) => tokio::time::sleep(std::time::Duration::from_secs(2)).await,
                    Err(_) => {
                        tracing::warn!(
                            "miniapp content preparation failed; inspect durable preparation state"
                        );
                        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                    }
                }
            }
        }));
    }
    let outbox_worker = crate::outbox::north::local::Worker::from_env(application.pool.clone())?;
    let outbox_task = tokio::spawn(async move {
        loop {
            if let Some(worker) = &outbox_worker {
                for _ in 0..20 {
                    match worker.run_once().await {
                        Ok(true) => {}
                        Ok(false) => break,
                        Err(_) => {
                            tracing::warn!("forum outbox will retry; inspect persisted state");
                            break;
                        }
                    }
                }
            }
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        }
    });
    let activity = application.activity.clone();
    let statistics_pool = pool.clone();
    let statistics_config = application.options.statistics.clone();
    let statistics_task =
        tokio::spawn(async move { activity.run(statistics_pool, statistics_config).await });
    let result = axum::serve(
        listener,
        http::app_router(application).into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await;
    statistics_task.abort();
    let _ = statistics_task.await;
    outbox_task.abort();
    let _ = outbox_task.await;
    artwork_worker.abort();
    let _ = artwork_worker.await;
    for task in publication_tasks {
        task.abort();
    }
    result?;
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl-C handler");
    };
    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {},
        () = terminate => {},
    }
    info!("shutdown requested");
}
