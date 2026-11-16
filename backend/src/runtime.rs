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
    database::migrate(&pool).await?;
    crate::source_access::south::adapter::postgres::Repository(pool.clone())
        .publish_environment()
        .await
        .map_err(|fault| std::io::Error::other(fault.0))?;
    let mut application = bootstrap::build_application(pool.clone());
    let configured_password = std::env::var("LINJIAN_DEV_ADMIN_PASSWORD").ok();
    let generated_password = (embedded && configured_password.is_none())
        .then(|| uuid::Uuid::new_v4().simple().to_string());
    if embedded {
        application.options.insecure_local_auth = true;
    }
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
        eprintln!("本地管理后台：{}/admin", application.options.public_origin);
        eprintln!("管理员用户名：admin");
        if let Some(password) = generated_password {
            eprintln!("本次临时管理员密码：{password}");
        } else {
            eprintln!("管理员密码：使用 LINJIAN_DEV_ADMIN_PASSWORD 设置的值");
        }
    }
    info!(address = %listener.local_addr()?, "Linjian API listening");
    let directory = application.integrations.directory();
    let interval = std::env::var("LINJIAN_WECOM_DIRECTORY_INTERVAL_SECONDS")
        .ok()
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(1800);
    if interval != 0 && interval < 60 {
        return Err("directory interval must be 0 or at least 60 seconds".into());
    }
    tracing::info!(
        server_configured = directory.server_configured,
        interval_seconds = interval,
        poll_seconds = 5,
        "wecom directory worker initialized; interval 0 disables scheduled sync only"
    );
    let worker = tokio::spawn(async move {
        loop {
            if directory.server_configured
                && let Err(error) = directory.run_once(interval).await
            {
                tracing::warn!(
                    error_code = crate::integrations::north::directory::error_code(&error),
                    "wecom directory worker failed; inspect task logs and directory configuration"
                );
            }
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        }
    });
    let artwork = crate::schools::south::adapter::postgres::Repository(application.pool.clone());
    let avatar_cleanup =
        crate::miniapp_users::south::adapter::postgres::Repository(application.pool.clone());
    let import_cleanup =
        crate::community::south::adapter::postgres::Repository(application.pool.clone());
    let artwork_worker = tokio::spawn(async move {
        loop {
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
    let result = axum::serve(
        listener,
        http::app_router(application).into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await;
    outbox_task.abort();
    let _ = outbox_task.await;
    artwork_worker.abort();
    let _ = artwork_worker.await;
    for task in publication_tasks {
        task.abort();
    }
    worker.abort();
    let _ = worker.await;
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
