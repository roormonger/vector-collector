-- Normalize null/empty hosts so uniqueness works in SQLite
UPDATE agents SET host_hint = 'unknown' WHERE host_hint IS NULL OR trim(host_hint) = '';

-- One agent row per ingest key + Vector host
CREATE UNIQUE INDEX IF NOT EXISTS idx_agents_key_host
    ON agents(api_key_id, host_hint);
