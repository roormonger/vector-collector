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
        description: "Windows Event Log (Application, System, Security)",
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

pub fn vector_agent_bundle(public_base_url: &str, token: &str, platform: Option<&str>) -> Value {
    let selected = normalize_platform(platform);
    let base = public_base_url.trim_end_matches('/');
    let uri = format!("{base}/v1/logs");
    let health = format!("{base}/v1/ingest/health");
    let env = format!(
        "INGEST_TOKEN={token}\nVECTOR_DANGEROUSLY_ALLOW_ENV_VAR_INTERPOLATION=true"
    );

    let presets: Vec<Value> = PRESETS
        .iter()
        .map(|p| {
            json!({
                "id": p.id,
                "label": p.label,
                "description": p.description,
                "yaml": preset_yaml(p.id, &uri, &health),
            })
        })
        .collect();

    let yaml = preset_yaml(selected, &uri, &health);

    json!({
        "token": token,
        "uri": uri,
        "env": env,
        "platform": selected,
        "yaml": yaml,
        "presets": presets,
    })
}

fn http_sink(inputs: &str, uri: &str, health: &str) -> String {
    format!(
        r#"sinks:
  vector_collector:
    type: http
    inputs: [{inputs}]
    uri: {uri}
    method: post
    encoding:
      codec: json
    compression: gzip
    auth:
      strategy: bearer
      token: "${{INGEST_TOKEN}}"
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

fn preset_yaml(platform: &str, uri: &str, health: &str) -> String {
    match platform {
        "linux" => linux_yaml(uri, health),
        "windows" => windows_yaml(uri, health),
        "macos" => macos_yaml(uri, health),
        "files" => files_yaml(uri, health),
        _ => docker_yaml(uri, health),
    }
}

fn docker_yaml(uri: &str, health: &str) -> String {
    format!(
        r#"# Vector Collector — Docker
# Requires access to the Docker socket (typically /var/run/docker.sock).
data_dir: /var/lib/vector

sources:
  docker:
    type: docker_logs

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
        sink = http_sink("normalize", uri, health)
    )
}

fn linux_yaml(uri: &str, health: &str) -> String {
    format!(
        r#"# Vector Collector — Linux (journald)
# Run Vector as a user in the systemd-journal group (or root).
data_dir: /var/lib/vector

sources:
  journal:
    type: journald
    current_boot_only: false

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
        sink = http_sink("normalize", uri, health)
    )
}

fn windows_yaml(uri: &str, health: &str) -> String {
    format!(
        r#"# Vector Collector — Windows Event Log
# Install the Windows build of Vector. Run as a user that can read these channels.
data_dir: "C:/ProgramData/vector"

sources:
  winevent:
    type: windows_event_log
    channels:
      - Application
      - System
      - Security
    render_message: true

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
        sink = http_sink("normalize", uri, health)
    )
}

fn macos_yaml(uri: &str, health: &str) -> String {
    format!(
        r#"# Vector Collector — macOS (file tails)
# Unified logging (log stream) is not used here; edit include paths as needed.
# May require Full Disk Access for the Vector process.
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

transforms:
  normalize:
    type: remap
    inputs: [files]
    source: |
      .message = string!(.message)
      .container_name = string(.file) ?? "macos"
      .source_type = "file"

{sink}"#,
        sink = http_sink("normalize", uri, health)
    )
}

fn files_yaml(uri: &str, health: &str) -> String {
    format!(
        r#"# Vector Collector — generic file tails
# Edit include globs for your app logs. data_dir must be writable.
data_dir: /var/lib/vector

sources:
  files:
    type: file
    include:
      - /var/log/**/*.log
      # - /path/to/your/app/*.log
    read_from: end
    ignore_not_found: true

transforms:
  normalize:
    type: remap
    inputs: [files]
    source: |
      .message = string!(.message)
      .container_name = string(.file) ?? "file"
      .source_type = "file"

{sink}"#,
        sink = http_sink("normalize", uri, health)
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
    }
}
