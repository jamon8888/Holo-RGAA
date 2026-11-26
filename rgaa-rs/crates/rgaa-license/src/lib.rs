//! Licence validation, tier enforcement and usage metering (#87).
//!
//! - [`token`]: PASETO v4.public licences, verified offline.
//! - [`cache`]: `license.toml` / `usage.toml` under `$XDG_CONFIG_HOME/rgaa/`.
//! - [`gate`]: [`LicenseGate::check`], the one decision every entry point uses.
//! - [`usage`]: the daily quota counter and non-blocking usage reporting.
//! - [`middleware`] (feature `axum`): the HTTP layer.
//!
//! Outbound LLM provider keys (`MYIA_API_KEY`, `RGAA_LLM_API_KEY`) are a
//! different concern and are not handled here.

pub mod cache;
pub mod clock;
pub mod error;
pub mod gate;
#[cfg(feature = "axum")]
pub mod middleware;
pub mod tiers;
pub mod token;
pub mod usage;

pub use cache::{LicenseCache, LicenseFile, UsageFile};
pub use clock::{Clock, ManualClock, SystemClock};
pub use error::LicenseError;
pub use gate::{Admission, Denial, GateConfig, LicenseGate, LicenseSnapshot, LicenseStatus};
pub use tiers::{Feature, Tier};
pub use token::{LicenseClaims, LicenseIssuer, LicenseVerifier};
pub use usage::{
    ConsumeError, NoopSink, RateInfo, ReporterConfig, UsageCounter, UsageEvent, UsageReporter,
    UsageSink,
};
