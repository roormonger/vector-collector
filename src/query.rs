use crate::config::Config;
use crate::db::Db;
use crate::error::{AppError, AppResult};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use rusqlite::{params, params_from_iter, types::Value as SqlValue};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use utoipa::ToSchema;

#[derive(Debug, Deserialize, Default, ToSchema)]
pub struct SearchRequest {
    pub filters: Option<Filters>,
    /// Full-text keywords (FTS5)
    pub text: Option<String>,
    /// Optional semantic query when embeddings are enabled
    pub semantic_query: Option<String>,
    pub limit: Option<usize>,
    pub cursor: Option<String>,
    pub fields: Option<Vec<String>>,
    pub include_raw: Option<bool>,
}

#[derive(Debug, Deserialize, Default, Clone, ToSchema)]
pub struct Filters {
    /// RFC3339
    pub ts_from: Option<String>,
    /// RFC3339
    pub ts_to: Option<String>,
    pub hosts: Option<Vec<String>>,
    pub containers: Option<Vec<String>>,
    pub streams: Option<Vec<String>>,
    pub agent_ids: Option<Vec<String>>,
    pub trace_id: Option<String>,
    pub request_id: Option<String>,
    pub label_equals: Option<BTreeMap<String, String>>,
}

#[derive(Debug, Deserialize, Default, ToSchema)]
pub struct FacetsRequest {
    pub filters: Option<Filters>,
}

#[derive(Debug, Serialize)]
pub struct SearchResponse {
    pub events: Vec<Value>,
    pub next_cursor: Option<String>,
    pub count: usize,
}

#[derive(Clone)]
struct Cursor {
    ts: String,
    id: String,
}

fn encode_cursor(c: &Cursor) -> String {
    URL_SAFE_NO_PAD.encode(format!("{}|{}", c.ts, c.id))
}

fn decode_cursor(s: &str) -> AppResult<Cursor> {
    let bytes = URL_SAFE_NO_PAD
        .decode(s)
        .map_err(|_| AppError::BadRequest("invalid cursor".into()))?;
    let text = String::from_utf8(bytes)
        .map_err(|_| AppError::BadRequest("invalid cursor".into()))?;
    let (ts, id) = text
        .split_once('|')
        .ok_or_else(|| AppError::BadRequest("invalid cursor".into()))?;
    Ok(Cursor {
        ts: ts.to_string(),
        id: id.to_string(),
    })
}

pub fn schema_document(config: &Config) -> Value {
    json!({
        "name": "vector-collector",
        "description": "Search logs ingested into Vector Collector across all machines. Prefer facets → search → context. Timestamps are UTC RFC3339.",
        "recommended_loop": [
            "Call logs_facets (or POST /v1/query/facets) with a time window to see busy hosts/containers",
            "Call logs_search with narrowed filters + keyword text from the user's natural language question",
            "Call logs_context on interesting hits to see surrounding lines",
            "Call logs_get only when full raw payload is needed"
        ],
        "fields": [
            "id", "ts", "host", "container_name", "container_id", "image", "stream",
            "source_type", "agent_id", "trace_id", "request_id", "message", "labels", "raw"
        ],
        "filters": {
            "ts_from": "RFC3339",
            "ts_to": "RFC3339",
            "hosts": ["string"],
            "containers": ["string"],
            "streams": ["stdout", "stderr"],
            "agent_ids": ["string"],
            "trace_id": "string",
            "request_id": "string",
            "label_equals": {"key": "value"}
        },
        "semantic_search_enabled": config.embeddings_enabled(),
        "defaults": {
            "limit": 25,
            "message_truncate_chars": 500
        }
    })
}

pub fn facets(db: &Db, filters: &Filters) -> AppResult<Value> {
    let conn = db.lock();
    let (where_sql, binds) = build_where(filters, None, None)?;
    let dims = ["host", "container_name", "image", "stream", "agent_id"];
    let mut out = serde_json::Map::new();
    for dim in dims {
        let sql = format!(
            "SELECT coalesce({dim}, '') AS k, COUNT(*) AS c
             FROM log_events {where_sql}
             GROUP BY k ORDER BY c DESC LIMIT 50"
        );
        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| AppError::Internal(e.into()))?;
        let rows = stmt
            .query_map(params_from_iter(binds.iter().cloned()), |row| {
                Ok(json!({
                    "value": row.get::<_, String>(0)?,
                    "count": row.get::<_, i64>(1)?,
                }))
            })
            .map_err(|e| AppError::Internal(e.into()))?;
        let mut items = Vec::new();
        for r in rows {
            items.push(r.map_err(|e| AppError::Internal(e.into()))?);
        }
        out.insert(dim.to_string(), Value::Array(items));
    }
    Ok(Value::Object(out))
}

pub fn search(
    db: &Db,
    req: &SearchRequest,
    query_embedding: Option<Vec<f32>>,
) -> AppResult<SearchResponse> {
    let limit = req.limit.unwrap_or(25).clamp(1, 200);
    let filters = req.filters.clone().unwrap_or_default();
    let cursor = match &req.cursor {
        Some(c) => Some(decode_cursor(c)?),
        None => None,
    };

    let conn = db.lock();
    let (where_sql, mut binds) = build_where(&filters, req.text.as_deref(), cursor.as_ref())?;

    let fts_q = req
        .text
        .as_ref()
        .map(|t| fts_query(t.trim()))
        .filter(|q| !q.is_empty());
    let use_fts = fts_q.is_some();

    let sql = if use_fts {
        format!(
            "SELECT e.id, e.ts, e.host, e.container_name, e.container_id, e.image, e.stream,
                    e.source_type, e.agent_id, e.trace_id, e.request_id, e.message, e.labels_json, e.raw_json,
                    bm25(log_events_fts) AS rank
             FROM log_events e
             JOIN log_events_fts ON log_events_fts.rowid = e.rowid
             {where_sql}
             ORDER BY e.ts DESC, e.id DESC
             LIMIT ?{}",
            binds.len() + 1
        )
    } else {
        format!(
            "SELECT e.id, e.ts, e.host, e.container_name, e.container_id, e.image, e.stream,
                    e.source_type, e.agent_id, e.trace_id, e.request_id, e.message, e.labels_json, e.raw_json,
                    0.0 AS rank
             FROM log_events e
             {where_sql}
             ORDER BY e.ts DESC, e.id DESC
             LIMIT ?{}",
            binds.len() + 1
        )
    };

    binds.push(SqlValue::Integer((limit as i64) + 1));

    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| AppError::Internal(e.into()))?;
    let rows = stmt
        .query_map(params_from_iter(binds.iter().cloned()), |row| {
            Ok(EventRow {
                id: row.get(0)?,
                ts: row.get(1)?,
                host: row.get(2)?,
                container_name: row.get(3)?,
                container_id: row.get(4)?,
                image: row.get(5)?,
                stream: row.get(6)?,
                source_type: row.get(7)?,
                agent_id: row.get(8)?,
                trace_id: row.get(9)?,
                request_id: row.get(10)?,
                message: row.get(11)?,
                labels_json: row.get(12)?,
                raw_json: row.get(13)?,
                rank: row.get(14)?,
                semantic: 0.0,
            })
        })
        .map_err(|e| AppError::Internal(e.into()))?;

    let mut events = Vec::new();
    for r in rows {
        events.push(r.map_err(|e| AppError::Internal(e.into()))?);
    }

    if let Some(qemb) = query_embedding.as_ref() {
        let extras = semantic_boost(&conn, &mut events, qemb, &filters)?;
        events.extend(extras);
        events.sort_by(|a, b| {
            b.score()
                .partial_cmp(&a.score())
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| b.ts.cmp(&a.ts))
        });
        events.dedup_by(|a, b| a.id == b.id);
    }

    let mut next_cursor = None;
    if events.len() > limit {
        let last = &events[limit - 1];
        next_cursor = Some(encode_cursor(&Cursor {
            ts: last.ts.clone(),
            id: last.id.clone(),
        }));
        events.truncate(limit);
    }

    let include_raw = req.include_raw.unwrap_or(false);
    let fields = req.fields.clone();
    let out: Vec<Value> = events
        .into_iter()
        .map(|e| e.to_json(include_raw, fields.as_deref()))
        .collect();

    Ok(SearchResponse {
        count: out.len(),
        events: out,
        next_cursor,
    })
}

struct EventRow {
    id: String,
    ts: String,
    host: Option<String>,
    container_name: Option<String>,
    container_id: Option<String>,
    image: Option<String>,
    stream: Option<String>,
    source_type: Option<String>,
    agent_id: Option<String>,
    trace_id: Option<String>,
    request_id: Option<String>,
    message: String,
    labels_json: String,
    raw_json: String,
    rank: f64,
    semantic: f64,
}

impl EventRow {
    fn score(&self) -> f64 {
        // Lower bm25 is better in sqlite; invert roughly + semantic cosine
        let text_score = if self.rank == 0.0 {
            0.0
        } else {
            1.0 / (1.0 + self.rank.abs())
        };
        0.6 * text_score + 0.4 * self.semantic
    }

    fn to_json(&self, include_raw: bool, fields: Option<&[String]>) -> Value {
        let (message, truncated) = truncate_msg(&self.message, 500);
        let mut map = serde_json::Map::new();
        let put = |m: &mut serde_json::Map<String, Value>, k: &str, v: Value| {
            if let Some(fields) = fields {
                if !fields.iter().any(|f| f == k) {
                    return;
                }
            }
            m.insert(k.to_string(), v);
        };

        put(&mut map, "id", json!(self.id));
        put(&mut map, "ts", json!(self.ts));
        put(&mut map, "host", json!(self.host));
        put(&mut map, "container_name", json!(self.container_name));
        put(&mut map, "container_id", json!(self.container_id));
        put(&mut map, "image", json!(self.image));
        put(&mut map, "stream", json!(self.stream));
        put(&mut map, "source_type", json!(self.source_type));
        put(&mut map, "agent_id", json!(self.agent_id));
        put(&mut map, "trace_id", json!(self.trace_id));
        put(&mut map, "request_id", json!(self.request_id));
        put(&mut map, "message", json!(message));
        put(&mut map, "message_truncated", json!(truncated));
        put(
            &mut map,
            "labels",
            serde_json::from_str(&self.labels_json).unwrap_or(json!({})),
        );
        put(&mut map, "score", json!(self.score()));
        if include_raw {
            put(
                &mut map,
                "raw",
                serde_json::from_str(&self.raw_json).unwrap_or(json!({})),
            );
        }
        Value::Object(map)
    }
}

impl Default for EventRow {
    fn default() -> Self {
        Self {
            id: String::new(),
            ts: String::new(),
            host: None,
            container_name: None,
            container_id: None,
            image: None,
            stream: None,
            source_type: None,
            agent_id: None,
            trace_id: None,
            request_id: None,
            message: String::new(),
            labels_json: "{}".into(),
            raw_json: "{}".into(),
            rank: 0.0,
            semantic: 0.0,
        }
    }
}

fn truncate_msg(s: &str, max: usize) -> (String, bool) {
    if s.chars().count() <= max {
        (s.to_string(), false)
    } else {
        (s.chars().take(max).collect::<String>() + "…", true)
    }
}

fn build_where(
    filters: &Filters,
    text: Option<&str>,
    cursor: Option<&Cursor>,
) -> AppResult<(String, Vec<SqlValue>)> {
    let mut clauses = Vec::new();
    let mut binds = Vec::new();

    // When using FTS join, qualify columns with e.
    let prefix = "e.";

    if let Some(v) = &filters.ts_from {
        clauses.push(format!("{prefix}ts >= ?{}", binds.len() + 1));
        binds.push(SqlValue::Text(v.clone()));
    }
    if let Some(v) = &filters.ts_to {
        clauses.push(format!("{prefix}ts <= ?{}", binds.len() + 1));
        binds.push(SqlValue::Text(v.clone()));
    }
    push_in(&mut clauses, &mut binds, &format!("{prefix}host"), filters.hosts.as_ref());
    push_in(
        &mut clauses,
        &mut binds,
        &format!("{prefix}container_name"),
        filters.containers.as_ref(),
    );
    push_in(
        &mut clauses,
        &mut binds,
        &format!("{prefix}stream"),
        filters.streams.as_ref(),
    );
    push_in(
        &mut clauses,
        &mut binds,
        &format!("{prefix}agent_id"),
        filters.agent_ids.as_ref(),
    );
    if let Some(v) = &filters.trace_id {
        clauses.push(format!("{prefix}trace_id = ?{}", binds.len() + 1));
        binds.push(SqlValue::Text(v.clone()));
    }
    if let Some(v) = &filters.request_id {
        clauses.push(format!("{prefix}request_id = ?{}", binds.len() + 1));
        binds.push(SqlValue::Text(v.clone()));
    }
    if let Some(labels) = &filters.label_equals {
        for (k, v) in labels {
            clauses.push(format!(
                "json_extract({prefix}labels_json, ?{}) = ?{}",
                binds.len() + 1,
                binds.len() + 2
            ));
            binds.push(SqlValue::Text(format!("$.{k}")));
            binds.push(SqlValue::Text(v.clone()));
        }
    }
    if let Some(c) = cursor {
        clauses.push(format!(
            "({prefix}ts < ?{a} OR ({prefix}ts = ?{a} AND {prefix}id < ?{b}))",
            a = binds.len() + 1,
            b = binds.len() + 2
        ));
        binds.push(SqlValue::Text(c.ts.clone()));
        binds.push(SqlValue::Text(c.id.clone()));
    }
    if let Some(t) = text {
        let t = t.trim();
        if !t.is_empty() {
            let q = fts_query(t);
            if !q.is_empty() {
                clauses.push(format!("log_events_fts MATCH ?{}", binds.len() + 1));
                binds.push(SqlValue::Text(q));
            }
        }
    }

    let where_sql = if clauses.is_empty() {
        // Always alias as e for consistent SQL
        "WHERE 1=1".to_string()
    } else {
        format!("WHERE {}", clauses.join(" AND "))
    };

    // Non-FTS queries also need alias - handled by callers using `e`
    let _ = prefix;
    Ok((where_sql, binds))
}

fn push_in(
    clauses: &mut Vec<String>,
    binds: &mut Vec<SqlValue>,
    col: &str,
    values: Option<&Vec<String>>,
) {
    let Some(values) = values else { return };
    if values.is_empty() {
        return;
    }
    let mut parts = Vec::new();
    for v in values {
        parts.push(format!("?{}", binds.len() + 1));
        binds.push(SqlValue::Text(v.clone()));
    }
    clauses.push(format!("{col} IN ({})", parts.join(",")));
}

fn fts_query(text: &str) -> String {
    text.split_whitespace()
        .map(|t| {
            let cleaned: String = t.chars().filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_').collect();
            if cleaned.is_empty() {
                String::new()
            } else {
                format!("\"{cleaned}\"")
            }
        })
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" AND ")
}

fn semantic_boost(
    conn: &rusqlite::Connection,
    events: &mut [EventRow],
    query: &[f32],
    filters: &Filters,
) -> AppResult<Vec<EventRow>> {
    if events.is_empty() {
        return Ok(Vec::new());
    }
    // Score embeddings for events already in the candidate set
    let ids: Vec<String> = events.iter().map(|e| e.id.clone()).collect();
    let mut placeholders = Vec::new();
    let mut binds: Vec<SqlValue> = Vec::new();
    for id in &ids {
        placeholders.push(format!("?{}", binds.len() + 1));
        binds.push(SqlValue::Text(id.clone()));
    }
    let sql = format!(
        "SELECT event_id, embedding FROM log_embeddings WHERE event_id IN ({})",
        placeholders.join(",")
    );
    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| AppError::Internal(e.into()))?;
    let rows = stmt
        .query_map(params_from_iter(binds), |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, Vec<u8>>(1)?))
        })
        .map_err(|e| AppError::Internal(e.into()))?;

    let mut scores = BTreeMap::new();
    for r in rows {
        let (id, blob) = r.map_err(|e| AppError::Internal(e.into()))?;
        if let Some(vec) = blob_to_f32(&blob) {
            scores.insert(id, cosine(query, &vec));
        }
    }
    for e in events.iter_mut() {
        if let Some(s) = scores.get(&e.id) {
            e.semantic = *s;
        }
    }

    // Also pull top semantic neighbors in the same filter window (limited)
    let (where_sql, mut binds) = build_where(filters, None, None)?;
    let sql = format!(
        "SELECT e.id, e.ts, e.host, e.container_name, e.container_id, e.image, e.stream,
                e.source_type, e.agent_id, e.trace_id, e.request_id, e.message, e.labels_json, e.raw_json,
                emb.embedding
         FROM log_embeddings emb
         JOIN log_events e ON e.id = emb.event_id
         {where_sql}
         ORDER BY e.ts DESC
         LIMIT 200"
    );
    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| AppError::Internal(e.into()))?;
    // rewrite where to use e. — already does
    let _ = &mut binds;
    let rows = stmt
        .query_map(params_from_iter(binds), |row| {
            Ok((
                EventRow {
                    id: row.get(0)?,
                    ts: row.get(1)?,
                    host: row.get(2)?,
                    container_name: row.get(3)?,
                    container_id: row.get(4)?,
                    image: row.get(5)?,
                    stream: row.get(6)?,
                    source_type: row.get(7)?,
                    agent_id: row.get(8)?,
                    trace_id: row.get(9)?,
                    request_id: row.get(10)?,
                    message: row.get(11)?,
                    labels_json: row.get(12)?,
                    raw_json: row.get(13)?,
                    rank: 0.0,
                    semantic: 0.0,
                },
                row.get::<_, Vec<u8>>(14)?,
            ))
        })
        .map_err(|e| AppError::Internal(e.into()))?;

    let existing: std::collections::HashSet<String> =
        events.iter().map(|e| e.id.clone()).collect();
    let mut extras = Vec::new();
    for r in rows {
        let (mut er, blob) = r.map_err(|e| AppError::Internal(e.into()))?;
        if existing.contains(&er.id) {
            continue;
        }
        if let Some(vec) = blob_to_f32(&blob) {
            er.semantic = cosine(query, &vec);
            if er.semantic > 0.35 {
                extras.push(er);
            }
        }
    }
    extras.sort_by(|a, b| {
        b.semantic
            .partial_cmp(&a.semantic)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    extras.truncate(25);
    Ok(extras)
}

fn blob_to_f32(blob: &[u8]) -> Option<Vec<f32>> {
    if blob.len() % 4 != 0 {
        return None;
    }
    let mut out = Vec::with_capacity(blob.len() / 4);
    for chunk in blob.chunks_exact(4) {
        out.push(f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]));
    }
    Some(out)
}

fn cosine(a: &[f32], b: &[f32]) -> f64 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let mut dot = 0.0f64;
    let mut na = 0.0f64;
    let mut nb = 0.0f64;
    for i in 0..a.len() {
        let x = a[i] as f64;
        let y = b[i] as f64;
        dot += x * y;
        na += x * x;
        nb += y * y;
    }
    if na == 0.0 || nb == 0.0 {
        return 0.0;
    }
    dot / (na.sqrt() * nb.sqrt())
}

pub fn get_event(db: &Db, id: &str) -> AppResult<Value> {
    let conn = db.lock();
    let mut stmt = conn
        .prepare(
            "SELECT id, ts, host, container_name, container_id, image, stream, source_type,
                    agent_id, trace_id, request_id, message, labels_json, raw_json
             FROM log_events WHERE id = ?1",
        )
        .map_err(|e| AppError::Internal(e.into()))?;
    let mut rows = stmt
        .query(params![id])
        .map_err(|e| AppError::Internal(e.into()))?;
    let Some(row) = rows.next().map_err(|e| AppError::Internal(e.into()))? else {
        return Err(AppError::NotFound);
    };
    let er = EventRow {
        id: row.get(0).map_err(|e| AppError::Internal(e.into()))?,
        ts: row.get(1).map_err(|e| AppError::Internal(e.into()))?,
        host: row.get(2).map_err(|e| AppError::Internal(e.into()))?,
        container_name: row.get(3).map_err(|e| AppError::Internal(e.into()))?,
        container_id: row.get(4).map_err(|e| AppError::Internal(e.into()))?,
        image: row.get(5).map_err(|e| AppError::Internal(e.into()))?,
        stream: row.get(6).map_err(|e| AppError::Internal(e.into()))?,
        source_type: row.get(7).map_err(|e| AppError::Internal(e.into()))?,
        agent_id: row.get(8).map_err(|e| AppError::Internal(e.into()))?,
        trace_id: row.get(9).map_err(|e| AppError::Internal(e.into()))?,
        request_id: row.get(10).map_err(|e| AppError::Internal(e.into()))?,
        message: row.get(11).map_err(|e| AppError::Internal(e.into()))?,
        labels_json: row.get(12).map_err(|e| AppError::Internal(e.into()))?,
        raw_json: row.get(13).map_err(|e| AppError::Internal(e.into()))?,
        rank: 0.0,
        semantic: 0.0,
    };
    Ok(er.to_json(true, None))
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ContextRequest {
    pub id: String,
    pub before: Option<usize>,
    pub after: Option<usize>,
}

pub fn context(db: &Db, req: &ContextRequest) -> AppResult<Value> {
    let before = req.before.unwrap_or(20).clamp(0, 100);
    let after = req.after.unwrap_or(20).clamp(0, 100);
    let event = get_event(db, &req.id)?;
    let container = event
        .get("container_name")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let trace_id = event
        .get("trace_id")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());
    let ts = event
        .get("ts")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let conn = db.lock();
    let (clause, bind_extra) = if let Some(tid) = &trace_id {
        ("trace_id = ?1", tid.clone())
    } else if let Some(c) = &container {
        ("container_name = ?1", c.clone())
    } else {
        ("host = ?1", event.get("host").and_then(|v| v.as_str()).unwrap_or("").to_string())
    };

    let before_sql = format!(
        "SELECT id, ts, host, container_name, stream, message FROM log_events
         WHERE {clause} AND ts <= ?2 AND id != ?3
         ORDER BY ts DESC, id DESC LIMIT ?4"
    );
    let mut stmt = conn
        .prepare(&before_sql)
        .map_err(|e| AppError::Internal(e.into()))?;
    let before_rows = stmt
        .query_map(params![bind_extra, ts, req.id, before as i64], |row| {
            Ok(json!({
                "id": row.get::<_, String>(0)?,
                "ts": row.get::<_, String>(1)?,
                "host": row.get::<_, Option<String>>(2)?,
                "container_name": row.get::<_, Option<String>>(3)?,
                "stream": row.get::<_, Option<String>>(4)?,
                "message": truncate_msg(&row.get::<_, String>(5)?, 500).0,
            }))
        })
        .map_err(|e| AppError::Internal(e.into()))?;
    let mut before_events = Vec::new();
    for r in before_rows {
        before_events.push(r.map_err(|e| AppError::Internal(e.into()))?);
    }
    before_events.reverse();

    let after_sql = format!(
        "SELECT id, ts, host, container_name, stream, message FROM log_events
         WHERE {clause} AND ts >= ?2 AND id != ?3
         ORDER BY ts ASC, id ASC LIMIT ?4"
    );
    let mut stmt = conn
        .prepare(&after_sql)
        .map_err(|e| AppError::Internal(e.into()))?;
    let after_rows = stmt
        .query_map(params![bind_extra, ts, req.id, after as i64], |row| {
            Ok(json!({
                "id": row.get::<_, String>(0)?,
                "ts": row.get::<_, String>(1)?,
                "host": row.get::<_, Option<String>>(2)?,
                "container_name": row.get::<_, Option<String>>(3)?,
                "stream": row.get::<_, Option<String>>(4)?,
                "message": truncate_msg(&row.get::<_, String>(5)?, 500).0,
            }))
        })
        .map_err(|e| AppError::Internal(e.into()))?;
    let mut after_events = Vec::new();
    for r in after_rows {
        after_events.push(r.map_err(|e| AppError::Internal(e.into()))?);
    }

    Ok(json!({
        "event": event,
        "before": before_events,
        "after": after_events,
    }))
}

pub fn list_agents(db: &Db) -> AppResult<Vec<Value>> {
    let conn = db.lock();
    let mut stmt = conn
        .prepare(
            "SELECT a.id, a.name, a.api_key_id, a.host_hint, a.last_seen_at, a.events_ingested, a.created_at,
                    k.name, k.key_prefix,
                    CASE WHEN k.secret IS NOT NULL AND trim(k.secret) != '' THEN 1 ELSE 0 END
             FROM agents a
             LEFT JOIN api_keys k ON k.id = a.api_key_id
             ORDER BY coalesce(a.last_seen_at, a.created_at) DESC",
        )
        .map_err(|e| AppError::Internal(e.into()))?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, i64>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, Option<String>>(7)?,
                row.get::<_, Option<String>>(8)?,
                row.get::<_, i64>(9)? != 0,
            ))
        })
        .map_err(|e| AppError::Internal(e.into()))?;

    let now = chrono::Utc::now();
    let mut out = Vec::new();
    for r in rows {
        let (
            id,
            name,
            api_key_id,
            host_hint,
            last_seen_at,
            events_ingested,
            created_at,
            key_name,
            key_prefix,
            has_connect_secret,
        ) = r.map_err(|e| AppError::Internal(e.into()))?;

        let status = agent_status(&last_seen_at, now);
        // Wizard agents use name as hostname; legacy rows use Vector host_hint.
        let host = if has_connect_secret {
            name.clone()
        } else {
            host_hint.clone().unwrap_or_else(|| "unknown".into())
        };
        let recent_containers = top_containers_for_host(&conn, &host, 5)?;

        out.push(json!({
            "id": id,
            "name": name,
            "api_key_id": api_key_id,
            "host": host,
            "host_hint": host_hint,
            "last_seen_at": last_seen_at,
            "events_ingested": events_ingested,
            "created_at": created_at,
            "status": status,
            "key_name": key_name,
            "key_prefix": key_prefix,
            "has_connect_secret": has_connect_secret,
            "recent_containers": recent_containers,
        }));
    }
    Ok(out)
}

fn agent_status(last_seen_at: &Option<String>, now: chrono::DateTime<chrono::Utc>) -> &'static str {
    let Some(raw) = last_seen_at else {
        return "never";
    };
    let Ok(ts) = chrono::DateTime::parse_from_rfc3339(raw) else {
        return "unknown";
    };
    let age = now.signed_duration_since(ts.with_timezone(&chrono::Utc));
    if age < chrono::Duration::minutes(2) {
        "online"
    } else if age < chrono::Duration::minutes(15) {
        "stale"
    } else {
        "offline"
    }
}

fn top_containers_for_host(
    conn: &rusqlite::Connection,
    host: &str,
    limit: usize,
) -> AppResult<Vec<Value>> {
    let mut stmt = conn
        .prepare(
            "SELECT coalesce(container_name, '') AS c, COUNT(*) AS n
             FROM log_events
             WHERE host = ?1 AND container_name IS NOT NULL AND container_name != ''
             GROUP BY c
             ORDER BY n DESC
             LIMIT ?2",
        )
        .map_err(|e| AppError::Internal(e.into()))?;
    let rows = stmt
        .query_map(params![host, limit as i64], |row| {
            Ok(json!({
                "name": row.get::<_, String>(0)?,
                "count": row.get::<_, i64>(1)?,
            }))
        })
        .map_err(|e| AppError::Internal(e.into()))?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r.map_err(|e| AppError::Internal(e.into()))?);
    }
    Ok(out)
}

pub fn stats(db: &Db) -> AppResult<Value> {
    let conn = db.lock();
    let total: i64 = conn
        .query_row("SELECT COUNT(*) FROM log_events", [], |r| r.get(0))
        .map_err(|e| AppError::Internal(e.into()))?;
    let agents: i64 = conn
        .query_row("SELECT COUNT(*) FROM agents", [], |r| r.get(0))
        .map_err(|e| AppError::Internal(e.into()))?;
    let embed_pending: i64 = conn
        .query_row("SELECT COUNT(*) FROM embed_queue", [], |r| r.get(0))
        .map_err(|e| AppError::Internal(e.into()))?;
    Ok(json!({
        "events": total,
        "agents": agents,
        "embed_queue": embed_pending,
    }))
}
