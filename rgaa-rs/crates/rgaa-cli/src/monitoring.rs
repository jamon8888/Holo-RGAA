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

/// Initializes process-wide structured logging to a JSON-lines file.
///
/// Returns the log file path actually used and a guard that must be kept
/// alive for the remainder of `main` — dropping it early can lose buffered
/// log lines that haven't been flushed to disk yet.
pub fn init(log_file: Option<&Path>) -> (PathBuf, WorkerGuard) {
    let path = log_file
        .map(Path::to_path_buf)
        .unwrap_or_else(default_log_path);

    let dir = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let _ = std::fs::create_dir_all(dir);

    let file_name = path
        .file_name()
        .unwrap_or_else(|| std::ffi::OsStr::new("rgaa-audit.jsonl"));
    let appender = tracing_appender::rolling::never(dir, file_name);
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

    (path, guard)
}

fn default_log_path() -> PathBuf {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    PathBuf::from(format!("logs/rgaa-audit-{timestamp}.jsonl"))
}
