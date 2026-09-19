use crate::backend::LlmBackend;
use crate::transport::ChatTransport;
pub use crate::transport::HoloResponse;
use async_trait::async_trait;
use rgaa_core::RgaaError;
use std::time::Duration;

const API_URL: &str = "https://api.hcompany.ai/v1/chat/completions";
const MODEL: &str = "holo3-1-35b-a3b";
const TIMEOUT: Duration = Duration::from_secs(30);

/// Remote Holo3 backend (H Company hosted API).
#[derive(Debug, Clone)]
pub struct HoloClient {
    transport: ChatTransport,
}

impl HoloClient {
    /// Creates a new HoloClient with the given API key.
    ///
    /// # Errors
    ///
    /// Returns `Err(RgaaError::Holo3)` if the HTTP client cannot be built
    /// (e.g., TLS initialization failure).
    pub fn new(api_key: String) -> Result<Self, RgaaError> {
        let transport = ChatTransport::new("holo3", API_URL, MODEL, Some(api_key), TIMEOUT)
            .map_err(|e| RgaaError::Holo3(e.to_string()))?;
        Ok(Self { transport })
    }

    /// Override the API base URL. Primarily used by tests against a mock server.
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.transport.endpoint = base_url.into();
        self
    }

    /// Sends a text-only evaluation prompt to the Holo3 API, retrying on
    /// transient failures (HTTP 429, network errors).
    pub async fn evaluate(&self, prompt: &str) -> Result<HoloResponse, RgaaError> {
        self.transport
            .complete(ChatTransport::text_messages(prompt))
            .await
    }

    /// Evaluate a prompt with an optional base64 PNG screenshot.
    ///
    /// # Errors
    ///
    /// Returns an error if `image_base64` is not valid base64, or if all retry
    /// attempts fail.
    pub async fn evaluate_multimodal(
        &self,
        prompt: &str,
        image_base64: Option<&str>,
    ) -> Result<HoloResponse, RgaaError> {
        let messages = ChatTransport::multimodal_messages(prompt, image_base64)?;
        self.transport.complete(messages).await
    }

    /// Attempts to extract a `HoloResponse` from raw text (OpenAI envelope or
    /// bare body; direct JSON, ```` ```json ```` block, or regex).
    pub fn extract_json(text: &str) -> Option<HoloResponse> {
        crate::transport::extract_json(text)
    }
}

#[async_trait]
impl LlmBackend for HoloClient {
    fn name(&self) -> &'static str {
        "holo3"
    }

    fn model(&self) -> &str {
        &self.transport.model
    }

    async fn evaluate(&self, prompt: &str) -> Result<HoloResponse, RgaaError> {
        HoloClient::evaluate(self, prompt).await
    }

    async fn evaluate_multimodal(
        &self,
        prompt: &str,
        image_base64: Option<&str>,
    ) -> Result<HoloResponse, RgaaError> {
        HoloClient::evaluate_multimodal(self, prompt, image_base64).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::spawn_mock_server;

    #[test]
    fn test_extract_json_direct() {
        let json = r#"{"verdict": "pass", "confidence": 0.95, "justification": "Test"}"#;
        let r = HoloClient::extract_json(json).unwrap();
        assert_eq!(r.verdict, "pass");
        assert_eq!(r.confidence, 0.95);
    }

    #[test]
    fn debug_redacts_api_key() {
        let client = HoloClient::new("super-secret".to_string()).unwrap();
        let dbg = format!("{client:?}");
        assert!(!dbg.contains("super-secret"));
        assert!(dbg.contains("[redacted]"));
    }

    #[tokio::test]
    async fn test_evaluate_parses_via_mock_server() {
        let server =
            spawn_mock_server(r#"{"verdict":"pass","confidence":0.9,"justification":"ok"}"#);
        let client = HoloClient::new("test-key".to_string())
            .unwrap()
            .with_base_url(server.url());

        let r = client.evaluate("prompt").await.unwrap();
        assert_eq!(r.verdict, "pass");
        assert_eq!(r.confidence, 0.9);
        let req = server.last_request();
        assert!(
            req.to_ascii_lowercase()
                .contains("authorization: bearer test-key"),
            "{req}"
        );
        assert!(req.contains(MODEL));
    }

    #[tokio::test]
    async fn test_evaluate_multimodal_text_only() {
        let server =
            spawn_mock_server(r#"{"verdict":"pass","confidence":0.9,"justification":"ok"}"#);
        let client = HoloClient::new("test-key".to_string())
            .unwrap()
            .with_base_url(server.url());
        let r = client.evaluate_multimodal("prompt", None).await.unwrap();
        assert_eq!(r.verdict, "pass");
    }

    #[tokio::test]
    async fn test_evaluate_multimodal_invalid_base64() {
        let client = HoloClient::new("test-key".to_string()).unwrap();
        let err = client
            .evaluate_multimodal("test prompt", Some("not-valid-base64!!!"))
            .await
            .unwrap_err();
        assert!(err.to_string().contains("base64"));
    }

    #[tokio::test]
    async fn test_evaluate_multimodal_with_image() {
        let server =
            spawn_mock_server(r#"{"verdict":"fail","confidence":0.85,"justification":"no alt"}"#);
        let client = HoloClient::new("test-key".to_string())
            .unwrap()
            .with_base_url(server.url());
        let r = client
            .evaluate_multimodal("describe this screenshot", Some("iVBORw0KGgoAAAANSUhEUg=="))
            .await
            .unwrap();
        assert_eq!(r.verdict, "fail");
        assert_eq!(r.confidence, 0.85);
    }

    #[tokio::test]
    async fn test_evaluate_concurrent_send() {
        let server =
            spawn_mock_server(r#"{"verdict":"na","confidence":1.0,"justification":"n/a"}"#);
        let client = std::sync::Arc::new(
            HoloClient::new("test-key".to_string())
                .unwrap()
                .with_base_url(server.url()),
        );

        let start = std::time::Instant::now();
        let mut set = tokio::task::JoinSet::new();
        for i in 0..10u32 {
            let c = std::sync::Arc::clone(&client);
            set.spawn(async move { c.evaluate(&format!("prompt-{i}")).await });
        }
        let mut ok = 0;
        while let Some(joined) = set.join_next().await {
            if joined.expect("task panicked").is_ok() {
                ok += 1;
            }
        }
        assert_eq!(ok, 10, "all concurrent calls should succeed");
        assert!(start.elapsed().as_secs() < 10);
    }

    #[tokio::test]
    async fn implements_llm_backend_as_dyn() {
        let server =
            spawn_mock_server(r#"{"verdict":"pass","confidence":0.5,"justification":"ok"}"#);
        let backend: Box<dyn LlmBackend> = Box::new(
            HoloClient::new("k".to_string())
                .unwrap()
                .with_base_url(server.url()),
        );
        assert_eq!(backend.name(), "holo3");
        assert_eq!(backend.model(), MODEL);
        assert_eq!(backend.evaluate("p").await.unwrap().verdict, "pass");
    }
}
