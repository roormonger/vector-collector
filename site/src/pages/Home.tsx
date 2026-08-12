const REPO = 'https://github.com/roormonger/vector-collector'

type Page = 'home' | 'docs' | 'generator'

export function HomePage({ onNavigate }: { onNavigate: (p: Page) => void }) {
  return (
    <div className="space-y-12">
      <section className="max-w-2xl space-y-5 pt-4">
        <p className="text-sm uppercase tracking-[0.2em] text-[var(--text-muted)]">Vector Collector</p>
        <h1 className="text-3xl font-semibold tracking-tight sm:text-4xl">
          All your machines&apos; logs in one place — searchable by AI agents.
        </h1>
        <p className="text-lg text-[var(--text-muted)]">
          Single-container collector for{' '}
          <a href="https://github.com/vectordotdev/vector" target="_blank" rel="noreferrer">
            Vector
          </a>{' '}
          agents, with REST + MCP search so you can query logs with natural language.
        </p>
        <div className="flex flex-wrap gap-3 pt-1">
          <button
            type="button"
            onClick={() => onNavigate('docs')}
            className="rounded-md bg-[var(--accent)] px-4 py-2 text-sm font-medium text-white hover:bg-[var(--accent-hover)]"
          >
            Read the docs
          </button>
          <button
            type="button"
            onClick={() => onNavigate('generator')}
            className="rounded-md border border-[var(--border)] bg-[var(--bg-elevated)] px-4 py-2 text-sm font-medium hover:border-[var(--accent)]"
          >
            YAML generator
          </button>
          <a
            href={REPO}
            target="_blank"
            rel="noreferrer"
            className="rounded-md border border-[var(--border)] px-4 py-2 text-sm font-medium text-[var(--text)] no-underline hover:border-[var(--accent)]"
          >
            GitHub
          </a>
        </div>
      </section>

      <section className="grid gap-6 sm:grid-cols-3">
        {[
          {
            title: 'Ingest',
            body: 'Vector HTTP sink → POST /v1/logs with gzip and per-agent API keys.',
          },
          {
            title: 'Search',
            body: 'SQLite + FTS5 on every line, plus optional semantic embeddings.',
          },
          {
            title: 'MCP',
            body: 'HTTP /mcp with logs_facets → logs_search → logs_context for agent clients.',
          },
        ].map((item) => (
          <div key={item.title} className="space-y-2 border-t border-[var(--border)] pt-4">
            <h2 className="text-base font-semibold">{item.title}</h2>
            <p className="text-sm text-[var(--text-muted)]">{item.body}</p>
          </div>
        ))}
      </section>

      <section className="space-y-3">
        <h2 className="text-xl font-semibold">Quick start</h2>
        <pre>{`docker compose up --build -d
# open http://localhost:8080 — admin / admin`}</pre>
        <p className="text-sm text-[var(--text-muted)]">
          One Docker image, one volume, one port. Full setup and env vars are in{' '}
          <button
            type="button"
            className="text-[var(--accent-hover)] underline"
            onClick={() => onNavigate('docs')}
          >
            Docs
          </button>
          .
        </p>
      </section>
    </div>
  )
}
