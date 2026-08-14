# Vector Collector query API

Load this when using REST instead of MCP, or when a call fails and you need the exact payload shape.

Base URL = Public URL from collector Settings (no trailing slash). Query-scoped bearer only.

OpenAPI clients (e.g. Open WebUI Tool Servers): `{base}/api-docs/query.json` — query endpoints only, not the full `/api-docs/openapi.json`.

```
Authorization: Bearer $LOGDB_QUERY_KEY
Content-Type: application/json
```

## Filters object

Used by facets and search:

```json
{
  "ts_from": "2026-08-12T21:00:00Z",
  "ts_to": "2026-08-12T22:00:00Z",
  "hosts": ["main-pc"],
  "containers": ["grafana"],
  "streams": ["stderr"],
  "agent_ids": [],
  "trace_id": null,
  "request_id": null,
  "label_equals": {}
}
```

All keys optional. Timestamps are UTC RFC3339.

## Endpoints

### Schema

```
GET /v1/query/schema
```

Check `semantic_search_enabled` and `recommended_loop`.

### Facets

```
POST /v1/query/facets
{"filters": { "ts_from": "...", "ts_to": "..." }}
```

Returns top counts for `host`, `container_name`, `image`, `stream`, `agent_id`.

### Search

```
POST /v1/query/search
{
  "filters": {},
  "text": "error timeout",
  "semantic_query": null,
  "limit": 25,
  "cursor": null
}
```

Messages are truncated (~500 chars). Use `next_cursor` to page. Default limit 25, max 200.

### One event

```
GET /v1/query/events/{id}
```

Full row including raw payload.

### Context

```
POST /v1/query/context
{"id": "<event-id>", "before": 20, "after": 20}
```

Surrounding lines for the same container (or trace when present).

## MCP tool mapping

| MCP | REST |
|-----|------|
| `logs_schema` | `GET /v1/query/schema` |
| `logs_facets` | `POST /v1/query/facets` |
| `logs_search` | `POST /v1/query/search` |
| `logs_get` | `GET /v1/query/events/{id}` |
| `logs_context` | `POST /v1/query/context` |
