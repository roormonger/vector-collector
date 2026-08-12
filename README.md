# Vector Collector

Single-container log collector for [Vector](https://github.com/vectordotdev/vector) agents, with search APIs designed for AI agents (REST + MCP) so you can query **all your machines’ logs from one place** with natural language.

**Docs site:** [roormonger.github.io/vector-collector](https://roormonger.github.io/vector-collector/) (guides + Vector YAML generator).

The Rust binary and Docker image remain named `logdb` for now.

- **Ingest:** `POST /v1/logs` (Vector HTTP sink, gzip, API keys)
- **Search:** SQLite + FTS5 (every line) + optional semantic embeddings
- **MCP:** HTTP at `/mcp` (`logs_facets` → `logs_search` → `logs_context`) for MCP-compatible clients (e.g. [Hermes](https://github.com/NousResearch/hermes-agent))
- **Admin UI:** React + Tailwind for agents, MCP token, retention
- **API docs:** Swagger UI at `/docs`
- **Deploy:** one Docker image, one volume, one port

## Quick start (Docker)

```bash
docker compose up --build -d
```

Open http://localhost:8080 — default login `admin` / `admin` (override with `ADMIN_USERNAME` / `ADMIN_PASSWORD`).

Or:

```bash
docker build -t logdb .
docker run -d -p 8080:8080 \
  -v logdb-data:/data \
  -e ADMIN_PASSWORD=admin \
  -e PUBLIC_BASE_URL=http://localhost:8080 \
  logdb
```

Copy `.env.example` to `.env` and edit before production use. Never commit `.env` or the `/data` volume contents.

## Local development

Requirements: Rust (MSVC on Windows), Node 20+.

Data is always stored under `/data` (`/data/logdb.sqlite`). On Windows that is drive-root `\data` (e.g. `C:\data`). Prefer Docker for day-to-day runs so the volume matches production.

```bash
# terminal 1 — API (after npm run build in web/, or set WEB_DIR)
set WEB_DIR=web/dist
cargo run

# terminal 2 — UI
cd web
npm install
npm run dev
```

Vite proxies `/v1` to `http://127.0.0.1:8080`.

## Public URL

In the admin UI → **Settings**, set **Public URL** to the URL agents and MCP clients use — e.g. `http://192.168.1.10:8080` on a LAN, or `https://logs.example.com` behind a reverse proxy. Do **not** leave it as `http://localhost:8080` if Vector runs on other machines.

`PUBLIC_BASE_URL` in `.env` is only a **first-boot default**; values already saved in Settings are kept across restarts.

## Connect Vector (each machine)

1. In the admin UI → **Agents** → **Create agent**. Re-enter the admin password, set a name (this becomes the log hostname, e.g. `app-server-1`), and pick a **platform preset**.
2. Copy the generated Vector yaml and env (URIs come from the Public URL in Settings). You can switch presets after create/reveal — same ingest token, different yaml. Vector **0.57+** requires `VECTOR_DANGEROUSLY_ALLOW_ENV_VAR_INTERPOLATION=true` for `${INGEST_TOKEN}` to expand (included in the wizard env block).
3. Run Vector on that machine. The collector forces `host` from the agent name on ingest — no Vector `AGENT_NAME` remap needed. Agent **Online/Offline** status uses authenticated contact on `/v1/ingest/health` (startup healthcheck + a `heartbeat` `http_client` source every 30s in the presets) and log ingest. Vector’s own `api.enabled` is not required.

### Platform presets

| Preset | Vector source | Notes |
|--------|---------------|--------|
| **Docker** | `docker_logs` | Needs Docker socket access |
| **Linux** | `journald` | Remaps `MESSAGE` / unit → searchable fields |
| **Windows** | `windows_event_log` | Application, System, Security; bearer **inlined** in YAML (no `.env`) |
| **macOS** | `file` | Common `/var/log` and `/Library/Logs` paths (edit as needed) |
| **Files** | `file` | Generic globs — edit `include` for your apps |

Example (Docker preset):

```yaml
data_dir: /var/lib/vector   # required when using disk buffers

sources:
  docker:
    type: docker_logs

transforms:
  normalize:
    type: remap
    inputs: [docker]
    source: |
      .message = string!(.message)
      if !exists(.source_type) {
        .source_type = "docker_logs"
      }

sinks:
  vector_collector:
    type: http
    inputs: [normalize]
    uri: http://YOUR_COLLECTOR:8080/v1/logs
    encoding: { codec: json }
    compression: gzip
    auth:
      strategy: bearer
      token: "${INGEST_TOKEN}"
    healthcheck:
      uri: http://YOUR_COLLECTOR:8080/v1/ingest/health
    batch:
      max_bytes: 1048576
      max_events: 500
      timeout_secs: 5
    buffer:
      type: disk
      max_size: 268435488
```

```yaml
environment:
  INGEST_TOKEN: lk_...
  # Required on Vector 0.57+ so ${INGEST_TOKEN} expands in the config
  VECTOR_DANGEROUSLY_ALLOW_ENV_VAR_INTERPOLATION: "true"
```

Many agents can POST at once. The collector uses a bounded write queue + `429` backpressure; Vector’s disk buffer absorbs longer spikes.

## Connect an MCP client

1. In the admin UI → **MCP** → enter admin password → **Reveal MCP token**.
2. Copy the MCP yaml (token is inlined). You can **Generate new token** after reveal to rotate it.
3. Add it to your client config (for Hermes: `~/.hermes/config.yaml`).

Example natural-language questions: “what errors happened on app-server-1 in the last hour?” — the client should facet/search/context through MCP. Keyword search covers **all** logs; semantic search is optional (configure embeddings in admin **Settings**).

Removing an agent (Agents → Remove) revokes that machine’s ingest key; its historical logs remain until retention deletes them.

## Query API (REST)

Interactive OpenAPI docs (Swagger UI): **`/docs`** — raw spec at `/api-docs/openapi.json`.

Bearer query key:

- `GET /v1/query/schema`
- `POST /v1/query/facets`
- `POST /v1/query/search`
- `GET /v1/query/events/:id`
- `POST /v1/query/context`

Ingest (bearer ingest key from an agent): `POST /v1/logs`, `GET /v1/ingest/health`.

## Configuration

**Data paths are fixed:** volume `/data`, SQLite `/data/logdb.sqlite`.

**Admin Settings UI** (preferred): public URL, retention, ingest limits (queue / body size / per-key RPS), and embeddings.

**Env vars** — deploy secrets and listen address. See [`.env.example`](.env.example). Operational knobs listed there (e.g. `PUBLIC_BASE_URL`) are **first-boot defaults** only; Settings wins after that.

| Var | Default | Purpose |
|-----|---------|---------|
| `BIND` | `0.0.0.0:8080` | Listen address (`0.0.0.0` = all interfaces; `127.0.0.1` = this machine only) |
| `WEB_DIR` | unset / image default | Static admin UI directory; if unset or missing, API-only |
| `ADMIN_USERNAME` | `admin` | Admin UI login (re-synced from env each start) |
| `ADMIN_PASSWORD` | `admin` | Admin UI password (re-synced from env each start) |
| `SESSION_SECRET` | `dev-session-secret-change-me` | Cookie signing — change in production |
| `PUBLIC_BASE_URL` | `http://localhost:8080` | First-boot default for Public URL setting |
| `BOOTSTRAP_INGEST_KEY` | unset | Seed ingest API key on boot |
| `BOOTSTRAP_QUERY_KEY` | unset | Seed query API key on boot |
| `RUST_LOG` | `info` | Log filter (`tracing-subscriber`) |

`INGEST_TOKEN` and `VECTOR_DANGEROUSLY_ALLOW_ENV_VAR_INTERPOLATION` are for **Vector agents**, not this process.

## Architecture notes

- SQLite WAL + single writer task (safe under multi-agent HTTP concurrency)
- Per-key rate limits + queue `429` when saturated
- FTS5 indexes every message; embeddings are selective (errors/stderr + sample)
- Retention worker deletes by age / max rows

## License

MIT — see [LICENSE](LICENSE).
