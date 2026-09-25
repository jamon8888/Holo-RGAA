//! Verbose, queryable audit-progress logging.
//!
//! `rgaa-orchestrator` already emits rich `tracing::info!`/`warn!` spans for
//! every audit phase (axe-core, gap-fix, page-context extraction, agentic
//! evaluation, per-page pass/fail/NA/compliance) and every per-page failure,
//! but nothing in `rgaa-cli` ever installed a subscriber to consume them —
//! they were silently discarded. This module wires them up to a JSON-lines
//! file so progress can be watched live (`tail -f <path> | jq`) while a
//! long-running audit is in flight, instead of only after it finishes.

use std::path::{Path, PathBuf};

use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling::{Builder, Rotation};

use crate::CliError;

/// Initializes process-wide structured logging to a JSON-lines file.
///
/// Returns the log file path actually used and a guard that must be kept
/// alive for the remainder of `main` — dropping it early can lose buffered
/// log lines that haven't been flushed to disk yet. Returns an error rather
/// than panicking if the log directory or file can't be created (e.g. an
/// unwritable `--log-file` destination) — the audit itself would otherwise
/// be aborted by a monitoring-only failure.
pub fn init(log_file: Option<&Path>) -> Result<(PathBuf, WorkerGuard), CliError> {
    let path = log_file
        .map(Path::to_path_buf)
        .unwrap_or_else(default_log_path);

    let dir = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(dir).map_err(|error| {
        CliError::execution(format!(
            "failed to create log directory {}: {error}",
            dir.display()
        ))
    })?;

    let file_name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "rgaa-audit.jsonl".to_string());
    let appender = Builder::new()
        .rotation(Rotation::NEVER)
        .filename_prefix(file_name)
        .build(dir)
        .map_err(|error| {
            CliError::execution(format!(
                "failed to open log file in {}: {error}",
                dir.display()
            ))
        })?;
    let (non_blocking, guard) = tracing_appender::non_blocking(appender);

    // Default: quiet on third-party crates (reqwest/hyper/datafusion/...),
    // verbose on the audit pipeline itself — precise, not noisy. Overridable
    // with RUST_LOG for one-off deeper digging.
    let filter = tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        tracing_subscriber::EnvFilter::new(
            "warn,rgaa_orchestrator=info,rgaa_agent=info,rgaa_holo=info,rgaa_spider=info,rgaa_browser_tools=info,rgaa_obscura=info",
        )
    });

    // try_init (not init): a second call in the same process, e.g. from a
    // test harness, must not panic.
    let _ = tracing_subscriber::fmt()
        .json()
        .with_env_filter(filter)
        .with_writer(non_blocking)
        .try_init();

    Ok((path, guard))
}

/// Includes the process id alongside the timestamp so two audits started
/// within the same second (e.g. launched by a script in a tight loop) get
/// distinct default log files instead of silently sharing one.
fn default_log_path() -> PathBuf {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let pid = std::process::id();
    PathBuf::from(format!("logs/rgaa-audit-{timestamp}-{pid}.jsonl"))
}
