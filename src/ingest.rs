use crate::auth::ApiKeyRecord;
use crate::db::Db;
use crate::error::{AppError, AppResult};
use chrono::{DateTime, Utc};
use regex::Regex;
use rusqlite::{params, Transaction};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use std::sync::OnceLock;
use tokio::sync::{mpsc, oneshot};
use ulid::Ulid;

#[derive(Debug, Clone)]
pub struct NormalizedEvent {
    pub id: String,
    pub ts: String,
    pub ingested_at: String,
    pub host: Option<String>,
    pub container_name: Option<String>,
    pub container_id: Option<String>,
    pub image: Option<String>,
    pub stream: Option<String>,
    pub source_type: Option<String>,
    pub agent_id: Option<String>,
    pub trace_id: Option<String>,
    pub request_id: Option<String>,
    pub message: String,
    pub labels_json: String,
    pub raw_json: String,
    pub content_hash: String,
    pub should_embed: bool,
}

pub struct IngestBatch {
    pub key: ApiKeyRecord,
    pub events: Vec<NormalizedEvent>,
    pub respond: oneshot::Sender<Result<usize, String>>,
}

#[derive(Clone)]
pub struct IngestQueue {
    tx: mpsc::Sender<IngestBatch>,
}

impl IngestQueue {
    pub fn new(capacity: usize, db: Db, embed_sample_rate: f64) -> (Self, mpsc::Receiver<IngestBatch>) {
        let (tx, rx) = mpsc::channel(capacity);
        let _ = (db, embed_sample_rate); // writer started separately
        (Self { tx }, rx)
    }

    pub fn depth_approx(&self) -> usize {
        self.tx.max_capacity().saturating_sub(self.tx.capacity())
    }

    pub async fn enqueue(&self, batch: IngestBatch) -> AppResult<()> {
        match self.tx.try_send(batch) {
            Ok(()) => Ok(()),
            Err(mpsc::error::TrySendError::Full(_batch)) => Err(AppError::TooManyRequests {
                retry_after_secs: 1,
            }),
            Err(mpsc::error::TrySendError::Closed(_)) => {
                Err(AppError::Unavailable("ingest writer stopped".into()))
            }
        }
    }
}

pub fn spawn_writer(mut rx: mpsc::Receiver<IngestBatch>, db: Db) {
    tokio::task::spawn_blocking(move || {
        while let Some(batch) = rx.blocking_recv() {
            let result = write_batch(&db, &batch);
            let _ = batch.respond.send(result);
        }
    });
}

fn write_batch(db: &Db, batch: &IngestBatch) -> Result<usize, String> {
    use std::collections::HashMap;

    let mut conn = db.lock();
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    let mut written = 0usize;
    let mut written_by_agent: HashMap<String, i64> = HashMap::new();
    let mut seen_agents: HashMap<String, ()> = HashMap::new();
    let now = Utc::now().to_rfc3339();

    // Wizard agents: one agent per ingest key with a stored secret → force hostname.
    let dedicated = dedicated_agent(&tx, &batch.key.id).map_err(|e| e.to_string())?;

    for mut event in batch.events.clone() {
        let (host, agent_id) = if let Some((id, hostname)) = &dedicated {
            (hostname.clone(), id.clone())
        } else {
            let host = normalize_host(event.host.as_deref());
            let agent_id = ensure_agent(&tx, &batch.key, &host).map_err(|e| e.to_string())?;
            (host, agent_id)
        };
        event.host = Some(host);
        event.agent_id = Some(agent_id.clone());
        seen_agents.insert(agent_id.clone(), ());

        let inserted = insert_event(&tx, &event).map_err(|e| e.to_string())?;
        if inserted {
            written += 1;
            *written_by_agent.entry(agent_id).or_insert(0) += 1;
            if event.should_embed {
                enqueue_embed(&tx, &event).map_err(|e| e.to_string())?;
            }
        }
    }

    for agent_id in seen_agents.keys() {
        let count = written_by_agent.get(agent_id).copied().unwrap_or(0);
        tx.execute(
            "UPDATE agents SET last_seen_at = ?1, events_ingested = events_ingested + ?2 WHERE id = ?3",
            params![now, count, agent_id],
        )
        .map_err(|e| e.to_string())?;
    }

    tx.commit().map_err(|e| e.to_string())?;
    Ok(written)
}

fn normalize_host(host: Option<&str>) -> String {
    match host.map(str::trim).filter(|s| !s.is_empty()) {
        Some(h) => h.to_string(),
        None => "unknown".to_string(),
    }
}

/// Wizard-created agents store the raw key secret; use that agent and its name as host.
fn dedicated_agent(tx: &Transaction<'_>, api_key_id: &str) -> anyhow::Result<Option<(String, String)>> {
    let row: Result<(String, String), _> = tx.query_row(
        "SELECT a.id, a.name
         FROM agents a
         INNER JOIN api_keys k ON k.id = a.api_key_id
         WHERE a.api_key_id = ?1 AND k.secret IS NOT NULL AND trim(k.secret) != ''
         LIMIT 1",
        params![api_key_id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    );
    match row {
        Ok(v) => Ok(Some(v)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

fn ensure_agent(tx: &Transaction<'_>, key: &ApiKeyRecord, host: &str) -> anyhow::Result<String> {
    let existing: Option<String> = tx
        .query_row(
            "SELECT id FROM agents WHERE api_key_id = ?1 AND host_hint = ?2 LIMIT 1",
            params![key.id, host],
            |r| r.get(0),
        )
        .ok();
    if let Some(id) = existing {
        return Ok(id);
    }

    let id = Ulid::new().to_string();
    let display_name = format!("{} @ {}", key.name, host);
    let now = Utc::now().to_rfc3339();
    match tx.execute(
        "INSERT INTO agents(id, name, api_key_id, host_hint, last_seen_at, events_ingested, created_at)
         VALUES(?1, ?2, ?3, ?4, ?5, 0, ?5)",
        params![id, display_name, key.id, host, now],
    ) {
        Ok(_) => Ok(id),
        Err(rusqlite::Error::SqliteFailure(err, _))
            if err.code == rusqlite::ErrorCode::ConstraintViolation =>
        {
            // Race-safe: another writer inserted the same key+host
            let id: String = tx.query_row(
                "SELECT id FROM agents WHERE api_key_id = ?1 AND host_hint = ?2 LIMIT 1",
                params![key.id, host],
                |r| r.get(0),
            )?;
            Ok(id)
        }
        Err(e) => Err(e.into()),
    }
}

fn insert_event(tx: &Transaction<'_>, event: &NormalizedEvent) -> anyhow::Result<bool> {
    let changed = tx.execute(
        "INSERT OR IGNORE INTO log_events(
            id, ts, ingested_at, host, container_name, container_id, image, stream, source_type,
            agent_id, trace_id, request_id, message, labels_json, raw_json, content_hash
         ) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16)",
        params![
            event.id,
            event.ts,
            event.ingested_at,
            event.host,
            event.container_name,
            event.container_id,
            event.image,
            event.stream,
            event.source_type,
            event.agent_id,
            event.trace_id,
            event.request_id,
            event.message,
            event.labels_json,
            event.raw_json,
            event.content_hash,
        ],
    )?;
    Ok(changed > 0)
}

fn enqueue_embed(tx: &Transaction<'_>, event: &NormalizedEvent) -> anyhow::Result<()> {
    tx.execute(
        "INSERT OR IGNORE INTO embed_queue(event_id, message, enqueued_at, attempts)
         VALUES(?1, ?2, ?3, 0)",
        params![event.id, event.message, Utc::now().to_rfc3339()],
    )?;
    Ok(())
}

pub fn normalize_events(
    payload: Value,
    embed_sample_rate: f64,
) -> AppResult<Vec<NormalizedEvent>> {
    let arr = match payload {
        Value::Array(a) => a,
        Value::Object(o) => {
            if let Some(Value::Array(a)) = o.get("events") {
                a.clone()
            } else {
                return Err(AppError::BadRequest(
                    "body must be a JSON array of Vector events".into(),
                ));
            }
        }
        _ => {
            return Err(AppError::BadRequest(
                "body must be a JSON array of Vector events".into(),
            ))
        }
    };

    let now = Utc::now();
    let mut out = Vec::with_capacity(arr.len());
    for raw in arr {
        let Value::Object(map) = raw else {
            continue;
        };
        out.push(normalize_one(&map, now, embed_sample_rate)?);
    }
    if out.is_empty() {
        return Err(AppError::BadRequest("no events in batch".into()));
    }
    Ok(out)
}

fn normalize_one(
    map: &Map<String, Value>,
    now: DateTime<Utc>,
    embed_sample_rate: f64,
) -> AppResult<NormalizedEvent> {
    let message = string_field(map, "message").unwrap_or_default();
    let ts = parse_ts(map).unwrap_or(now).to_rfc3339();
    let host = string_field(map, "host");
    let container_name = string_field(map, "container_name");
    let container_id = string_field(map, "container_id");
    let image = string_field(map, "image");
    let stream = string_field(map, "stream");
    let source_type = string_field(map, "source_type");
    let labels = map.get("label").cloned().unwrap_or(Value::Object(Map::new()));
    let labels_json = serde_json::to_string(&labels).unwrap_or_else(|_| "{}".into());
    let (trace_id, request_id) = extract_correlation(map, &message, &labels);
    let raw_json = serde_json::to_string(&Value::Object(map.clone()))
        .map_err(|e| AppError::BadRequest(e.to_string()))?;

    let mut hasher = Sha256::new();
    hasher.update(ts.as_bytes());
    hasher.update(host.as_deref().unwrap_or("").as_bytes());
    hasher.update(container_id.as_deref().unwrap_or("").as_bytes());
    hasher.update(message.as_bytes());
    let content_hash = hex::encode(hasher.finalize());

    let should_embed = should_embed_event(stream.as_deref(), &message, embed_sample_rate);

    Ok(NormalizedEvent {
        id: Ulid::new().to_string(),
        ts,
        ingested_at: now.to_rfc3339(),
        host,
        container_name,
        container_id,
        image,
        stream,
        source_type,
        agent_id: None,
        trace_id,
        request_id,
        message,
        labels_json,
        raw_json,
        content_hash,
        should_embed,
    })
}

fn string_field(map: &Map<String, Value>, key: &str) -> Option<String> {
    match map.get(key)? {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

fn parse_ts(map: &Map<String, Value>) -> Option<DateTime<Utc>> {
    let s = string_field(map, "timestamp")?;
    DateTime::parse_from_rfc3339(&s)
        .ok()
        .map(|d| d.with_timezone(&Utc))
}

fn extract_correlation(
    map: &Map<String, Value>,
    message: &str,
    labels: &Value,
) -> (Option<String>, Option<String>) {
    let mut trace = string_field(map, "trace_id").or_else(|| string_field(map, "traceId"));
    let mut request = string_field(map, "request_id").or_else(|| string_field(map, "requestId"));

    if let Value::Object(l) = labels {
        if trace.is_none() {
            trace = string_field(l, "trace_id").or_else(|| string_field(l, "traceId"));
        }
        if request.is_none() {
            request = string_field(l, "request_id").or_else(|| string_field(l, "requestId"));
        }
    }

    static TRACE_RE: OnceLock<Regex> = OnceLock::new();
    static REQ_RE: OnceLock<Regex> = OnceLock::new();
    let trace_re = TRACE_RE.get_or_init(|| {
        Regex::new(r#"(?i)\b(?:trace[_-]?id|traceid)[=: ]+([a-f0-9-]{8,})\b"#).unwrap()
    });
    let req_re = REQ_RE.get_or_init(|| {
        Regex::new(r#"(?i)\b(?:request[_-]?id|reqid|requestid)[=: ]+([a-zA-Z0-9-]{6,})\b"#).unwrap()
    });

    if trace.is_none() {
        if let Some(c) = trace_re.captures(message) {
            trace = c.get(1).map(|m| m.as_str().to_string());
        }
    }
    if request.is_none() {
        if let Some(c) = req_re.captures(message) {
            request = c.get(1).map(|m| m.as_str().to_string());
        }
    }

    (trace, request)
}

fn should_embed_event(stream: Option<&str>, message: &str, sample_rate: f64) -> bool {
    let msg = message.to_ascii_lowercase();
    if message.trim().is_empty() || message.len() < 8 {
        return false;
    }
    if msg.contains("healthcheck") || msg.contains("healthz") || msg.contains("/ready") {
        return false;
    }
    if stream == Some("stderr") {
        return true;
    }
    if msg.contains("error") || msg.contains("exception") || msg.contains("panic") || msg.contains("warn")
    {
        return true;
    }
    if sample_rate <= 0.0 {
        return false;
    }
    use rand::Rng;
    rand::thread_rng().gen::<f64>() < sample_rate
}

#[derive(Default)]
pub struct IngestStats {
    pub accepted: std::sync::atomic::AtomicU64,
    pub rejected_429: std::sync::atomic::AtomicU64,
    pub written: std::sync::atomic::AtomicU64,
}

pub type SharedIngestStats = Arc<IngestStats>;
