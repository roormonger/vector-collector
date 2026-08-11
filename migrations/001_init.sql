PRAGMA journal_mode = WAL;
PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS app_settings (
    key TEXT PRIMARY KEY NOT NULL,
    value TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS api_keys (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    key_prefix TEXT NOT NULL,
    key_hash TEXT NOT NULL UNIQUE,
    scope_ingest INTEGER NOT NULL DEFAULT 0,
    scope_query INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    revoked_at TEXT
);

CREATE TABLE IF NOT EXISTS agents (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    api_key_id TEXT REFERENCES api_keys(id),
    host_hint TEXT,
    last_seen_at TEXT,
    events_ingested INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS log_events (
    id TEXT PRIMARY KEY NOT NULL,
    ts TEXT NOT NULL,
    ingested_at TEXT NOT NULL,
    host TEXT,
    container_name TEXT,
    container_id TEXT,
    image TEXT,
    stream TEXT,
    source_type TEXT,
    agent_id TEXT,
    trace_id TEXT,
    request_id TEXT,
    message TEXT NOT NULL,
    labels_json TEXT NOT NULL DEFAULT '{}',
    raw_json TEXT NOT NULL,
    content_hash TEXT
);

CREATE INDEX IF NOT EXISTS idx_log_events_ts_id ON log_events(ts, id);
CREATE INDEX IF NOT EXISTS idx_log_events_host_ts ON log_events(host, ts);
CREATE INDEX IF NOT EXISTS idx_log_events_container_ts ON log_events(container_name, ts);
CREATE INDEX IF NOT EXISTS idx_log_events_agent_ts ON log_events(agent_id, ts);
CREATE INDEX IF NOT EXISTS idx_log_events_trace ON log_events(trace_id);
CREATE INDEX IF NOT EXISTS idx_log_events_request ON log_events(request_id);
CREATE UNIQUE INDEX IF NOT EXISTS idx_log_events_hash
    ON log_events(agent_id, content_hash)
    WHERE agent_id IS NOT NULL AND content_hash IS NOT NULL;

CREATE VIRTUAL TABLE IF NOT EXISTS log_events_fts USING fts5(
    message,
    host,
    container_name,
    content='log_events',
    content_rowid='rowid'
);

CREATE TRIGGER IF NOT EXISTS log_events_ai AFTER INSERT ON log_events BEGIN
    INSERT INTO log_events_fts(rowid, message, host, container_name)
    VALUES (new.rowid, new.message, coalesce(new.host, ''), coalesce(new.container_name, ''));
END;

CREATE TRIGGER IF NOT EXISTS log_events_ad AFTER DELETE ON log_events BEGIN
    INSERT INTO log_events_fts(log_events_fts, rowid, message, host, container_name)
    VALUES ('delete', old.rowid, old.message, coalesce(old.host, ''), coalesce(old.container_name, ''));
END;

CREATE TABLE IF NOT EXISTS log_embeddings (
    event_id TEXT PRIMARY KEY NOT NULL REFERENCES log_events(id) ON DELETE CASCADE,
    dim INTEGER NOT NULL,
    embedding BLOB NOT NULL,
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS embed_queue (
    event_id TEXT PRIMARY KEY NOT NULL REFERENCES log_events(id) ON DELETE CASCADE,
    message TEXT NOT NULL,
    enqueued_at TEXT NOT NULL,
    attempts INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS sessions (
    id TEXT PRIMARY KEY NOT NULL,
    created_at TEXT NOT NULL,
    expires_at TEXT NOT NULL
);
