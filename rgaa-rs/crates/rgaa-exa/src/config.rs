use crate::error::{ExaError, Result};
use std::time::Duration;

/// Default Exa API root. Override with `EXA_BASE_URL` only for a proxy.
pub const DEFAULT_BASE_URL: &str = "https://api.exa.ai";

/// Default per-request timeout. Exa's `auto` search type answers well inside
/// this; deeper types are not used by this crate.
pub const DEFAULT_TIMEOUT_SECS: u64 = 30;

/// Credentials and transport settings for [`crate::ExaClient`].
///
/// The API key is never printed: [`Debug`] is implemented by hand.
#[derive(Clone)]
pub struct ExaConfig {
    pub api_key: String,
    pub base_url: String,
    pub timeout: Duration,
}

impl ExaConfig {
    /// Builds a config from an explicit key, leaving the other fields at their
    /// defaults.
    #[must_use]
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: DEFAULT_BASE_URL.to_string(),
            timeout: Duration::from_secs(DEFAULT_TIMEOUT_SECS),
        }
    }

    /// Reads `EXA_API_KEY` (required), `EXA_BASE_URL` and `EXA_TIMEOUT_SECS`
    /// (both optional) from the process environment.
    ///
    /// # Errors
    /// Returns [`ExaError::MissingApiKey`] when `EXA_API_KEY` is unset or blank.
    pub fn from_env() -> Result<Self> {
        Self::from_env_with(|k| std::env::var(k).ok())
    }

    /// [`Self::from_env`] with an injectable lookup, so tests never touch the
    /// real process environment.
    ///
    /// # Errors
    /// Returns [`ExaError::MissingApiKey`] when the lookup yields no non-empty
    /// `EXA_API_KEY`.
    pub fn from_env_with<F>(get: F) -> Result<Self>
    where
        F: Fn(&str) -> Option<String>,
    {
        let api_key = get("EXA_API_KEY")
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
            .ok_or(ExaError::MissingApiKey)?;

        let base_url = get("EXA_BASE_URL")
            .map(|v| v.trim().trim_end_matches('/').to_string())
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| DEFAULT_BASE_URL.to_string());

        let timeout = get("EXA_TIMEOUT_SECS")
            .and_then(|v| v.trim().parse::<u64>().ok())
            .map_or(
                Duration::from_secs(DEFAULT_TIMEOUT_SECS),
                Duration::from_secs,
            );

        Ok(Self {
            api_key,
            base_url,
            timeout,
        })
    }
}

impl std::fmt::Debug for ExaConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExaConfig")
            .field("api_key", &"<redacted>")
            .field("base_url", &self.base_url)
            .field("timeout", &self.timeout)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env_of(pairs: &[(&'static str, &'static str)]) -> impl Fn(&str) -> Option<String> {
        let owned: Vec<(String, String)> = pairs
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect();
        move |k| {
            owned
                .iter()
                .find(|(key, _)| key == k)
                .map(|(_, v)| v.clone())
        }
    }

    #[test]
    fn from_env_requires_a_key() {
        let err = ExaConfig::from_env_with(env_of(&[])).unwrap_err();
        assert!(matches!(err, ExaError::MissingApiKey));
    }

    #[test]
    fn blank_key_is_treated_as_missing() {
        let err = ExaConfig::from_env_with(env_of(&[("EXA_API_KEY", "   ")])).unwrap_err();
        assert!(matches!(err, ExaError::MissingApiKey));
    }

    #[test]
    fn defaults_fill_in_around_the_key() {
        let cfg = ExaConfig::from_env_with(env_of(&[("EXA_API_KEY", "k")])).unwrap();
        assert_eq!(cfg.base_url, DEFAULT_BASE_URL);
        assert_eq!(cfg.timeout, Duration::from_secs(DEFAULT_TIMEOUT_SECS));
    }

    #[test]
    fn overrides_are_read_and_normalised() {
        let cfg = ExaConfig::from_env_with(env_of(&[
            ("EXA_API_KEY", " k "),
            ("EXA_BASE_URL", "https://proxy.internal/exa/"),
            ("EXA_TIMEOUT_SECS", "12"),
        ]))
        .unwrap();
        assert_eq!(cfg.api_key, "k");
        assert_eq!(cfg.base_url, "https://proxy.internal/exa");
        assert_eq!(cfg.timeout, Duration::from_secs(12));
    }

    #[test]
    fn debug_never_leaks_the_key() {
        let dbg = format!("{:?}", ExaConfig::new("sk-secret-value"));
        assert!(!dbg.contains("sk-secret-value"), "{dbg}");
        assert!(dbg.contains("<redacted>"));
    }
}
