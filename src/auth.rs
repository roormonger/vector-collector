use crate::db::{setting_get, setting_set, Db};
use crate::error::{AppError, AppResult};
use anyhow::Context;
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use chrono::{Duration, Utc};
use hmac::{Hmac, Mac};
use rand::rngs::OsRng;
use rand::RngCore;
use rusqlite::{params, OptionalExtension};
use sha2::{Digest, Sha256};
use ulid::Ulid;

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyScopes {
    pub ingest: bool,
    pub query: bool,
}

#[derive(Debug, Clone)]
pub struct ApiKeyRecord {
    pub id: String,
    pub name: String,
    pub key_prefix: String,
    pub scopes: KeyScopes,
}

pub fn hash_password(password: &str) -> anyhow::Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| anyhow::anyhow!("hash password: {e}"))?
        .to_string();
    Ok(hash)
}

pub fn verify_password(password: &str, hash: &str) -> bool {
    let Ok(parsed) = PasswordHash::new(hash) else {
        return false;
    };
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok()
}

pub fn sync_admin_credentials(db: &Db, username: &str, password: &str) -> anyhow::Result<()> {
    let hash = hash_password(password)?;
    let conn = db.lock();
    setting_set(&conn, "admin_username", username)?;
    setting_set(&conn, "admin_password_hash", &hash)?;
    Ok(())
}

pub fn check_admin_login(db: &Db, username: &str, password: &str) -> AppResult<bool> {
    let conn = db.lock();
    let stored_user = setting_get(&conn, "admin_username")
        .map_err(AppError::Internal)?
        .unwrap_or_else(|| "admin".into());
    let stored_hash = setting_get(&conn, "admin_password_hash")
        .map_err(AppError::Internal)?
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("admin not initialized")))?;
    Ok(stored_user == username && verify_password(password, &stored_hash))
}

/// Re-auth with admin password only (session already required by caller).
pub fn check_admin_password(db: &Db, password: &str) -> AppResult<bool> {
    let conn = db.lock();
    let stored_hash = setting_get(&conn, "admin_password_hash")
        .map_err(AppError::Internal)?
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("admin not initialized")))?;
    Ok(verify_password(password, &stored_hash))
}

pub fn create_session(db: &Db, ttl_hours: i64) -> AppResult<String> {
    let id = Ulid::new().to_string();
    let now = Utc::now();
    let expires = now + Duration::hours(ttl_hours);
    let conn = db.lock();
    conn.execute(
        "INSERT INTO sessions(id, created_at, expires_at) VALUES(?1, ?2, ?3)",
        params![id, now.to_rfc3339(), expires.to_rfc3339()],
    )
    .map_err(|e| AppError::Internal(e.into()))?;
    Ok(id)
}

pub fn validate_session(db: &Db, session_id: &str) -> AppResult<bool> {
    let conn = db.lock();
    let mut stmt = conn
        .prepare("SELECT expires_at FROM sessions WHERE id = ?1")
        .map_err(|e| AppError::Internal(e.into()))?;
    let mut rows = stmt
        .query(params![session_id])
        .map_err(|e| AppError::Internal(e.into()))?;
    let Some(row) = rows.next().map_err(|e| AppError::Internal(e.into()))? else {
        return Ok(false);
    };
    let expires_at: String = row.get(0).map_err(|e| AppError::Internal(e.into()))?;
    let expires = chrono::DateTime::parse_from_rfc3339(&expires_at)
        .map_err(|e| AppError::Internal(e.into()))?
        .with_timezone(&Utc);
    if expires < Utc::now() {
        let _ = conn.execute("DELETE FROM sessions WHERE id = ?1", params![session_id]);
        return Ok(false);
    }
    Ok(true)
}

pub fn destroy_session(db: &Db, session_id: &str) -> AppResult<()> {
    let conn = db.lock();
    conn.execute("DELETE FROM sessions WHERE id = ?1", params![session_id])
        .map_err(|e| AppError::Internal(e.into()))?;
    Ok(())
}

pub fn hash_api_key(raw: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(raw.as_bytes());
    hex::encode(hasher.finalize())
}

pub fn generate_api_key() -> String {
    let mut bytes = [0u8; 24];
    OsRng.fill_bytes(&mut bytes);
    format!("lk_{}", hex::encode(bytes))
}

pub fn create_api_key_with_options(
    db: &Db,
    name: &str,
    scopes: KeyScopes,
    store_secret: bool,
) -> AppResult<(ApiKeyRecord, String)> {
    let id = Ulid::new().to_string();
    let raw = generate_api_key();
    let key_hash = hash_api_key(&raw);
    let key_prefix = raw.chars().take(10).collect::<String>();
    let now = Utc::now().to_rfc3339();
    let secret = if store_secret { Some(raw.as_str()) } else { None };
    let conn = db.lock();
    conn.execute(
        "INSERT INTO api_keys(id, name, key_prefix, key_hash, scope_ingest, scope_query, created_at, secret)
         VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            id,
            name,
            key_prefix,
            key_hash,
            scopes.ingest as i64,
            scopes.query as i64,
            now,
            secret
        ],
    )
    .map_err(|e| AppError::Internal(e.into()))?;

    Ok((
        ApiKeyRecord {
            id,
            name: name.to_string(),
            key_prefix,
            scopes,
        },
        raw,
    ))
}

/// Create a dedicated ingest agent: API key (secret stored) + agent row named as hostname.
pub fn create_agent(
    db: &Db,
    hostname: &str,
) -> AppResult<(serde_json::Value, String)> {
    let hostname = hostname.trim();
    if hostname.is_empty() {
        return Err(AppError::BadRequest("name is required".into()));
    }
    if hostname.len() > 128 {
        return Err(AppError::BadRequest("name too long".into()));
    }

    let (key, raw) = create_api_key_with_options(
        db,
        &format!("agent:{hostname}"),
        KeyScopes {
            ingest: true,
            query: false,
        },
        true,
    )?;

    let agent_id = Ulid::new().to_string();
    let now = Utc::now().to_rfc3339();
    let conn = db.lock();
    conn.execute(
        "INSERT INTO agents(id, name, api_key_id, host_hint, last_seen_at, events_ingested, created_at)
         VALUES(?1, ?2, ?3, ?2, NULL, 0, ?4)",
        params![agent_id, hostname, key.id, now],
    )
    .map_err(|e| AppError::Internal(e.into()))?;

    Ok((
        serde_json::json!({
            "id": agent_id,
            "name": hostname,
            "host": hostname,
            "host_hint": hostname,
            "api_key_id": key.id,
            "key_prefix": key.key_prefix,
            "key_name": key.name,
            "events_ingested": 0,
            "created_at": now,
            "last_seen_at": serde_json::Value::Null,
            "status": "offline",
            "has_connect_secret": true,
        }),
        raw,
    ))
}

pub const MCP_QUERY_KEY_NAME: &str = "mcp-query";

/// Remove agent row and revoke its ingest key. log_events are left for retention.
pub fn delete_agent(db: &Db, agent_id: &str) -> AppResult<()> {
    let conn = db.lock();
    let found: Option<Option<String>> = conn
        .query_row(
            "SELECT api_key_id FROM agents WHERE id = ?1",
            params![agent_id],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| AppError::Internal(e.into()))?;
    let Some(api_key_id) = found else {
        return Err(AppError::NotFound);
    };
    conn.execute("DELETE FROM agents WHERE id = ?1", params![agent_id])
        .map_err(|e| AppError::Internal(e.into()))?;
    if let Some(key_id) = api_key_id {
        let _ = conn.execute(
            "UPDATE api_keys SET revoked_at = ?1, secret = NULL WHERE id = ?2 AND revoked_at IS NULL",
            params![Utc::now().to_rfc3339(), key_id],
        );
    }
    Ok(())
}

/// Active MCP query token (creates one on first use). Returns (key_id, raw_token).
pub fn ensure_mcp_query_key(db: &Db) -> AppResult<(String, String)> {
    {
        let conn = db.lock();
        let existing: Result<(String, String), _> = conn.query_row(
            "SELECT id, secret FROM api_keys
             WHERE name = ?1 AND scope_query = 1 AND revoked_at IS NULL
               AND secret IS NOT NULL AND trim(secret) != ''
             ORDER BY created_at DESC LIMIT 1",
            params![MCP_QUERY_KEY_NAME],
            |r| Ok((r.get(0)?, r.get(1)?)),
        );
        if let Ok((id, secret)) = existing {
            return Ok((id, secret));
        }
    }
    let (rec, raw) = create_api_key_with_options(
        db,
        MCP_QUERY_KEY_NAME,
        KeyScopes {
            ingest: false,
            query: true,
        },
        true,
    )?;
    Ok((rec.id, raw))
}

/// Revoke current MCP query key(s) and issue a new one. Returns (key_id, raw_token).
pub fn rotate_mcp_query_key(db: &Db) -> AppResult<(String, String)> {
    {
        let conn = db.lock();
        conn.execute(
            "UPDATE api_keys SET revoked_at = ?1, secret = NULL
             WHERE name = ?2 AND revoked_at IS NULL",
            params![Utc::now().to_rfc3339(), MCP_QUERY_KEY_NAME],
        )
        .map_err(|e| AppError::Internal(e.into()))?;
    }
    let (rec, raw) = create_api_key_with_options(
        db,
        MCP_QUERY_KEY_NAME,
        KeyScopes {
            ingest: false,
            query: true,
        },
        true,
    )?;
    Ok((rec.id, raw))
}

/// Returns (hostname, token) for a wizard-created agent.
pub fn agent_connect_secret(db: &Db, agent_id: &str) -> AppResult<(String, String)> {
    let conn = db.lock();
    let row: Result<(String, Option<String>, Option<String>), _> = conn.query_row(
        "SELECT a.name, k.secret, k.revoked_at
         FROM agents a
         LEFT JOIN api_keys k ON k.id = a.api_key_id
         WHERE a.id = ?1",
        params![agent_id],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    );
    let (name, secret, revoked_at) = match row {
        Ok(v) => v,
        Err(rusqlite::Error::QueryReturnedNoRows) => return Err(AppError::NotFound),
        Err(e) => return Err(AppError::Internal(e.into())),
    };
    if revoked_at.is_some() {
        return Err(AppError::BadRequest("agent key revoked".into()));
    }
    let Some(token) = secret.filter(|s| !s.is_empty()) else {
        return Err(AppError::NotFound);
    };
    Ok((name, token))
}

pub fn ensure_bootstrap_key(db: &Db, name: &str, raw: &str, scopes: KeyScopes) -> anyhow::Result<()> {
    let key_hash = hash_api_key(raw);
    let conn = db.lock();
    let exists: i64 = conn.query_row(
        "SELECT COUNT(1) FROM api_keys WHERE key_hash = ?1",
        params![key_hash],
        |r| r.get(0),
    )?;
    if exists > 0 {
        return Ok(());
    }
    let id = Ulid::new().to_string();
    let key_prefix = raw.chars().take(10).collect::<String>();
    conn.execute(
        "INSERT INTO api_keys(id, name, key_prefix, key_hash, scope_ingest, scope_query, created_at)
         VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            id,
            name,
            key_prefix,
            key_hash,
            scopes.ingest as i64,
            scopes.query as i64,
            Utc::now().to_rfc3339()
        ],
    )?;
    Ok(())
}

pub fn lookup_api_key(db: &Db, raw: &str) -> AppResult<Option<ApiKeyRecord>> {
    let key_hash = hash_api_key(raw);
    let conn = db.lock();
    let mut stmt = conn
        .prepare(
            "SELECT id, name, key_prefix, scope_ingest, scope_query
             FROM api_keys WHERE key_hash = ?1 AND revoked_at IS NULL",
        )
        .map_err(|e| AppError::Internal(e.into()))?;
    let mut rows = stmt
        .query(params![key_hash])
        .map_err(|e| AppError::Internal(e.into()))?;
    let Some(row) = rows.next().map_err(|e| AppError::Internal(e.into()))? else {
        return Ok(None);
    };
    Ok(Some(ApiKeyRecord {
        id: row.get(0).map_err(|e| AppError::Internal(e.into()))?,
        name: row.get(1).map_err(|e| AppError::Internal(e.into()))?,
        key_prefix: row.get(2).map_err(|e| AppError::Internal(e.into()))?,
        scopes: KeyScopes {
            ingest: row.get::<_, i64>(3).map_err(|e| AppError::Internal(e.into()))? != 0,
            query: row.get::<_, i64>(4).map_err(|e| AppError::Internal(e.into()))? != 0,
        },
    }))
}

/// Heartbeat / healthcheck contact — updates last_seen without counting events.
pub fn touch_agent_last_seen(db: &Db, api_key_id: &str) -> AppResult<()> {
    let conn = db.lock();
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE agents SET last_seen_at = ?1 WHERE api_key_id = ?2",
        params![now, api_key_id],
    )
    .map_err(|e| AppError::Internal(e.into()))?;
    Ok(())
}

pub fn sign_cookie_value(secret: &str, value: &str) -> anyhow::Result<String> {
    let mut mac =
        HmacSha256::new_from_slice(secret.as_bytes()).context("hmac key")?;
    mac.update(value.as_bytes());
    let sig = hex::encode(mac.finalize().into_bytes());
    Ok(format!("{value}.{sig}"))
}

pub fn verify_cookie_value(secret: &str, signed: &str) -> Option<String> {
    let (value, sig) = signed.rsplit_once('.')?;
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).ok()?;
    mac.update(value.as_bytes());
    let expected = hex::encode(mac.finalize().into_bytes());
    if constant_time_eq(sig.as_bytes(), expected.as_bytes()) {
        Some(value.to_string())
    } else {
        None
    }
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b.iter()).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

