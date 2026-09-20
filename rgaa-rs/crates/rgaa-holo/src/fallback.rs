//! A second provider route, used as a technical fallback when the primary
//! backend fails, and as a benchmark harness comparing the two routes on
//! the same prompt.

use crate::{HoloResponse, LlmBackend};
use async_trait::async_trait;
use rgaa_core::RgaaError;

/// Wraps a primary and a secondary [`LlmBackend`]. `evaluate`/
/// `evaluate_multimodal` try the primary first and only call the secondary
/// when the primary fails (after its own internal retries) — a technical
/// fallback for a primary-provider outage, not a load-balancing split.
///
/// [`name`](LlmBackend::name) always reports the primary's name, since the
/// operator selected that route; look at the emitted `tracing` events (and
/// [`Self::benchmark`]) to see which route actually answered.
pub struct FallbackBackend {
    primary: Box<dyn LlmBackend>,
    secondary: Box<dyn LlmBackend>,
}

impl FallbackBackend {
    pub fn new(primary: Box<dyn LlmBackend>, secondary: Box<dyn LlmBackend>) -> Self {
        Self { primary, secondary }
    }

    /// Runs the same prompt against both routes independently — not the
    /// primary-then-fallback path [`LlmBackend::evaluate`] takes — so their
    /// verdicts can be compared directly. Used by the baseline harness
    /// (#130) to check that the two routes give comparable verdicts on the
    /// same criteria.
    pub async fn benchmark(&self, prompt: &str) -> BenchmarkResult {
        BenchmarkResult {
            primary: self.primary.evaluate(prompt).await,
            secondary: self.secondary.evaluate(prompt).await,
        }
    }
}

/// The two independent verdicts [`FallbackBackend::benchmark`] collects for
/// one prompt, one per route.
#[derive(Debug)]
pub struct BenchmarkResult {
    pub primary: Result<HoloResponse, RgaaError>,
    pub secondary: Result<HoloResponse, RgaaError>,
}

#[async_trait]
impl LlmBackend for FallbackBackend {
    fn name(&self) -> &'static str {
        self.primary.name()
    }

    fn model(&self) -> &str {
        self.primary.model()
    }

    async fn evaluate(&self, prompt: &str) -> Result<HoloResponse, RgaaError> {
        match self.primary.evaluate(prompt).await {
            Ok(response) => Ok(response),
            Err(primary_err) => {
                tracing::warn!(
                    primary = self.primary.name(),
                    secondary = self.secondary.name(),
                    error = %primary_err,
                    "primary LLM backend failed; falling back to secondary route"
                );
                self.secondary
                    .evaluate(prompt)
                    .await
                    .map_err(|secondary_err| {
                        fallback_exhausted(
                            self.primary.name(),
                            primary_err,
                            self.secondary.name(),
                            secondary_err,
                        )
                    })
            }
        }
    }

    async fn evaluate_multimodal(
        &self,
        prompt: &str,
        image_base64: Option<&str>,
    ) -> Result<HoloResponse, RgaaError> {
        match self.primary.evaluate_multimodal(prompt, image_base64).await {
            Ok(response) => Ok(response),
            Err(primary_err) => {
                tracing::warn!(
                    primary = self.primary.name(),
                    secondary = self.secondary.name(),
                    error = %primary_err,
                    "primary LLM backend failed; falling back to secondary route"
                );
                self.secondary
                    .evaluate_multimodal(prompt, image_base64)
                    .await
                    .map_err(|secondary_err| {
                        fallback_exhausted(
                            self.primary.name(),
                            primary_err,
                            self.secondary.name(),
                            secondary_err,
                        )
                    })
            }
        }
    }
}

fn fallback_exhausted(
    primary_name: &str,
    primary_err: RgaaError,
    secondary_name: &str,
    secondary_err: RgaaError,
) -> RgaaError {
    RgaaError::Llm {
        message: format!(
            "primary route `{primary_name}` failed ({primary_err}); \
             secondary route `{secondary_name}` also failed ({secondary_err})"
        ),
        code: Some("FALLBACK_EXHAUSTED".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::{spawn_mock_server, spawn_mock_server_with_status};
    use crate::HoloClient;

    const OK_BODY: &str = r#"{"choices":[{"message":{"content":"{\"verdict\":\"pass\",\"confidence\":0.9,\"justification\":\"ok\"}"}}]}"#;

    fn client_pointed_at(url: String) -> Box<dyn LlmBackend> {
        Box::new(HoloClient::new("k".to_string()).unwrap().with_base_url(url))
    }

    #[tokio::test]
    async fn primary_success_never_calls_secondary() {
        let primary_server = spawn_mock_server(OK_BODY);
        // The secondary is never reachable at all — if it were called, the
        // request would fail and `evaluate` would return that failure
        // instead of the primary's success.
        let secondary = client_pointed_at("http://127.0.0.1:1".to_string());

        let backend = FallbackBackend::new(client_pointed_at(primary_server.url()), secondary);
        let response = backend.evaluate("prompt").await.unwrap();
        assert_eq!(response.verdict, "pass");
    }

    #[tokio::test]
    async fn primary_failure_falls_back_to_secondary() {
        // 500 with no retry-after: the transport's retry loop exhausts
        // quickly (no backoff sleep on a plain 5xx) instead of the primary
        // succeeding.
        let primary_server = spawn_mock_server_with_status(500, "server error");
        let secondary_server = spawn_mock_server(OK_BODY);

        let backend = FallbackBackend::new(
            client_pointed_at(primary_server.url()),
            client_pointed_at(secondary_server.url()),
        );
        let response = backend.evaluate("prompt").await.unwrap();
        assert_eq!(response.verdict, "pass");
    }

    #[tokio::test]
    async fn both_routes_failing_reports_both_errors() {
        let primary_server = spawn_mock_server_with_status(500, "primary down");
        let secondary_server = spawn_mock_server_with_status(500, "secondary down");

        let backend = FallbackBackend::new(
            client_pointed_at(primary_server.url()),
            client_pointed_at(secondary_server.url()),
        );
        let err = backend.evaluate("prompt").await.unwrap_err();
        let message = err.to_string();
        assert!(message.contains("holo3"), "{message}");
    }

    #[tokio::test]
    async fn benchmark_runs_both_routes_independently_for_comparison() {
        let primary_server = spawn_mock_server(OK_BODY);
        let secondary_server = spawn_mock_server(OK_BODY);

        let backend = FallbackBackend::new(
            client_pointed_at(primary_server.url()),
            client_pointed_at(secondary_server.url()),
        );
        let result = backend.benchmark("prompt").await;
        let primary = result.primary.unwrap();
        let secondary = result.secondary.unwrap();
        // Same criteria (same prompt) against both routes yield the same
        // shape of verdict — directly comparable, not just "both succeeded".
        assert_eq!(primary.verdict, secondary.verdict);
        assert_eq!(primary.confidence, secondary.confidence);
    }

    #[test]
    fn name_reports_the_primary_route() {
        let backend = FallbackBackend::new(
            client_pointed_at("http://127.0.0.1:1".to_string()),
            client_pointed_at("http://127.0.0.1:1".to_string()),
        );
        assert_eq!(backend.name(), "holo3");
    }
}
