mod auth;
mod config;
mod db;
mod embeddings;
mod error;
mod ingest;
mod mcp;
mod openapi;
mod query;
mod rate_limit;
mod routes;
mod state;
mod workers;

use crate::auth::{ensure_bootstrap_key, sync_admin_credentials, KeyScopes};
use crate::config::Config;
use crate::db::{open_db, setting_set};
use crate::embeddings::EmbeddingClient;
use crate::ingest::{spawn_writer, IngestQueue, IngestStats};
use crate::rate_limit::RateLimiter;
use crate::state::AppState;
use axum::extract::DefaultBodyLimit;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::info;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    let config = Config::from_env();
    std::fs::create_dir_all(&config.data_dir)?;

    let db = open_db(&config.database_path)?;
    sync_admin_credentials(&db, &config.admin_username, &config.admin_password)?;
    {
        let conn = db.lock();
        setting_set(&conn, "retention_days", &config.retention_days.to_string())?;
        if let Some(max) = config.max_events {
            setting_set(&conn, "max_events", &max.to_string())?;
        }
    }

    if let Some(key) = &config.bootstrap_ingest_key {
        ensure_bootstrap_key(
            &db,
            "bootstrap-ingest",
            key,
            KeyScopes {
                ingest: true,
                query: false,
            },
        )?;
        info!("bootstrap ingest key ensured");
    }
    if let Some(key) = &config.bootstrap_query_key {
        ensure_bootstrap_key(
            &db,
            "bootstrap-query",
            key,
            KeyScopes {
                ingest: false,
                query: true,
            },
        )?;
        info!("bootstrap query key ensured");
    }

    let (ingest, rx) = IngestQueue::new(config.write_queue_capacity, db.clone(), config.embed_sample_rate);
    spawn_writer(rx, db.clone());

    let embeddings = EmbeddingClient::from_config(&config);
    if embeddings.is_some() {
        info!("embeddings client enabled");
    } else {
        info!("embeddings disabled (set EMBEDDINGS_BASE_URL + EMBEDDINGS_MODEL to enable)");
    }

    workers::spawn_workers(db.clone(), config.clone(), embeddings.clone());

    let state = AppState {
        db,
        config: Arc::new(config.clone()),
        ingest,
        ingest_stats: Arc::new(IngestStats::default()),
        rate_limiter: RateLimiter::new(config.per_key_rps),
        embeddings,
    };

    let web_dir = std::env::var("WEB_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("web/dist"));

    let app = routes::app_router(state.clone(), Some(web_dir))
        .merge(mcp::mcp_router(state))
        .layer(DefaultBodyLimit::max(config.max_body_bytes));

    let addr: SocketAddr = config.bind.parse()?;
    info!(%addr, "Vector Collector listening");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
