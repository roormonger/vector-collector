import { useEffect, useState, type ReactNode } from 'react'
import { LayoutDashboard, LogOut, Menu, Plug, Server, Settings, X } from 'lucide-react'
import {
  api,
  type Agent,
  type AgentConnectInfo,
  type McpConnectInfo,
  type RecentEvent,
  type Settings as SettingsData,
  type SettingsUpdate,
  type Stats,
} from './lib/api'
import { Badge, Button, Card, Input, Label, Select, Textarea } from './components/ui'
import { generateConfig } from './lib/vectorPresets'
import { cn } from './lib/utils'

type Tab = 'overview' | 'hosts' | 'connect' | 'settings'

const NAV: { id: Tab; label: string; icon: typeof LayoutDashboard }[] = [
  { id: 'overview', label: 'Overview', icon: LayoutDashboard },
  { id: 'hosts', label: 'Hosts', icon: Server },
  { id: 'connect', label: 'Connect', icon: Plug },
  { id: 'settings', label: 'Settings', icon: Settings },
]

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
    <AdminShell
      user={user}
      tab={tab}
      onTab={setTab}
      onLogout={async () => {
        await api.logout()
        setUser(null)
      }}
    >
      {tab === 'overview' && <Overview />}
      {tab === 'hosts' && <HostsPanel />}
      {tab === 'connect' && <ConnectPanel />}
      {tab === 'settings' && <SettingsPanel />}
    </AdminShell>
  )
}

function AdminShell({
  user,
  tab,
  onTab,
  onLogout,
  children,
}: {
  user: string
  tab: Tab
  onTab: (tab: Tab) => void
  onLogout: () => void
  children: ReactNode
}) {
  const [drawerOpen, setDrawerOpen] = useState(false)
  const title = NAV.find((item) => item.id === tab)?.label ?? ''

  const sidebar = (onNavigate: () => void) => (
    <>
      <div className="flex items-center gap-2 px-4 py-4">
        <img src="/vc-icon.png" alt="" className="size-6 shrink-0 rounded-sm" />
        <p className="text-sm font-semibold tracking-tight">Vector Collector</p>
      </div>
      <nav className="flex flex-1 flex-col gap-0.5 px-2">
        {NAV.map(({ id, label, icon: Icon }) => (
          <button
            key={id}
            type="button"
            className={cn(
              'flex w-full items-center gap-2.5 rounded-md px-3 py-2 text-left text-sm',
              tab === id
                ? 'bg-[var(--bg-muted)] text-[var(--text)]'
                : 'text-[var(--text-muted)] hover:bg-[var(--bg-muted)]/50 hover:text-[var(--text)]',
            )}
            onClick={() => {
              onTab(id)
              onNavigate()
            }}
          >
            <Icon className="size-4 shrink-0" />
            {label}
          </button>
        ))}
      </nav>
      <div className="mt-auto border-t border-[var(--border)] px-3 py-3">
        <p className="truncate px-1 text-xs text-[var(--text-muted)]">{user}</p>
        <button
          type="button"
          className="mt-1 flex w-full items-center gap-2.5 rounded-md px-3 py-2 text-left text-sm text-[var(--text-muted)] hover:bg-[var(--bg-muted)]/50 hover:text-[var(--text)]"
          onClick={onLogout}
        >
          <LogOut className="size-4 shrink-0" />
          Log out
        </button>
      </div>
    </>
  )

  return (
    <div className="flex h-svh overflow-hidden">
      <aside className="hidden h-svh w-56 shrink-0 flex-col overflow-hidden border-r border-[var(--border)] bg-[var(--bg-elevated)] md:flex">
        {sidebar(() => undefined)}
      </aside>

      {drawerOpen && (
        <div className="fixed inset-0 z-40 md:hidden">
          <button
            type="button"
            className="absolute inset-0 bg-black/50"
            aria-label="Close menu"
            onClick={() => setDrawerOpen(false)}
          />
          <aside className="relative flex h-full w-56 flex-col bg-[var(--bg-elevated)] shadow-lg">
            <button
              type="button"
              className="absolute right-2 top-3 rounded-md p-1 text-[var(--text-muted)] hover:bg-[var(--bg-muted)]"
              aria-label="Close menu"
              onClick={() => setDrawerOpen(false)}
            >
              <X className="size-4" />
            </button>
            {sidebar(() => setDrawerOpen(false))}
          </aside>
        </div>
      )}

      <div className="flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden">
        <header className="flex h-14 shrink-0 items-center gap-3 border-b border-[var(--border)] px-4">
          <button
            type="button"
            className="rounded-md p-1.5 text-[var(--text-muted)] hover:bg-[var(--bg-muted)] md:hidden"
            aria-label="Open menu"
            onClick={() => setDrawerOpen(true)}
          >
            <Menu className="size-5" />
          </button>
          <h1 className="text-lg font-medium">{title}</h1>
        </header>
        <main className="min-h-0 flex-1 overflow-y-auto px-4 py-6 sm:px-6">{children}</main>
      </div>
    </div>
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
          Admin console for Vector hosts, ingest keys, and MCP access.
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
  const [events, setEvents] = useState<RecentEvent[]>([])
  const [liveError, setLiveError] = useState<string | null>(null)
  const [paused, setPaused] = useState(false)
  const [hostFilter, setHostFilter] = useState('')
  const [keyword, setKeyword] = useState('')
  const [debouncedKeyword, setDebouncedKeyword] = useState('')
  const [hosts, setHosts] = useState<string[]>([])

  const filtered = Boolean(hostFilter || debouncedKeyword)

  useEffect(() => {
    api
      .agents()
      .then((rows) => {
        const names = [
          ...new Set(rows.map((a) => a.host || a.name).filter((n) => n.trim().length > 0)),
        ]
        names.sort((a, b) => a.localeCompare(b))
        setHosts(names)
      })
      .catch(console.error)
  }, [])

  useEffect(() => {
    const id = window.setTimeout(() => setDebouncedKeyword(keyword.trim()), 300)
    return () => window.clearTimeout(id)
  }, [keyword])

  useEffect(() => {
    let cancelled = false
    const tick = () => {
      api
        .stats()
        .then((s) => {
          if (!cancelled) setStats(s)
        })
        .catch(console.error)
    }
    tick()
    const id = window.setInterval(tick, 5000)
    return () => {
      cancelled = true
      window.clearInterval(id)
    }
  }, [])

  useEffect(() => {
    let cancelled = false
    const tick = () => {
      api
        .recentEvents({
          host: hostFilter || undefined,
          text: debouncedKeyword || undefined,
        })
        .then((rows) => {
          if (!cancelled) {
            setEvents(rows)
            setLiveError(null)
          }
        })
        .catch((err) => {
          if (!cancelled) {
            setLiveError(err instanceof Error ? err.message : 'Failed to load events')
          }
        })
    }
    tick()
    if (paused) {
      return () => {
        cancelled = true
      }
    }
    const id = window.setInterval(tick, 2500)
    return () => {
      cancelled = true
      window.clearInterval(id)
    }
  }, [paused, hostFilter, debouncedKeyword])

  return (
    <div className="space-y-6">
      <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-6">
        {stats ? (
          <>
            <Stat title="Events stored" value={stats.events} />
            <Stat title="Hosts" value={stats.agents} />
            <Stat title="Queue depth" value={stats.queue_depth ?? 0} />
            <Stat title="Ingest accepted" value={stats.ingest_accepted ?? 0} />
            <Stat title="Ingest 429s" value={stats.ingest_429 ?? 0} />
            <Stat title="Embed queue" value={stats.embed_queue} />
          </>
        ) : (
          <Card className="sm:col-span-2 lg:col-span-3 xl:col-span-6">Loading stats…</Card>
        )}
      </div>

      {stats && (
        <div className="flex flex-wrap gap-4 text-sm text-[var(--text-muted)]">
          <span>
            Semantic search:{' '}
            <span className="text-[var(--text)]">
              {stats.embeddings_enabled ? 'enabled' : 'FTS only'}
            </span>
          </span>
          <a className="text-[var(--accent)] underline-offset-2 hover:underline" href="/docs">
            API docs
          </a>
        </div>
      )}

      <Card className="overflow-hidden p-0">
        <div className="flex flex-wrap items-center justify-between gap-3 border-b border-[var(--border)] px-4 py-3">
          <div>
            <h2 className="text-lg font-medium">Live logs</h2>
            <p className="text-sm text-[var(--text-muted)]">
              {filtered ? 'Filtered search' : 'Newest events'} · polls every 2.5s
              {paused ? ' · paused' : ''}
            </p>
          </div>
          <div className="flex flex-wrap items-center gap-2">
            <Select
              className="w-auto min-w-[10rem]"
              value={hostFilter}
              onChange={(e) => setHostFilter(e.target.value)}
              aria-label="Filter by host"
            >
              <option value="">All hosts</option>
              {hosts.map((h) => (
                <option key={h} value={h}>
                  {h}
                </option>
              ))}
            </Select>
            <Input
              className="w-48"
              value={keyword}
              onChange={(e) => setKeyword(e.target.value)}
              placeholder="Keywords"
              aria-label="Search keywords"
            />
            <Button variant="ghost" onClick={() => setPaused((p) => !p)}>
              {paused ? 'Resume' : 'Pause'}
            </Button>
          </div>
        </div>
        {liveError && (
          <p className="border-b border-[var(--border)] px-4 py-2 text-sm text-[var(--danger)]">
            {liveError}
          </p>
        )}
        <div className="max-h-[min(60vh,640px)] overflow-auto font-mono text-xs leading-relaxed">
          {events.length === 0 && !liveError ? (
            <p className="px-4 py-8 text-center text-[var(--text-muted)]">
              {filtered ? 'No matching events' : 'No events yet'}
            </p>
          ) : (
            <table className="w-full border-collapse text-left">
              <tbody>
                {events.map((ev) => (
                  <tr
                    key={ev.id}
                    className="border-b border-[var(--border)]/60 hover:bg-[var(--bg-muted)]/40"
                  >
                    <td className="whitespace-nowrap px-3 py-1.5 align-top text-[var(--text-muted)]">
                      {formatShortTime(ev.ts)}
                    </td>
                    <td className="max-w-[8rem] truncate px-2 py-1.5 align-top text-emerald-300/90">
                      {ev.host ?? '—'}
                    </td>
                    <td className="max-w-[10rem] truncate px-2 py-1.5 align-top text-sky-300/90">
                      {ev.container_name ?? '—'}
                    </td>
                    <td className="px-3 py-1.5 align-top break-words text-[var(--text)]">
                      {ev.message}
                      {ev.message_truncated ? '…' : ''}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
        </div>
      </Card>
    </div>
  )
}

function formatShortTime(value: string) {
  const d = new Date(value)
  if (Number.isNaN(d.getTime())) return value
  return d.toLocaleTimeString(undefined, {
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
    hour12: false,
  })
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
    case 'offline':
    default:
      return 'bg-rose-500/15 text-rose-300'
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

function HostsPanel() {
  const [hosts, setHosts] = useState<Agent[]>([])
  const [error, setError] = useState<string | null>(null)
  const [removeHost, setRemoveHost] = useState<Agent | null>(null)
  const [name, setName] = useState('')
  const [password, setPassword] = useState('')
  const [busy, setBusy] = useState(false)
  const [createdMsg, setCreatedMsg] = useState<string | null>(null)

  const reload = () =>
    api
      .agents()
      .then((rows) => {
        setHosts(rows)
        setError(null)
      })
      .catch((e) => setError(e instanceof Error ? e.message : 'Failed to load hosts'))

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
            <h2 className="text-lg font-medium">Hosts</h2>
            <p className="text-sm text-[var(--text-muted)]">
              Register a machine name and ingest key. The name is stored as the log{' '}
              <code className="text-xs">host</code> field. Copy Vector config on{' '}
              <strong>Connect</strong>. Online means that machine heartbeated or sent logs within ~2
              minutes. Remove revokes the ingest key; stored logs stay until retention.
            </p>
          </div>
          <Button variant="ghost" onClick={reload}>
            Refresh
          </Button>
        </div>

        <form
          className="mb-4 flex flex-wrap items-end gap-2 border-b border-[var(--border)] pb-4"
          onSubmit={async (e) => {
            e.preventDefault()
            if (!name.trim() || !password || busy) return
            setBusy(true)
            setError(null)
            setCreatedMsg(null)
            try {
              const res = await api.createAgent({ password, name: name.trim() })
              setName('')
              setPassword('')
              setCreatedMsg(
                `Created ${res.agent.host ?? res.agent.name}. Copy Vector yaml on Connect.`,
              )
              reload()
            } catch (err) {
              setError(err instanceof Error ? err.message : 'Failed to create host')
            } finally {
              setBusy(false)
            }
          }}
        >
          <div className="min-w-[10rem] flex-1">
            <Label>Host name</Label>
            <Input
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="app-server-1"
            />
          </div>
          <div className="min-w-[10rem] flex-1">
            <Label>Admin password</Label>
            <Input
              type="password"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              autoComplete="current-password"
            />
          </div>
          <Button type="submit" disabled={!name.trim() || !password || busy}>
            {busy ? 'Creating…' : 'Create'}
          </Button>
        </form>

        {createdMsg && <p className="mb-3 text-sm text-[var(--accent)]">{createdMsg}</p>}
        {error && <p className="mb-3 text-sm text-[var(--danger)]">{error}</p>}
        <div className="space-y-3">
          {hosts.map((a) => (
            <div
              key={a.id}
              className="rounded-lg border border-[var(--border)] bg-[var(--bg)]/50 p-3 last:mb-0"
            >
              <div className="flex flex-wrap items-start justify-between gap-2">
                <div>
                  <p className="font-medium">{a.host}</p>
                  <p className="text-sm text-[var(--text-muted)]">
                    {a.has_connect_secret ? 'Dedicated ingest key' : a.name}
                  </p>
                </div>
                <div className="flex flex-wrap items-center gap-2">
                  <span className={`rounded-full px-2 py-0.5 text-xs capitalize ${statusStyles(a.status)}`}>
                    {a.status}
                  </span>
                  <Button variant="danger" onClick={() => setRemoveHost(a)}>
                    Remove
                  </Button>
                </div>
              </div>
              <p className="mt-2 text-sm text-[var(--text-muted)]">
                last seen {formatTime(a.last_seen_at)} · {a.events_ingested.toLocaleString()} events
                {a.key_prefix ? ` · key ${a.key_prefix}…` : ''}
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
          {hosts.length === 0 && (
            <p className="text-[var(--text-muted)]">No hosts yet — create a name and ingest key above.</p>
          )}
        </div>
      </Card>

      {removeHost && (
        <RemoveHostModal
          host={removeHost}
          onClose={() => setRemoveHost(null)}
          onRemoved={() => {
            setRemoveHost(null)
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
  wide,
}: {
  title: string
  children: ReactNode
  onClose: () => void
  wide?: boolean
}) {
  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4" onClick={onClose}>
      <div
        className={`max-h-[90vh] w-full overflow-y-auto rounded-xl border border-[var(--border)] bg-[var(--bg-elevated)] p-5 shadow-lg ${
          wide ? 'max-w-2xl' : 'max-w-lg'
        }`}
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

const PLATFORM_OPTIONS = [
  {
    id: 'docker',
    label: 'Docker',
    description: 'Container logs via docker_logs (Linux/macOS Docker Engine or Desktop)',
  },
  {
    id: 'linux',
    label: 'Linux',
    description: 'systemd journal (journald) with message remap',
  },
  {
    id: 'windows',
    label: 'Windows',
    description: 'Windows Event Log — token inlined in YAML (no env file)',
  },
  {
    id: 'macos',
    label: 'macOS',
    description: 'Common system/app log files under /var/log and /Library/Logs',
  },
  {
    id: 'files',
    label: 'Files',
    description: 'Tail arbitrary log files — edit the include paths',
  },
] as const

function PlatformPicker({
  value,
  onChange,
  description,
}: {
  value: string
  onChange: (id: string) => void
  description?: string | null
}) {
  return (
    <div className="space-y-2">
      <Label>Platform preset</Label>
      <div className="flex flex-wrap gap-1">
        {PLATFORM_OPTIONS.map((p) => (
          <Button
            key={p.id}
            type="button"
            variant={value === p.id ? 'default' : 'ghost'}
            onClick={() => onChange(p.id)}
          >
            {p.label}
          </Button>
        ))}
      </div>
      {description && <p className="text-sm text-[var(--text-muted)]">{description}</p>}
    </div>
  )
}

function downloadText(filename: string, value: string) {
  const blob = new Blob([value], { type: 'text/plain;charset=utf-8' })
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = filename
  a.click()
  URL.revokeObjectURL(url)
}

function SnippetBox({
  label,
  value,
  filename,
  heightClass = 'h-72',
}: {
  label: string
  value: string
  filename?: string
  heightClass?: string
}) {
  const [copied, setCopied] = useState(false)
  return (
    <div>
      <div className="mb-1 flex items-center justify-between gap-2">
        <Label>{label}</Label>
        <div className="flex gap-1">
          <Button
            variant="ghost"
            onClick={async () => {
              try {
                await navigator.clipboard.writeText(value)
                setCopied(true)
              } catch {
                setCopied(false)
              }
            }}
          >
            {copied ? 'Copied' : 'Copy'}
          </Button>
          {filename && (
            <Button variant="ghost" onClick={() => downloadText(filename, value)}>
              Download
            </Button>
          )}
        </div>
      </div>
      <Textarea
        readOnly
        spellCheck={false}
        value={value}
        className={cn(heightClass, 'resize-none overflow-y-scroll whitespace-pre')}
      />
    </div>
  )
}

function CopyBlocks({
  tokenLabel,
  yamlLabel,
  info,
  platform,
  onPlatformChange,
  yamlFilename,
}: {
  tokenLabel: string
  yamlLabel: string
  info: AgentConnectInfo | McpConnectInfo
  platform?: string
  onPlatformChange?: (id: string) => void
  yamlFilename?: string
}) {
  const [copiedToken, setCopiedToken] = useState(false)
  const presets = 'presets' in info ? info.presets : undefined
  const activePlatform = platform ?? ('platform' in info ? info.platform : undefined) ?? 'docker'
  const activePreset = presets?.find((p) => p.id === activePlatform)
  const yaml = activePreset?.yaml ?? info.yaml
  const env = activePreset?.env ?? info.env
  const inlineToken =
    activePreset?.inline_token ??
    ('inline_token' in info ? info.inline_token : undefined) ??
    false
  const downloadName =
    yamlFilename ??
    (presets ? `vector-collector-${activePlatform}.yaml` : 'vector-collector.yaml')
  return (
    <div className="space-y-3">
      <div>
        <div className="mb-1 flex items-center justify-between gap-2">
          <Label>{tokenLabel}</Label>
          <Button
            variant="ghost"
            onClick={async () => {
              try {
                await navigator.clipboard.writeText(info.token)
                setCopiedToken(true)
              } catch {
                setCopiedToken(false)
              }
            }}
          >
            {copiedToken ? 'Copied' : 'Copy'}
          </Button>
        </div>
        <code className="block break-all rounded-md bg-[var(--bg)] px-2 py-2 text-xs">{info.token}</code>
      </div>
      {presets && onPlatformChange && (
        <PlatformPicker
          value={activePlatform}
          onChange={onPlatformChange}
          description={activePreset?.description}
        />
      )}
      {!inlineToken && (
        <SnippetBox label="Environment" value={env} filename="vector-collector.env" heightClass="h-28" />
      )}
      {inlineToken && (
        <p className="text-sm text-[var(--text-muted)]">
          This preset inlines the bearer token in the YAML — no env file or Vector env-interpolation
          flag required. Treat the downloaded file as secret.
        </p>
      )}
      <SnippetBox label={yamlLabel} value={yaml} filename={downloadName} />
    </div>
  )
}

function RemoveHostModal({
  host,
  onClose,
  onRemoved,
}: {
  host: Agent
  onClose: () => void
  onRemoved: () => void
}) {
  const [password, setPassword] = useState('')
  const [error, setError] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)

  return (
    <ModalShell title={`Remove ${host.host}`} onClose={onClose}>
      <form
        className="space-y-3"
        onSubmit={async (e) => {
          e.preventDefault()
          if (!password || busy) return
          setBusy(true)
          setError(null)
          try {
            await api.removeAgent(host.id, password)
            onRemoved()
          } catch (err) {
            setError(err instanceof Error ? err.message : 'Failed')
          } finally {
            setBusy(false)
          }
        }}
      >
        <p className="text-sm text-[var(--text-muted)]">
          This revokes the host’s ingest API key so Vector can no longer push. Existing logs for this host
          stay searchable until retention deletes them.
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
            {busy ? 'Removing…' : 'Remove host'}
          </Button>
        </div>
      </form>
    </ModalShell>
  )
}

function ConnectPanel() {
  const [hosts, setHosts] = useState<Agent[]>([])
  const [hostId, setHostId] = useState('')
  const [password, setPassword] = useState('')
  const [unlockedPassword, setUnlockedPassword] = useState<string | null>(null)
  const [mcp, setMcp] = useState<McpConnectInfo | null>(null)
  const [vector, setVector] = useState<AgentConnectInfo | null>(null)
  const [platform, setPlatform] = useState('docker')
  const [error, setError] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)
  const publicBaseUrl = usePublicBaseUrl()
  const base = (publicBaseUrl ?? 'http://localhost:8080').replace(/\/$/, '')
  const redacted = 'lk_••••'
  const redactedVector = generateConfig(base, redacted, platform)

  useEffect(() => {
    api
      .agents()
      .then((rows) => {
        setHosts(rows)
        setHostId((current) => {
          if (current && rows.some((h) => h.id === current)) return current
          return rows[0]?.id ?? ''
        })
      })
      .catch(console.error)
  }, [])

  useEffect(() => {
    if (!unlockedPassword || !hostId) {
      setVector(null)
      return
    }
    let cancelled = false
    api
      .agentConnectInfo(hostId, unlockedPassword, platform)
      .then((info) => {
        if (!cancelled) setVector(info)
      })
      .catch((err) => {
        if (!cancelled) setError(err instanceof Error ? err.message : 'Failed to load Vector config')
      })
    return () => {
      cancelled = true
    }
  }, [unlockedPassword, hostId, platform])

  const redactedMcpYaml = `mcp_servers:
  vector_collector:
    url: "${base}/mcp"
    headers:
      Authorization: "Bearer ${redacted}"
    timeout: 120
    tools:
      resources: false
      prompts: false
`

  return (
    <div className="space-y-4">
      <Card className="space-y-3">
        <div>
          <p className="text-sm text-[var(--text-muted)]">
            Copy Vector yaml for a host (ingest key), MCP yaml, and the query-only OpenAPI URL. Keys stay
            hidden until you enter the admin password. Leaving this page hides them again. MCP and OpenAPI
            use one collector-wide query token; Vector uses the selected host’s ingest key.
          </p>
          <PublicUrlHint url={publicBaseUrl} />
        </div>
        <form
          className="flex flex-wrap items-end gap-2"
          onSubmit={async (e) => {
            e.preventDefault()
            if (!password || busy) return
            setBusy(true)
            setError(null)
            try {
              const info = await api.mcpConnectInfo(password)
              setMcp(info)
              setUnlockedPassword(password)
            } catch (err) {
              setError(err instanceof Error ? err.message : 'Failed')
              setMcp(null)
              setUnlockedPassword(null)
            } finally {
              setBusy(false)
            }
          }}
        >
          <div className="min-w-[10rem] flex-1">
            <Label>Host</Label>
            <Select
              value={hostId}
              onChange={(e) => setHostId(e.target.value)}
              disabled={hosts.length === 0}
            >
              {hosts.length === 0 ? (
                <option value="">No hosts — add one first</option>
              ) : (
                hosts.map((h) => (
                  <option key={h.id} value={h.id}>
                    {h.host || h.name}
                  </option>
                ))
              )}
            </Select>
          </div>
          <div className="min-w-[10rem] flex-1">
            <Label>Admin password</Label>
            <Input
              type="password"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              autoComplete="current-password"
            />
          </div>
          <Button type="submit" disabled={!password || busy}>
            {busy ? 'Checking…' : mcp ? 'Refresh' : 'Reveal keys'}
          </Button>
        </form>
        {error && <p className="text-sm text-[var(--danger)]">{error}</p>}
      </Card>

      <Card className="space-y-3">
        <h2 className="text-lg font-medium">Vector</h2>
        {!hostId ? (
          <p className="text-sm text-[var(--text-muted)]">Add a host on Hosts, then select it above.</p>
        ) : vector && mcp ? (
          <CopyBlocks
            tokenLabel="Ingest API key"
            yamlLabel="Vector yaml"
            info={vector}
            platform={platform}
            onPlatformChange={setPlatform}
          />
        ) : (
          <>
            <PlatformPicker
              value={platform}
              onChange={setPlatform}
              description={PLATFORM_OPTIONS.find((p) => p.id === platform)?.description}
            />
            <p className="text-sm text-[var(--text-muted)]">Enter the admin password to reveal the ingest key.</p>
            {!redactedVector.inlineToken && (
              <SnippetBox
                label="Environment"
                value={redactedVector.env}
                filename="vector-collector.env"
                heightClass="h-28"
              />
            )}
            <SnippetBox
              label="Vector yaml"
              value={redactedVector.yaml}
              filename={`vector-collector-${platform}.yaml`}
            />
          </>
        )}
      </Card>

      <Card className="space-y-3">
        <h2 className="text-lg font-medium">MCP</h2>
        {mcp ? (
          <>
            <CopyBlocks
              tokenLabel="Query token"
              yamlLabel="MCP client config"
              info={mcp}
              yamlFilename="mcp-vector-collector.yaml"
            />
            <Button
              variant="danger"
              disabled={busy || !unlockedPassword}
              onClick={async () => {
                if (!unlockedPassword) return
                setBusy(true)
                setError(null)
                try {
                  setMcp(await api.mcpRotate(unlockedPassword))
                } catch (e) {
                  setError(e instanceof Error ? e.message : 'Failed to rotate')
                } finally {
                  setBusy(false)
                }
              }}
            >
              {busy ? 'Rotating…' : 'Generate new token'}
            </Button>
          </>
        ) : (
          <>
            <p className="text-sm text-[var(--text-muted)]">Collector-wide query token — same for every host.</p>
            <SnippetBox
              label="MCP client config"
              value={redactedMcpYaml}
              filename="mcp-vector-collector.yaml"
            />
          </>
        )}
      </Card>

      <Card className="space-y-3">
        <OpenApiConnectBlock baseUrl={base} token={mcp?.token ?? null} />
      </Card>
    </div>
  )
}

const OPENAPI_SPEC_PATH = '/api-docs/query.json'

function CopyField({ label, value }: { label: string; value: string }) {
  const [copied, setCopied] = useState(false)
  return (
    <div>
      <div className="mb-1 flex items-center justify-between gap-2">
        <Label>{label}</Label>
        <Button
          variant="ghost"
          onClick={async () => {
            try {
              await navigator.clipboard.writeText(value)
              setCopied(true)
            } catch {
              setCopied(false)
            }
          }}
        >
          {copied ? 'Copied' : 'Copy'}
        </Button>
      </div>
      <code className="block break-all rounded-md bg-[var(--bg)] px-2 py-2 text-xs">{value}</code>
    </div>
  )
}

function openWebUiImportJson(baseUrl: string, token: string) {
  return JSON.stringify(
    [
      {
        type: 'openapi',
        url: baseUrl.replace(/\/$/, ''),
        spec_type: 'url',
        spec: '',
        path: OPENAPI_SPEC_PATH,
        auth_type: 'bearer',
        key: token,
        info: {
          id: '',
          name: 'Vector Collector',
          description: 'Query logs via REST (no ingest)',
        },
      },
    ],
    null,
    2,
  )
}

function OpenApiConnectBlock({ baseUrl, token }: { baseUrl: string; token: string | null }) {
  const origin = baseUrl.replace(/\/$/, '')
  return (
    <div className="space-y-3">
      <h2 className="text-lg font-medium">OpenAPI</h2>
      <p className="text-sm text-[var(--text-muted)]">
        Open WebUI Tool Servers use two fields: <strong>URL</strong> (collector origin) and{' '}
        <strong>Advanced → OpenAPI Spec</strong> (path). Do not paste the spec URL into URL or WebUI
        will request <code>{origin}{OPENAPI_SPEC_PATH}/openapi.json</code>. Do not use{' '}
        <code>/api-docs/openapi.json</code> — that spec includes ingest.
      </p>
      <CopyField label="URL" value={origin} />
      <CopyField label="OpenAPI Spec path" value={OPENAPI_SPEC_PATH} />
      {token ? (
        <div>
          <div className="mb-1 flex items-center justify-between gap-2">
            <Label>Open WebUI import</Label>
            <Button
              variant="ghost"
              onClick={() =>
                downloadText('open-webui-vector-collector.json', openWebUiImportJson(origin, token))
              }
            >
              Download
            </Button>
          </div>
          <p className="text-sm text-[var(--text-muted)]">
            Import this in Add Connection. It includes the query bearer token — treat the file as
            secret.
          </p>
        </div>
      ) : (
        <p className="text-sm text-[var(--text-muted)]">
          Reveal keys above to download an Open WebUI import file (includes the query token). Auth is
          Bearer with that same token.
        </p>
      )}
    </div>
  )
}

function SettingsPanel() {
  const [settings, setSettings] = useState<SettingsData | null>(null)
  const [publicUrl, setPublicUrl] = useState('')
  const [days, setDays] = useState('14')
  const [maxEvents, setMaxEvents] = useState('')
  const [queueCap, setQueueCap] = useState('64')
  const [maxBody, setMaxBody] = useState(String(10 * 1024 * 1024))
  const [perKeyRps, setPerKeyRps] = useState('50')
  const [embUrl, setEmbUrl] = useState('')
  const [embModel, setEmbModel] = useState('')
  const [embKey, setEmbKey] = useState('')
  const [embDim, setEmbDim] = useState('1536')
  const [embSample, setEmbSample] = useState('0.02')
  const [msg, setMsg] = useState<string | null>(null)
  const [restartNote, setRestartNote] = useState<string | null>(null)
  const [saving, setSaving] = useState(false)
  const [embTestMsg, setEmbTestMsg] = useState<string | null>(null)
  const [embTesting, setEmbTesting] = useState(false)

  useEffect(() => {
    api.settings().then((s) => {
      setSettings(s)
      setPublicUrl(s.public_base_url)
      setDays(String(s.retention_days))
      setMaxEvents(s.max_events != null ? String(s.max_events) : '')
      setQueueCap(String(s.write_queue_capacity))
      setMaxBody(String(s.max_body_bytes))
      setPerKeyRps(String(s.per_key_rps))
      setEmbUrl(s.embeddings_base_url ?? '')
      setEmbModel(s.embeddings_model ?? '')
      setEmbKey('')
      setEmbDim(String(s.embedding_dim))
      setEmbSample(String(s.embed_sample_rate))
    })
  }, [])

  if (!settings) return <Card>Loading…</Card>

  return (
    <div className="space-y-4">
      {restartNote && (
        <p className="rounded-md border border-[var(--border)] bg-[var(--bg-elevated)] px-3 py-2 text-sm text-[var(--text-muted)]">
          {restartNote}
        </p>
      )}

      <Card className="space-y-3">
        <h2 className="text-lg font-medium">Public URL</h2>
        <p className="text-sm text-[var(--text-muted)]">
          Used in Vector and MCP snippets. Use your LAN IP or reverse-proxy origin so remote machines can
          reach this collector — not localhost if Vector runs elsewhere.
        </p>
        <div>
          <Label>Public URL</Label>
          <Input value={publicUrl} onChange={(e) => setPublicUrl(e.target.value)} />
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
      </Card>

      <Card className="space-y-3">
        <h2 className="text-lg font-medium">Ingest limits</h2>
        <p className="text-sm text-[var(--text-muted)]">
          Queue capacity and max body size apply after a process restart. Per-key rate limit applies
          immediately.
        </p>
        <div className="grid gap-3 sm:grid-cols-3">
          <div>
            <Label>Queue capacity</Label>
            <Input value={queueCap} onChange={(e) => setQueueCap(e.target.value)} />
          </div>
          <div>
            <Label>Max body bytes</Label>
            <Input value={maxBody} onChange={(e) => setMaxBody(e.target.value)} />
          </div>
          <div>
            <Label>Per-key req/s</Label>
            <Input value={perKeyRps} onChange={(e) => setPerKeyRps(e.target.value)} />
          </div>
        </div>
      </Card>

      <Card className="space-y-3">
        <h2 className="text-lg font-medium">Semantic search</h2>
        <p className="text-sm text-[var(--text-muted)]">
          Optional OpenAI-compatible embeddings. Set base URL and model to enable. Changing model or
          dimension may invalidate existing embeddings. Leave API key blank to keep the current secret.
        </p>
        <div className="grid gap-3 sm:grid-cols-2">
          <div className="sm:col-span-2">
            <Label>Embeddings base URL</Label>
            <Input
              value={embUrl}
              onChange={(e) => setEmbUrl(e.target.value)}
              placeholder="https://api.openai.com/v1"
            />
          </div>
          <div>
            <Label>Model</Label>
            <Input
              value={embModel}
              onChange={(e) => setEmbModel(e.target.value)}
              placeholder="text-embedding-3-small"
            />
          </div>
          <div>
            <Label>API key {settings.embeddings_api_key_set ? '(set)' : '(not set)'}</Label>
            <Input
              type="password"
              value={embKey}
              onChange={(e) => setEmbKey(e.target.value)}
              placeholder={settings.embeddings_api_key_set ? '••••••••' : 'optional'}
              autoComplete="new-password"
            />
          </div>
          <div>
            <Label>Embedding dim</Label>
            <Input value={embDim} onChange={(e) => setEmbDim(e.target.value)} />
          </div>
          <div>
            <Label>Sample rate (0–1)</Label>
            <Input value={embSample} onChange={(e) => setEmbSample(e.target.value)} />
          </div>
        </div>
        <p className="text-xs text-[var(--text-muted)]">
          Status: {settings.embeddings_enabled ? 'enabled' : 'disabled'}
        </p>
        {embTestMsg && (
          <p
            className={`text-sm ${
              embTestMsg.startsWith('OK') ? 'text-[var(--accent)]' : 'text-[var(--danger)]'
            }`}
          >
            {embTestMsg}
          </p>
        )}
        <Button
          type="button"
          variant="ghost"
          disabled={embTesting || (!embUrl.trim() && !settings.embeddings_base_url) || (!embModel.trim() && !settings.embeddings_model)}
          onClick={async () => {
            setEmbTesting(true)
            setEmbTestMsg(null)
            try {
              const body: {
                embeddings_base_url?: string
                embeddings_model?: string
                embeddings_api_key?: string
                embedding_dim?: number
              } = {}
              if (embUrl.trim()) body.embeddings_base_url = embUrl.trim()
              if (embModel.trim()) body.embeddings_model = embModel.trim()
              if (embKey.trim()) body.embeddings_api_key = embKey.trim()
              const dim = Number(embDim)
              if (Number.isFinite(dim) && dim >= 1) body.embedding_dim = dim
              const res = await api.testEmbeddings(body)
              const dimNote = res.dim_match
                ? 'matches configured dim'
                : `configured dim is ${res.configured_dim} — update Embedding dim if searches look wrong`
              setEmbTestMsg(
                `OK — ${res.dimensions}-d vector from ${res.model} in ${res.latency_ms}ms (${dimNote})`,
              )
            } catch (err) {
              setEmbTestMsg(err instanceof Error ? err.message : 'Test failed')
            } finally {
              setEmbTesting(false)
            }
          }}
        >
          {embTesting ? 'Testing…' : 'Test connection'}
        </Button>
      </Card>

      {msg && <p className="text-sm text-[var(--accent)]">{msg}</p>}
      <Button
        disabled={saving}
        onClick={async () => {
          setSaving(true)
          setMsg(null)
          try {
            const body: SettingsUpdate = {
              retention_days: Number(days),
              max_events: maxEvents.trim() === '' ? null : Number(maxEvents),
              public_base_url: publicUrl.trim(),
              write_queue_capacity: Number(queueCap),
              max_body_bytes: Number(maxBody),
              per_key_rps: Number(perKeyRps),
              embeddings_base_url: embUrl.trim() === '' ? null : embUrl.trim(),
              embeddings_model: embModel.trim() === '' ? null : embModel.trim(),
              embedding_dim: Number(embDim),
              embed_sample_rate: Number(embSample),
            }
            if (embKey.trim() !== '') {
              body.embeddings_api_key = embKey.trim()
            }
            const res = await api.saveSettings(body)
            const refreshed = await api.settings()
            setSettings(refreshed)
            setEmbKey('')
            setMsg('Saved')
            if (res.restart_required && res.restart_required.length > 0) {
              setRestartNote(
                `Restart Vector Collector for these to take effect: ${res.restart_required.join(', ')}.`,
              )
            } else {
              setRestartNote(null)
            }
          } catch (e) {
            setMsg(e instanceof Error ? e.message : 'Save failed')
          } finally {
            setSaving(false)
          }
        }}
      >
        {saving ? 'Saving…' : 'Save settings'}
      </Button>
    </div>
  )
}
