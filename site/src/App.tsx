import { useEffect, useState } from 'react'
import { DocsPage } from './pages/Docs'
import { GeneratorPage } from './pages/Generator'
import { HomePage } from './pages/Home'

export type Page = 'home' | 'docs' | 'generator'

const REPO = 'https://github.com/roormonger/vector-collector'

function pageFromHash(): Page {
  const raw = window.location.hash.replace(/^#\/?/, '').split(/[/?#]/)[0]
  if (raw === 'docs' || raw === 'generator') return raw
  return 'home'
}

function hashFor(page: Page): string {
  return page === 'home' ? '#/' : `#/${page}`
}

export default function App() {
  const [page, setPage] = useState<Page>(() => pageFromHash())

  useEffect(() => {
    const onHash = () => setPage(pageFromHash())
    window.addEventListener('hashchange', onHash)
    return () => window.removeEventListener('hashchange', onHash)
  }, [])

  function navigate(next: Page) {
    window.location.hash = hashFor(next)
    setPage(next)
  }

  return (
    <div className="min-h-screen">
      <header className="sticky top-0 z-20 border-b border-[var(--border)] bg-[var(--bg)]/90 backdrop-blur-md">
        <div className="mx-auto flex max-w-6xl flex-wrap items-center justify-between gap-3 px-4 py-3 sm:px-6">
          <div className="flex min-w-0 flex-wrap items-center gap-3 sm:gap-5">
            <button
              type="button"
              onClick={() => navigate('home')}
              className="min-w-0 text-left"
            >
              <p className="truncate text-base font-semibold tracking-tight">Vector Collector</p>
              <p className="truncate text-xs text-[var(--text-muted)]">Docs &amp; config</p>
            </button>
            <nav className="flex flex-wrap gap-1">
              {(
                [
                  ['home', 'Home'],
                  ['docs', 'Docs'],
                  ['generator', 'Generator'],
                ] as const
              ).map(([id, label]) => (
                <button
                  key={id}
                  type="button"
                  onClick={() => navigate(id)}
                  className={`rounded-md px-3 py-1.5 text-sm font-medium ${
                    page === id
                      ? 'bg-[var(--accent)] text-white'
                      : 'text-[var(--text-muted)] hover:bg-[var(--bg-muted)] hover:text-[var(--text)]'
                  }`}
                >
                  {label}
                </button>
              ))}
            </nav>
          </div>
          <a
            href={REPO}
            target="_blank"
            rel="noreferrer"
            className="rounded-md border border-[var(--border)] px-3 py-1.5 text-sm font-medium text-[var(--text)] no-underline hover:border-[var(--accent)]"
          >
            GitHub
          </a>
        </div>
      </header>

      <main className="mx-auto max-w-6xl px-4 py-8 sm:px-6">
        {page === 'home' && <HomePage onNavigate={navigate} />}
        {page === 'docs' && <DocsPage />}
        {page === 'generator' && <GeneratorPage />}
      </main>

      <footer className="border-t border-[var(--border)] py-6 text-center text-xs text-[var(--text-muted)]">
        MIT ·{' '}
        <a href={REPO} target="_blank" rel="noreferrer">
          roormonger/vector-collector
        </a>
      </footer>
    </div>
  )
}
