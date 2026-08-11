//! Minimal Streamable-HTTP-compatible MCP endpoint for AI agents.
//! Handles initialize / tools/list / tools/call over JSON-RPC 2.0 POST.

use crate::auth::lookup_api_key;
use crate::error::{AppError, AppResult};
use crate::query::{self, ContextRequest, Filters, SearchRequest};
use crate::state::AppState;
use axum::body::Bytes;
use axum::extract::State;
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};

pub fn mcp_router(state: AppState) -> Router {
    Router::new()
        .route("/mcp", post(mcp_post).get(mcp_get))
        .with_state(state)
}

async fn mcp_get() -> impl IntoResponse {
    // Optional SSE channel; some clients probe GET. Keep empty/ok.
    (
        StatusCode::METHOD_NOT_ALLOWED,
        Json(json!({"error": "use POST for MCP JSON-RPC"})),
    )
}

async fn mcp_post(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> AppResult<Response> {
    require_query_key(&state, &headers)?;

    let msg: Value = serde_json::from_slice(&body)
        .map_err(|e| AppError::BadRequest(format!("invalid json: {e}")))?;

    // Batch support
    if let Value::Array(items) = &msg {
        let mut out = Vec::new();
        for item in items {
            if let Some(resp) = handle_message(&state, item).await? {
                out.push(resp);
            }
        }
        return Ok(Json(Value::Array(out)).into_response());
    }

    match handle_message(&state, &msg).await? {
        Some(resp) => Ok(Json(resp).into_response()),
        None => Ok(StatusCode::ACCEPTED.into_response()),
    }
}

fn require_query_key(state: &AppState, headers: &HeaderMap) -> AppResult<()> {
    let auth = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .ok_or(AppError::Unauthorized)?;
    let token = auth
        .strip_prefix("Bearer ")
        .or_else(|| auth.strip_prefix("bearer "))
        .ok_or(AppError::Unauthorized)?
        .trim();
    let key = lookup_api_key(&state.db, token)?.ok_or(AppError::Unauthorized)?;
    if !key.scopes.query {
        return Err(AppError::Forbidden);
    }
    Ok(())
}

async fn handle_message(state: &AppState, msg: &Value) -> AppResult<Option<Value>> {
    let method = msg.get("method").and_then(|m| m.as_str()).unwrap_or("");
    let id = msg.get("id").cloned();
    let params = msg.get("params").cloned().unwrap_or(json!({}));

    // Notifications have no id
    if id.is_none() {
        return Ok(None);
    }

    let result = match method {
        "initialize" => json!({
            "protocolVersion": "2025-03-26",
            "capabilities": { "tools": {} },
            "serverInfo": {
                "name": "vector-collector",
                "version": env!("CARGO_PKG_VERSION")
            },
            "instructions": "Search logs from Vector Collector. Preferred loop: logs_facets → logs_search → logs_context. Translate the user's natural language into filters + text keywords. Use semantic_query when embeddings are enabled and keywords are unclear."
        }),
        "ping" => json!({}),
        "tools/list" => json!({ "tools": tool_defs() }),
        "tools/call" => {
            let name = params
                .get("name")
                .and_then(|v| v.as_str())
                .ok_or_else(|| AppError::BadRequest("missing tool name".into()))?;
            let args = params.get("arguments").cloned().unwrap_or(json!({}));
            call_tool(state, name, args).await?
        }
        "notifications/initialized" => return Ok(None),
        other => {
            return Ok(Some(json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": { "code": -32601, "message": format!("method not found: {other}") }
            })));
        }
    };

    Ok(Some(json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": result
    })))
}

fn tool_defs() -> Vec<Value> {
    vec![
        tool(
            "logs_schema",
            "Describe log fields, filters, and the recommended query loop for investigating logs across machines.",
            json!({"type": "object", "properties": {}}),
        ),
        tool(
            "logs_facets",
            "Return top counts for host/container/image/stream/agent_id under optional filters. Call this first to narrow scope from a natural-language question.",
            json!({
                "type": "object",
                "properties": {
                    "filters": { "type": "object", "description": "ts_from, ts_to, hosts, containers, streams, agent_ids, trace_id, request_id, label_equals" }
                }
            }),
        ),
        tool(
            "logs_search",
            "Search logs with structured filters and/or keyword text (and optional semantic_query). Returns compact truncated messages. Use after facets.",
            json!({
                "type": "object",
                "properties": {
                    "filters": { "type": "object" },
                    "text": { "type": "string", "description": "Keyword/full-text query derived from the user question" },
                    "semantic_query": { "type": "string", "description": "Natural language semantic search when embeddings are configured" },
                    "limit": { "type": "integer" },
                    "cursor": { "type": "string" }
                }
            }),
        ),
        tool(
            "logs_get",
            "Fetch a single log event by id, including raw payload.",
            json!({
                "type": "object",
                "properties": { "id": { "type": "string" } },
                "required": ["id"]
            }),
        ),
        tool(
            "logs_context",
            "Fetch surrounding log lines for an event (same container or trace). Use after finding an interesting hit.",
            json!({
                "type": "object",
                "properties": {
                    "id": { "type": "string" },
                    "before": { "type": "integer" },
                    "after": { "type": "integer" }
                },
                "required": ["id"]
            }),
        ),
    ]
}

fn tool(name: &str, description: &str, input_schema: Value) -> Value {
    json!({
        "name": name,
        "description": description,
        "inputSchema": input_schema
    })
}

async fn call_tool(state: &AppState, name: &str, args: Value) -> AppResult<Value> {
    let payload = match name {
        "logs_schema" => query::schema_document(&state.config),
        "logs_facets" => {
            let filters: Filters =
                serde_json::from_value(args.get("filters").cloned().unwrap_or(json!({})))
                    .map_err(|e| AppError::BadRequest(e.to_string()))?;
            query::facets(&state.db, &filters)?
        }
        "logs_search" => {
            let req: SearchRequest =
                serde_json::from_value(args).map_err(|e| AppError::BadRequest(e.to_string()))?;
            let emb = if let (Some(q), Some(client)) = (
                req.semantic_query.as_ref().filter(|s| !s.trim().is_empty()),
                state.embeddings.as_ref(),
            ) {
                client
                    .embed(&[q.clone()])
                    .await
                    .map_err(AppError::Internal)?
                    .into_iter()
                    .next()
            } else {
                None
            };
            let result = query::search(&state.db, &req, emb)?;
            serde_json::to_value(result).map_err(|e| AppError::Internal(e.into()))?
        }
        "logs_get" => {
            #[derive(Deserialize)]
            struct A {
                id: String,
            }
            let a: A = serde_json::from_value(args).map_err(|e| AppError::BadRequest(e.to_string()))?;
            query::get_event(&state.db, &a.id)?
        }
        "logs_context" => {
            let req: ContextRequest =
                serde_json::from_value(args).map_err(|e| AppError::BadRequest(e.to_string()))?;
            query::context(&state.db, &req)?
        }
        other => {
            return Ok(json!({
                "content": [{ "type": "text", "text": format!("unknown tool: {other}") }],
                "isError": true
            }));
        }
    };

    Ok(json!({
        "content": [{
            "type": "text",
            "text": serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "{}".into())
        }],
        "structuredContent": payload
    }))
}
