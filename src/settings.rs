use crate::config::SettingDefaults;
use crate::db::{setting_get, setting_set, setting_set_if_absent, Db};
use crate::embeddings::EmbeddingClient;
use anyhow::Result;
use parking_lot::RwLock;
use rusqlite::Connection;
use serde::Serialize;
use std::sync::Arc;

#[derive(Debug, Clone, Serialize)]
pub struct RuntimeSettings {
    pub retention_days: u32,
    pub max_events: Option<u64>,
    pub write_queue_capacity: usize,
    pub max_body_bytes: usize,
    pub per_key_rps: f64,
    pub public_base_url: String,
    pub embeddings_base_url: Option<String>,
    pub embeddings_model: Option<String>,
    #[serde(skip_serializing)]
    pub embeddings_api_key: Option<String>,
    pub embedding_dim: usize,
    pub embed_sample_rate: f64,
}

impl RuntimeSettings {
    pub fn embeddings_enabled(&self) -> bool {
        self.embeddings_base_url
            .as_ref()
            .map(|s| !s.is_empty())
            .unwrap_or(false)
            && self
                .embeddings_model
                .as_ref()
                .map(|s| !s.is_empty())
                .unwrap_or(false)
    }

    pub fn embedding_client(&self) -> Option<EmbeddingClient> {
        EmbeddingClient::from_parts(
            self.embeddings_base_url.clone()?,
            self.embeddings_model.clone()?,
            self.embeddings_api_key.clone(),
            self.embedding_dim,
        )
    }
}

pub type SharedRuntimeSettings = Arc<RwLock<RuntimeSettings>>;
pub type SharedEmbeddings = Arc<RwLock<Option<EmbeddingClient>>>;

/// Insert env/defaults into SQLite only when the key is missing (never overwrite GUI values).
pub fn seed_settings(conn: &Connection, defaults: &SettingDefaults) -> Result<()> {
    setting_set_if_absent(conn, "retention_days", &defaults.retention_days.to_string())?;
    if let Some(max) = defaults.max_events {
        setting_set_if_absent(conn, "max_events", &max.to_string())?;
    }
    setting_set_if_absent(
        conn,
        "write_queue_capacity",
        &defaults.write_queue_capacity.to_string(),
    )?;
    setting_set_if_absent(conn, "max_body_bytes", &defaults.max_body_bytes.to_string())?;
    setting_set_if_absent(conn, "per_key_rps", &defaults.per_key_rps.to_string())?;
    setting_set_if_absent(conn, "public_base_url", &defaults.public_base_url)?;
    if let Some(v) = &defaults.embeddings_base_url {
        setting_set_if_absent(conn, "embeddings_base_url", v)?;
    }
    if let Some(v) = &defaults.embeddings_model {
        setting_set_if_absent(conn, "embeddings_model", v)?;
    }
    if let Some(v) = &defaults.embeddings_api_key {
        setting_set_if_absent(conn, "embeddings_api_key", v)?;
    }
    setting_set_if_absent(conn, "embedding_dim", &defaults.embedding_dim.to_string())?;
    setting_set_if_absent(
        conn,
        "embed_sample_rate",
        &defaults.embed_sample_rate.to_string(),
    )?;
    Ok(())
}

pub fn load_settings(conn: &Connection, defaults: &SettingDefaults) -> Result<RuntimeSettings> {
    Ok(RuntimeSettings {
        retention_days: setting_get(conn, "retention_days")?
            .and_then(|s| s.parse().ok())
            .unwrap_or(defaults.retention_days),
        max_events: setting_get(conn, "max_events")?
            .and_then(|s| s.parse().ok())
            .or(defaults.max_events),
        write_queue_capacity: setting_get(conn, "write_queue_capacity")?
            .and_then(|s| s.parse().ok())
            .unwrap_or(defaults.write_queue_capacity)
            .max(1),
        max_body_bytes: setting_get(conn, "max_body_bytes")?
            .and_then(|s| s.parse().ok())
            .unwrap_or(defaults.max_body_bytes)
            .max(1024),
        per_key_rps: setting_get(conn, "per_key_rps")?
            .and_then(|s| s.parse().ok())
            .unwrap_or(defaults.per_key_rps)
            .max(1.0),
        public_base_url: setting_get(conn, "public_base_url")?
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| defaults.public_base_url.clone()),
        embeddings_base_url: setting_get(conn, "embeddings_base_url")?
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .or_else(|| defaults.embeddings_base_url.clone()),
        embeddings_model: setting_get(conn, "embeddings_model")?
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .or_else(|| defaults.embeddings_model.clone()),
        embeddings_api_key: setting_get(conn, "embeddings_api_key")?
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .or_else(|| defaults.embeddings_api_key.clone()),
        embedding_dim: setting_get(conn, "embedding_dim")?
            .and_then(|s| s.parse().ok())
            .unwrap_or(defaults.embedding_dim)
            .max(1),
        embed_sample_rate: setting_get(conn, "embed_sample_rate")?
            .and_then(|s| s.parse().ok())
            .unwrap_or(defaults.embed_sample_rate)
            .clamp(0.0, 1.0),
    })
}

pub fn persist_settings(conn: &Connection, s: &RuntimeSettings) -> Result<()> {
    setting_set(conn, "retention_days", &s.retention_days.to_string())?;
    match s.max_events {
        Some(n) => setting_set(conn, "max_events", &n.to_string())?,
        None => {
            let _ = conn.execute("DELETE FROM app_settings WHERE key = 'max_events'", []);
        }
    }
    setting_set(
        conn,
        "write_queue_capacity",
        &s.write_queue_capacity.to_string(),
    )?;
    setting_set(conn, "max_body_bytes", &s.max_body_bytes.to_string())?;
    setting_set(conn, "per_key_rps", &s.per_key_rps.to_string())?;
    setting_set(conn, "public_base_url", &s.public_base_url)?;
    match &s.embeddings_base_url {
        Some(v) if !v.is_empty() => setting_set(conn, "embeddings_base_url", v)?,
        _ => {
            let _ = conn.execute(
                "DELETE FROM app_settings WHERE key = 'embeddings_base_url'",
                [],
            );
        }
    }
    match &s.embeddings_model {
        Some(v) if !v.is_empty() => setting_set(conn, "embeddings_model", v)?,
        _ => {
            let _ = conn.execute("DELETE FROM app_settings WHERE key = 'embeddings_model'", []);
        }
    }
    match &s.embeddings_api_key {
        Some(v) if !v.is_empty() => setting_set(conn, "embeddings_api_key", v)?,
        _ => {
            let _ = conn.execute(
                "DELETE FROM app_settings WHERE key = 'embeddings_api_key'",
                [],
            );
        }
    }
    setting_set(conn, "embedding_dim", &s.embedding_dim.to_string())?;
    setting_set(conn, "embed_sample_rate", &s.embed_sample_rate.to_string())?;
    Ok(())
}

pub fn load_shared(db: &Db, defaults: &SettingDefaults) -> Result<(SharedRuntimeSettings, SharedEmbeddings)> {
    let conn = db.lock();
    seed_settings(&conn, defaults)?;
    let settings = load_settings(&conn, defaults)?;
    let embeddings = settings.embedding_client();
    Ok((
        Arc::new(RwLock::new(settings)),
        Arc::new(RwLock::new(embeddings)),
    ))
}
