import type { ReactNode } from 'react'

export function DocsPage() {
  return (
    <div className="mx-auto max-w-3xl space-y-12 pb-16">
      <header className="space-y-2">
        <h1 className="text-3xl font-semibold tracking-tight">Docs</h1>
        <p className="text-[var(--text-muted)]">
          Deploy, configure, and connect Vector agents and MCP clients to Vector Collector.
        </p>
      </header>

      <DocSection id="quick-start" title="Quick start (Docker)">
        <pre>{`docker compose up --build -d`}</pre>
        <p>
          Open <code>http://localhost:8080</code> — default login <code>admin</code> /{' '}
          <code>admin</code> (override with <code>ADMIN_USERNAME</code> /{' '}
          <code>ADMIN_PASSWORD</code>).
        </p>
        <p>Or:</p>
        <pre>{`docker build -t logdb .
docker run -d -p 8080:8080 \\
  -v logdb-data:/data \\
  -e ADMIN_PASSWORD=admin \\
  -e PUBLIC_BASE_URL=http://localhost:8080 \\
  logdb`}</pre>
        <p>
          Copy <code>.env.example</code> to <code>.env</code> and edit before production use. Never
          commit <code>.env</code> or the <code>/data</code> volume contents.
        </p>
      </DocSection>

      <DocSection id="public-url" title="Public URL (PUBLIC_BASE_URL)">
        <p>
          Set <code>PUBLIC_BASE_URL</code> to the URL <strong>agents and MCP clients use to reach
          this collector</strong> — e.g. <code>http://192.168.1.10:8080</code> on a LAN, or{' '}
          <code>https://logs.example.com</code> behind a reverse proxy. Do <strong>not</strong>{' '}
          leave it as <code>http://localhost:8080</code> if Vector runs on other machines.
        </p>
        <p>
          Generated Vector/MCP snippets in the admin UI are filled from this value. Restart after
          changing it.
        </p>
      </DocSection>

      <DocSection id="connect-vector" title="Connect Vector (each machine)">
        <ol className="list-decimal space-y-2 pl-5 text-[var(--text-muted)]">
          <li>
            In the admin UI → <strong className="text-[var(--text)]">Agents</strong> →{' '}
            <strong className="text-[var(--text)]">Create agent</strong>. Re-enter the admin
            password, set a name (this becomes the log hostname, e.g. <code>app-server-1</code>),
            and pick a <strong className="text-[var(--text)]">platform preset</strong>.
          </li>
          <li>
            Copy the generated Vector yaml and env (URIs come from{' '}
            <code>PUBLIC_BASE_URL</code>). You can switch presets after create/reveal — same ingest
            token, different yaml. Vector <strong className="text-[var(--text)]">0.57+</strong>{' '}
            requires <code>VECTOR_DANGEROUSLY_ALLOW_ENV_VAR_INTERPOLATION=true</code> for{' '}
            <code>${'{INGEST_TOKEN}'}</code> to expand (included in the wizard env block).
          </li>
          <li>
            Run Vector on that machine. The collector forces <code>host</code> from the agent name
            on ingest. Agent Online/Offline status uses authenticated contact on{' '}
            <code>/v1/ingest/health</code> (startup healthcheck + a <code>heartbeat</code>{' '}
            <code>http_client</code> source every 30s) and log ingest.
          </li>
        </ol>
        <p className="pt-2">
          Prefer drafting offline? Use the{' '}
          <a href="#generator">YAML generator</a> — paste a real token from the admin UI when ready.
        </p>

        <h3 className="pt-4 text-base font-semibold text-[var(--text)]">Platform presets</h3>
        <div className="overflow-x-auto">
          <table className="w-full min-w-[28rem] border-collapse text-left text-sm">
            <thead>
              <tr className="border-b border-[var(--border)] text-[var(--text-muted)]">
                <th className="py-2 pr-3 font-medium">Preset</th>
                <th className="py-2 pr-3 font-medium">Vector source</th>
                <th className="py-2 font-medium">Notes</th>
              </tr>
            </thead>
            <tbody className="text-[var(--text-muted)]">
              <tr className="border-b border-[var(--border)]/60">
                <td className="py-2 pr-3 text-[var(--text)]">Docker</td>
                <td className="py-2 pr-3">
                  <code>docker_logs</code>
                </td>
                <td className="py-2">Needs Docker socket access</td>
              </tr>
              <tr className="border-b border-[var(--border)]/60">
                <td className="py-2 pr-3 text-[var(--text)]">Linux</td>
                <td className="py-2 pr-3">
                  <code>journald</code>
                </td>
                <td className="py-2">Remaps MESSAGE / unit → searchable fields</td>
              </tr>
              <tr className="border-b border-[var(--border)]/60">
                <td className="py-2 pr-3 text-[var(--text)]">Windows</td>
                <td className="py-2 pr-3">
                  <code>windows_event_log</code>
                </td>
                <td className="py-2">Bearer inlined in YAML (no .env)</td>
              </tr>
              <tr className="border-b border-[var(--border)]/60">
                <td className="py-2 pr-3 text-[var(--text)]">macOS</td>
                <td className="py-2 pr-3">
                  <code>file</code>
                </td>
                <td className="py-2">Common /var/log and /Library/Logs paths</td>
              </tr>
              <tr>
                <td className="py-2 pr-3 text-[var(--text)]">Files</td>
                <td className="py-2 pr-3">
                  <code>file</code>
                </td>
                <td className="py-2">Generic globs — edit include for your apps</td>
              </tr>
            </tbody>
          </table>
        </div>
      </DocSection>

      <DocSection id="mcp" title="Connect an MCP client">
        <ol className="list-decimal space-y-2 pl-5 text-[var(--text-muted)]">
          <li>
            In the admin UI → <strong className="text-[var(--text)]">MCP</strong> → enter admin
            password → <strong className="text-[var(--text)]">Reveal MCP token</strong>.
          </li>
          <li>
            Copy the MCP yaml (token is inlined). You can{' '}
            <strong className="text-[var(--text)]">Generate new token</strong> after reveal to
            rotate it.
          </li>
          <li>
            Add it to your client config (for Hermes:{' '}
            <code>~/.hermes/config.yaml</code>).
          </li>
        </ol>
        <p>
          Example questions: “what errors happened on app-server-1 in the last hour?” — the client
          should facet/search/context through MCP. Keyword search covers all logs; semantic search
          is optional (set <code>EMBEDDINGS_*</code>).
        </p>
      </DocSection>

      <DocSection id="query-api" title="Query API (REST)">
        <p>
          Interactive OpenAPI docs (Swagger UI): <code>/docs</code> — raw spec at{' '}
          <code>/api-docs/openapi.json</code>.
        </p>
        <p>Bearer query key:</p>
        <ul className="list-disc space-y-1 pl-5 text-[var(--text-muted)]">
          <li>
            <code>GET /v1/query/schema</code>
          </li>
          <li>
            <code>POST /v1/query/facets</code>
          </li>
          <li>
            <code>POST /v1/query/search</code>
          </li>
          <li>
            <code>GET /v1/query/events/:id</code>
          </li>
          <li>
            <code>POST /v1/query/context</code>
          </li>
        </ul>
        <p>
          Ingest (bearer ingest key from an agent): <code>POST /v1/logs</code>,{' '}
          <code>GET /v1/ingest/health</code>.
        </p>
      </DocSection>

      <DocSection id="env" title="Env vars">
        <p>
          See <code>.env.example</code> in the repo for a copy-paste template.
        </p>
        <div className="overflow-x-auto">
          <table className="w-full min-w-[36rem] border-collapse text-left text-sm">
            <thead>
              <tr className="border-b border-[var(--border)] text-[var(--text-muted)]">
                <th className="py-2 pr-3 font-medium">Var</th>
                <th className="py-2 pr-3 font-medium">Default</th>
                <th className="py-2 font-medium">Purpose</th>
              </tr>
            </thead>
            <tbody className="text-[var(--text-muted)]">
              {ENV_ROWS.map(([name, def, purpose]) => (
                <tr key={name} className="border-b border-[var(--border)]/60 align-top">
                  <td className="py-2 pr-3 whitespace-nowrap">
                    <code className="text-[var(--text)]">{name}</code>
                  </td>
                  <td className="py-2 pr-3">
                    <code>{def}</code>
                  </td>
                  <td className="py-2">{purpose}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
        <p className="pt-2">
          <code>INGEST_TOKEN</code> and{' '}
          <code>VECTOR_DANGEROUSLY_ALLOW_ENV_VAR_INTERPOLATION</code> are for{' '}
          <strong className="text-[var(--text)]">Vector agents</strong>, not this process.
        </p>
      </DocSection>

      <DocSection id="architecture" title="Architecture notes">
        <ul className="list-disc space-y-1 pl-5 text-[var(--text-muted)]">
          <li>SQLite WAL + single writer task (safe under multi-agent HTTP concurrency)</li>
          <li>Per-key rate limits + queue 429 when saturated</li>
          <li>FTS5 indexes every message; embeddings are selective (errors/stderr + sample)</li>
          <li>Retention worker deletes by age / max rows</li>
        </ul>
      </DocSection>
    </div>
  )
}

function DocSection({
  id,
  title,
  children,
}: {
  id: string
  title: string
  children: ReactNode
}) {
  return (
    <section id={id} className="scroll-mt-24 space-y-3">
      <h2 className="border-b border-[var(--border)] pb-2 text-xl font-semibold">{title}</h2>
      <div className="space-y-3 text-[var(--text-muted)] [&_strong]:text-[var(--text)]">
        {children}
      </div>
    </section>
  )
}

const ENV_ROWS: [string, string, string][] = [
  ['BIND', '0.0.0.0:8080', 'Listen address'],
  ['DATA_DIR', '/data', 'Data directory (created on start)'],
  ['DATABASE_PATH', '$DATA_DIR/logdb.sqlite', 'SQLite file path'],
  ['WEB_DIR', 'unset', 'Static admin UI directory; if unset or missing, API-only'],
  ['ADMIN_USERNAME', 'admin', 'Admin UI login'],
  ['ADMIN_PASSWORD', 'admin', 'Admin UI password'],
  ['SESSION_SECRET', 'dev-session-secret-change-me', 'Cookie signing — change in production'],
  ['PUBLIC_BASE_URL', 'http://localhost:8080', 'Base URL in Vector/MCP snippets'],
  ['RETENTION_DAYS', '14', 'Auto-delete events older than N days'],
  ['MAX_EVENTS', 'unset', 'Optional max row cap (oldest trimmed)'],
  ['WRITE_QUEUE_CAPACITY', '64', 'In-memory ingest queue size'],
  ['MAX_BODY_BYTES', '10485760 (10 MiB)', 'Max ingest request body'],
  ['PER_KEY_RPS', '50', 'Per-API-key rate limit'],
  ['BOOTSTRAP_INGEST_KEY', 'unset', 'Seed ingest API key on boot'],
  ['BOOTSTRAP_QUERY_KEY', 'unset', 'Seed query API key on boot'],
  ['EMBEDDINGS_BASE_URL', 'unset', 'OpenAI-compatible embeddings API base'],
  ['EMBEDDINGS_MODEL', 'unset', 'Embedding model (URL + model both required for semantic search)'],
  ['EMBEDDINGS_API_KEY', 'unset', 'Optional auth for embeddings API'],
  ['EMBEDDING_DIM', '1536', 'Expected embedding vector size'],
  ['EMBED_SAMPLE_RATE', '0.02', 'Fraction of events queued for embedding'],
  ['RUST_LOG', 'info', 'Log filter (tracing-subscriber)'],
]
