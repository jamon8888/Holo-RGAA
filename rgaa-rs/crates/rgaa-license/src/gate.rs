//! The single decision point: "may this installation use `feature`, at this
//! cost, now?" Used by the Axum middleware, by MCP tool handlers (HTTP and
//! stdio) and by CLI commands, so every entry point applies the same rules.
//!
//! Order of checks, so a refused request never consumes quota:
//! 1. a present but invalid token refuses everything (401);
//! 2. the effective tier must include the feature (402);
//! 3. metered requests consume quota (429 when exhausted).

use std::sync::{Arc, RwLock};

use chrono::{DateTime, Duration, Utc};
use serde_json::{json, Value};

use crate::cache::{LicenseCache, LicenseFile, LICENSE_FILE, LICENSE_KEY_ENV, USAGE_FILE};
use crate::clock::{Clock, SystemClock};
use crate::error::LicenseError;
use crate::tiers::{Feature, Tier};
use crate::token::{LicenseClaims, LicenseVerifier};
use crate::usage::{RateInfo, UsageCounter, UsageEvent, UsageReporter};

/// Response header names. Servers must list these in CORS `expose_headers`,
/// or browser clients cannot read them.
pub mod headers {
    pub const LICENSE_TIER: &str = "x-license-tier";
    pub const LICENSE_STATUS: &str = "x-license-status";
    pub const LICENSE_GRACE_REMAINING: &str = "x-license-grace-remaining";
    pub const RATELIMIT_LIMIT: &str = "x-ratelimit-limit";
    pub const RATELIMIT_REMAINING: &str = "x-ratelimit-remaining";
    pub const RATELIMIT_RESET: &str = "x-ratelimit-reset";
    pub const RETRY_AFTER: &str = "retry-after";

    pub const EXPOSED: [&str; 7] = [
        LICENSE_TIER,
        LICENSE_STATUS,
        LICENSE_GRACE_REMAINING,
        RATELIMIT_LIMIT,
        RATELIMIT_REMAINING,
        RATELIMIT_RESET,
        RETRY_AFTER,
    ];
}

/// JSON-RPC error codes (implementation-defined server range) for MCP clients,
/// which never see the HTTP status on stdio.
pub const JSONRPC_LICENSE_REQUIRED: i32 = -32002;
pub const JSONRPC_QUOTA_EXCEEDED: i32 = -32003;

/// Set to the licence service base URL to enable online revalidation and the
/// offline grace period.
pub const LICENSE_SERVER_ENV: &str = "RGAA_LICENSE_SERVER";
/// Overrides the 7-day default grace period.
pub const GRACE_DAYS_ENV: &str = "RGAA_LICENSE_GRACE_DAYS";
pub const DEFAULT_GRACE_DAYS: i64 = 7;

/// Records when an `RGAA_LICENSE_KEY` token was last seen validated. Kept apart
/// from `license.toml` so an env override never replaces the stored licence.
const ENV_LICENSE_FILE: &str = "license-env.toml";

/// `last_validated` for a token given through `RGAA_LICENSE_KEY` when grace is
/// enabled. Without this, every process start would stamp the token "validated
/// now" and a restart would reset the offline grace window indefinitely.
///
/// - The same token already recorded: its persisted time is returned, so a
///   restart continues the existing window.
/// - A token seen for the first time (or a different one): `now` is recorded
///   and returned.
///
/// No config directory means the time cannot be recorded, and granting a
/// window that every restart would reset is refused instead.
fn env_token_last_validated(
    dir: Option<&std::path::Path>,
    token: &str,
    now: DateTime<Utc>,
) -> Result<DateTime<Utc>, LicenseError> {
    let Some(dir) = dir else {
        return Err(LicenseError::Cache(format!(
            "{LICENSE_KEY_ENV} with {LICENSE_SERVER_ENV} needs a config directory to record its validation time"
        )));
    };
    let path = dir.join(ENV_LICENSE_FILE);
    match LicenseCache::load_license(&path)? {
        Some(recorded) if recorded.token == token => Ok(recorded.last_validated),
        _ => {
            LicenseCache::store_license(
                &path,
                &LicenseFile {
                    token: token.to_string(),
                    last_validated: now,
                },
            )?;
            Ok(now)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LicenseStatus {
    /// No licence installed: Free tier.
    Unlicensed,
    Valid,
    /// Not confirmed online recently, still inside the grace period.
    Grace {
        until: DateTime<Utc>,
    },
    /// Token past `exp`: runs as Free.
    Expired {
        at: DateTime<Utc>,
    },
    /// Offline too long: runs as Free until the next successful online check.
    GraceExceeded {
        since: DateTime<Utc>,
    },
    /// A token is present but unusable: every gated request is refused.
    Invalid(LicenseError),
}

impl LicenseStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Unlicensed => "unlicensed",
            Self::Valid => "valid",
            Self::Grace { .. } => "grace",
            Self::Expired { .. } => "expired",
            Self::GraceExceeded { .. } => "grace_exceeded",
            Self::Invalid(_) => "invalid",
        }
    }
}

#[derive(Debug, Clone)]
pub struct GateConfig {
    /// Offline grace counted from `last_validated`. `None` disables it: the
    /// token's own `exp` is then the only expiry. Disabled by default while no
    /// licence service exists to revalidate against, otherwise every licence
    /// would decay to Free a week after activation.
    pub grace: Option<Duration>,
    /// Past this age without an online check the status reports `grace`.
    pub revalidate_after: Duration,
    /// Off for local stdio/CLI use, where a counter in a user-writable file is
    /// not enforceable; tier gating still applies.
    pub enforce_quota: bool,
    pub upgrade_url: Option<String>,
}

impl Default for GateConfig {
    fn default() -> Self {
        Self {
            grace: None,
            revalidate_after: Duration::hours(24),
            enforce_quota: true,
            upgrade_url: None,
        }
    }
}

#[derive(Debug, Clone)]
enum Loaded {
    None,
    Licensed {
        claims: LicenseClaims,
        last_validated: DateTime<Utc>,
    },
    Invalid(LicenseError),
    /// Test double: Team tier, no quota, no checks.
    Unrestricted,
}

/// Point-in-time view for `rgaa licence status` and `config show`.
#[derive(Debug, Clone)]
pub struct LicenseSnapshot {
    pub status: LicenseStatus,
    pub tier: Tier,
    pub claims: Option<LicenseClaims>,
    pub last_validated: Option<DateTime<Utc>>,
    pub rate: RateInfo,
}

pub struct LicenseGate {
    verifier: LicenseVerifier,
    loaded: RwLock<Loaded>,
    counter: Arc<UsageCounter>,
    clock: Arc<dyn Clock>,
    config: GateConfig,
    reporter: Option<UsageReporter>,
}

impl LicenseGate {
    /// An unlicensed (Free) gate. Install a licence with [`Self::install`].
    pub fn new(
        verifier: LicenseVerifier,
        counter: Arc<UsageCounter>,
        clock: Arc<dyn Clock>,
        config: GateConfig,
    ) -> Self {
        Self {
            verifier,
            loaded: RwLock::new(Loaded::None),
            counter,
            clock,
            config,
            reporter: None,
        }
    }

    /// Admits everything as Team with no quota. For tests of code that sits
    /// behind the gate, and for nothing else.
    pub fn unrestricted() -> Self {
        let gate = Self::new(
            LicenseVerifier::new(vec![]),
            Arc::new(UsageCounter::in_memory()),
            Arc::new(SystemClock),
            GateConfig {
                enforce_quota: false,
                ..GateConfig::default()
            },
        );
        *gate.write() = Loaded::Unrestricted;
        gate
    }

    /// Production wiring from the environment and `~/.config/rgaa/`:
    /// - token from `RGAA_LICENSE_KEY`, else `license.toml`;
    /// - verification keys from the built-in list plus `RGAA_LICENSE_PUBLIC_KEYS`;
    /// - grace enabled only when `RGAA_LICENSE_SERVER` is set
    ///   (`RGAA_LICENSE_GRACE_DAYS`, default 7);
    /// - usage persisted to `usage.toml` when a config dir exists.
    ///
    /// Never fails: an unusable licence becomes [`LicenseStatus::Invalid`]
    /// and is reported on use, not at startup.
    pub fn from_env(enforce_quota: bool, upgrade_url: Option<String>) -> Self {
        let grace = std::env::var_os(LICENSE_SERVER_ENV)
            .filter(|v| !v.is_empty())
            .map(|_| {
                let days = std::env::var(GRACE_DAYS_ENV)
                    .ok()
                    .and_then(|v| v.trim().parse::<i64>().ok())
                    .filter(|d| *d >= 0)
                    .unwrap_or(DEFAULT_GRACE_DAYS);
                Duration::days(days)
            });
        let grace_enabled = grace.is_some();
        let config = GateConfig {
            grace,
            enforce_quota,
            upgrade_url,
            ..GateConfig::default()
        };
        let dir = LicenseCache::default_dir();
        let counter = match &dir {
            Some(d) => UsageCounter::persistent(d.join(USAGE_FILE)),
            None => UsageCounter::in_memory(),
        };
        let clock: Arc<dyn Clock> = Arc::new(SystemClock);
        let now = clock.now();

        let (verifier, key_error) = match LicenseVerifier::from_env() {
            Ok(v) => (v, None),
            Err(e) => (LicenseVerifier::new(vec![]), Some(e)),
        };
        let gate = Self::new(verifier, Arc::new(counter), clock, config);

        let env_token = std::env::var(LICENSE_KEY_ENV)
            .ok()
            .filter(|t| !t.trim().is_empty());
        let file = match (env_token, &dir) {
            (Some(token), _) if grace_enabled => {
                env_token_last_validated(dir.as_deref(), &token, now).map(|last_validated| {
                    Some(LicenseFile {
                        token,
                        last_validated,
                    })
                })
            }
            (Some(token), _) => Ok(Some(LicenseFile {
                token,
                last_validated: now,
            })),
            (None, Some(d)) => LicenseCache::load_license(&d.join(LICENSE_FILE)),
            (None, None) => Ok(None),
        };
        match (key_error, file) {
            (_, Ok(None)) => {}
            (Some(e), Ok(Some(_))) | (_, Err(e)) => gate.install_error(e),
            (None, Ok(Some(f))) => gate.install(Some(&f)),
        }
        gate
    }

    pub fn with_reporter(mut self, reporter: UsageReporter) -> Self {
        self.reporter = Some(reporter);
        self
    }

    fn read(&self) -> std::sync::RwLockReadGuard<'_, Loaded> {
        self.loaded.read().unwrap_or_else(|p| p.into_inner())
    }

    fn write(&self) -> std::sync::RwLockWriteGuard<'_, Loaded> {
        self.loaded.write().unwrap_or_else(|p| p.into_inner())
    }

    /// Verifies and installs a licence (`None` uninstalls). A bad token is
    /// kept as `Invalid` rather than silently treated as Free, so the user
    /// learns their licence is broken.
    pub fn install(&self, file: Option<&LicenseFile>) {
        let loaded = match file {
            None => Loaded::None,
            Some(f) => match self.verifier.verify(&f.token) {
                Ok(claims) => Loaded::Licensed {
                    claims,
                    last_validated: f.last_validated,
                },
                Err(e) => Loaded::Invalid(e),
            },
        };
        *self.write() = loaded;
    }

    pub fn install_error(&self, error: LicenseError) {
        *self.write() = Loaded::Invalid(error);
    }

    /// Verifies a token without installing it (for `rgaa licence activate`).
    pub fn verify(&self, token: &str) -> Result<LicenseClaims, LicenseError> {
        self.verifier.verify(token)
    }

    pub fn config(&self) -> &GateConfig {
        &self.config
    }

    fn evaluate(&self, loaded: &Loaded, now: DateTime<Utc>) -> (LicenseStatus, Tier, u32) {
        match loaded {
            Loaded::None => (
                LicenseStatus::Unlicensed,
                Tier::Free,
                Tier::Free.daily_quota(),
            ),
            Loaded::Invalid(e) => (
                LicenseStatus::Invalid(e.clone()),
                Tier::Free,
                Tier::Free.daily_quota(),
            ),
            Loaded::Unrestricted => (LicenseStatus::Valid, Tier::Team, u32::MAX),
            Loaded::Licensed {
                claims,
                last_validated,
            } => {
                let free = Tier::Free.daily_quota();
                if let Some(nbf) = claims.nbf {
                    if now < nbf {
                        let e = LicenseError::NotYetValid { at: nbf };
                        return (LicenseStatus::Invalid(e), Tier::Free, free);
                    }
                }
                if now >= claims.exp {
                    return (LicenseStatus::Expired { at: claims.exp }, Tier::Free, free);
                }
                let age = now - *last_validated;
                if let Some(grace) = self.config.grace {
                    // Exactly `grace` old is still inside (inclusive bound).
                    if age > grace {
                        let since = *last_validated;
                        return (LicenseStatus::GraceExceeded { since }, Tier::Free, free);
                    }
                    if age > self.config.revalidate_after {
                        let until = *last_validated + grace;
                        return (
                            LicenseStatus::Grace { until },
                            claims.tier,
                            claims.daily_quota(),
                        );
                    }
                }
                (LicenseStatus::Valid, claims.tier, claims.daily_quota())
            }
        }
    }

    pub fn snapshot(&self) -> LicenseSnapshot {
        let now = self.clock.now();
        let loaded = self.read().clone();
        let (status, tier, limit) = self.evaluate(&loaded, now);
        let (claims, last_validated) = match &loaded {
            Loaded::Licensed {
                claims,
                last_validated,
            } => (Some(claims.clone()), Some(*last_validated)),
            _ => (None, None),
        };
        LicenseSnapshot {
            status,
            tier,
            claims,
            last_validated,
            rate: self.counter.peek(limit, now),
        }
    }

    /// The decision. `cost == 0` checks access without metering (status
    /// polls, reads of past results).
    pub fn check(&self, feature: Feature, cost: u32) -> Result<Admission, Box<Denial>> {
        let now = self.clock.now();
        let loaded = self.read().clone();
        if matches!(&loaded, Loaded::Unrestricted) {
            return Ok(Admission {
                tier: Tier::Team,
                status: LicenseStatus::Valid,
                rate: None,
                now,
            });
        }
        let (status, tier, limit) = self.evaluate(&loaded, now);
        let deny = |error: LicenseError, rate: Option<RateInfo>| {
            Box::new(Denial {
                error,
                feature,
                tier,
                status: status.clone(),
                rate,
                upgrade_url: self.config.upgrade_url.clone(),
                now,
            })
        };

        if let LicenseStatus::Invalid(e) = &status {
            return Err(deny(e.clone(), None));
        }
        if !tier.allows(feature) {
            let error = match &status {
                LicenseStatus::Expired { at } => LicenseError::Expired { at: *at },
                LicenseStatus::GraceExceeded { since } => {
                    LicenseError::GraceExceeded { since: *since }
                }
                _ => LicenseError::FeatureNotInTier {
                    feature,
                    tier,
                    required: feature.minimum_tier(),
                },
            };
            return Err(deny(error, None));
        }
        if !self.config.enforce_quota {
            return Ok(Admission {
                tier,
                status,
                rate: None,
                now,
            });
        }
        let rate = if cost == 0 {
            self.counter.peek(limit, now)
        } else {
            match self.counter.try_consume(cost, limit, now) {
                Ok(rate) => rate,
                Err(rate) => {
                    let error = LicenseError::QuotaExceeded {
                        limit,
                        reset_at: rate.reset_at,
                    };
                    return Err(deny(error, Some(rate)));
                }
            }
        };
        if cost > 0 {
            if let Some(reporter) = &self.reporter {
                let subject = match &loaded {
                    Loaded::Licensed { claims, .. } => Some(claims.sub.clone()),
                    _ => None,
                };
                reporter.record(UsageEvent {
                    feature,
                    units: cost,
                    at: now,
                    subject,
                });
            }
        }
        Ok(Admission {
            tier,
            status,
            rate: Some(rate),
            now,
        })
    }
}

fn license_headers(
    tier: Tier,
    status: &LicenseStatus,
    rate: Option<&RateInfo>,
    now: DateTime<Utc>,
) -> Vec<(&'static str, String)> {
    let mut out = vec![
        (headers::LICENSE_TIER, tier.as_str().to_string()),
        (headers::LICENSE_STATUS, status.as_str().to_string()),
    ];
    if let LicenseStatus::Grace { until } = status {
        let secs = (*until - now).num_seconds().max(0);
        out.push((headers::LICENSE_GRACE_REMAINING, secs.to_string()));
    }
    if let Some(r) = rate {
        out.push((headers::RATELIMIT_LIMIT, r.limit.to_string()));
        out.push((headers::RATELIMIT_REMAINING, r.remaining.to_string()));
        out.push((headers::RATELIMIT_RESET, r.reset_at.timestamp().to_string()));
    }
    out
}

/// A granted request.
#[derive(Debug, Clone)]
pub struct Admission {
    pub tier: Tier,
    pub status: LicenseStatus,
    /// `None` when quotas are not enforced.
    pub rate: Option<RateInfo>,
    now: DateTime<Utc>,
}

impl Admission {
    pub fn headers(&self) -> Vec<(&'static str, String)> {
        license_headers(self.tier, &self.status, self.rate.as_ref(), self.now)
    }
}

/// A refused request, with everything needed to render it as HTTP or JSON-RPC.
#[derive(Debug, Clone)]
pub struct Denial {
    pub error: LicenseError,
    pub feature: Feature,
    pub tier: Tier,
    pub status: LicenseStatus,
    pub rate: Option<RateInfo>,
    pub upgrade_url: Option<String>,
    now: DateTime<Utc>,
}

impl Denial {
    /// 401 invalid token, 402 tier/expiry, 429 quota.
    pub fn http_status(&self) -> u16 {
        match self.error {
            LicenseError::QuotaExceeded { .. } => 429,
            LicenseError::FeatureNotInTier { .. }
            | LicenseError::Expired { .. }
            | LicenseError::GraceExceeded { .. } => 402,
            _ => 401,
        }
    }

    pub fn jsonrpc_code(&self) -> i32 {
        match self.error {
            LicenseError::QuotaExceeded { .. } => JSONRPC_QUOTA_EXCEEDED,
            _ => JSONRPC_LICENSE_REQUIRED,
        }
    }

    /// Only a quota refusal has a `Retry-After`: waiting does not fix a tier.
    pub fn retry_after_secs(&self) -> Option<u64> {
        match self.error {
            LicenseError::QuotaExceeded { .. } => {
                self.rate.as_ref().map(|r| r.retry_after_secs(self.now))
            }
            _ => None,
        }
    }

    pub fn headers(&self) -> Vec<(&'static str, String)> {
        let mut out = license_headers(self.tier, &self.status, self.rate.as_ref(), self.now);
        if let Some(secs) = self.retry_after_secs() {
            out.push((headers::RETRY_AFTER, secs.to_string()));
        }
        out
    }

    /// REST error body. Extends the existing `{"error": "..."}` shape, so old
    /// clients that read only `error` keep working. Also used verbatim as
    /// JSON-RPC `error.data`.
    pub fn body(&self) -> Value {
        let mut obj = serde_json::Map::new();
        obj.insert("error".into(), json!(self.error.to_string()));
        obj.insert("code".into(), json!(self.error.code()));
        obj.insert("tier".into(), json!(self.tier));
        obj.insert("feature".into(), json!(self.feature));
        if let LicenseError::FeatureNotInTier { required, .. } = &self.error {
            obj.insert("required_tier".into(), json!(required));
        }
        if let Some(r) = &self.rate {
            obj.insert("limit".into(), json!(r.limit));
            obj.insert("remaining".into(), json!(r.remaining));
            obj.insert("reset_at".into(), json!(r.reset_at));
        }
        if let Some(secs) = self.retry_after_secs() {
            obj.insert("retry_after".into(), json!(secs));
        }
        if self.http_status() == 402 {
            if let Some(url) = &self.upgrade_url {
                obj.insert("upgrade_url".into(), json!(url));
            }
        }
        Value::Object(obj)
    }

    /// The JSON-RPC `error` object for MCP transports.
    pub fn jsonrpc_error(&self) -> Value {
        json!({
            "code": self.jsonrpc_code(),
            "message": self.error.to_string(),
            "data": self.body(),
        })
    }
}

#[cfg(test)]
mod env_token_tests {
    use super::*;
    use chrono::TimeZone;

    fn day(d: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 11, d, 12, 0, 0).unwrap()
    }

    #[test]
    fn first_sight_records_now_and_persists_it() {
        let dir = tempfile::tempdir().unwrap();
        let got = env_token_last_validated(Some(dir.path()), "tok-a", day(1)).unwrap();
        assert_eq!(got, day(1));
        let recorded = LicenseCache::load_license(&dir.path().join(ENV_LICENSE_FILE))
            .unwrap()
            .unwrap();
        assert_eq!(recorded.token, "tok-a");
        assert_eq!(recorded.last_validated, day(1));
    }

    /// The restart case: a later start with the same token must keep the
    /// original validation time, so the grace window keeps running down.
    #[test]
    fn restart_keeps_the_persisted_validation_time() {
        let dir = tempfile::tempdir().unwrap();
        env_token_last_validated(Some(dir.path()), "tok-a", day(1)).unwrap();
        let after_restart = env_token_last_validated(Some(dir.path()), "tok-a", day(6)).unwrap();
        assert_eq!(after_restart, day(1));
        // Repeated restarts do not move it either.
        let again = env_token_last_validated(Some(dir.path()), "tok-a", day(9)).unwrap();
        assert_eq!(again, day(1));
    }

    #[test]
    fn a_different_token_starts_a_new_window() {
        let dir = tempfile::tempdir().unwrap();
        env_token_last_validated(Some(dir.path()), "tok-a", day(1)).unwrap();
        let got = env_token_last_validated(Some(dir.path()), "tok-b", day(6)).unwrap();
        assert_eq!(got, day(6));
    }

    #[test]
    fn missing_config_dir_is_refused_not_granted_a_fresh_window() {
        assert!(matches!(
            env_token_last_validated(None, "tok-a", day(1)),
            Err(LicenseError::Cache(_))
        ));
    }

    #[test]
    fn corrupt_record_is_an_error_not_a_reset() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(ENV_LICENSE_FILE), "token = [broken").unwrap();
        assert!(matches!(
            env_token_last_validated(Some(dir.path()), "tok-a", day(1)),
            Err(LicenseError::Cache(_))
        ));
    }
}
