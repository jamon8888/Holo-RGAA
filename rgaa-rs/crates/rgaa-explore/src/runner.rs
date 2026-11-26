//! Subprocess runner for agent-browser with the pinned Obscura binary.

use crate::pin::{
    agent_browser_version_matches, obscura_version_matches, AGENT_BROWSER_PACKAGE,
    AGENT_BROWSER_PINNED_VERSION, OBSCURA_PINNED_VERSION,
};
use crate::types::{
    Candidate, EngineInfo, ExplorationResult, ExploreOptions, ObservationKind, Observations,
    RawText, ScreenshotObservation, SessionInfo, SnapshotObservation, TargetInfo,
};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::process::Command as TokioCommand;

/// Maximum accepted per-command timeout.
const MAX_TIMEOUT_MS: u64 = 60_000;
/// Maximum retained characters per textual observation.
const MAX_OUTPUT: usize = 100_000;

/// Errors from an exploratory run.
#[derive(Debug, thiserror::Error)]
pub enum ExploreError {
    /// The URL was not an http(s) URL or could not be parsed.
    #[error("invalid URL: {0}")]
    InvalidUrl(String),
    /// Private-network access was requested without the explicit flag.
    #[error("private network access requires allow_private_network")]
    PrivateNetwork,
    /// The Obscura binary reported the wrong version.
    #[error("obscura version mismatch: got '{got}', want '{want}'")]
    ObscuraVersion {
        /// Reported version string.
        got: String,
        /// Required version string.
        want: &'static str,
    },
    /// The pinned agent-browser CLI reported the wrong version.
    #[error("agent-browser version mismatch: got '{got}', want '{want}'")]
    CliVersion {
        /// Reported version string.
        got: String,
        /// Required version string.
        want: &'static str,
    },
    /// agent-browser exited unsuccessfully.
    #[error("agent-browser failed: {0}")]
    Command(String),
    /// agent-browser exceeded the configured timeout.
    #[error("agent-browser timed out after {0} ms")]
    Timeout(u64),
    /// agent-browser returned output that could not be parsed.
    #[error("invalid agent-browser output: {0}")]
    InvalidOutput(String),
    /// Filesystem or process error.
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

fn truncate(text: String, max: usize) -> RawText {
    let truncated = text.chars().count() > max;
    RawText {
        text: if truncated {
            text.chars().take(max).collect()
        } else {
            text
        },
        truncated,
    }
}

fn is_private_host(host: &str) -> bool {
    let host = host.trim_matches(['[', ']']).to_ascii_lowercase();
    host == "localhost"
        || host == "::1"
        || host.starts_with("127.")
        || host.starts_with("10.")
        || host.starts_with("192.168.")
        || host
            .strip_prefix("172.")
            .and_then(|rest| rest.split('.').next())
            .and_then(|second| second.parse::<u8>().ok())
            .is_some_and(|second| (16..=31).contains(&second))
        || host.ends_with(".local")
        || host.ends_with(".internal")
}

fn validate_url(url: &str, allow_private: bool) -> Result<(String, String), ExploreError> {
    let parsed =
        url::Url::parse(url).map_err(|error| ExploreError::InvalidUrl(error.to_string()))?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err(ExploreError::InvalidUrl(format!(
            "unsupported scheme '{}'",
            parsed.scheme()
        )));
    }
    let host = parsed
        .host_str()
        .ok_or_else(|| ExploreError::InvalidUrl("missing host".to_owned()))?;
    if !allow_private && is_private_host(host) {
        return Err(ExploreError::PrivateNetwork);
    }
    let port = parsed
        .port()
        .map(|value| format!(":{value}"))
        .unwrap_or_default();
    Ok((
        parsed.to_string(),
        format!("{}://{}{}", parsed.scheme(), host, port),
    ))
}

fn generated_names() -> (String, String) {
    // agent-browser builds a Unix socket path from the session name and caps
    // it at 103 bytes. Keep both names short (and the run-dir prefix short)
    // so a fresh run never trips that ceiling.
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let name = format!("e{:x}{:x}", std::process::id() & 0xffff, nanos & 0xfffff);
    (name.clone(), name)
}

/// Obscura's CDP engine rejects proxy-bypass settings, and `NO_PROXY` is
/// interpreted as one. Strip those from browser commands; npm-facing version
/// checks keep the environment so package downloads still work behind a proxy.
fn scrub_proxy_bypass(command: &mut TokioCommand) {
    for name in ["NO_PROXY", "no_proxy", "AGENT_BROWSER_PROXY_BYPASS"] {
        command.env_remove(name);
    }
}

fn base_command(run_dir: &Path, executable: &Path, keep_open: bool) -> TokioCommand {
    let mut command = TokioCommand::new("npx");
    command
        .arg("-y")
        .arg(AGENT_BROWSER_PACKAGE)
        .env("npm_config_cache", run_dir.join("npm-cache"))
        .env("XDG_RUNTIME_DIR", run_dir.join("xdg-runtime"))
        .env("AGENT_BROWSER_SOCKET_DIR", run_dir.join("sock"))
        .env("TMPDIR", run_dir.join("tmp"))
        .env(
            "AGENT_BROWSER_IDLE_TIMEOUT_MS",
            if keep_open { "0" } else { "60000" },
        )
        .kill_on_drop(true);
    scrub_proxy_bypass(&mut command);
    let _ = executable;
    command
}

fn browser_command(
    run_dir: &Path,
    executable: &Path,
    namespace: &str,
    session: &str,
    keep_open: bool,
    allow_private: bool,
) -> TokioCommand {
    let mut command = base_command(run_dir, executable, keep_open);
    command
        .arg("--engine")
        .arg("obscura")
        .arg("--executable-path")
        .arg(executable)
        .arg("--namespace")
        .arg(namespace)
        .arg("--session")
        .arg(session);
    if allow_private {
        command.env("OBSCURA_ALLOW_PRIVATE_NETWORK", "1");
    }
    command
}

async fn run_command(mut command: TokioCommand, timeout_ms: u64) -> Result<String, ExploreError> {
    let output = tokio::time::timeout(Duration::from_millis(timeout_ms), command.output())
        .await
        .map_err(|_| ExploreError::Timeout(timeout_ms))??;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        return Err(ExploreError::Command(format!(
            "exit {}: {}",
            output.status,
            if stderr.is_empty() {
                String::from_utf8_lossy(&output.stdout).trim().to_owned()
            } else {
                stderr
            }
        )));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

#[derive(Debug, serde::Deserialize)]
struct SnapshotEnvelope {
    success: bool,
    #[serde(default)]
    data: Option<SnapshotData>,
    #[serde(default)]
    error: Option<serde_json::Value>,
}

#[derive(Debug, serde::Deserialize)]
struct SnapshotData {
    #[serde(default)]
    snapshot: String,
    #[serde(default)]
    refs: serde_json::Value,
}

/// Run one exploratory study.
///
/// # Errors
///
/// Returns an error on an invalid URL, a version-pin mismatch, a failed or
/// timed-out agent-browser command, or an unreadable observation.
pub async fn run(options: ExploreOptions) -> Result<ExplorationResult, ExploreError> {
    let timeout_ms = options.timeout_ms.clamp(1_000, MAX_TIMEOUT_MS);
    let max_output = options.max_output.min(MAX_OUTPUT);
    let (url, origin) = validate_url(&options.url, options.allow_private_network)?;
    if options.executable_path.as_os_str().is_empty() {
        return Err(ExploreError::Command(
            "no Obscura executable path supplied".to_owned(),
        ));
    }

    let temp = tempfile::Builder::new().prefix("rgae-").tempdir()?;
    for child in ["xdg-runtime", "sock", "tmp", "npm-cache", "artifacts"] {
        std::fs::create_dir_all(temp.path().join(child))?;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(
            temp.path().join("xdg-runtime"),
            std::fs::Permissions::from_mode(0o700),
        )?;
    }
    let (run_dir, _guard): (PathBuf, Option<tempfile::TempDir>) = if options.keep_open {
        (temp.keep(), None)
    } else {
        (temp.path().to_path_buf(), Some(temp))
    };

    let screenshot_dir = options
        .artifact_dir
        .clone()
        .unwrap_or_else(|| run_dir.join("artifacts"));
    std::fs::create_dir_all(&screenshot_dir)?;

    // Pin gates. The binary is invoked directly; the CLI version check also
    // primes the npm cache inside the run directory.
    let mut version_command = TokioCommand::new(&options.executable_path);
    version_command.arg("--version").kill_on_drop(true);
    let observed_version = run_command(version_command, timeout_ms).await?;
    if !obscura_version_matches(&observed_version) {
        return Err(ExploreError::ObscuraVersion {
            got: observed_version.trim().to_owned(),
            want: OBSCURA_PINNED_VERSION,
        });
    }
    let mut cli_version_command = TokioCommand::new("npx");
    cli_version_command
        .arg("-y")
        .arg(AGENT_BROWSER_PACKAGE)
        .arg("--version")
        .env("npm_config_cache", run_dir.join("npm-cache"))
        .env("TMPDIR", run_dir.join("tmp"))
        .kill_on_drop(true);
    let cli_version = run_command(cli_version_command, timeout_ms).await?;
    if !agent_browser_version_matches(&cli_version) {
        return Err(ExploreError::CliVersion {
            got: cli_version.trim().to_owned(),
            want: AGENT_BROWSER_PINNED_VERSION,
        });
    }

    let (default_namespace, default_session) = generated_names();
    let namespace = options.namespace.unwrap_or(default_namespace);
    let session = options.session.unwrap_or(default_session);
    let make_command = |json: bool| {
        let mut command = browser_command(
            &run_dir,
            &options.executable_path,
            &namespace,
            &session,
            options.keep_open,
            options.allow_private_network,
        );
        if json {
            command.arg("--json");
        }
        command
    };

    let mut open = make_command(false);
    open.arg("open").arg(&url);
    let _ = run_command(open, timeout_ms).await?;

    if let Some((width, height)) = options.viewport {
        let mut command = make_command(false);
        command
            .arg("set")
            .arg("viewport")
            .arg(width.to_string())
            .arg(height.to_string());
        let _ = run_command(command, timeout_ms).await?;
    }

    let mut observations = Observations::default();
    for kind in &options.include {
        match kind {
            ObservationKind::Snapshot => {
                let mut command = make_command(true);
                command.arg("snapshot");
                let raw = run_command(command, timeout_ms).await?;
                let envelope: SnapshotEnvelope = serde_json::from_str(&raw)
                    .map_err(|error| ExploreError::InvalidOutput(error.to_string()))?;
                if !envelope.success {
                    return Err(ExploreError::Command(format!(
                        "snapshot failed: {}",
                        envelope
                            .error
                            .map(|value| value.to_string())
                            .unwrap_or_else(|| "unknown error".to_owned())
                    )));
                }
                let data = envelope.data.ok_or_else(|| {
                    ExploreError::InvalidOutput("missing snapshot data".to_owned())
                })?;
                let text = truncate(data.snapshot, max_output);
                observations.snapshot = Some(SnapshotObservation {
                    text: text.text,
                    refs: data.refs,
                    truncated: text.truncated,
                });
            }
            ObservationKind::Screenshot => {
                let path = screenshot_dir.join("preview.png");
                let mut command = make_command(false);
                command.arg("screenshot").arg(&path);
                let _ = run_command(command, timeout_ms).await?;
                if path.is_file() {
                    observations.screenshot = Some(ScreenshotObservation {
                        path: path.to_string_lossy().into_owned(),
                        preview_only: true,
                    });
                }
            }
            ObservationKind::Console => {
                let mut command = make_command(true);
                command.arg("console");
                observations.console = Some(truncate(
                    run_command(command, timeout_ms).await?,
                    max_output,
                ));
            }
            ObservationKind::Errors => {
                let mut command = make_command(true);
                command.arg("errors");
                observations.errors = Some(truncate(
                    run_command(command, timeout_ms).await?,
                    max_output,
                ));
            }
        }
    }

    let reported_socket_dir = {
        let mut command = make_command(false);
        command.arg("session").arg("info").arg("--json");
        match run_command(command, timeout_ms).await {
            Ok(raw) => {
                let start = raw.find('{').unwrap_or(0);
                serde_json::from_str::<serde_json::Value>(&raw[start..])
                    .ok()
                    .and_then(|value| {
                        value
                            .get("data")?
                            .get("socketDir")?
                            .as_str()
                            .map(ToOwned::to_owned)
                    })
            }
            Err(_) => None,
        }
    };

    let final_url = {
        let mut command = make_command(false);
        command.arg("get").arg("url");
        match run_command(command, timeout_ms).await {
            Ok(text) => text.trim().to_owned(),
            Err(_) => url.clone(),
        }
    };

    if !options.keep_open {
        let mut command = make_command(false);
        command.arg("close");
        if let Err(error) = run_command(command, timeout_ms).await {
            tracing::debug!(%error, "failed to close exploration session");
        }
    }

    let socket_dir = reported_socket_dir.unwrap_or_else(|| {
        run_dir
            .join("xdg-runtime")
            .join("agent-browser")
            .join("namespaces")
            .join(&namespace)
            .join("run")
            .to_string_lossy()
            .into_owned()
    });

    Ok(ExplorationResult::new(
        EngineInfo {
            name: "obscura",
            requested_version: OBSCURA_PINNED_VERSION,
            observed_version: observed_version.trim().to_owned(),
            cli_version: AGENT_BROWSER_PINNED_VERSION,
            executable_path: options.executable_path.to_string_lossy().into_owned(),
        },
        SessionInfo {
            name: session,
            namespace,
            socket_dir,
            left_open: options.keep_open,
            run_dir: run_dir.to_string_lossy().into_owned(),
        },
        TargetInfo {
            url: if final_url.is_empty() { url } else { final_url },
            origin,
            private_network: options.allow_private_network,
        },
        observations,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn private_hosts_are_detected() {
        for host in [
            "localhost",
            "127.0.0.1",
            "10.1.2.3",
            "192.168.0.1",
            "172.16.0.1",
            "172.31.255.1",
            "[::1]",
            "app.local",
        ] {
            assert!(is_private_host(host), "{host} should be private");
        }
        for host in ["example.com", "172.32.0.1", "8.8.8.8"] {
            assert!(!is_private_host(host), "{host} should be public");
        }
    }

    #[test]
    fn url_validation_defaults_to_public() {
        assert!(validate_url("https://example.com/a?b=1", false).is_ok());
        assert!(matches!(
            validate_url("http://localhost:3000", false),
            Err(ExploreError::PrivateNetwork)
        ));
        assert!(validate_url("http://localhost:3000", true).is_ok());
        assert!(matches!(
            validate_url("file:///etc/passwd", false),
            Err(ExploreError::InvalidUrl(_))
        ));
    }

    #[test]
    fn generated_names_fit_the_agent_browser_socket_limit() {
        let (namespace, session) = generated_names();
        assert_eq!(namespace, session);
        assert!(namespace.len() <= 12, "name too long: {namespace}");
    }

    #[test]
    fn truncation_reports_when_it_cuts() {
        let raw = truncate("0123456789".to_owned(), 4);
        assert_eq!(raw.text, "0123");
        assert!(raw.truncated);
        let short = truncate("abc".to_owned(), 4);
        assert!(!short.truncated);
    }
}

// Candidates are deliberately empty in this phase: a hint must be interpreted
// by a human or agent, never emitted as a settled classification. Keep the type
// referenced so the contract stays stable when axe-based hints are added.
#[allow(dead_code)]
fn _candidate_type_is_part_of_the_contract(candidate: Candidate) -> String {
    candidate.id
}
