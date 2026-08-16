import { useEffect, useState, type ReactNode } from 'react'

export type DocSectionId =
  | 'overview'
  | 'install-vector'
  | 'install-collector'
  | 'configure-collector'
  | 'configure-vector'
  | 'mcp'
  | 'query-api'
  | 'architecture'

type NavItem = { id: DocSectionId; label: string }

const NAV: NavItem[] = [
  { id: 'overview', label: 'Overview' },
  { id: 'install-vector', label: 'Install Vector' },
  { id: 'install-collector', label: 'Install Collector' },
  { id: 'configure-collector', label: 'Configure Collector' },
  { id: 'configure-vector', label: 'Configure Vector' },
  { id: 'mcp', label: 'Query with MCP' },
  { id: 'query-api', label: 'Query API' },
  { id: 'architecture', label: 'Architecture' },
]

function sectionFromHash(): DocSectionId {
  const parts = window.location.hash.replace(/^#\/?/, '').split(/[/?#]/)
  const id = parts[0] === 'docs' ? parts[1] : undefined
  if (id && NAV.some((n) => n.id === id)) return id as DocSectionId
  return 'overview'
}

export function DocsPage() {
  const [section, setSection] = useState<DocSectionId>(() => sectionFromHash())

  useEffect(() => {
    const onHash = () => setSection(sectionFromHash())
    window.addEventListener('hashchange', onHash)
    return () => window.removeEventListener('hashchange', onHash)
  }, [])

  function go(id: DocSectionId) {
    window.location.hash = `#/docs/${id}`
    setSection(id)
    window.scrollTo({ top: 0, behavior: 'smooth' })
  }

  const idx = NAV.findIndex((n) => n.id === section)
  const prev = idx > 0 ? NAV[idx - 1] : null
  const next = idx >= 0 && idx < NAV.length - 1 ? NAV[idx + 1] : null

  return (
    <div className="flex flex-col gap-8 pb-16 lg:flex-row lg:gap-10">
      <aside className="lg:w-56 lg:shrink-0">
        <div className="lg:sticky lg:top-24">
          <p className="mb-3 text-xs font-semibold uppercase tracking-[0.15em] text-[var(--text-muted)]">
            Docs
          </p>
          <nav className="flex gap-1 overflow-x-auto pb-1 lg:flex-col lg:overflow-visible lg:pb-0">
            {NAV.map((item) => (
              <button
                key={item.id}
                type="button"
                onClick={() => go(item.id)}
                className={`whitespace-nowrap rounded-md px-3 py-1.5 text-left text-sm ${
                  section === item.id
                    ? 'bg-[var(--bg-muted)] font-medium text-[var(--text)]'
                    : 'text-[var(--text-muted)] hover:bg-[var(--bg-elevated)] hover:text-[var(--text)]'
                }`}
              >
                {item.label}
              </button>
            ))}
          </nav>
        </div>
      </aside>

      <div className="min-w-0 flex-1 space-y-8">
        {section === 'overview' && <Overview />}
        {section === 'install-vector' && <InstallVector />}
        {section === 'install-collector' && <InstallCollector />}
        {section === 'configure-collector' && <ConfigureCollector />}
        {section === 'configure-vector' && <ConfigureVector />}
        {section === 'mcp' && <McpDocs />}
        {section === 'query-api' && <QueryApi />}
        {section === 'architecture' && <Architecture />}

        <div className="flex flex-wrap items-center justify-between gap-3 border-t border-[var(--border)] pt-6">
          {prev ? (
            <button
              type="button"
              onClick={() => go(prev.id)}
              className="text-sm text-[var(--text-muted)] hover:text-[var(--accent-hover)]"
            >
              ← {prev.label}
            </button>
          ) : (
            <span />
          )}
          {next ? (
            <button
              type="button"
              onClick={() => go(next.id)}
              className="text-sm font-medium text-[var(--accent-hover)] hover:underline"
            >
              {next.label} →
            </button>
          ) : (
            <span />
          )}
        </div>
      </div>
    </div>
  )
}

function Section({ title, lead, children }: { title: string; lead?: string; children: ReactNode }) {
  return (
    <article className="space-y-4">
      <header className="space-y-2">
        <h1 className="text-3xl font-semibold tracking-tight">{title}</h1>
        {lead && <p className="text-lg text-[var(--text-muted)]">{lead}</p>}
      </header>
      <div className="space-y-4 text-[var(--text-muted)] [&_strong]:text-[var(--text)]">{children}</div>
    </article>
  )
}

function Overview() {
  return (
    <Section
      title="Overview"
      lead="One collector for every machine’s logs — searchable by people and AI agents."
    >
      <p>
        <strong>Vector Collector</strong> is a single-container log store that receives events from{' '}
        <a href="https://github.com/vectordotdev/vector" target="_blank" rel="noreferrer">
          Vector
        </a>{' '}
        agents, indexes them in SQLite (full-text + optional embeddings), and exposes search over REST
        and MCP so coding agents can ask questions like “what errors happened on app-server-1 in the
        last hour?”
      </p>

      <h2 className="pt-2 text-lg font-semibold text-[var(--text)]">When to use it</h2>
      <ul className="list-disc space-y-2 pl-5">
        <li>
          You already run (or can run) Vector on hosts/containers and want a <strong>self-hosted</strong>{' '}
          destination that stays on your LAN or VPS.
        </li>
        <li>
          You care more about <strong>agent-friendly search</strong> (facets → search → context) than a
          full observability suite with metrics and traces.
        </li>
        <li>
          You want <strong>one Docker image, one volume, one port</strong> — not a cluster of ingest,
          index, and UI services.
        </li>
      </ul>

      <h2 className="pt-2 text-lg font-semibold text-[var(--text)]">How it compares</h2>
      <div className="overflow-x-auto">
        <table className="w-full min-w-[32rem] border-collapse text-left text-sm">
          <thead>
            <tr className="border-b border-[var(--border)]">
              <th className="py-2 pr-3 font-medium text-[var(--text-muted)]">Approach</th>
              <th className="py-2 font-medium text-[var(--text-muted)]">Trade-off</th>
            </tr>
          </thead>
          <tbody>
            <tr className="border-b border-[var(--border)]/60 align-top">
              <td className="py-2 pr-3 text-[var(--text)]">ELK / Loki / cloud log platforms</td>
              <td className="py-2">
                Powerful dashboards and scale — heavier to run and operate for a small fleet.
              </td>
            </tr>
            <tr className="border-b border-[var(--border)]/60 align-top">
              <td className="py-2 pr-3 text-[var(--text)]">SSH + journalctl / docker logs</td>
              <td className="py-2">Fine for one box; painful across many hosts and useless to MCP clients.</td>
            </tr>
            <tr className="align-top">
              <td className="py-2 pr-3 text-[var(--text)]">Vector Collector</td>
              <td className="py-2">
                Thin collector focused on ingest + FTS/MCP search. You keep Vector’s reliability for
                shipping; we keep the store simple.
              </td>
            </tr>
          </tbody>
        </table>
      </div>

      <h2 className="pt-2 text-lg font-semibold text-[var(--text)]">Typical path</h2>
      <ol className="list-decimal space-y-2 pl-5">
        <li>Install Vector on each machine that produces logs.</li>
        <li>Install Vector Collector once (Docker) — data always under <code>/data</code>.</li>
        <li>
          In admin <strong>Settings</strong>, set Public URL, retention, and any ingest/embeddings
          options.
        </li>
        <li>Create an agent and paste the generated Vector config on that host.</li>
        <li>Connect an MCP client (or the REST API) and start querying.</li>
      </ol>
      <p>Follow the sidebar top-to-bottom — that’s the same order.</p>
    </Section>
  )
}

function InstallVector() {
  return (
    <Section
      title="Install Vector"
      lead="Vector runs on each machine and ships logs to the collector."
    >
      <p>
        Install the official Vector agent for your OS. Use Vector <strong>0.57+</strong> if you want
        env-var interpolation (<code>${'{INGEST_TOKEN}'}</code>) in YAML — our presets enable that
        flag for you.
      </p>
      <ul className="list-disc space-y-2 pl-5">
        <li>
          Docs:{' '}
          <a href="https://vector.dev/docs/setup/installation/" target="_blank" rel="noreferrer">
            vector.dev — Installation
          </a>
        </li>
        <li>
          Releases:{' '}
          <a href="https://github.com/vectordotdev/vector/releases" target="_blank" rel="noreferrer">
            GitHub releases
          </a>
        </li>
      </ul>

      <h2 className="pt-2 text-lg font-semibold text-[var(--text)]">What you need per host</h2>
      <ul className="list-disc space-y-2 pl-5">
        <li>Vector binary or package installed and able to start as a service or container.</li>
        <li>
          Permission to read the logs you care about (Docker socket, journald group, Windows Event Log
          rights, or file paths / Full Disk Access on macOS).
        </li>
        <li>
          Network reachability to the collector Public URL from Settings (HTTP or HTTPS).
        </li>
      </ul>

      <p>
        You don’t configure sinks yet — that comes after the collector is up and you’ve created an
        agent. Next: install Vector Collector.
      </p>
    </Section>
  )
}

function InstallCollector() {
  return (
    <Section
      title="Install Vector Collector"
      lead="One container, one volume, one port — run this once."
    >
      <h2 className="text-lg font-semibold text-[var(--text)]">Docker Compose (recommended)</h2>
      <p>
        From the{' '}
        <a href="https://github.com/roormonger/vector-collector" target="_blank" rel="noreferrer">
          repository
        </a>
        :
      </p>
      <pre>{`docker compose up --build -d`}</pre>
      <p>
        Open <code>http://localhost:8080</code> — default login <code>admin</code> /{' '}
        <code>admin</code>.
      </p>

      <h2 className="pt-2 text-lg font-semibold text-[var(--text)]">Docker run</h2>
      <pre>{`docker build -t logdb .
docker run -d -p 8080:8080 \\
  -v logdb-data:/data \\
  -e ADMIN_PASSWORD=admin \\
  -e PUBLIC_BASE_URL=http://localhost:8080 \\
  logdb`}</pre>
      <p>
        Copy <code>.env.example</code> to <code>.env</code> before production use. Never commit{' '}
        <code>.env</code> or the contents of the data volume.
      </p>

      <h2 className="pt-2 text-lg font-semibold text-[var(--text)]">Local development</h2>
      <p>Requirements: Rust (MSVC on Windows), Node 20+.</p>
      <pre>{`# terminal 1 — API
set WEB_DIR=web/dist
cargo run

# terminal 2 — UI
cd web
npm install
npm run dev`}</pre>
      <p>
        Vite proxies <code>/v1</code> to <code>http://127.0.0.1:8080</code>. Data always lives at{' '}
        <code>/data</code> (SQLite <code>/data/logdb.sqlite</code>) — on Windows that is drive-root{' '}
        <code>\data</code>. Prefer Docker so the volume matches production.
      </p>
      <p>
        Note: the Rust binary and Docker image are still named <code>logdb</code> for now.
      </p>
    </Section>
  )
}

function ConfigureCollector() {
  return (
    <Section
      title="Configure the collector"
      lead="Use the admin Settings UI for day-to-day knobs; keep secrets and listen address in env."
    >
      <h2 className="text-lg font-semibold text-[var(--text)]">Public URL (required for remote agents)</h2>
      <p>
        In the admin UI → <strong>Settings</strong>, set <strong>Public URL</strong> to the URL{' '}
        <strong>Vector agents and MCP clients use</strong> — e.g.{' '}
        <code>http://192.168.1.10:8080</code> on a LAN, or <code>https://logs.example.com</code>{' '}
        behind a reverse proxy.
      </p>
      <p>
        Do <strong>not</strong> leave it as <code>http://localhost:8080</code> if Vector runs on other
        machines. Generated Vector/MCP snippets use this value immediately after Save.{' '}
        <code>PUBLIC_BASE_URL</code> in <code>.env</code> is only a first-boot default.
      </p>

      <h2 className="pt-2 text-lg font-semibold text-[var(--text)]">Security basics</h2>
      <ul className="list-disc space-y-2 pl-5">
        <li>
          Change <code>ADMIN_USERNAME</code> / <code>ADMIN_PASSWORD</code> from the defaults.
        </li>
        <li>
          Set a long random <code>SESSION_SECRET</code> for cookie signing.
        </li>
        <li>
          Prefer HTTPS at a reverse proxy if the collector is reachable beyond your LAN.
        </li>
      </ul>

      <h2 className="pt-2 text-lg font-semibold text-[var(--text)]">Settings UI</h2>
      <ul className="list-disc space-y-2 pl-5">
        <li>Retention days and optional max events</li>
        <li>Ingest queue capacity, max body size (restart required), per-key rate limit (live)</li>
        <li>Semantic search: embeddings base URL, model, API key, dimension, sample rate</li>
      </ul>

      <h2 className="pt-2 text-lg font-semibold text-[var(--text)]">Data paths</h2>
      <p>
        Fixed: volume <code>/data</code>, database <code>/data/logdb.sqlite</code>. Not configurable.
      </p>

      <h2 className="pt-2 text-lg font-semibold text-[var(--text)]">Listen address (`BIND`)</h2>
      <p>
        <code>0.0.0.0:8080</code> (default) listens on all interfaces — correct for Docker published
        ports. <code>127.0.0.1:8080</code> is this machine only (bare-metal local access). Not a GUI
        setting.
      </p>

      <h2 className="pt-2 text-lg font-semibold text-[var(--text)]">Environment reference</h2>
      <p>
        Full template: <code>.env.example</code> in the repo. Operational knobs belong in Settings;
        env values for those are first-boot defaults only.
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
          <tbody>
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
      <p>
        <code>INGEST_TOKEN</code> and <code>VECTOR_DANGEROUSLY_ALLOW_ENV_VAR_INTERPOLATION</code> are
        for <strong>Vector agents</strong>, not this process.
      </p>
    </Section>
  )
}

function ConfigureVector() {
  return (
    <Section
      title="Configure Vector"
      lead="Create an agent in the admin UI, then run the generated config on that host."
    >
      <ol className="list-decimal space-y-3 pl-5">
        <li>
          In the admin UI → <strong>Agents</strong> → <strong>Create agent</strong>. Re-enter the
          admin password, set a name (this becomes the log hostname, e.g. <code>app-server-1</code>),
          and pick a <strong>platform preset</strong>.
        </li>
        <li>
          Copy the Vector YAML and env block. URIs come from the Public URL in{' '}
          <strong>Settings</strong>. You can switch presets after create/reveal — same ingest token,
          different YAML.
        </li>
        <li>
          On that machine, install the config (and env if needed) and start Vector. Vector{' '}
          <strong>0.57+</strong> needs{' '}
          <code>VECTOR_DANGEROUSLY_ALLOW_ENV_VAR_INTERPOLATION=true</code> so{' '}
          <code>${'{INGEST_TOKEN}'}</code> expands — included in the wizard env block.
        </li>
      </ol>

      <p>
        The collector forces <code>host</code> from the agent name on ingest — no Vector{' '}
        <code>AGENT_NAME</code> remap needed. Online/Offline uses authenticated contact on{' '}
        <code>/v1/ingest/health</code> (startup healthcheck + a <code>heartbeat</code>{' '}
        <code>http_client</code> source every 30s) and log ingest. Vector’s own{' '}
        <code>api.enabled</code> is not required.
      </p>

      <p>
        Draft configs offline with the{' '}
        <a href="#/generator">YAML generator</a>, then paste a real token from the admin UI.
      </p>

      <h2 className="pt-2 text-lg font-semibold text-[var(--text)]">Platform presets</h2>
      <div className="overflow-x-auto">
        <table className="w-full min-w-[28rem] border-collapse text-left text-sm">
          <thead>
            <tr className="border-b border-[var(--border)] text-[var(--text-muted)]">
              <th className="py-2 pr-3 font-medium">Preset</th>
              <th className="py-2 pr-3 font-medium">Vector source</th>
              <th className="py-2 font-medium">Notes</th>
            </tr>
          </thead>
          <tbody>
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

      <h2 className="pt-2 text-lg font-semibold text-[var(--text)]">Removing an agent</h2>
      <p>
        Agents → Remove revokes that machine’s ingest key. Historical logs stay until retention
        deletes them.
      </p>
    </Section>
  )
}

function McpDocs() {
  return (
    <Section
      title="Query with MCP"
      lead="Point an MCP-compatible client at /mcp and ask questions in natural language."
    >
      <ol className="list-decimal space-y-3 pl-5">
        <li>
          Admin UI → <strong>MCP</strong> → enter admin password → <strong>Reveal MCP token</strong>.
        </li>
        <li>
          Copy the MCP YAML (token is inlined). Use <strong>Generate new token</strong> after reveal
          to rotate.
        </li>
        <li>
          Add it to your client config (for{' '}
          <a href="https://github.com/NousResearch/hermes-agent" target="_blank" rel="noreferrer">
            Hermes
          </a>
          : <code>~/.hermes/config.yaml</code>).
        </li>
      </ol>

      <p>
        Tools are ordered for agents: <code>logs_facets</code> → <code>logs_search</code> →{' '}
        <code>logs_context</code>.         Keyword search covers all logs; semantic search is optional (configure embeddings in admin
        Settings).
      </p>
      <p>
        Example: “what errors happened on app-server-1 in the last hour?”
      </p>
    </Section>
  )
}

function QueryApi() {
  return (
    <Section
      title="Query API"
      lead="REST endpoints for scripts and custom clients — same data MCP uses."
    >
      <p>
        Interactive OpenAPI (Swagger UI) on the running collector: <code>/docs</code> — full spec at{' '}
        <code>/api-docs/openapi.json</code> (includes ingest).
      </p>
      <p>
        Open WebUI Tool Servers: set URL to the collector origin and OpenAPI Spec to{' '}
        <code>/api-docs/query.json</code> (not the default <code>openapi.json</code>), plus the query
        bearer token from Connect. That spec has no ingest endpoints. MCP clients keep using{' '}
        <code>/mcp</code>.
      </p>

      <h2 className="text-lg font-semibold text-[var(--text)]">Query (bearer query key)</h2>
      <ul className="list-disc space-y-1 pl-5">
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

      <h2 className="pt-2 text-lg font-semibold text-[var(--text)]">Ingest (bearer agent key)</h2>
      <ul className="list-disc space-y-1 pl-5">
        <li>
          <code>POST /v1/logs</code>
        </li>
        <li>
          <code>GET /v1/ingest/health</code>
        </li>
      </ul>
      <p>
        Bootstrap keys are optional via <code>BOOTSTRAP_INGEST_KEY</code> /{' '}
        <code>BOOTSTRAP_QUERY_KEY</code> on startup.
      </p>
    </Section>
  )
}

function Architecture() {
  return (
    <Section title="Architecture" lead="A few notes on how the collector behaves under load.">
      <ul className="list-disc space-y-2 pl-5">
        <li>SQLite WAL + single writer task (safe under multi-agent HTTP concurrency)</li>
        <li>Per-key rate limits + queue 429 when saturated</li>
        <li>FTS5 indexes every message; embeddings are selective (errors/stderr + sample)</li>
        <li>Retention worker deletes by age / max rows</li>
      </ul>
      <p>License: MIT — see the LICENSE file in the repository.</p>
    </Section>
  )
}

const ENV_ROWS: [string, string, string][] = [
  ['BIND', '0.0.0.0:8080', 'Listen address (all interfaces vs 127.0.0.1 loopback)'],
  ['WEB_DIR', 'unset / image default', 'Static admin UI directory; if unset or missing, API-only'],
  ['ADMIN_USERNAME', 'admin', 'Admin UI login'],
  ['ADMIN_PASSWORD', 'admin', 'Admin UI password'],
  ['SESSION_SECRET', 'dev-session-secret-change-me', 'Cookie signing — change in production'],
  ['PUBLIC_BASE_URL', 'http://localhost:8080', 'First-boot default for Public URL (Settings)'],
  ['BOOTSTRAP_INGEST_KEY', 'unset', 'Seed ingest API key on boot'],
  ['BOOTSTRAP_QUERY_KEY', 'unset', 'Seed query API key on boot'],
  ['RUST_LOG', 'info', 'Log filter (tracing-subscriber)'],
]
