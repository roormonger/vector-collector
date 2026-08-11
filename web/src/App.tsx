import { useEffect, useState, type ReactNode } from 'react'
import {
  api,
  type Agent,
  type AgentConnectInfo,
  type McpConnectInfo,
  type Settings,
  type Stats,
} from './lib/api'
import { Badge, Button, Card, Input, Label } from './components/ui'

type Tab = 'overview' | 'agents' | 'mcp' | 'retention'

export default function App() {
  const [user, setUser] = useState<string | null>(null)
  const [loading, setLoading] = useState(true)
  const [tab, setTab] = useState<Tab>('overview')

  useEffect(() => {
    api
      .me()
      .then((m) => setUser(m.username))
      .catch(() => setUser(null))
      .finally(() => setLoading(false))
  }, [])

  if (loading) {
    return <Shell>Loading…</Shell>
  }
  if (!user) {
    return <Login onSuccess={(u) => setUser(u)} />
  }

  return (
    <Shell>
      <header className="mb-8 flex flex-wrap items-end justify-between gap-4">
        <div>
          <p className="text-sm uppercase tracking-[0.2em] text-[var(--text-muted)]">Vector Collector</p>
          <h1 className="mt-1 text-3xl font-semibold tracking-tight">Admin</h1>
          <p className="mt-1 text-[var(--text-muted)]">Signed in as {user}</p>
        </div>
        <Button
          variant="ghost"
          onClick={async () => {
            await api.logout()
            setUser(null)
          }}
        >
          Log out
        </Button>
      </header>

      <nav className="mb-6 flex flex-wrap gap-2">
        {(
          [
            ['overview', 'Overview'],
            ['agents', 'Agents'],
            ['mcp', 'MCP'],
            ['retention', 'Retention'],
          ] as const
        ).map(([id, label]) => (
          <Button key={id} variant={tab === id ? 'default' : 'ghost'} onClick={() => setTab(id)}>
            {label}
          </Button>
        ))}
      </nav>

      {tab === 'overview' && <Overview />}
      {tab === 'agents' && <AgentsPanel />}
      {tab === 'mcp' && <McpPanel />}
      {tab === 'retention' && <RetentionPanel />}
    </Shell>
  )
}

function Shell({ children }: { children: ReactNode }) {
  return <div className="mx-auto max-w-5xl px-4 py-10 sm:px-6">{children}</div>
}

function Login({ onSuccess }: { onSuccess: (user: string) => void }) {
  const [username, setUsername] = useState('admin')
  const [password, setPassword] = useState('admin')
  const [error, setError] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)

  return (
    <Shell>
      <div className="mx-auto mt-16 max-w-md">
        <p className="text-sm uppercase tracking-[0.2em] text-[var(--text-muted)]">Vector Collector</p>
        <h1 className="mt-2 text-3xl font-semibold">Sign in</h1>
        <p className="mt-2 text-[var(--text-muted)]">
          Admin console for Vector agents and MCP access.
        </p>
        <Card className="mt-6">
          <form
            className="space-y-4"
            onSubmit={async (e) => {
              e.preventDefault()
              setBusy(true)
              setError(null)
              try {
                await api.login(username, password)
                onSuccess(username)
              } catch (err) {
                setError(err instanceof Error ? err.message : 'Login failed')
              } finally {
                setBusy(false)
              }
            }}
          >
            <div>
              <Label>Username</Label>
              <Input value={username} onChange={(e) => setUsername(e.target.value)} autoComplete="username" />
            </div>
            <div>
              <Label>Password</Label>
              <Input
                type="password"
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                autoComplete="current-password"
              />
            </div>
            {error && <p className="text-sm text-[var(--danger)]">{error}</p>}
            <Button className="w-full" type="submit" disabled={busy}>
              {busy ? 'Signing in…' : 'Sign in'}
            </Button>
          </form>
        </Card>
      </div>
    </Shell>
  )
}

function Overview() {
  const [stats, setStats] = useState<Stats | null>(null)
  useEffect(() => {
    api.stats().then(setStats).catch(console.error)
  }, [])
  if (!stats) return <Card>Loading stats…</Card>
  return (
    <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
      <Stat title="Events stored" value={stats.events} />
      <Stat title="Agents" value={stats.agents} />
      <Stat title="Queue depth" value={stats.queue_depth ?? 0} />
      <Stat title="Ingest accepted" value={stats.ingest_accepted ?? 0} />
      <Stat title="Ingest 429s" value={stats.ingest_429 ?? 0} />
      <Stat title="Embed queue" value={stats.embed_queue} />
      <Card className="sm:col-span-2 lg:col-span-3">
        <p className="text-sm text-[var(--text-muted)]">Semantic search</p>
        <p className="mt-1 text-lg">
          {stats.embeddings_enabled ? 'Enabled (OpenAI-compatible embeddings)' : 'Disabled — keyword/FTS only'}
        </p>
      </Card>
      <Card className="sm:col-span-2 lg:col-span-3">
        <p className="text-sm text-[var(--text-muted)]">REST API</p>
        <p className="mt-1 text-lg">
          <a className="text-[var(--accent)] underline-offset-2 hover:underline" href="/docs">
            OpenAPI / Swagger docs
          </a>
        </p>
      </Card>
    </div>
  )
}

function Stat({ title, value }: { title: string; value: number }) {
  return (
    <Card>
      <p className="text-sm text-[var(--text-muted)]">{title}</p>
      <p className="mt-2 text-3xl font-semibold tabular-nums">{value.toLocaleString()}</p>
    </Card>
  )
}

function statusStyles(status: Agent['status']) {
  switch (status) {
    case 'online':
      return 'bg-emerald-500/20 text-emerald-300'
    case 'stale':
      return 'bg-amber-500/20 text-amber-200'
    case 'offline':
      return 'bg-rose-500/15 text-rose-300'
    default:
      return 'bg-[var(--bg-muted)] text-[var(--text-muted)]'
  }
}

function formatTime(value?: string | null) {
  if (!value) return 'never'
  const d = new Date(value)
  if (Number.isNaN(d.getTime())) return value
  return d.toLocaleString()
}

function usePublicBaseUrl() {
  const [url, setUrl] = useState<string | null>(null)
  useEffect(() => {
    api
      .settings()
      .then((s) => setUrl(s.public_base_url))
      .catch(() => setUrl(null))
  }, [])
  return url
}

function PublicUrlHint({ url }: { url: string | null }) {
  if (!url) return null
  return (
    <p className="mt-2 text-sm text-[var(--text-muted)]">
      Snippets use public URL:{' '}
      <code className="text-xs text-[var(--text)]">{url}</code> (from{' '}
      <code className="text-xs">PUBLIC_BASE_URL</code>)
    </p>
  )
}

function AgentsPanel() {
  const [agents, setAgents] = useState<Agent[]>([])
  const [error, setError] = useState<string | null>(null)
  const [wizardOpen, setWizardOpen] = useState(false)
  const [viewAgent, setViewAgent] = useState<Agent | null>(null)
  const [removeAgent, setRemoveAgent] = useState<Agent | null>(null)
  const publicBaseUrl = usePublicBaseUrl()

  const reload = () =>
    api
      .agents()
      .then((rows) => {
        setAgents(rows)
        setError(null)
      })
      .catch((e) => setError(e instanceof Error ? e.message : 'Failed to load agents'))

  useEffect(() => {
    reload()
    const id = window.setInterval(reload, 10000)
    return () => window.clearInterval(id)
  }, [])

  return (
    <>
      <Card>
        <div className="mb-3 flex flex-wrap items-center justify-between gap-2">
          <div>
            <h2 className="text-lg font-medium">Agents</h2>
            <p className="text-sm text-[var(--text-muted)]">
              Create an agent per machine. The name becomes the log hostname. Each agent gets its own ingest
              key and Vector config. Removing an agent revokes its key; stored logs stay until retention
              trims them. On Vector 0.57+, keep{' '}
              <code className="text-xs">VECTOR_DANGEROUSLY_ALLOW_ENV_VAR_INTERPOLATION=true</code>.
            </p>
            <PublicUrlHint url={publicBaseUrl} />
          </div>
          <div className="flex gap-2">
            <Button variant="ghost" onClick={reload}>
              Refresh
            </Button>
            <Button onClick={() => setWizardOpen(true)}>Create agent</Button>
          </div>
        </div>
        {error && <p className="mb-3 text-sm text-[var(--danger)]">{error}</p>}
        <div className="space-y-3">
          {agents.map((a) => (
            <div
              key={a.id}
              className="rounded-lg border border-[var(--border)] bg-[var(--bg)]/50 p-3 last:mb-0"
            >
              <div className="flex flex-wrap items-start justify-between gap-2">
                <div>
                  <p className="font-medium">{a.host}</p>
                  <p className="text-sm text-[var(--text-muted)]">
                    {a.has_connect_secret ? 'Wizard agent' : a.name}
                  </p>
                </div>
                <div className="flex flex-wrap items-center gap-2">
                  <span className={`rounded-full px-2 py-0.5 text-xs capitalize ${statusStyles(a.status)}`}>
                    {a.status}
                  </span>
                  {a.has_connect_secret && (
                    <Button variant="ghost" onClick={() => setViewAgent(a)}>
                      View connect info
                    </Button>
                  )}
                  <Button variant="danger" onClick={() => setRemoveAgent(a)}>
                    Remove
                  </Button>
                </div>
              </div>
              <p className="mt-2 text-sm text-[var(--text-muted)]">
                last seen {formatTime(a.last_seen_at)} · {a.events_ingested.toLocaleString()} events
                {a.key_prefix ? ` · ${a.key_prefix}…` : ''}
              </p>
              {a.recent_containers?.length > 0 && (
                <div className="mt-2 flex flex-wrap gap-2">
                  {a.recent_containers.map((c) => (
                    <Badge key={c.name}>
                      {c.name} ({c.count})
                    </Badge>
                  ))}
                </div>
              )}
            </div>
          ))}
          {agents.length === 0 && (
            <p className="text-[var(--text-muted)]">
              No agents yet — create one to get Vector yaml and an ingest token for that machine.
            </p>
          )}
        </div>
      </Card>

      {wizardOpen && (
        <AgentWizardModal
          onClose={() => setWizardOpen(false)}
          onCreated={() => {
            reload()
          }}
        />
      )}
      {viewAgent && (
        <AgentConnectModal agent={viewAgent} onClose={() => setViewAgent(null)} />
      )}
      {removeAgent && (
        <RemoveAgentModal
          agent={removeAgent}
          onClose={() => setRemoveAgent(null)}
          onRemoved={() => {
            setRemoveAgent(null)
            reload()
          }}
        />
      )}
    </>
  )
}

function ModalShell({
  title,
  children,
  onClose,
}: {
  title: string
  children: ReactNode
  onClose: () => void
}) {
  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4" onClick={onClose}>
      <div
        className="max-h-[90vh] w-full max-w-lg overflow-y-auto rounded-xl border border-[var(--border)] bg-[var(--bg-elevated)] p-5 shadow-lg"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="mb-4 flex items-start justify-between gap-3">
          <h3 className="text-lg font-medium">{title}</h3>
          <Button variant="ghost" onClick={onClose}>
            Close
          </Button>
        </div>
        {children}
      </div>
    </div>
  )
}

function CopyBlocks({
  tokenLabel,
  yamlLabel,
  info,
}: {
  tokenLabel: string
  yamlLabel: string
  info: { token: string; env: string; yaml: string }
}) {
  const [copied, setCopied] = useState<string | null>(null)
  const copy = async (label: string, value: string) => {
    try {
      await navigator.clipboard.writeText(value)
      setCopied(label)
    } catch {
      setCopied(null)
    }
  }
  return (
    <div className="space-y-3">
      <div>
        <div className="mb-1 flex items-center justify-between gap-2">
          <Label>{tokenLabel}</Label>
          <Button variant="ghost" onClick={() => copy('token', info.token)}>
            {copied === 'token' ? 'Copied' : 'Copy'}
          </Button>
        </div>
        <code className="block break-all rounded-md bg-[var(--bg)] px-2 py-2 text-xs">{info.token}</code>
      </div>
      <div>
        <div className="mb-1 flex items-center justify-between gap-2">
          <Label>Environment</Label>
          <Button variant="ghost" onClick={() => copy('env', info.env)}>
            {copied === 'env' ? 'Copied' : 'Copy'}
          </Button>
        </div>
        <pre className="overflow-x-auto rounded-md bg-[var(--bg)] p-2 text-xs">{info.env}</pre>
      </div>
      <div>
        <div className="mb-1 flex items-center justify-between gap-2">
          <Label>{yamlLabel}</Label>
          <Button variant="ghost" onClick={() => copy('yaml', info.yaml)}>
            {copied === 'yaml' ? 'Copied' : 'Copy'}
          </Button>
        </div>
        <pre className="max-h-64 overflow-auto rounded-md bg-[var(--bg)] p-2 text-xs">{info.yaml}</pre>
      </div>
    </div>
  )
}

function AgentWizardModal({
  onClose,
  onCreated,
}: {
  onClose: () => void
  onCreated: () => void
}) {
  const [step, setStep] = useState<'password' | 'name' | 'done'>('password')
  const [password, setPassword] = useState('')
  const [name, setName] = useState('')
  const [error, setError] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)
  const [info, setInfo] = useState<(AgentConnectInfo & { agent: Agent }) | null>(null)

  return (
    <ModalShell title="Create agent" onClose={onClose}>
      {step === 'password' && (
        <form
          className="space-y-3"
          onSubmit={(e) => {
            e.preventDefault()
            setError(null)
            if (!password) return
            setStep('name')
          }}
        >
          <p className="text-sm text-[var(--text-muted)]">Confirm your admin password to continue.</p>
          <div>
            <Label>Admin password</Label>
            <Input
              type="password"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              autoComplete="current-password"
              autoFocus
            />
          </div>
          {error && <p className="text-sm text-[var(--danger)]">{error}</p>}
          <Button type="submit" disabled={!password || busy}>
            Continue
          </Button>
        </form>
      )}
      {step === 'name' && (
        <form
          className="space-y-3"
          onSubmit={async (e) => {
            e.preventDefault()
            if (!name.trim() || busy) return
            setBusy(true)
            setError(null)
            try {
              const res = await api.createAgent({ password, name: name.trim() })
              setInfo(res)
              setStep('done')
              onCreated()
            } catch (err) {
              setError(err instanceof Error ? err.message : 'Failed')
              if (err instanceof Error && err.message === 'unauthorized') {
                setStep('password')
              }
            } finally {
              setBusy(false)
            }
          }}
        >
          <p className="text-sm text-[var(--text-muted)]">
            This name is stored as the log hostname (e.g. <code className="text-xs">app-server-1</code>).
          </p>
          <div>
            <Label>Agent name / hostname</Label>
            <Input
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="app-server-1"
              autoFocus
            />
          </div>
          {error && <p className="text-sm text-[var(--danger)]">{error}</p>}
          <div className="flex gap-2">
            <Button type="button" variant="ghost" onClick={() => setStep('password')}>
              Back
            </Button>
            <Button type="submit" disabled={!name.trim() || busy}>
              {busy ? 'Creating…' : 'Create'}
            </Button>
          </div>
        </form>
      )}
      {step === 'done' && info && (
        <div className="space-y-3">
          <p className="text-sm text-[var(--accent)]">
            Agent <strong>{info.agent.name}</strong> created. Copy the Vector config onto that machine.
          </p>
          <CopyBlocks tokenLabel="Ingest token" yamlLabel="Vector yaml" info={info} />
          <Button onClick={onClose}>Done</Button>
        </div>
      )}
    </ModalShell>
  )
}

function AgentConnectModal({ agent, onClose }: { agent: Agent; onClose: () => void }) {
  const [password, setPassword] = useState('')
  const [error, setError] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)
  const [info, setInfo] = useState<AgentConnectInfo | null>(null)

  return (
    <ModalShell title={`Connect info — ${agent.host}`} onClose={onClose}>
      {!info ? (
        <form
          className="space-y-3"
          onSubmit={async (e) => {
            e.preventDefault()
            if (!password || busy) return
            setBusy(true)
            setError(null)
            try {
              setInfo(await api.agentConnectInfo(agent.id, password))
            } catch (err) {
              setError(err instanceof Error ? err.message : 'Failed')
            } finally {
              setBusy(false)
            }
          }}
        >
          <p className="text-sm text-[var(--text-muted)]">
            Re-enter your admin password to reveal the ingest token and Vector yaml.
          </p>
          <div>
            <Label>Admin password</Label>
            <Input
              type="password"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              autoComplete="current-password"
              autoFocus
            />
          </div>
          {error && <p className="text-sm text-[var(--danger)]">{error}</p>}
          <Button type="submit" disabled={!password || busy}>
            {busy ? 'Checking…' : 'Reveal'}
          </Button>
        </form>
      ) : (
        <div className="space-y-3">
          <CopyBlocks tokenLabel="Ingest token" yamlLabel="Vector yaml" info={info} />
          <Button onClick={onClose}>Done</Button>
        </div>
      )}
    </ModalShell>
  )
}

function RemoveAgentModal({
  agent,
  onClose,
  onRemoved,
}: {
  agent: Agent
  onClose: () => void
  onRemoved: () => void
}) {
  const [password, setPassword] = useState('')
  const [error, setError] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)

  return (
    <ModalShell title={`Remove ${agent.host}`} onClose={onClose}>
      <form
        className="space-y-3"
        onSubmit={async (e) => {
          e.preventDefault()
          if (!password || busy) return
          setBusy(true)
          setError(null)
          try {
            await api.removeAgent(agent.id, password)
            onRemoved()
          } catch (err) {
            setError(err instanceof Error ? err.message : 'Failed')
          } finally {
            setBusy(false)
          }
        }}
      >
        <p className="text-sm text-[var(--text-muted)]">
          This revokes the agent’s ingest key so Vector can no longer push. Existing logs for this host stay
          searchable until retention deletes them.
        </p>
        <div>
          <Label>Admin password</Label>
          <Input
            type="password"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            autoComplete="current-password"
            autoFocus
          />
        </div>
        {error && <p className="text-sm text-[var(--danger)]">{error}</p>}
        <div className="flex gap-2">
          <Button type="button" variant="ghost" onClick={onClose}>
            Cancel
          </Button>
          <Button type="submit" variant="danger" disabled={!password || busy}>
            {busy ? 'Removing…' : 'Remove agent'}
          </Button>
        </div>
      </form>
    </ModalShell>
  )
}

function McpPanel() {
  const [password, setPassword] = useState('')
  const [error, setError] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)
  const [info, setInfo] = useState<McpConnectInfo | null>(null)
  const [unlockedPassword, setUnlockedPassword] = useState<string | null>(null)
  const publicBaseUrl = usePublicBaseUrl()

  return (
    <Card className="space-y-4">
      <div>
        <h2 className="text-lg font-medium">MCP</h2>
        <p className="mt-1 text-sm text-[var(--text-muted)]">
          Reveal the query token and MCP client config. After unlocking you can rotate the token to a new
          random value (old token stops working immediately).
        </p>
        <PublicUrlHint url={publicBaseUrl} />
      </div>

      {!info ? (
        <form
          className="space-y-3"
          onSubmit={async (e) => {
            e.preventDefault()
            if (!password || busy) return
            setBusy(true)
            setError(null)
            try {
              const res = await api.mcpConnectInfo(password)
              setInfo(res)
              setUnlockedPassword(password)
            } catch (err) {
              setError(err instanceof Error ? err.message : 'Failed')
            } finally {
              setBusy(false)
            }
          }}
        >
          <div>
            <Label>Admin password</Label>
            <Input
              type="password"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              autoComplete="current-password"
            />
          </div>
          {error && <p className="text-sm text-[var(--danger)]">{error}</p>}
          <Button type="submit" disabled={!password || busy}>
            {busy ? 'Checking…' : 'Reveal MCP token'}
          </Button>
        </form>
      ) : (
        <div className="space-y-3">
          <CopyBlocks tokenLabel="Query token" yamlLabel="MCP client config" info={info} />
          <div className="flex flex-wrap gap-2">
            <Button
              variant="danger"
              disabled={busy || !unlockedPassword}
              onClick={async () => {
                if (!unlockedPassword) return
                setBusy(true)
                setError(null)
                try {
                  setInfo(await api.mcpRotate(unlockedPassword))
                } catch (e) {
                  setError(e instanceof Error ? e.message : 'Failed to rotate')
                } finally {
                  setBusy(false)
                }
              }}
            >
              {busy ? 'Rotating…' : 'Generate new token'}
            </Button>
            <Button
              variant="ghost"
              onClick={() => {
                setInfo(null)
                setUnlockedPassword(null)
                setPassword('')
                setError(null)
              }}
            >
              Hide
            </Button>
          </div>
          {error && <p className="text-sm text-[var(--danger)]">{error}</p>}
        </div>
      )}
    </Card>
  )
}

function RetentionPanel() {
  const [settings, setSettings] = useState<Settings | null>(null)
  const [days, setDays] = useState('14')
  const [maxEvents, setMaxEvents] = useState('')
  const [msg, setMsg] = useState<string | null>(null)

  useEffect(() => {
    api.settings().then((s) => {
      setSettings(s)
      setDays(String(s.retention_days))
      setMaxEvents(s.max_events != null ? String(s.max_events) : '')
    })
  }, [])

  if (!settings) return <Card>Loading…</Card>

  return (
    <div className="space-y-4">
      <Card className="space-y-3">
        <h2 className="text-lg font-medium">Public URL</h2>
        <p className="text-sm text-[var(--text-muted)]">
          Used in Vector and MCP snippets. Set via the <code className="text-xs">PUBLIC_BASE_URL</code> env
          var (restart Vector Collector to change). Use your LAN IP or reverse-proxy origin so remote agents
          can reach this host.
        </p>
        <div>
          <Label>Public URL</Label>
          <Input value={settings.public_base_url} readOnly />
        </div>
      </Card>
      <Card className="space-y-3">
        <h2 className="text-lg font-medium">Retention</h2>
        <p className="text-sm text-[var(--text-muted)]">
          Older events are deleted automatically. Optional max row cap trims the oldest first.
        </p>
        <div className="grid gap-3 sm:grid-cols-2">
          <div>
            <Label>Retention days</Label>
            <Input value={days} onChange={(e) => setDays(e.target.value)} />
          </div>
          <div>
            <Label>Max events (optional)</Label>
            <Input value={maxEvents} onChange={(e) => setMaxEvents(e.target.value)} placeholder="e.g. 5000000" />
          </div>
        </div>
        {msg && <p className="text-sm text-[var(--accent)]">{msg}</p>}
        <Button
          onClick={async () => {
            await api.saveSettings({
              retention_days: Number(days),
              max_events: maxEvents.trim() === '' ? null : Number(maxEvents),
            })
            setMsg('Saved')
          }}
        >
          Save
        </Button>
      </Card>
    </div>
  )
}
