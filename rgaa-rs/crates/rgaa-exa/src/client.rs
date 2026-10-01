//! HTTP client for the Exa API.

use crate::config::ExaConfig;
use crate::error::{ExaError, Result};
use crate::search::{SearchRequest, SearchResponse};
use std::time::Duration;
use tracing::{debug, warn};

/// How many times a single search is attempted before giving up. Covers the
/// 429s an audit hits when it fans out over many criteria at once.
const MAX_ATTEMPTS: u32 = 3;

/// Base backoff between attempts; doubled each retry.
const BASE_BACKOFF: Duration = Duration::from_millis(500);

/// A configured Exa API client.
///
/// Cheap to clone (the inner `reqwest::Client` is an `Arc` internally), so
/// share one across an audit rather than building one per criterion.
#[derive(Clone)]
pub struct ExaClient {
    http: reqwest::Client,
    config: ExaConfig,
}

impl ExaClient {
    /// Builds a client from an explicit config.
    ///
    /// # Errors
    /// Returns [`ExaError::Transport`] when the TLS backend or HTTP client
    /// cannot be constructed. Never panics.
    pub fn new(config: ExaConfig) -> Result<Self> {
        let http = reqwest::Client::builder()
            .timeout(config.timeout)
            .build()
            .map_err(|e| ExaError::Transport(format!("HTTP client init failed: {e}")))?;
        Ok(Self { http, config })
    }

    /// Builds a client from `EXA_API_KEY` and friends.
    ///
    /// # Errors
    /// Propagates [`ExaError::MissingApiKey`] from [`ExaConfig::from_env`] and
    /// any client-construction failure from [`Self::new`].
    pub fn from_env() -> Result<Self> {
        Self::new(ExaConfig::from_env()?)
    }

    /// Runs one `POST /search`, retrying transient failures (429 and 5xx) with
    /// exponential backoff.
    ///
    /// # Errors
    /// [`ExaError::Api`] for a non-retryable non-2xx status,
    /// [`ExaError::RateLimited`] when every attempt was throttled,
    /// [`ExaError::Transport`] / [`ExaError::Decode`] otherwise.
    #[tracing::instrument(name = "exa.search", skip(self, request), fields(query = %request.query))]
    pub async fn search(&self, request: &SearchRequest) -> Result<SearchResponse> {
        let url = format!("{}/search", self.config.base_url);
        let mut throttled = false;

        for attempt in 1..=MAX_ATTEMPTS {
            let response = self
                .http
                .post(&url)
                .header("x-api-key", &self.config.api_key)
                .json(request)
                .send()
                .await;

            let response = match response {
                Ok(r) => r,
                Err(e) if attempt < MAX_ATTEMPTS => {
                    warn!(attempt, error = %e, "Exa request failed, retrying");
                    tokio::time::sleep(backoff(attempt)).await;
                    continue;
                }
                Err(e) => return Err(e.into()),
            };

            let status = response.status();
            if status.is_success() {
                let parsed: SearchResponse = response.json().await?;
                debug!(
                    attempt,
                    results = parsed.results.len(),
                    "Exa search succeeded"
                );
                return Ok(parsed);
            }

            let retryable = status.as_u16() == 429 || status.is_server_error();
            if status.as_u16() == 429 {
                throttled = true;
            }

            let body = response.text().await.unwrap_or_default();
            if retryable && attempt < MAX_ATTEMPTS {
                warn!(
                    attempt,
                    status = status.as_u16(),
                    "Exa retryable error, backing off"
                );
                tokio::time::sleep(backoff(attempt)).await;
                continue;
            }

            if throttled {
                return Err(ExaError::RateLimited {
                    attempts: MAX_ATTEMPTS,
                });
            }
            return Err(ExaError::Api {
                status: status.as_u16(),
                body: truncate(&body, 500),
            });
        }

        Err(ExaError::RateLimited {
            attempts: MAX_ATTEMPTS,
        })
    }

    /// The configuration this client was built with (API key redacted in
    /// `Debug`).
    #[must_use]
    pub fn config(&self) -> &ExaConfig {
        &self.config
    }
}

impl std::fmt::Debug for ExaClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExaClient")
            .field("config", &self.config)
            .finish()
    }
}

fn backoff(attempt: u32) -> Duration {
    BASE_BACKOFF * 2_u32.pow(attempt - 1)
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        return s.to_string();
    }
    let mut end = max;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}…", &s[..end])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_builds_without_panicking() {
        let client = ExaClient::new(ExaConfig::new("k")).expect("client builds");
        assert_eq!(client.config().base_url, crate::config::DEFAULT_BASE_URL);
    }

    #[test]
    fn debug_never_leaks_the_key() {
        let client = ExaClient::new(ExaConfig::new("sk-secret-value")).unwrap();
        assert!(!format!("{client:?}").contains("sk-secret-value"));
    }

    #[test]
    fn backoff_doubles() {
        assert_eq!(backoff(1), Duration::from_millis(500));
        assert_eq!(backoff(2), Duration::from_millis(1000));
        assert_eq!(backoff(3), Duration::from_millis(2000));
    }

    #[test]
    fn truncate_respects_char_boundaries() {
        let s = "критère".repeat(200);
        let cut = truncate(&s, 500);
        assert!(cut.ends_with('…'));
        assert!(cut.len() <= 503);
    }

    #[test]
    fn truncate_leaves_short_bodies_alone() {
        assert_eq!(truncate("short", 500), "short");
    }
}
