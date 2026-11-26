//! Daily quota counting and the asynchronous usage report to the licence service.

use std::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};
use fs2::FileExt;
use serde::Serialize;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use crate::cache::{LicenseCache, UsageFile};
use crate::error::LicenseError;
use crate::tiers::Feature;

/// The quota position after a decision, as sent in `X-RateLimit-*` headers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RateInfo {
    pub limit: u32,
    pub remaining: u32,
    /// Next 00:00 UTC.
    pub reset_at: DateTime<Utc>,
}

impl RateInfo {
    /// Whole seconds until the reset, rounded up and at least 1, for `Retry-After`.
    pub fn retry_after_secs(&self, now: DateTime<Utc>) -> u64 {
        let millis = (self.reset_at - now).num_milliseconds().max(0) as u64;
        millis.div_ceil(1000).max(1)
    }
}

fn next_midnight(day: NaiveDate) -> DateTime<Utc> {
    day.succ_opt()
        .and_then(|d| d.and_hms_opt(0, 0, 0))
        .map(|dt| dt.and_utc())
        .unwrap_or(DateTime::<Utc>::MAX_UTC)
}

/// Why [`UsageCounter::try_consume`] consumed nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConsumeError {
    /// The request does not fit in what is left today.
    Exhausted(RateInfo),
    /// The persistent counter could not be locked or written.
    Storage(String),
}

/// Units consumed today (UTC), for the whole installation.
///
/// A mutex guards in-process requests; a file lock serializes persistent
/// read-modify-write operations across processes.
pub struct UsageCounter {
    state: Mutex<UsageFile>,
    persist: Option<PathBuf>,
}

impl UsageCounter {
    pub fn in_memory() -> Self {
        Self {
            state: Mutex::new(UsageFile {
                day: NaiveDate::MIN,
                count: 0,
            }),
            persist: None,
        }
    }

    /// Loads `usage.toml` if present. A corrupt file is logged and replaced:
    /// a local counter is not a security boundary, and refusing to start
    /// would punish the user for a disk error.
    pub fn persistent(path: PathBuf) -> Self {
        let initial = match LicenseCache::load_usage(&path) {
            Ok(Some(u)) => u,
            Ok(None) => UsageFile {
                day: NaiveDate::MIN,
                count: 0,
            },
            Err(e) => {
                tracing::warn!(error = %e, "ignoring unreadable usage file");
                UsageFile {
                    day: NaiveDate::MIN,
                    count: 0,
                }
            }
        };
        Self {
            state: Mutex::new(initial),
            persist: Some(path),
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, UsageFile> {
        self.state.lock().unwrap_or_else(|p| p.into_inner())
    }

    fn roll(state: &mut UsageFile, today: NaiveDate) {
        if today > state.day {
            *state = UsageFile {
                day: today,
                count: 0,
            };
        }
    }

    /// Current position without consuming anything.
    pub fn peek(&self, limit: u32, now: DateTime<Utc>) -> RateInfo {
        let mut state = self.lock();
        // A caller that sampled `now` before waiting must not move the day back.
        let today = now.date_naive().max(state.day);
        if let Some(path) = &self.persist {
            match lock_usage_file(path)
                .and_then(|_guard| Ok(LicenseCache::load_usage(path)?.unwrap_or(*state)))
            {
                Ok(latest) => *state = latest,
                Err(error) => tracing::warn!(%error, "failed to read persistent usage counter"),
            }
        }
        Self::roll(&mut state, today);
        RateInfo {
            limit,
            remaining: limit.saturating_sub(state.count),
            reset_at: next_midnight(state.day),
        }
    }

    /// Consumes `cost` units if all of them fit. A request that does not fit
    /// consumes nothing (a batch of 20 with 5 left is refused whole).
    pub fn try_consume(
        &self,
        cost: u32,
        limit: u32,
        now: DateTime<Utc>,
    ) -> Result<RateInfo, ConsumeError> {
        let mut state = self.lock();
        // A caller that sampled `now` before waiting must not move the day back.
        let today = now.date_naive().max(state.day);
        let persistent_lock = match &self.persist {
            Some(path) => match lock_usage_file(path) {
                Ok(lock) => Some(lock),
                Err(error) => {
                    tracing::error!(%error, "failed to lock persistent usage counter");
                    return Err(ConsumeError::Storage(error.to_string()));
                }
            },
            None => None,
        };
        if let Some(path) = &self.persist {
            match LicenseCache::load_usage(path) {
                Ok(Some(latest)) => *state = latest,
                Ok(None) => {}
                Err(error) => tracing::warn!(%error, "ignoring unreadable usage file"),
            }
        }
        Self::roll(&mut state, today);
        let reset_at = next_midnight(state.day);
        let remaining = limit.saturating_sub(state.count);
        if cost > remaining {
            return Err(ConsumeError::Exhausted(RateInfo {
                limit,
                remaining,
                reset_at,
            }));
        }
        let previous = *state;
        state.count += cost;
        if let Some(path) = &self.persist {
            if let Err(error) = LicenseCache::store_usage(path, &state) {
                *state = previous;
                tracing::error!(%error, "failed to persist usage counter");
                return Err(ConsumeError::Storage(error.to_string()));
            }
        }
        drop(persistent_lock);
        let after = limit.saturating_sub(state.count);
        Ok(RateInfo {
            limit,
            remaining: after,
            reset_at,
        })
    }
}

fn lock_usage_file(path: &Path) -> Result<File, Box<dyn std::error::Error + Send + Sync>> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(parent)?;
    let mut lock_path = path.as_os_str().to_os_string();
    lock_path.push(".lock");
    let lock = OpenOptions::new()
        .create(true)
        .truncate(false) // a lock file: its contents are never read
        .read(true)
        .write(true)
        .open(lock_path)?;
    lock.lock_exclusive()?;
    Ok(lock)
}

/// One metered operation, reported to the licence service.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UsageEvent {
    pub feature: Feature,
    pub units: u32,
    pub at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
}

/// Where usage is reported. The licence service endpoint is not defined yet
/// (#87), so the only implementation shipped is [`NoopSink`].
#[async_trait]
pub trait UsageSink: Send + Sync {
    async fn report(&self, events: &[UsageEvent]) -> Result<(), LicenseError>;
}

pub struct NoopSink;

#[async_trait]
impl UsageSink for NoopSink {
    async fn report(&self, _events: &[UsageEvent]) -> Result<(), LicenseError> {
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ReporterConfig {
    pub flush_every: Duration,
    /// A report slower than this counts as failed and is retried next flush.
    pub sink_timeout: Duration,
    /// Oldest events are dropped beyond this many pending, so an unreachable
    /// service cannot grow memory without bound.
    pub max_pending: usize,
}

impl Default for ReporterConfig {
    fn default() -> Self {
        Self {
            flush_every: Duration::from_secs(60),
            sink_timeout: Duration::from_secs(10),
            max_pending: 10_000,
        }
    }
}

/// Non-blocking usage reporting. [`UsageReporter::record`] never awaits and
/// never fails the caller: events go to a background task that batches them
/// and keeps a failed batch for the next flush.
#[derive(Clone)]
pub struct UsageReporter {
    tx: mpsc::Sender<UsageEvent>,
}

impl UsageReporter {
    /// Must be called inside a Tokio runtime. Dropping every clone of the
    /// returned reporter makes the task flush once more and exit.
    pub fn spawn(sink: Arc<dyn UsageSink>, config: ReporterConfig) -> (Self, JoinHandle<()>) {
        let (tx, rx) = mpsc::channel(config.max_pending.max(1));
        let handle = tokio::spawn(run_reporter(sink, config, rx));
        (Self { tx }, handle)
    }

    pub fn record(&self, event: UsageEvent) {
        if let Err(e) = self.tx.try_send(event) {
            tracing::warn!(error = %e, "usage event dropped");
        }
    }
}

async fn run_reporter(
    sink: Arc<dyn UsageSink>,
    config: ReporterConfig,
    mut rx: mpsc::Receiver<UsageEvent>,
) {
    let mut pending: Vec<UsageEvent> = Vec::new();
    let mut tick = tokio::time::interval(config.flush_every);
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    tick.tick().await; // the first tick fires immediately

    loop {
        tokio::select! {
            ev = rx.recv() => match ev {
                Some(ev) => {
                    pending.push(ev);
                    if pending.len() > config.max_pending {
                        let excess = pending.len() - config.max_pending;
                        pending.drain(..excess);
                        tracing::warn!(dropped = excess, "usage backlog full");
                    }
                }
                None => {
                    flush(&*sink, &config, &mut pending).await;
                    return;
                }
            },
            _ = tick.tick() => flush(&*sink, &config, &mut pending).await,
        }
    }
}

async fn flush(sink: &dyn UsageSink, config: &ReporterConfig, pending: &mut Vec<UsageEvent>) {
    if pending.is_empty() {
        return;
    }
    match tokio::time::timeout(config.sink_timeout, sink.report(pending)).await {
        Ok(Ok(())) => pending.clear(),
        Ok(Err(e)) => {
            tracing::warn!(error = %e, pending = pending.len(), "usage sync failed; will retry")
        }
        Err(_) => tracing::warn!(pending = pending.len(), "usage sync timed out; will retry"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn at(h: u32, m: u32, s: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 11, 19, h, m, s).unwrap()
    }

    #[test]
    fn nth_passes_and_n_plus_one_fails() {
        let c = UsageCounter::in_memory();
        for i in 1..=10 {
            let r = c.try_consume(1, 10, at(12, 0, 0)).unwrap();
            assert_eq!(r.remaining, 10 - i);
        }
        let Err(ConsumeError::Exhausted(refused)) = c.try_consume(1, 10, at(12, 0, 0)) else {
            panic!("expected exhaustion");
        };
        assert_eq!(refused.remaining, 0);
        assert_eq!(
            refused.reset_at,
            Utc.with_ymd_and_hms(2026, 11, 20, 0, 0, 0).unwrap()
        );
    }

    #[test]
    fn resets_at_utc_midnight() {
        let c = UsageCounter::in_memory();
        for _ in 0..10 {
            c.try_consume(1, 10, at(23, 59, 59)).unwrap();
        }
        assert!(c.try_consume(1, 10, at(23, 59, 59)).is_err());
        let next = Utc.with_ymd_and_hms(2026, 11, 20, 0, 0, 0).unwrap();
        assert_eq!(c.try_consume(1, 10, next).unwrap().remaining, 9);
    }

    #[test]
    fn oversized_request_consumes_nothing() {
        let c = UsageCounter::in_memory();
        c.try_consume(8, 10, at(9, 0, 0)).unwrap();
        assert!(c.try_consume(3, 10, at(9, 0, 0)).is_err());
        assert_eq!(c.peek(10, at(9, 0, 0)).remaining, 2);
        assert_eq!(c.try_consume(2, 10, at(9, 0, 0)).unwrap().remaining, 0);
    }

    #[test]
    fn earlier_clock_date_does_not_reset_a_newer_day() {
        let c = UsageCounter::in_memory();
        let later = at(12, 0, 0) + chrono::Duration::days(1);
        c.try_consume(4, 10, later).unwrap();

        let stale = c.try_consume(1, 10, at(12, 0, 0)).unwrap();
        assert_eq!(stale.remaining, 5);
        assert_eq!(
            stale.reset_at,
            Utc.with_ymd_and_hms(2026, 11, 21, 0, 0, 0).unwrap()
        );
        assert_eq!(c.peek(10, later).remaining, 5);
    }

    #[test]
    fn retry_after_rounds_up_to_the_reset() {
        let info = RateInfo {
            limit: 10,
            remaining: 0,
            reset_at: Utc.with_ymd_and_hms(2026, 11, 20, 0, 0, 0).unwrap(),
        };
        assert_eq!(info.retry_after_secs(at(23, 59, 59)), 1);
        assert_eq!(info.retry_after_secs(at(21, 0, 0)), 3 * 3600);
        let half = at(23, 59, 58) + chrono::Duration::milliseconds(500);
        assert_eq!(info.retry_after_secs(half), 2);
    }

    #[test]
    fn persistent_counter_survives_restart() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("usage.toml");
        let c = UsageCounter::persistent(path.clone());
        for _ in 0..4 {
            c.try_consume(1, 10, at(10, 0, 0)).unwrap();
        }
        drop(c);
        let c = UsageCounter::persistent(path);
        assert_eq!(c.peek(10, at(10, 0, 0)).remaining, 6);
    }

    #[test]
    fn persistent_reload_does_not_move_the_day_back() {
        let dir = tempfile::tempdir().unwrap();
        let c = UsageCounter::persistent(dir.path().join("usage.toml"));
        c.try_consume(3, 10, at(10, 0, 0)).unwrap();
        // `peek` rolls memory to the next day without writing the file.
        let next_day = at(0, 0, 1) + chrono::Duration::days(1);
        c.peek(10, next_day);
        // A request that sampled `now` before midnight is charged to the new day.
        let late = c.try_consume(1, 10, at(23, 59, 59)).unwrap();
        assert_eq!(late.remaining, 9);
        assert_eq!(late.reset_at, next_midnight(next_day.date_naive()));
    }

    #[test]
    fn unwritable_counter_is_a_storage_error_not_exhaustion() {
        let dir = tempfile::tempdir().unwrap();
        // The parent of the usage file is a regular file, so nothing can be created under it.
        let blocker = dir.path().join("blocker");
        std::fs::write(&blocker, "").unwrap();
        let c = UsageCounter::persistent(blocker.join("usage.toml"));
        assert!(matches!(
            c.try_consume(1, 10, at(10, 0, 0)),
            Err(ConsumeError::Storage(_))
        ));
    }

    #[test]
    fn corrupt_usage_file_starts_from_zero() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("usage.toml");
        std::fs::write(&path, "garbage = [").unwrap();
        let c = UsageCounter::persistent(path);
        assert_eq!(c.peek(10, at(10, 0, 0)).remaining, 10);
    }

    #[test]
    fn persistent_counter_process_worker() {
        let Some(path) = std::env::var_os("RGAA_USAGE_TEST_WORKER") else {
            return;
        };
        let counter = UsageCounter::persistent(PathBuf::from(path));
        for _ in 0..100 {
            counter.try_consume(1, 200, at(10, 0, 0)).unwrap();
        }
    }

    #[test]
    fn concurrent_processes_do_not_lose_usage_increments() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("usage.toml");
        let executable = std::env::current_exe().unwrap();
        let mut workers = Vec::new();
        for _ in 0..2 {
            workers.push(
                std::process::Command::new(&executable)
                    .arg("--exact")
                    .arg("usage::tests::persistent_counter_process_worker")
                    .arg("--nocapture")
                    .env("RGAA_USAGE_TEST_WORKER", &path)
                    .spawn()
                    .unwrap(),
            );
        }
        for mut worker in workers {
            assert!(worker.wait().unwrap().success());
        }
        let counter = UsageCounter::persistent(path);
        assert_eq!(counter.peek(200, at(10, 0, 0)).remaining, 0);
    }
}
