/** Port of src/vector_presets.rs — keep in sync when agent presets change. */

export type PlatformId = 'docker' | 'linux' | 'windows' | 'macos' | 'files'

export type PlatformPreset = {
  id: PlatformId
  label: string
  description: string
}

export const PRESETS: PlatformPreset[] = [
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
]

export function normalizePlatform(raw: string | undefined | null): PlatformId {
  const id = (raw ?? 'docker').trim().toLowerCase()
  return (PRESETS.find((p) => p.id === id)?.id ?? 'docker') as PlatformId
}

export function usesInlineToken(platform: string): boolean {
  return platform === 'windows'
}

export function presetEnv(platform: string, token: string): string {
  if (usesInlineToken(platform)) {
    return (
      '# Token is inlined in the YAML — no env file needed on Windows.\n' +
      '# Re-download connect info after rotating/removing the agent key.'
    )
  }
  return `INGEST_TOKEN=${token}\nVECTOR_DANGEROUSLY_ALLOW_ENV_VAR_INTERPOLATION=true`
}

function yamlQuote(s: string): string {
  return `"${s.replace(/\\/g, '\\\\').replace(/"/g, '\\"').replace(/\n/g, '\\n')}"`
}

function httpSink(
  inputs: string,
  uri: string,
  health: string,
  inlineToken: string | null,
): string {
  const tokenYaml = inlineToken !== null ? yamlQuote(inlineToken) : '"${INGEST_TOKEN}"'
  return `sinks:
  vector_collector:
    type: http
    inputs: [${inputs}]
    uri: ${uri}
    method: POST
    encoding:
      codec: json
    compression: gzip
    auth:
      strategy: bearer
      token: ${tokenYaml}
    healthcheck:
      enabled: true
      uri: ${health}
    batch:
      max_bytes: 1048576
      max_events: 500
      timeout_secs: 5
    buffer:
      type: disk
      max_size: 268435488
`
}

function heartbeatSource(health: string, inlineToken: string | null): string {
  const tokenYaml = inlineToken !== null ? yamlQuote(inlineToken) : '"${INGEST_TOKEN}"'
  return `  heartbeat:
    type: http_client
    endpoint: ${health}
    method: GET
    scrape_interval_secs: 30
    auth:
      strategy: bearer
      token: ${tokenYaml}
`
}

function sinkFor(
  platform: string,
  inputs: string,
  uri: string,
  health: string,
  token: string,
): string {
  const inline = usesInlineToken(platform) ? token : null
  return httpSink(inputs, uri, health, inline)
}

function heartbeatFor(platform: string, health: string, token: string): string {
  const inline = usesInlineToken(platform) ? token : null
  return heartbeatSource(health, inline)
}

function dockerYaml(uri: string, health: string, token: string): string {
  return `# Vector Collector — Docker
# Requires access to the Docker socket (typically /var/run/docker.sock).
# Set INGEST_TOKEN in the environment (see companion .env).
# \`heartbeat\` scrapes /v1/ingest/health every 30s for Online status (not sent as logs).
data_dir: /var/lib/vector

sources:
  docker:
    type: docker_logs
${heartbeatFor('docker', health, token)}transforms:
  normalize:
    type: remap
    inputs: [docker]
    source: |
      .message = string!(.message)
      if !exists(.source_type) {
        .source_type = "docker_logs"
      }

${sinkFor('docker', 'normalize', uri, health, token)}`
}

function linuxYaml(uri: string, health: string, token: string): string {
  return `# Vector Collector — Linux (journald)
# Run Vector as a user in the systemd-journal group (or root).
# Set INGEST_TOKEN in the environment (see companion .env).
# \`heartbeat\` scrapes /v1/ingest/health every 30s for Online status (not sent as logs).
data_dir: /var/lib/vector

sources:
  journal:
    type: journald
    current_boot_only: false
${heartbeatFor('linux', health, token)}transforms:
  normalize:
    type: remap
    inputs: [journal]
    source: |
      .message = string(.message) ?? string(.MESSAGE) ?? ""
      .container_name = string(.SYSLOG_IDENTIFIER) ?? string(._SYSTEMD_UNIT) ?? string(.CONTAINER_NAME) ?? "journal"
      if exists(.PRIORITY) {
        .stream = string!(.PRIORITY)
      }
      .source_type = "journald"

${sinkFor('linux', 'normalize', uri, health, token)}`
}

function windowsYaml(uri: string, health: string, token: string): string {
  return `# Vector Collector — Windows Event Log
# Install the Windows build of Vector. Run as a user that can read these channels.
# Bearer token is inlined below — no .env or VECTOR_DANGEROUSLY_ALLOW_ENV_VAR_INTERPOLATION needed.
# Treat this file as secret; re-download after rotating the agent key.
# Use forward slashes in paths (YAML "C:\\Users\\..." breaks — \\U is read as an escape).
# \`heartbeat\` scrapes /v1/ingest/health every 30s for Online status (not sent as logs).
data_dir: "C:/ProgramData/vector"

sources:
  winevent:
    type: windows_event_log
    channels:
      - Application
      - System
      - Security
    render_message: true
${heartbeatFor('windows', health, token)}transforms:
  normalize:
    type: remap
    inputs: [winevent]
    source: |
      .message = string(.message) ?? string(.Message) ?? ""
      .container_name = string(.Channel) ?? string(.channel) ?? "windows"
      if exists(.Provider) && is_object(.Provider) && exists(.Provider.Name) {
        .image = string!(.Provider.Name)
      } else if exists(.provider_name) {
        .image = string!(.provider_name)
      }
      .source_type = "windows_event_log"

${sinkFor('windows', 'normalize', uri, health, token)}`
}

function macosYaml(uri: string, health: string, token: string): string {
  return `# Vector Collector — macOS (file tails)
# Unified logging (log stream) is not used here; edit include paths as needed.
# May require Full Disk Access for the Vector process.
# Set INGEST_TOKEN in the environment (see companion .env).
# \`heartbeat\` scrapes /v1/ingest/health every 30s for Online status (not sent as logs).
data_dir: /usr/local/var/lib/vector

sources:
  files:
    type: file
    include:
      - /var/log/system.log
      - /var/log/*.log
      - /Library/Logs/**/*.log
    read_from: end
    ignore_not_found: true
${heartbeatFor('macos', health, token)}transforms:
  normalize:
    type: remap
    inputs: [files]
    source: |
      .message = string!(.message)
      .container_name = string(.file) ?? "macos"
      .source_type = "file"

${sinkFor('macos', 'normalize', uri, health, token)}`
}

function filesYaml(uri: string, health: string, token: string): string {
  return `# Vector Collector — generic file tails
# Edit include globs for your app logs. data_dir must be writable.
# Set INGEST_TOKEN in the environment (see companion .env).
# \`heartbeat\` scrapes /v1/ingest/health every 30s for Online status (not sent as logs).
data_dir: /var/lib/vector

sources:
  files:
    type: file
    include:
      - /var/log/**/*.log
      # - /path/to/your/app/*.log
    read_from: end
    ignore_not_found: true
${heartbeatFor('files', health, token)}transforms:
  normalize:
    type: remap
    inputs: [files]
    source: |
      .message = string!(.message)
      .container_name = string(.file) ?? "file"
      .source_type = "file"

${sinkFor('files', 'normalize', uri, health, token)}`
}

export function presetYaml(platform: string, uri: string, health: string, token: string): string {
  switch (normalizePlatform(platform)) {
    case 'linux':
      return linuxYaml(uri, health, token)
    case 'windows':
      return windowsYaml(uri, health, token)
    case 'macos':
      return macosYaml(uri, health, token)
    case 'files':
      return filesYaml(uri, health, token)
    default:
      return dockerYaml(uri, health, token)
  }
}

export type GeneratedConfig = {
  platform: PlatformId
  uri: string
  health: string
  yaml: string
  env: string
  inlineToken: boolean
}

export function generateConfig(
  publicBaseUrl: string,
  token: string,
  platform: string,
): GeneratedConfig {
  const selected = normalizePlatform(platform)
  const base = publicBaseUrl.trim().replace(/\/+$/, '') || 'http://localhost:8080'
  const effectiveToken = token.trim() || 'lk_your_ingest_token'
  const uri = `${base}/v1/logs`
  const health = `${base}/v1/ingest/health`
  return {
    platform: selected,
    uri,
    health,
    yaml: presetYaml(selected, uri, health, effectiveToken),
    env: presetEnv(selected, effectiveToken),
    inlineToken: usesInlineToken(selected),
  }
}
