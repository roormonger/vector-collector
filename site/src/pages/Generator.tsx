import { useMemo, useState, type ReactNode } from 'react'
import {
  generateConfig,
  PRESETS,
  type PlatformId,
} from '../lib/vectorPresets'

export function GeneratorPage() {
  const [platform, setPlatform] = useState<PlatformId>('docker')
  const [baseUrl, setBaseUrl] = useState('http://192.168.1.10:8080')
  const [token, setToken] = useState('')
  const [copied, setCopied] = useState<'yaml' | 'env' | null>(null)

  const config = useMemo(
    () => generateConfig(baseUrl, token, platform),
    [baseUrl, token, platform],
  )

  async function copy(kind: 'yaml' | 'env', value: string) {
    await navigator.clipboard.writeText(value)
    setCopied(kind)
    window.setTimeout(() => setCopied(null), 1500)
  }

  function download(filename: string, value: string) {
    const blob = new Blob([value], { type: 'text/yaml;charset=utf-8' })
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = filename
    a.click()
    URL.revokeObjectURL(url)
  }

  return (
    <div className="mx-auto max-w-3xl space-y-8 pb-16">
      <header className="space-y-2">
        <h1 className="text-3xl font-semibold tracking-tight">YAML generator</h1>
        <p className="text-[var(--text-muted)]">
          Draft Vector agent configs offline. Real ingest tokens still come from the admin UI after
          you create an agent — paste them here when ready.
        </p>
      </header>

      <div className="grid gap-4 sm:grid-cols-2">
        <label className="block space-y-1.5 sm:col-span-2">
          <span className="text-sm font-medium">Collector URL</span>
          <input
            value={baseUrl}
            onChange={(e) => setBaseUrl(e.target.value)}
            placeholder="http://192.168.1.10:8080"
            className="w-full rounded-md border border-[var(--border)] bg-[var(--bg-elevated)] px-3 py-2 text-sm outline-none focus:border-[var(--accent)] focus:ring-1 focus:ring-[var(--ring)]"
          />
          <span className="text-xs text-[var(--text-muted)]">
            Same idea as <code>PUBLIC_BASE_URL</code> — LAN IP or reverse-proxy origin, not localhost
            if agents are remote.
          </span>
        </label>

        <label className="block space-y-1.5">
          <span className="text-sm font-medium">Ingest token</span>
          <input
            value={token}
            onChange={(e) => setToken(e.target.value)}
            placeholder="lk_… (optional)"
            className="w-full rounded-md border border-[var(--border)] bg-[var(--bg-elevated)] px-3 py-2 text-sm outline-none focus:border-[var(--accent)] focus:ring-1 focus:ring-[var(--ring)]"
          />
        </label>

        <label className="block space-y-1.5">
          <span className="text-sm font-medium">Platform preset</span>
          <select
            value={platform}
            onChange={(e) => setPlatform(e.target.value as PlatformId)}
            className="w-full rounded-md border border-[var(--border)] bg-[var(--bg-elevated)] px-3 py-2 text-sm outline-none focus:border-[var(--accent)] focus:ring-1 focus:ring-[var(--ring)]"
          >
            {PRESETS.map((p) => (
              <option key={p.id} value={p.id}>
                {p.label}
              </option>
            ))}
          </select>
        </label>
      </div>

      <p className="text-sm text-[var(--text-muted)]">
        {PRESETS.find((p) => p.id === platform)?.description}
      </p>

      {config.inlineToken && (
        <p className="rounded-md border border-[var(--border)] bg-[var(--bg-elevated)] px-3 py-2 text-sm text-[var(--text-muted)]">
          This preset inlines the bearer token in the YAML — no env file or Vector env-interpolation
          flag needed. Treat the file as secret.
        </p>
      )}

      <div className="space-y-2">
        <div className="flex flex-wrap items-center justify-between gap-2">
          <h2 className="text-base font-semibold">Vector yaml</h2>
          <div className="flex gap-2">
            <GhostButton onClick={() => copy('yaml', config.yaml)}>
              {copied === 'yaml' ? 'Copied' : 'Copy'}
            </GhostButton>
            <GhostButton
              onClick={() => download(`vector-collector-${platform}.yaml`, config.yaml)}
            >
              Download
            </GhostButton>
          </div>
        </div>
        <pre className="max-h-[28rem] overflow-auto">{config.yaml}</pre>
      </div>

      <div className="space-y-2">
        <div className="flex flex-wrap items-center justify-between gap-2">
          <h2 className="text-base font-semibold">Env</h2>
          <div className="flex gap-2">
            <GhostButton onClick={() => copy('env', config.env)}>
              {copied === 'env' ? 'Copied' : 'Copy'}
            </GhostButton>
            {!config.inlineToken && (
              <GhostButton onClick={() => download('vector-collector.env', config.env)}>
                Download
              </GhostButton>
            )}
          </div>
        </div>
        <pre>{config.env}</pre>
      </div>
    </div>
  )
}

function GhostButton({
  children,
  onClick,
}: {
  children: ReactNode
  onClick: () => void
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      className="rounded-md border border-[var(--border)] bg-[var(--bg-elevated)] px-2.5 py-1 text-xs font-medium hover:border-[var(--accent)]"
    >
      {children}
    </button>
  )
}
