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

/// A non-empty value enables the offline grace period. It is not read and no
/// online revalidation is currently performed.
pub const LICENSE_SERVER_ENV: &str = "RGAA_LICENSE_SERVER";
/// Overrides the 7-day default grace period.
pub const GRACE_DAYS_ENV: &str = "RGAA_LICENSE_GRACE_DAYS";
pub const DEFAULT_GRACE_DAYS: i64 = 7;
const ENV_LICENSE_BASELINE_FILE: &str = "environment-license.toml";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LicenseStatus {
    /// No licence installed: Free tier.
    Unlicensed,
    Valid,
    /// The local activation baseline is old, but still inside the grace period.
    Grace {
        until: DateTime<Utc>,
    },
    /// Token past `exp`: runs as Free.
    Expired {
        at: DateTime<Utc>,
    },
    /// The local activation baseline exceeded grace: runs as Free.
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
    /// Offline grace counted from the local activation baseline. `None`
    /// disables it, leaving the token's own `exp` as the only expiry. Disabled
    /// by default because no online revalidation is currently performed.
    pub grace: Option<Duration>,
    /// Past this age the status reports `grace`; no online check is performed.
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
    /// - grace enabled only when `RGAA_LICENSE_SERVER` is non-empty
    ///   (`RGAA_LICENSE_GRACE_DAYS`, default 7); this does not perform online checks;
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
        let file = match env_token {
            Some(token) => {
                let cached = dir.as_ref().and_then(|d| {
                    match LicenseCache::load_license(&d.join(ENV_LICENSE_BASELINE_FILE)) {
                        Ok(file) => file,
                        Err(e) => {
                            tracing::warn!(error = %e, "ignoring unreadable environment licence baseline");
                            None
                        }
                    }
                });
                let has_env_baseline = cached.as_ref().is_some_and(|f| f.token == token);
                let cached = cached.filter(|file| file.token == token).or_else(|| {
                    dir.as_ref().and_then(|d| {
                        LicenseCache::load_license(&d.join(LICENSE_FILE))
                            .ok()
                            .flatten()
                            .filter(|file| file.token == token)
                    })
                });
                let file = environment_license_file(token, cached, now);
                if !has_env_baseline {
                    if let Some(d) = &dir {
                        if let Err(e) =
                            LicenseCache::store_license(&d.join(ENV_LICENSE_BASELINE_FILE), &file)
                        {
                            tracing::warn!(error = %e, "failed to persist licence baseline");
                        }
                    }
                }
                Ok(Some(file))
            }
            None => match &dir {
                Some(d) => LicenseCache::load_license(&d.join(LICENSE_FILE)),
                None => Ok(None),
            },
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

    fn evaluate(&self, now: DateTime<Utc>) -> (LicenseStatus, Tier, u32) {
        match &*self.read() {
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
        let (status, tier, limit) = self.evaluate(now);
        let (claims, last_validated) = match &*self.read() {
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
        if matches!(&*self.read(), Loaded::Unrestricted) {
            return Ok(Admission {
                tier: Tier::Team,
                status: LicenseStatus::Valid,
                rate: None,
                now,
            });
        }
        let (status, tier, limit) = self.evaluate(now);
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
                let subject = match &*self.read() {
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

fn environment_license_file(
    token: String,
    cached: Option<LicenseFile>,
    now: DateTime<Utc>,
) -> LicenseFile {
    cached
        .filter(|file| file.token == token)
        .unwrap_or(LicenseFile {
            token,
            last_validated: now,
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn environment_token_keeps_its_saved_grace_baseline() {
        let saved_at = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        let started_at = Utc.with_ymd_and_hms(2026, 1, 9, 0, 0, 0).unwrap();
        let file = environment_license_file(
            "same-token".into(),
            Some(LicenseFile {
                token: "same-token".into(),
                last_validated: saved_at,
            }),
            started_at,
        );
        assert_eq!(file.last_validated, saved_at);
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
