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
mod settings;
mod state;
mod vector_presets;
mod workers;

use crate::auth::{ensure_bootstrap_key, sync_admin_credentials, KeyScopes};
use crate::config::{database_path, Config, DATA_DIR};
use crate::db::open_db;
use crate::ingest::{spawn_writer, IngestQueue, IngestStats};
use crate::rate_limit::RateLimiter;
use crate::settings::load_shared;
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
    std::fs::create_dir_all(DATA_DIR)?;

    let db = open_db(&database_path())?;
    sync_admin_credentials(&db, &config.admin_username, &config.admin_password)?;

    let (runtime_settings, embeddings) = load_shared(&db, &config.defaults)?;
    let applied = runtime_settings.read().clone();
    if embeddings.read().is_some() {
        info!("embeddings client enabled");
    } else {
        info!("embeddings disabled (configure in Settings → Semantic search)");
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

    let (ingest, rx) = IngestQueue::new(applied.write_queue_capacity);
    spawn_writer(rx, db.clone());

    workers::spawn_workers(db.clone(), embeddings.clone());

    let state = AppState {
        db,
        config: Arc::new(config.clone()),
        settings: runtime_settings,
        ingest,
        ingest_stats: Arc::new(IngestStats::default()),
        rate_limiter: RateLimiter::new(applied.per_key_rps),
        embeddings,
    };

    let web_dir = std::env::var("WEB_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("web/dist"));

    let max_body = applied.max_body_bytes;
    let app = routes::app_router(state.clone(), Some(web_dir))
        .merge(mcp::mcp_router(state))
        .layer(DefaultBodyLimit::max(max_body));

    let addr: SocketAddr = config.bind.parse()?;
    info!(%addr, data_dir = DATA_DIR, "Vector Collector listening");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
