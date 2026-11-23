//! Daily quota counting and the asynchronous usage report to the licence service.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};
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

/// Units consumed today (UTC), for the whole installation.
///
/// Within one process, `persist_lock` and `state` serialise every admission, so
/// concurrent requests never admit more than the limit (lock order:
/// `persist_lock`, then `state`). Optionally mirrored to `usage.toml` so a
/// restart does not reset the quota. When persistent, each admission first
/// re-reads the file and adopts a higher count for the same day, so a second
/// counter sharing the file sees the first one's consumption and cannot
/// overwrite it with a smaller count.
///
/// Limit: this is not atomic across processes. Two processes can read the same
/// count between their read and their write and both admit; the later write
/// then wins. Closing that needs an advisory file lock held across the whole
/// read-modify-write, which is deliberately out of scope here.
pub struct UsageCounter {
    state: Mutex<UsageFile>,
    persist: Option<PathBuf>,
    persist_lock: Mutex<()>,
}

impl UsageCounter {
    pub fn in_memory() -> Self {
        Self {
            state: Mutex::new(UsageFile {
                day: NaiveDate::MIN,
                count: 0,
            }),
            persist: None,
            persist_lock: Mutex::new(()),
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
            persist_lock: Mutex::new(()),
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, UsageFile> {
        self.state.lock().unwrap_or_else(|p| p.into_inner())
    }

    /// Advances the counter to a newer UTC day, never back to an older one.
    /// `now` is captured by the caller before it takes the locks, so a request
    /// delayed across midnight must not reset the new day's consumption with
    /// yesterday's timestamp. Returns the effective day to use for this call.
    fn roll(state: &mut UsageFile, today: NaiveDate) -> NaiveDate {
        if state.day < today {
            *state = UsageFile {
                day: today,
                count: 0,
            };
        }
        state.day
    }

    /// Adopts a higher count for today from the shared file, if another counter
    /// (or process) has written one. Never lowers the count, so a stale file
    /// cannot undo consumption. A missing, unreadable or other-day file is
    /// ignored: the in-memory state stays authoritative.
    ///
    /// Must be called with `persist_lock` held (see the lock order on
    /// [`UsageCounter`]).
    fn refresh_from_disk(&self, state: &mut UsageFile, today: NaiveDate) {
        let Some(path) = &self.persist else { return };
        if let Ok(Some(on_disk)) = LicenseCache::load_usage(path) {
            if on_disk.day == today && on_disk.count > state.count {
                state.count = on_disk.count;
            }
        }
    }

    /// Current position without consuming anything.
    pub fn peek(&self, limit: u32, now: DateTime<Utc>) -> RateInfo {
        let today = now.date_naive();
        let _file = self.persist_lock();
        let mut state = self.lock();
        let day = Self::roll(&mut state, today);
        self.refresh_from_disk(&mut state, day);
        RateInfo {
            limit,
            remaining: limit.saturating_sub(state.count),
            reset_at: next_midnight(day),
        }
    }

    /// Consumes `cost` units if all of them fit. A request that does not fit
    /// consumes nothing (a batch of 20 with 5 left is refused whole).
    pub fn try_consume(
        &self,
        cost: u32,
        limit: u32,
        now: DateTime<Utc>,
    ) -> Result<RateInfo, RateInfo> {
        let today = now.date_naive();
        // Lock order everywhere: `persist_lock`, then `state`.
        let file = self.persist_lock();
        let (after, snapshot, reset_at) = {
            let mut state = self.lock();
            let day = Self::roll(&mut state, today);
            self.refresh_from_disk(&mut state, day);
            let reset_at = next_midnight(day);
            let remaining = limit.saturating_sub(state.count);
            if cost > remaining {
                return Err(RateInfo {
                    limit,
                    remaining,
                    reset_at,
                });
            }
            state.count += cost;
            (limit.saturating_sub(state.count), *state, reset_at)
        };
        self.write_snapshot(&snapshot);
        drop(file);
        Ok(RateInfo {
            limit,
            remaining: after,
            reset_at,
        })
    }

    fn persist_lock(&self) -> std::sync::MutexGuard<'_, ()> {
        self.persist_lock.lock().unwrap_or_else(|p| p.into_inner())
    }

    /// Writes the snapshot to `usage.toml` through a per-write unique temp file
    /// (see `LicenseCache::store_usage`). Called with `persist_lock` held.
    fn write_snapshot(&self, snapshot: &UsageFile) {
        let Some(path) = &self.persist else { return };
        if let Err(e) = LicenseCache::store_usage(path, snapshot) {
            tracing::warn!(error = %e, "failed to persist usage counter");
        }
    }
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
        let refused = c.try_consume(1, 10, at(12, 0, 0)).unwrap_err();
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

    /// `now` is captured before the locks are taken. A request delayed across
    /// midnight must not rewind the new day's exhausted quota with yesterday's
    /// timestamp — neither by consuming nor by reporting the remaining quota.
    #[test]
    fn a_delayed_request_cannot_rewind_the_day() {
        let c = UsageCounter::in_memory();
        let next_day = Utc.with_ymd_and_hms(2026, 11, 20, 12, 0, 0).unwrap();
        for _ in 0..10 {
            c.try_consume(1, 10, next_day).unwrap();
        }
        let stale = at(23, 59, 59); // still 19 November, captured before midnight
        assert!(
            c.try_consume(1, 10, stale).is_err(),
            "un timestamp périmé ne peut pas rouvrir le quota du jour suivant"
        );
        let info = c.peek(10, stale);
        assert_eq!(info.remaining, 0, "le solde du jour courant reste épuisé");
        assert_eq!(
            info.reset_at,
            Utc.with_ymd_and_hms(2026, 11, 21, 0, 0, 0).unwrap()
        );
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
    fn corrupt_usage_file_starts_from_zero() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("usage.toml");
        std::fs::write(&path, "garbage = [").unwrap();
        let c = UsageCounter::persistent(path);
        assert_eq!(c.peek(10, at(10, 0, 0)).remaining, 10);
    }

    /// Two counters on one file: the second, created before the first consumed,
    /// must see that consumption instead of writing its stale count over it.
    #[test]
    fn two_counters_sharing_a_file_do_not_clobber_each_other() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("usage.toml");
        let first = UsageCounter::persistent(path.clone());
        let second = UsageCounter::persistent(path.clone());
        for _ in 0..4 {
            first.try_consume(1, 10, at(10, 0, 0)).unwrap();
        }
        let info = second.try_consume(1, 10, at(10, 0, 0)).unwrap();
        assert_eq!(
            info.remaining, 5,
            "second counter must include the first's 4"
        );
        let on_disk = LicenseCache::load_usage(&path).unwrap().unwrap();
        assert_eq!(on_disk.count, 5);
    }

    /// The limit holds across counters: the second cannot admit beyond what the
    /// first left, even though its own in-memory state started at zero.
    #[test]
    fn shared_file_enforces_the_limit_across_counters() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("usage.toml");
        let first = UsageCounter::persistent(path.clone());
        let second = UsageCounter::persistent(path);
        first.try_consume(6, 10, at(10, 0, 0)).unwrap();
        second.try_consume(4, 10, at(10, 0, 0)).unwrap();
        assert!(second.try_consume(1, 10, at(10, 0, 0)).is_err());
        assert!(first.try_consume(1, 10, at(10, 0, 0)).is_err());
    }
}
