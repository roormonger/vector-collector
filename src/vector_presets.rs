//! Ready-to-paste Vector agent configs for common platforms.

use serde_json::{json, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlatformPreset {
    pub id: &'static str,
    pub label: &'static str,
    pub description: &'static str,
}

pub const PRESETS: &[PlatformPreset] = &[
    PlatformPreset {
        id: "docker",
        label: "Docker",
        description: "Container logs via docker_logs (Linux/macOS Docker Engine or Desktop)",
    },
    PlatformPreset {
        id: "linux",
        label: "Linux",
        description: "systemd journal (journald) with message remap",
    },
    PlatformPreset {
        id: "windows",
        label: "Windows",
        description: "Windows Event Log — token inlined in YAML (no env file)",
    },
    PlatformPreset {
        id: "macos",
        label: "macOS",
        description: "Common system/app log files under /var/log and /Library/Logs",
    },
    PlatformPreset {
        id: "files",
        label: "Files",
        description: "Tail arbitrary log files — edit the include paths",
    },
];

pub fn normalize_platform(raw: Option<&str>) -> &'static str {
    let id = raw.unwrap_or("docker").trim().to_ascii_lowercase();
    PRESETS
        .iter()
        .find(|p| p.id == id)
        .map(|p| p.id)
        .unwrap_or("docker")
}

pub fn uses_inline_token(platform: &str) -> bool {
    platform == "windows"
}

pub fn vector_agent_bundle(public_base_url: &str, token: &str, platform: Option<&str>) -> Value {
    let selected = normalize_platform(platform);
    let base = public_base_url.trim_end_matches('/');
    let uri = format!("{base}/v1/logs");
    let health = format!("{base}/v1/ingest/health");

    let presets: Vec<Value> = PRESETS
        .iter()
        .map(|p| {
            let inline = uses_inline_token(p.id);
            json!({
                "id": p.id,
                "label": p.label,
                "description": p.description,
                "yaml": preset_yaml(p.id, &uri, &health, token),
                "env": preset_env(p.id, token),
                "inline_token": inline,
            })
        })
        .collect();

    let yaml = preset_yaml(selected, &uri, &health, token);
    let env = preset_env(selected, token);

    json!({
        "token": token,
        "uri": uri,
        "env": env,
        "inline_token": uses_inline_token(selected),
        "platform": selected,
        "yaml": yaml,
        "presets": presets,
    })
}

fn preset_env(platform: &str, token: &str) -> String {
    if uses_inline_token(platform) {
        "# Token is inlined in the YAML — no env file needed on Windows.\n# Re-download connect info after rotating/removing the agent key.".into()
    } else {
        // Vector 0.57+ leaves ${INGEST_TOKEN} literal unless interpolation is explicitly enabled.
        format!(
            "INGEST_TOKEN={token}\nVECTOR_DANGEROUSLY_ALLOW_ENV_VAR_INTERPOLATION=true"
        )
    }
}

fn yaml_quote(s: &str) -> String {
    format!(
        "\"{}\"",
        s.replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
    )
}

/// `inline_token`: Some(raw token) embeds bearer in YAML; None uses `${INGEST_TOKEN}`.
fn http_sink(inputs: &str, uri: &str, health: &str, inline_token: Option<&str>) -> String {
    let token_yaml = match inline_token {
        Some(t) => yaml_quote(t),
        None => "\"${INGEST_TOKEN}\"".to_string(),
    };
    format!(
        r#"sinks:
  vector_collector:
    type: http
    inputs: [{inputs}]
    uri: {uri}
    method: POST
    encoding:
      codec: json
    compression: gzip
    auth:
      strategy: bearer
      token: {token_yaml}
    healthcheck:
      enabled: true
      uri: {health}
    batch:
      max_bytes: 1048576
      max_events: 500
      timeout_secs: 5
    buffer:
      type: disk
      max_size: 268435488
"#
    )
}

/// Outbound scrape of collector health — not wired into the sink (no fake log lines).
/// Keeps agent Online in the admin UI while Vector is running.
fn heartbeat_source(health: &str, inline_token: Option<&str>) -> String {
    let token_yaml = match inline_token {
        Some(t) => yaml_quote(t),
        None => "\"${INGEST_TOKEN}\"".to_string(),
    };
    format!(
        r#"  heartbeat:
    type: http_client
    endpoint: {health}
    method: GET
    scrape_interval_secs: 30
    auth:
      strategy: bearer
      token: {token_yaml}
"#
    )
}

fn sink_for(platform: &str, inputs: &str, uri: &str, health: &str, token: &str) -> String {
    let inline = if uses_inline_token(platform) {
        Some(token)
    } else {
        None
    };
    http_sink(inputs, uri, health, inline)
}

fn heartbeat_for(platform: &str, health: &str, token: &str) -> String {
    let inline = if uses_inline_token(platform) {
        Some(token)
    } else {
        None
    };
    heartbeat_source(health, inline)
}

fn preset_yaml(platform: &str, uri: &str, health: &str, token: &str) -> String {
    match platform {
        "linux" => linux_yaml(uri, health, token),
        "windows" => windows_yaml(uri, health, token),
        "macos" => macos_yaml(uri, health, token),
        "files" => files_yaml(uri, health, token),
        _ => docker_yaml(uri, health, token),
    }
}

fn docker_yaml(uri: &str, health: &str, token: &str) -> String {
    format!(
        r#"# Vector Collector — Docker
# Requires access to the Docker socket (typically /var/run/docker.sock).
# Set INGEST_TOKEN in the environment (see companion .env).
# `heartbeat` scrapes /v1/ingest/health every 30s for Online status (not sent as logs).
data_dir: /var/lib/vector

sources:
  docker:
    type: docker_logs
{heartbeat}
transforms:
  normalize:
    type: remap
    inputs: [docker]
    source: |
      .message = string!(.message)
      if !exists(.source_type) {{
        .source_type = "docker_logs"
      }}

{sink}"#,
        heartbeat = heartbeat_for("docker", health, token),
        sink = sink_for("docker", "normalize", uri, health, token)
    )
}

fn linux_yaml(uri: &str, health: &str, token: &str) -> String {
    format!(
        r#"# Vector Collector — Linux (journald)
# Run Vector as a user in the systemd-journal group (or root).
# Set INGEST_TOKEN in the environment (see companion .env).
# `heartbeat` scrapes /v1/ingest/health every 30s for Online status (not sent as logs).
data_dir: /var/lib/vector

sources:
  journal:
    type: journald
    current_boot_only: false
{heartbeat}
transforms:
  normalize:
    type: remap
    inputs: [journal]
    source: |
      .message = string(.message) ?? string(.MESSAGE) ?? ""
      .container_name = string(.SYSLOG_IDENTIFIER) ?? string(._SYSTEMD_UNIT) ?? string(.CONTAINER_NAME) ?? "journal"
      if exists(.PRIORITY) {{
        .stream = string!(.PRIORITY)
      }}
      .source_type = "journald"

{sink}"#,
        heartbeat = heartbeat_for("linux", health, token),
        sink = sink_for("linux", "normalize", uri, health, token)
    )
}

fn windows_yaml(uri: &str, health: &str, token: &str) -> String {
    format!(
        r#"# Vector Collector — Windows Event Log
# Install the Windows build of Vector. Run as a user that can read these channels.
# Bearer token is inlined below — no .env or VECTOR_DANGEROUSLY_ALLOW_ENV_VAR_INTERPOLATION needed.
# Treat this file as secret; re-download after rotating the agent key.
# Use forward slashes in paths (YAML "C:\Users\..." breaks — \U is read as an escape).
# `heartbeat` scrapes /v1/ingest/health every 30s for Online status (not sent as logs).
data_dir: "C:/ProgramData/vector"

sources:
  winevent:
    type: windows_event_log
    channels:
      - Application
      - System
      - Security
    render_message: true
{heartbeat}
transforms:
  normalize:
    type: remap
    inputs: [winevent]
    source: |
      .message = string(.message) ?? string(.Message) ?? ""
      .container_name = string(.Channel) ?? string(.channel) ?? "windows"
      if exists(.Provider) && is_object(.Provider) && exists(.Provider.Name) {{
        .image = string!(.Provider.Name)
      }} else if exists(.provider_name) {{
        .image = string!(.provider_name)
      }}
      .source_type = "windows_event_log"

{sink}"#,
        heartbeat = heartbeat_for("windows", health, token),
        sink = sink_for("windows", "normalize", uri, health, token)
    )
}

fn macos_yaml(uri: &str, health: &str, token: &str) -> String {
    format!(
        r#"# Vector Collector — macOS (file tails)
# Unified logging (log stream) is not used here; edit include paths as needed.
# May require Full Disk Access for the Vector process.
# Set INGEST_TOKEN in the environment (see companion .env).
# `heartbeat` scrapes /v1/ingest/health every 30s for Online status (not sent as logs).
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
{heartbeat}
transforms:
  normalize:
    type: remap
    inputs: [files]
    source: |
      .message = string!(.message)
      .container_name = string(.file) ?? "macos"
      .source_type = "file"

{sink}"#,
        heartbeat = heartbeat_for("macos", health, token),
        sink = sink_for("macos", "normalize", uri, health, token)
    )
}

fn files_yaml(uri: &str, health: &str, token: &str) -> String {
    format!(
        r#"# Vector Collector — generic file tails
# Edit include globs for your app logs. data_dir must be writable.
# Set INGEST_TOKEN in the environment (see companion .env).
# `heartbeat` scrapes /v1/ingest/health every 30s for Online status (not sent as logs).
data_dir: /var/lib/vector

sources:
  files:
    type: file
    include:
      - /var/log/**/*.log
      # - /path/to/your/app/*.log
    read_from: end
    ignore_not_found: true
{heartbeat}
transforms:
  normalize:
    type: remap
    inputs: [files]
    source: |
      .message = string!(.message)
      .container_name = string(.file) ?? "file"
      .source_type = "file"

{sink}"#,
        heartbeat = heartbeat_for("files", health, token),
        sink = sink_for("files", "normalize", uri, health, token)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_falls_back_to_docker() {
        assert_eq!(normalize_platform(None), "docker");
        assert_eq!(normalize_platform(Some("nope")), "docker");
        assert_eq!(normalize_platform(Some("Windows")), "windows");
    }

    #[test]
    fn bundle_includes_all_presets() {
        let v = vector_agent_bundle("http://example:8080", "lk_test", Some("linux"));
        assert_eq!(v["platform"], "linux");
        assert!(v["yaml"].as_str().unwrap().contains("journald"));
        assert_eq!(v["presets"].as_array().unwrap().len(), PRESETS.len());
        assert!(v["yaml"].as_str().unwrap().contains("http://example:8080/v1/logs"));
        assert!(v["yaml"].as_str().unwrap().contains("${INGEST_TOKEN}"));
        assert!(v["yaml"].as_str().unwrap().contains("type: http_client"));
        assert!(!v["inline_token"].as_bool().unwrap());
    }

    #[test]
    fn windows_inlines_token() {
        let v = vector_agent_bundle("http://example:8080", "lk_secret_token", Some("windows"));
        let yaml = v["yaml"].as_str().unwrap();
        assert!(yaml.contains("lk_secret_token"));
        assert!(!yaml.contains("${INGEST_TOKEN}"));
        assert!(yaml.contains("type: http_client"));
        assert!(v["inline_token"].as_bool().unwrap());
        let win = v["presets"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["id"] == "windows")
            .unwrap();
        assert!(win["inline_token"].as_bool().unwrap());
        assert!(win["yaml"].as_str().unwrap().contains("lk_secret_token"));
    }
}
