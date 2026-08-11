use crate::config::Config;
use crate::db::Db;
use crate::embeddings::EmbeddingClient;
use crate::ingest::{IngestQueue, SharedIngestStats};
use crate::rate_limit::RateLimiter;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub db: Db,
    pub config: Arc<Config>,
    pub ingest: IngestQueue,
    pub ingest_stats: SharedIngestStats,
    pub rate_limiter: RateLimiter,
    pub embeddings: Option<EmbeddingClient>,
}
