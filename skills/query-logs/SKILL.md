---
name: query-logs
description: >-
  Search Vector Collector logs across registered hosts (Docker, Linux, Windows,
  files). Use when the user asks about errors, crashes, recent logs, what
  happened on a host, container, or Windows Event Log channel. Prefers MCP
  tools logs_facets → logs_search → logs_context; REST /v1/query/* as fallback.
version: 1.0.0
metadata:
  hermes:
    tags: [logs, observability, vector, mcp]
    category: devops
---

# Query Vector Collector logs

Logs live in **Vector Collector**, not this repo’s files. Query via MCP (preferred) or REST. Never use ingest API keys.

## Auth

- **MCP:** server `vector_collector` (yaml from admin **MCP** tab). Tools already authenticated.
- **REST:** `Authorization: Bearer <query key>` against the collector Public URL (Settings). Env `LOGDB_QUERY_KEY` if set. Do not invent a URL — ask or use the MCP server URL minus `/mcp`.

## Loop (always)

1. `logs_schema` — note `hosts` (registered names) and `semantic_search_enabled`.
2. `logs_facets` with `filters.since` (`15m`, `1h`, `24h`). If you omit time, the API uses the last 1 hour.
3. `logs_search` with narrowed `filters` + `text` keywords. `limit` 25.
4. `logs_context` on interesting `id`s (same container/channel or trace).
5. `logs_get` only when the truncated message is not enough.

Do not dump hundreds of lines. Summarize hits (host, container_name, ts, message) then pull context.

## Translate the question

| User says | Filter / field |
|-----------|----------------|
| a machine name (`main-pc`, `mini-2`) | `filters.hosts` — this is the **registered host**, not OS hostname |
| a Docker container | `filters.containers` |
| stderr / Application / System | `filters.streams` or `container_name` (see remap below) |
| last N minutes/hours | `filters.since` (`15m`, `1h`, `24h`) or `last_minutes` |

`container_name` depends on the Vector preset:

- Docker → container name
- Linux journald → syslog identifier / unit
- Windows → Event Log **Channel** (Application, System, Security)
- files / macOS → file path

## Search

- Always set `text` (FTS keywords). Covers **every** stored line. Adjacent words are AND; use `OR` between alternatives (`error OR fail OR exception`).
- Add `semantic_query` only if schema says semantic search is enabled **and** keywords are vague.
- Embeddings are sampled; keyword search is the source of truth.
- Empty results include a `hint` (window, hosts, text). Widen `since`, drop hosts, or simplify keywords.

Example MCP `logs_search` arguments:

```json
{
  "filters": {
    "since": "1h",
    "hosts": ["main-pc"]
  },
  "text": "error OR fail OR exception",
  "limit": 25
}
```

## REST fallback

Same loop, same JSON bodies. See [references/api.md](references/api.md).

```
GET  {base}/v1/query/schema
POST {base}/v1/query/facets
POST {base}/v1/query/search
GET  {base}/v1/query/events/{id}
POST {base}/v1/query/context
```

## Answer style

Lead with what happened and where (`host` + `container_name`). Quote a few log lines. Say if the window had no matches and what you searched.
