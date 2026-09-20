use crate::backend::LlmBackend;
use crate::transport::{ChatTransport, HoloResponse};
use async_trait::async_trait;
use rgaa_core::RgaaError;
use std::time::Duration;

/// Local Ollama backend through its OpenAI-compatible endpoint. No API key,
/// no outbound traffic: the privacy boundary the remote Holo3 path can't offer.
///
/// The model is deliberately not defaulted: pick a quantized ~7B instruct
/// model for interactive use and ~14B for scheduled batch on a CPU-only host.
#[derive(Debug, Clone)]
pub struct OllamaClient {
    transport: ChatTransport,
}

impl OllamaClient {
    pub const DEFAULT_ENDPOINT: &'static str = "http://localhost:11434/v1/chat/completions";
    /// CPU inference of a 7B-14B model can take minutes per call; the remote
    /// backend's 30s would abort most local runs.
    pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(600);

    /// # Errors
    ///
    /// Returns `Err(RgaaError::Llm)` if the HTTP client cannot be built.
    pub fn new(model: impl Into<String>) -> Result<Self, RgaaError> {
        let transport = ChatTransport::new(
            "ollama",
            Self::DEFAULT_ENDPOINT,
            model,
            None,
            Self::DEFAULT_TIMEOUT,
        )?;
        Ok(Self { transport })
    }

    /// Full chat-completions URL of the Ollama server (default: localhost).
    pub fn with_endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.transport.endpoint = endpoint.into();
        self
    }

    pub fn endpoint(&self) -> &str {
        &self.transport.endpoint
    }
}

#[async_trait]
impl LlmBackend for OllamaClient {
    fn name(&self) -> &'static str {
        "ollama"
    }

    fn model(&self) -> &str {
        &self.transport.model
    }

    async fn evaluate(&self, prompt: &str) -> Result<HoloResponse, RgaaError> {
        self.transport
            .complete(ChatTransport::text_messages(prompt))
            .await
    }

    async fn evaluate_multimodal(
        &self,
        prompt: &str,
        image_base64: Option<&str>,
    ) -> Result<HoloResponse, RgaaError> {
        let messages = ChatTransport::multimodal_messages(prompt, image_base64)?;
        self.transport.complete(messages).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::spawn_mock_server;

    const ENVELOPE: &str = r#"{"id":"chatcmpl-1","object":"chat.completion","model":"qwen2.5:7b-instruct","choices":[{"index":0,"message":{"role":"assistant","content":"{\"verdict\":\"fail\",\"confidence\":0.66,\"justification\":\"contraste insuffisant\"}"},"finish_reason":"stop"}]}"#;

    #[test]
    fn defaults_point_at_localhost_without_key() {
        let c = OllamaClient::new("qwen2.5:7b-instruct").unwrap();
        assert_eq!(c.endpoint(), OllamaClient::DEFAULT_ENDPOINT);
        assert_eq!(c.model(), "qwen2.5:7b-instruct");
        assert_eq!(c.name(), "ollama");
    }

    #[tokio::test]
    async fn parses_openai_envelope_and_sends_no_authorization() {
        let server = spawn_mock_server(ENVELOPE);
        let c = OllamaClient::new("qwen2.5:7b-instruct")
            .unwrap()
            .with_endpoint(server.url());

        let r = c.evaluate("prompt").await.unwrap();
        assert_eq!(r.verdict, "fail");
        assert_eq!(r.confidence, 0.66);

        let req = server.last_request();
        assert!(!req.to_ascii_lowercase().contains("authorization"), "{req}");
        assert!(req.contains(r#""model":"qwen2.5:7b-instruct""#), "{req}");
    }

    #[tokio::test]
    async fn multimodal_uses_same_wire_format() {
        let server = spawn_mock_server(ENVELOPE);
        let c = OllamaClient::new("llava")
            .unwrap()
            .with_endpoint(server.url());
        let r = c
            .evaluate_multimodal("p", Some("iVBORw0KGgoAAAANSUhEUg=="))
            .await
            .unwrap();
        assert_eq!(r.verdict, "fail");
        assert!(server.last_request().contains("image_url"));
    }

    #[tokio::test]
    async fn unreachable_server_reports_ollama_error() {
        let c = OllamaClient::new("m")
            .unwrap()
            .with_endpoint("http://127.0.0.1:1/v1/chat/completions");
        let err = c.evaluate("p").await.unwrap_err();
        assert!(err.to_string().contains("ollama"), "{err}");
    }
}
