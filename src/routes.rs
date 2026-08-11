use crate::auth::{
    agent_connect_secret, check_admin_login, check_admin_password, create_agent, create_session,
    delete_agent, destroy_session, ensure_mcp_query_key, lookup_api_key, rotate_mcp_query_key,
    sign_cookie_value, touch_agent_last_seen, validate_session, verify_cookie_value,
};
use crate::error::{AppError, AppResult};
use crate::ingest::{normalize_events, IngestBatch};
use crate::openapi::{
    ApiDoc, ErrorBody, HealthResponse, IngestAccepted, IngestHealthResponse, ReadyResponse,
};
use crate::query::{self, ContextRequest, FacetsRequest, SearchRequest};
use crate::state::AppState;
use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use cookie::time::Duration as CookieDuration;
use flate2::read::GzDecoder;
use serde::Deserialize;
use serde_json::{json, Value};
use std::io::Read;
use std::sync::atomic::Ordering;
use tokio::sync::oneshot;
use tower_http::services::{ServeDir, ServeFile};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

pub fn app_router(state: AppState, web_dir: Option<std::path::PathBuf>) -> Router {
    let api = Router::new()
        .merge(SwaggerUi::new("/docs").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .route("/v1/ingest/health", get(ingest_health).head(ingest_health))
        .route("/v1/logs", post(ingest_logs))
        .route("/v1/query/schema", get(query_schema))
        .route("/v1/query/facets", post(query_facets))
        .route("/v1/query/search", post(query_search))
        .route("/v1/query/events/{id}", get(query_event))
        .route("/v1/query/context", post(query_context))
        .route("/v1/admin/login", post(admin_login))
        .route("/v1/admin/logout", post(admin_logout))
        .route("/v1/admin/me", get(admin_me))
        .route("/v1/admin/agents", get(admin_agents).post(admin_create_agent))
        .route(
            "/v1/admin/agents/{id}/connect-info",
            post(admin_agent_connect_info),
        )
        .route("/v1/admin/agents/{id}/remove", post(admin_delete_agent))
        .route("/v1/admin/mcp/connect-info", post(admin_mcp_connect_info))
        .route("/v1/admin/mcp/rotate", post(admin_mcp_rotate))
        .route("/v1/admin/stats", get(admin_stats))
        .route("/v1/admin/recent-events", get(admin_recent_events))
        .route("/v1/admin/settings", get(admin_get_settings).put(admin_put_settings))
        .with_state(state);

    if let Some(dir) = web_dir {
        if dir.exists() {
            let index = dir.join("index.html");
            return api.fallback_service(
                ServeDir::new(dir).not_found_service(ServeFile::new(index)),
            );
        }
    }
    api
}

#[utoipa::path(
    get,
    path = "/healthz",
    tag = "health",
    responses((status = 200, description = "Service is up", body = HealthResponse))
)]
pub async fn healthz() -> Json<HealthResponse> {
    Json(HealthResponse { ok: true })
}

/// Authenticated liveness for Vector's HTTP sink healthcheck (validates ingest API key).
#[utoipa::path(
    get,
    path = "/v1/ingest/health",
    tag = "ingest",
    security(("bearer_ingest" = [])),
    responses(
        (status = 200, description = "Ingest key is valid", body = IngestHealthResponse),
        (status = 401, description = "Missing or invalid key", body = ErrorBody),
        (status = 403, description = "Key lacks ingest scope", body = ErrorBody)
    )
)]
pub async fn ingest_health(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<IngestHealthResponse>> {
    let raw_key = bearer_from_headers(&headers).ok_or(AppError::Unauthorized)?;
    let key = lookup_api_key(&state.db, &raw_key)?.ok_or(AppError::Unauthorized)?;
    if !key.scopes.ingest {
        return Err(AppError::Forbidden);
    }
    // Vector sink healthcheck (startup) + optional http_client heartbeat scrape.
    let _ = touch_agent_last_seen(&state.db, &key.id);
    Ok(Json(IngestHealthResponse {
        ok: true,
        key: key.name,
    }))
}

#[utoipa::path(
    get,
    path = "/readyz",
    tag = "health",
    responses(
        (status = 200, description = "DB reachable", body = ReadyResponse),
        (status = 503, description = "Not ready", body = ErrorBody)
    )
)]
pub async fn readyz(State(state): State<AppState>) -> AppResult<Json<ReadyResponse>> {
    let conn = state.db.lock();
    conn.query_row("SELECT 1", [], |_| Ok(()))
        .map_err(|e| AppError::Unavailable(e.to_string()))?;
    Ok(Json(ReadyResponse {
        ok: true,
        queue_depth: state.ingest.depth_approx(),
    }))
}

#[utoipa::path(
    post,
    path = "/v1/logs",
    tag = "ingest",
    security(("bearer_ingest" = [])),
    request_body(
        content = serde_json::Value,
        description = "JSON event or array of events (optionally gzip-encoded body)",
        content_type = "application/json"
    ),
    responses(
        (status = 202, description = "Accepted", body = IngestAccepted),
        (status = 400, description = "Bad request", body = ErrorBody),
        (status = 401, description = "Unauthorized", body = ErrorBody),
        (status = 403, description = "Forbidden", body = ErrorBody),
        (status = 429, description = "Rate limited / queue full", body = ErrorBody)
    )
)]
pub async fn ingest_logs(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> AppResult<impl IntoResponse> {
    let raw_key = bearer_from_headers(&headers).ok_or_else(|| {
        tracing::warn!("ingest unauthorized: missing/invalid Authorization header");
        AppError::Unauthorized
    })?;
    let key = match lookup_api_key(&state.db, &raw_key)? {
        Some(k) => k,
        None => {
            let prefix: String = raw_key.chars().take(10).collect();
            tracing::warn!(%prefix, len = raw_key.len(), "ingest unauthorized: unknown or revoked key");
            return Err(AppError::Unauthorized);
        }
    };
    if !key.scopes.ingest {
        tracing::warn!(key = %key.name, "ingest forbidden: key lacks ingest scope");
        return Err(AppError::Forbidden);
    }
    if !state.rate_limiter.check(&key.id) {
        state.ingest_stats.rejected_429.fetch_add(1, Ordering::Relaxed);
        return Err(AppError::TooManyRequests {
            retry_after_secs: 1,
        });
    }

    let bytes = maybe_gunzip(&headers, &body)?;
    let payload: Value = serde_json::from_slice(&bytes)
        .map_err(|e| AppError::BadRequest(format!("invalid json: {e}")))?;
    let events = normalize_events(payload, state.config.embed_sample_rate)?;

    let (respond_tx, respond_rx) = oneshot::channel();
    let enqueue = state
        .ingest
        .enqueue(IngestBatch {
            key,
            events,
            respond: respond_tx,
        })
        .await;
    if let Err(AppError::TooManyRequests { .. }) = &enqueue {
        state.ingest_stats.rejected_429.fetch_add(1, Ordering::Relaxed);
    }
    enqueue?;

    let written = respond_rx
        .await
        .map_err(|_| AppError::Unavailable("ingest writer dropped".into()))?
        .map_err(AppError::Unavailable)?;

    state.ingest_stats.accepted.fetch_add(1, Ordering::Relaxed);
    state
        .ingest_stats
        .written
        .fetch_add(written as u64, Ordering::Relaxed);

    Ok((StatusCode::ACCEPTED, Json(json!({ "written": written }))))
}

fn bearer_from_headers(headers: &HeaderMap) -> Option<String> {
    let auth = headers.get(header::AUTHORIZATION)?.to_str().ok()?;
    let token = auth
        .strip_prefix("Bearer ")
        .or_else(|| auth.strip_prefix("bearer "))?;
    let token = token.trim().trim_matches('"').trim_matches('\'').trim();
    if token.is_empty() {
        None
    } else {
        Some(token.to_string())
    }
}

fn maybe_gunzip(headers: &HeaderMap, body: &Bytes) -> AppResult<Vec<u8>> {
    let encoding = headers
        .get(header::CONTENT_ENCODING)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if encoding.eq_ignore_ascii_case("gzip") {
        let mut decoder = GzDecoder::new(body.as_ref());
        let mut out = Vec::new();
        decoder
            .read_to_end(&mut out)
            .map_err(|e| AppError::BadRequest(format!("gzip decode: {e}")))?;
        Ok(out)
    } else {
        Ok(body.to_vec())
    }
}

async fn require_query_key(state: &AppState, headers: &HeaderMap) -> AppResult<()> {
    let raw = bearer_from_headers(headers).ok_or(AppError::Unauthorized)?;
    let key = lookup_api_key(&state.db, &raw)?.ok_or(AppError::Unauthorized)?;
    if !key.scopes.query {
        return Err(AppError::Forbidden);
    }
    Ok(())
}

#[utoipa::path(
    get,
    path = "/v1/query/schema",
    tag = "query",
    security(("bearer_query" = [])),
    responses(
        (status = 200, description = "Schema / recommended search loop", body = serde_json::Value),
        (status = 401, description = "Unauthorized", body = ErrorBody),
        (status = 403, description = "Forbidden", body = ErrorBody)
    )
)]
pub async fn query_schema(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<Value>> {
    require_query_key(&state, &headers).await?;
    Ok(Json(query::schema_document(&state.config)))
}

#[utoipa::path(
    post,
    path = "/v1/query/facets",
    tag = "query",
    security(("bearer_query" = [])),
    request_body = FacetsRequest,
    responses(
        (status = 200, description = "Facet counts by host/container/etc.", body = serde_json::Value),
        (status = 401, description = "Unauthorized", body = ErrorBody),
        (status = 403, description = "Forbidden", body = ErrorBody)
    )
)]
pub async fn query_facets(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<FacetsRequest>,
) -> AppResult<Json<Value>> {
    require_query_key(&state, &headers).await?;
    let filters = body.filters.unwrap_or_default();
    Ok(Json(query::facets(&state.db, &filters)?))
}

#[utoipa::path(
    post,
    path = "/v1/query/search",
    tag = "query",
    security(("bearer_query" = [])),
    request_body = SearchRequest,
    responses(
        (status = 200, description = "Matching events", body = serde_json::Value),
        (status = 401, description = "Unauthorized", body = ErrorBody),
        (status = 403, description = "Forbidden", body = ErrorBody)
    )
)]
pub async fn query_search(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<SearchRequest>,
) -> AppResult<Json<Value>> {
    require_query_key(&state, &headers).await?;
    let emb = if let (Some(q), Some(client)) = (
        req.semantic_query.as_ref().filter(|s| !s.trim().is_empty()),
        state.embeddings.as_ref(),
    ) {
        client
            .embed(&[q.clone()])
            .await
            .map_err(|e| AppError::Internal(e))?
            .into_iter()
            .next()
    } else {
        None
    };
    let result = query::search(&state.db, &req, emb)?;
    Ok(Json(serde_json::to_value(result).map_err(|e| AppError::Internal(e.into()))?))
}

#[utoipa::path(
    get,
    path = "/v1/query/events/{id}",
    tag = "query",
    security(("bearer_query" = [])),
    params(("id" = String, Path, description = "Event id")),
    responses(
        (status = 200, description = "Single event", body = serde_json::Value),
        (status = 401, description = "Unauthorized", body = ErrorBody),
        (status = 404, description = "Not found", body = ErrorBody)
    )
)]
pub async fn query_event(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> AppResult<Json<Value>> {
    require_query_key(&state, &headers).await?;
    Ok(Json(query::get_event(&state.db, &id)?))
}

#[utoipa::path(
    post,
    path = "/v1/query/context",
    tag = "query",
    security(("bearer_query" = [])),
    request_body = ContextRequest,
    responses(
        (status = 200, description = "Surrounding events", body = serde_json::Value),
        (status = 401, description = "Unauthorized", body = ErrorBody),
        (status = 404, description = "Not found", body = ErrorBody)
    )
)]
pub async fn query_context(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<ContextRequest>,
) -> AppResult<Json<Value>> {
    require_query_key(&state, &headers).await?;
    Ok(Json(query::context(&state.db, &req)?))
}

#[derive(Deserialize)]
struct LoginBody {
    username: String,
    password: String,
}

async fn admin_login(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(body): Json<LoginBody>,
) -> AppResult<(CookieJar, Json<Value>)> {
    if !check_admin_login(&state.db, &body.username, &body.password)? {
        return Err(AppError::Unauthorized);
    }
    let sid = create_session(&state.db, 24 * 7)?;
    let signed = sign_cookie_value(&state.config.session_secret, &sid)
        .map_err(AppError::Internal)?;
    let cookie = Cookie::build(("logdb_session", signed))
        .http_only(true)
        .same_site(SameSite::Lax)
        .path("/")
        .max_age(CookieDuration::weeks(1))
        .build();
    Ok((jar.add(cookie), Json(json!({"ok": true}))))
}

async fn admin_logout(
    State(state): State<AppState>,
    jar: CookieJar,
) -> AppResult<(CookieJar, Json<Value>)> {
    if let Some(sid) = session_id_from_jar(&state, &jar) {
        let _ = destroy_session(&state.db, &sid);
    }
    let cookie = Cookie::build(("logdb_session", ""))
        .http_only(true)
        .path("/")
        .max_age(CookieDuration::seconds(0))
        .build();
    Ok((jar.add(cookie), Json(json!({"ok": true}))))
}

async fn admin_me(State(state): State<AppState>, jar: CookieJar) -> AppResult<impl IntoResponse> {
    require_admin(&state, &jar)?;
    Ok(Json(json!({
        "username": state.config.admin_username,
    })))
}

fn session_id_from_jar(state: &AppState, jar: &CookieJar) -> Option<String> {
    let c = jar.get("logdb_session")?;
    verify_cookie_value(&state.config.session_secret, c.value())
}

fn require_admin(state: &AppState, jar: &CookieJar) -> AppResult<()> {
    let sid = session_id_from_jar(state, jar).ok_or(AppError::Unauthorized)?;
    if !validate_session(&state.db, &sid)? {
        return Err(AppError::Unauthorized);
    }
    Ok(())
}

async fn admin_agents(
    State(state): State<AppState>,
    jar: CookieJar,
) -> AppResult<impl IntoResponse> {
    require_admin(&state, &jar)?;
    Ok(Json(query::list_agents(&state.db)?))
}

#[derive(Deserialize)]
struct CreateAgentBody {
    password: String,
    name: String,
    #[serde(default)]
    platform: Option<String>,
}

#[derive(Deserialize)]
struct PasswordBody {
    password: String,
    #[serde(default)]
    platform: Option<String>,
}

fn require_admin_password(state: &AppState, jar: &CookieJar, password: &str) -> AppResult<()> {
    require_admin(state, jar)?;
    if !check_admin_password(&state.db, password)? {
        return Err(AppError::Unauthorized);
    }
    Ok(())
}

async fn admin_create_agent(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(body): Json<CreateAgentBody>,
) -> AppResult<impl IntoResponse> {
    require_admin_password(&state, &jar, &body.password)?;
    let (agent, token) = create_agent(&state.db, &body.name)?;
    let mut bundle = crate::vector_presets::vector_agent_bundle(
        &state.config.public_base_url,
        &token,
        body.platform.as_deref(),
    );
    if let Some(obj) = bundle.as_object_mut() {
        obj.insert("agent".into(), agent);
    }
    Ok(Json(bundle))
}

async fn admin_agent_connect_info(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(id): Path<String>,
    Json(body): Json<PasswordBody>,
) -> AppResult<impl IntoResponse> {
    require_admin_password(&state, &jar, &body.password)?;
    let (_hostname, token) = agent_connect_secret(&state.db, &id)?;
    Ok(Json(crate::vector_presets::vector_agent_bundle(
        &state.config.public_base_url,
        &token,
        body.platform.as_deref(),
    )))
}

async fn admin_delete_agent(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(id): Path<String>,
    Json(body): Json<PasswordBody>,
) -> AppResult<impl IntoResponse> {
    require_admin_password(&state, &jar, &body.password)?;
    delete_agent(&state.db, &id)?;
    Ok(Json(json!({ "ok": true })))
}

fn mcp_bundle(public_base_url: &str, token: &str) -> Value {
    let url = format!("{}/mcp", public_base_url.trim_end_matches('/'));
    let yaml = format!(
        r#"mcp_servers:
  vector_collector:
    url: "{url}"
    headers:
      Authorization: "Bearer {token}"
    timeout: 120
    tools:
      resources: false
      prompts: false
"#
    );
    let env = format!("LOGDB_QUERY_KEY={token}");
    json!({
        "token": token,
        "url": url,
        "env": env,
        "yaml": yaml,
    })
}

async fn admin_mcp_connect_info(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(body): Json<PasswordBody>,
) -> AppResult<impl IntoResponse> {
    require_admin_password(&state, &jar, &body.password)?;
    let (_id, token) = ensure_mcp_query_key(&state.db)?;
    Ok(Json(mcp_bundle(&state.config.public_base_url, &token)))
}

async fn admin_mcp_rotate(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(body): Json<PasswordBody>,
) -> AppResult<impl IntoResponse> {
    require_admin_password(&state, &jar, &body.password)?;
    let (_id, token) = rotate_mcp_query_key(&state.db)?;
    Ok(Json(mcp_bundle(&state.config.public_base_url, &token)))
}

async fn admin_stats(
    State(state): State<AppState>,
    jar: CookieJar,
) -> AppResult<impl IntoResponse> {
    require_admin(&state, &jar)?;
    let mut stats = query::stats(&state.db)?;
    if let Some(obj) = stats.as_object_mut() {
        obj.insert(
            "ingest_accepted".into(),
            json!(state.ingest_stats.accepted.load(Ordering::Relaxed)),
        );
        obj.insert(
            "ingest_429".into(),
            json!(state.ingest_stats.rejected_429.load(Ordering::Relaxed)),
        );
        obj.insert(
            "ingest_written".into(),
            json!(state.ingest_stats.written.load(Ordering::Relaxed)),
        );
        obj.insert("queue_depth".into(), json!(state.ingest.depth_approx()));
        obj.insert(
            "embeddings_enabled".into(),
            json!(state.config.embeddings_enabled()),
        );
    }
    Ok(Json(stats))
}

async fn admin_recent_events(
    State(state): State<AppState>,
    jar: CookieJar,
) -> AppResult<impl IntoResponse> {
    require_admin(&state, &jar)?;
    Ok(Json(query::recent_events(&state.db, 80)?))
}

#[derive(Deserialize)]
struct SettingsBody {
    retention_days: Option<u32>,
    max_events: Option<Option<u64>>,
}

async fn admin_get_settings(
    State(state): State<AppState>,
    jar: CookieJar,
) -> AppResult<impl IntoResponse> {
    require_admin(&state, &jar)?;
    let conn = state.db.lock();
    let retention_days = crate::db::setting_get(&conn, "retention_days")
        .map_err(AppError::Internal)?
        .and_then(|s| s.parse().ok())
        .unwrap_or(state.config.retention_days);
    let max_events = crate::db::setting_get(&conn, "max_events")
        .map_err(AppError::Internal)?
        .and_then(|s| s.parse().ok())
        .or(state.config.max_events);
    Ok(Json(json!({
        "retention_days": retention_days,
        "max_events": max_events,
        "public_base_url": state.config.public_base_url,
    })))
}

async fn admin_put_settings(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(body): Json<SettingsBody>,
) -> AppResult<impl IntoResponse> {
    require_admin(&state, &jar)?;
    let conn = state.db.lock();
    if let Some(days) = body.retention_days {
        crate::db::setting_set(&conn, "retention_days", &days.to_string())
            .map_err(AppError::Internal)?;
    }
    if let Some(max) = body.max_events {
        match max {
            Some(n) => crate::db::setting_set(&conn, "max_events", &n.to_string())
                .map_err(AppError::Internal)?,
            None => {
                let _ = conn.execute("DELETE FROM app_settings WHERE key = 'max_events'", []);
            }
        }
    }
    Ok(Json(json!({"ok": true})))
}

