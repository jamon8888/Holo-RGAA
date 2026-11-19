use chrono::{DateTime, Utc};
use thiserror::Error;

use crate::tiers::{Feature, Tier};

/// Everything that can refuse a licensed operation, or fail while loading one.
///
/// Kept local to this crate so a quota refusal is never confused with
/// [`rgaa_core::RgaaError::RateLimited`], which means an upstream LLM provider
/// throttled us. It converts into [`rgaa_core::RgaaError::License`].
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum LicenseError {
    #[error("licence token is malformed: {0}")]
    Malformed(String),

    #[error("licence signature is invalid")]
    InvalidSignature,

    #[error("no licence verification key is configured")]
    NoVerificationKey,

    #[error("licence is not valid before {at}")]
    NotYetValid { at: DateTime<Utc> },

    #[error("licence expired at {at}")]
    Expired { at: DateTime<Utc> },

    #[error("offline grace period exceeded: last online check was {since}")]
    GraceExceeded { since: DateTime<Utc> },

    #[error("`{feature}` requires the {required} tier (current tier: {tier})")]
    FeatureNotInTier {
        feature: Feature,
        tier: Tier,
        required: Tier,
    },

    #[error("daily quota of {limit} reached; resets at {reset_at}")]
    QuotaExceeded { limit: u32, reset_at: DateTime<Utc> },

    #[error("licence cache error: {0}")]
    Cache(String),

    #[error("usage sync error: {0}")]
    Sync(String),
}

impl LicenseError {
    /// Stable machine-readable code, sent as `code` in REST error bodies and
    /// in JSON-RPC `error.data`.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Malformed(_)
            | Self::InvalidSignature
            | Self::NoVerificationKey
            | Self::NotYetValid { .. } => "license_invalid",
            Self::Expired { .. } | Self::GraceExceeded { .. } => "license_expired",
            Self::FeatureNotInTier { .. } => "feature_not_in_tier",
            Self::QuotaExceeded { .. } => "quota_exceeded",
            Self::Cache(_) => "license_cache",
            Self::Sync(_) => "usage_sync",
        }
    }
}

impl From<LicenseError> for rgaa_core::RgaaError {
    fn from(e: LicenseError) -> Self {
        rgaa_core::RgaaError::License(e.to_string())
    }
}
