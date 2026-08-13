async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const res = await fetch(path, {
    credentials: 'include',
    headers: {
      'Content-Type': 'application/json',
      ...(init?.headers ?? {}),
    },
    ...init,
  })
  if (!res.ok) {
    let message = res.statusText
    try {
      const body = await res.json()
      message = body.error ?? message
    } catch {
      /* ignore */
    }
    throw new Error(message)
  }
  return res.json() as Promise<T>
}

export const api = {
  me: () => request<{ username: string }>('/v1/admin/me'),
  login: (username: string, password: string) =>
    request<{ ok: boolean }>('/v1/admin/login', {
      method: 'POST',
      body: JSON.stringify({ username, password }),
    }),
  logout: () => request<{ ok: boolean }>('/v1/admin/logout', { method: 'POST' }),
  agents: () => request<Agent[]>('/v1/admin/agents'),
  createAgent: (body: { password: string; name: string; platform?: string }) =>
    request<AgentConnectInfo & { agent: Agent }>('/v1/admin/agents', {
      method: 'POST',
      body: JSON.stringify(body),
    }),
  agentConnectInfo: (id: string, password: string, platform?: string) =>
    request<AgentConnectInfo>(`/v1/admin/agents/${id}/connect-info`, {
      method: 'POST',
      body: JSON.stringify({ password, platform }),
    }),
  removeAgent: (id: string, password: string) =>
    request<{ ok: boolean }>(`/v1/admin/agents/${id}/remove`, {
      method: 'POST',
      body: JSON.stringify({ password }),
    }),
  mcpConnectInfo: (password: string) =>
    request<McpConnectInfo>('/v1/admin/mcp/connect-info', {
      method: 'POST',
      body: JSON.stringify({ password }),
    }),
  mcpRotate: (password: string) =>
    request<McpConnectInfo>('/v1/admin/mcp/rotate', {
      method: 'POST',
      body: JSON.stringify({ password }),
    }),
  stats: () => request<Stats>('/v1/admin/stats'),
  recentEvents: () => request<RecentEvent[]>('/v1/admin/recent-events'),
  settings: () => request<Settings>('/v1/admin/settings'),
  saveSettings: (body: SettingsUpdate) =>
    request<{ ok: boolean; restart_required?: string[] }>('/v1/admin/settings', {
      method: 'PUT',
      body: JSON.stringify(body),
    }),
  testEmbeddings: (body?: {
    embeddings_base_url?: string
    embeddings_model?: string
    embeddings_api_key?: string
    embedding_dim?: number
  }) =>
    request<EmbeddingsTestResult>('/v1/admin/embeddings/test', {
      method: 'POST',
      body: JSON.stringify(body ?? {}),
    }),
}

export type AgentStatus = 'online' | 'offline'

export type Agent = {
  id: string
  name: string
  host: string
  host_hint?: string | null
  last_seen_at?: string | null
  events_ingested: number
  created_at: string
  status: AgentStatus
  key_name?: string | null
  key_prefix?: string | null
  has_connect_secret?: boolean
  recent_containers: { name: string; count: number }[]
}

export type VectorPreset = {
  id: string
  label: string
  description: string
  yaml: string
  env?: string
  inline_token?: boolean
}

export type AgentConnectInfo = {
  token: string
  uri: string
  env: string
  yaml: string
  platform?: string
  inline_token?: boolean
  presets?: VectorPreset[]
}

export type McpConnectInfo = {
  token: string
  url: string
  env: string
  yaml: string
}

export type RecentEvent = {
  id: string
  ts: string
  host?: string | null
  container_name?: string | null
  stream?: string | null
  message: string
  message_truncated?: boolean
}

export type Stats = {
  events: number
  agents: number
  embed_queue: number
  ingest_accepted?: number
  ingest_429?: number
  ingest_written?: number
  queue_depth?: number
  embeddings_enabled?: boolean
}

export type Settings = {
  retention_days: number
  max_events?: number | null
  public_base_url: string
  write_queue_capacity: number
  max_body_bytes: number
  per_key_rps: number
  embeddings_base_url?: string | null
  embeddings_model?: string | null
  embeddings_api_key_set: boolean
  embedding_dim: number
  embed_sample_rate: number
  embeddings_enabled: boolean
}

export type SettingsUpdate = {
  retention_days?: number
  max_events?: number | null
  public_base_url?: string
  write_queue_capacity?: number
  max_body_bytes?: number
  per_key_rps?: number
  embeddings_base_url?: string | null
  embeddings_model?: string | null
  embeddings_api_key?: string | null
  embedding_dim?: number
  embed_sample_rate?: number
}

export type EmbeddingsTestResult = {
  ok: boolean
  model: string
  base_url: string
  dimensions: number
  configured_dim: number
  dim_match: boolean
  latency_ms: number
}
