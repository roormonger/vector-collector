use std::env;
use std::path::{Path, PathBuf};

/// Fixed data directory (Docker volume mount). Not configurable.
pub const DATA_DIR: &str = "/data";

pub fn database_path() -> PathBuf {
    Path::new(DATA_DIR).join("logdb.sqlite")
}

/// Process/deploy config from env. Operational knobs live in RuntimeSettings (SQLite + GUI).
#[derive(Debug, Clone)]
pub struct Config {
    pub bind: String,
    pub admin_username: String,
    pub admin_password: String,
    pub session_secret: String,
    pub bootstrap_ingest_key: Option<String>,
    pub bootstrap_query_key: Option<String>,
    /// First-boot defaults when a settings key is missing from SQLite.
    pub defaults: SettingDefaults,
}

#[derive(Debug, Clone)]
pub struct SettingDefaults {
    pub retention_days: u32,
    pub max_events: Option<u64>,
    pub write_queue_capacity: usize,
    pub max_body_bytes: usize,
    pub per_key_rps: f64,
    pub public_base_url: String,
    pub embeddings_base_url: Option<String>,
    pub embeddings_model: Option<String>,
    pub embeddings_api_key: Option<String>,
    pub embedding_dim: usize,
    pub embed_sample_rate: f64,
}

impl Config {
    pub fn from_env() -> Self {
        let admin_username = env_nonempty("ADMIN_USERNAME").unwrap_or_else(|| "admin".into());
        let admin_password = env_nonempty("ADMIN_PASSWORD").unwrap_or_else(|| "admin".into());

        Self {
            bind: env_or("BIND", "0.0.0.0:8080"),
            admin_username,
            admin_password,
            session_secret: env_or("SESSION_SECRET", "dev-session-secret-change-me"),
            bootstrap_ingest_key: env_nonempty("BOOTSTRAP_INGEST_KEY"),
            bootstrap_query_key: env_nonempty("BOOTSTRAP_QUERY_KEY"),
            defaults: SettingDefaults {
                retention_days: env_parse("RETENTION_DAYS", 14),
                max_events: env::var("MAX_EVENTS")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .filter(|&n: &u64| n > 0),
                write_queue_capacity: env_parse("WRITE_QUEUE_CAPACITY", 64),
                max_body_bytes: env_parse("MAX_BODY_BYTES", 10 * 1024 * 1024),
                per_key_rps: env_parse("PER_KEY_RPS", 50.0),
                public_base_url: env_or("PUBLIC_BASE_URL", "http://localhost:8080"),
                embeddings_base_url: env_nonempty("EMBEDDINGS_BASE_URL"),
                embeddings_model: env_nonempty("EMBEDDINGS_MODEL"),
                embeddings_api_key: env_nonempty("EMBEDDINGS_API_KEY"),
                embedding_dim: env_parse("EMBEDDING_DIM", 1536),
                embed_sample_rate: env_parse("EMBED_SAMPLE_RATE", 0.02),
            },
        }
    }
}

fn env_or(key: &str, default: &str) -> String {
    env::var(key).unwrap_or_else(|_| default.to_string())
}

fn env_nonempty(key: &str) -> Option<String> {
    env::var(key)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn env_parse<T: std::str::FromStr>(key: &str, default: T) -> T {
    env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}
