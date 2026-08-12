use crate::config::Config;
use crate::db::Db;
use crate::ingest::{IngestQueue, SharedIngestStats};
use crate::rate_limit::RateLimiter;
use crate::settings::{SharedEmbeddings, SharedRuntimeSettings};
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub db: Db,
    pub config: Arc<Config>,
    pub settings: SharedRuntimeSettings,
    pub ingest: IngestQueue,
    pub ingest_stats: SharedIngestStats,
    pub rate_limiter: RateLimiter,
    pub embeddings: SharedEmbeddings,
}
