//! Provider-agnostic [`LlmBackend`] over the shared chat transport.
//!
//! [`HoloClient`](crate::HoloClient) and [`OllamaClient`](crate::OllamaClient)
//! remain as the two named, directly-constructible clients; this is the one
//! every `RGAA_LLM_PROVIDER` resolves to, built from an
//! [`LlmSettings`] rather than from a hardcoded endpoint.

use crate::backend::LlmBackend;
use crate::transport::{ChatTransport, HoloResponse};
use async_trait::async_trait;
use rgaa_core::{LlmSettings, RgaaError};

/// A backend bound to one resolved provider route.
#[derive(Debug, Clone)]
pub struct ChatBackend {
    transport: ChatTransport,
}

impl ChatBackend {
    /// Builds a backend for `settings`, on the tier-agnostic
    /// [`LlmSettings::model`].
    ///
    /// # Errors
    /// Returns [`RgaaError::Llm`] if the HTTP client cannot be built.
    pub fn new(settings: &LlmSettings) -> Result<Self, RgaaError> {
        Self::with_model(settings, settings.model.clone())
    }

    /// As [`Self::new`], on an explicit model — the caller picks it from
    /// [`LlmSettings::model_tactical`] or
    /// [`LlmSettings::model_reasoning`] when it runs tiered.
    ///
    /// # Errors
    /// Returns [`RgaaError::Llm`] if the HTTP client cannot be built.
    pub fn with_model(settings: &LlmSettings, model: String) -> Result<Self, RgaaError> {
        // The provider table is `&'static`, so its name can serve as the
        // transport's static label without leaking a `String`.
        let transport = ChatTransport::new(
            settings.provider.name,
            settings.chat_completions_url(),
            model,
            settings.api_key_opt(),
            settings.timeout,
        )?;
        Ok(Self { transport })
    }

    /// Full chat-completions URL this backend posts to.
    #[must_use]
    pub fn endpoint(&self) -> &str {
        &self.transport.endpoint
    }

    /// Override the endpoint. Primarily used by tests against a mock server.
    #[must_use]
    pub fn with_endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.transport.endpoint = endpoint.into();
        self
    }
}

#[async_trait]
impl LlmBackend for ChatBackend {
    fn name(&self) -> &'static str {
        self.transport.label
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

    fn settings(pairs: &[(&str, &str)]) -> LlmSettings {
        let owned: Vec<(String, String)> = pairs
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect();
        LlmSettings::from_env_with(move |k| {
            owned
                .iter()
                .find(|(key, _)| key == k)
                .map(|(_, v)| v.clone())
        })
        .unwrap()
    }

    #[test]
    fn name_and_model_come_from_the_resolved_route() {
        let b = ChatBackend::new(&settings(&[
            ("RGAA_LLM_PROVIDER", "groq"),
            ("RGAA_LLM_API_KEY", "k"),
            ("RGAA_LLM_MODEL", "llama-3.3-70b-versatile"),
        ]))
        .unwrap();
        assert_eq!(b.name(), "groq");
        assert_eq!(b.model(), "llama-3.3-70b-versatile");
        assert_eq!(
            b.endpoint(),
            "https://api.groq.com/openai/v1/chat/completions"
        );
    }

    #[test]
    fn a_tier_model_overrides_the_default_one() {
        let s = settings(&[
            ("RGAA_LLM_PROVIDER", "ollama"),
            ("RGAA_LLM_MODEL", "base"),
            ("RGAA_LLM_MODEL_REASONING", "big"),
        ]);
        let b = ChatBackend::with_model(&s, s.model_reasoning.clone()).unwrap();
        assert_eq!(b.model(), "big");
    }

    #[test]
    fn debug_redacts_the_api_key() {
        let b = ChatBackend::new(&settings(&[
            ("RGAA_LLM_PROVIDER", "openai"),
            ("RGAA_LLM_API_KEY", "sk-super-secret"),
            ("RGAA_LLM_MODEL", "gpt-4o-mini"),
        ]))
        .unwrap();
        let dbg = format!("{b:?}");
        assert!(!dbg.contains("sk-super-secret"), "{dbg}");
        assert!(dbg.contains("[redacted]"), "{dbg}");
    }

    #[tokio::test]
    async fn evaluate_sends_the_key_and_model_and_parses_the_verdict() {
        let server =
            spawn_mock_server(r#"{"verdict":"fail","confidence":0.8,"justification":"x"}"#);
        let backend = ChatBackend::new(&settings(&[
            ("RGAA_LLM_PROVIDER", "openai"),
            ("RGAA_LLM_API_KEY", "test-key"),
            ("RGAA_LLM_MODEL", "gpt-4o-mini"),
        ]))
        .unwrap()
        .with_endpoint(server.url());

        let r = backend.evaluate("prompt").await.unwrap();
        assert_eq!(r.verdict, "fail");
        let req = server.last_request();
        assert!(
            req.to_ascii_lowercase()
                .contains("authorization: bearer test-key"),
            "{req}"
        );
        assert!(req.contains("gpt-4o-mini"), "{req}");
    }

    #[tokio::test]
    async fn a_keyless_provider_sends_no_authorization_header() {
        let server =
            spawn_mock_server(r#"{"verdict":"pass","confidence":1.0,"justification":"ok"}"#);
        let backend = ChatBackend::new(&settings(&[
            ("RGAA_LLM_PROVIDER", "ollama"),
            ("RGAA_LLM_MODEL", "qwen2.5:7b-instruct"),
        ]))
        .unwrap()
        .with_endpoint(server.url());

        assert_eq!(backend.evaluate("p").await.unwrap().verdict, "pass");
        assert!(
            !server
                .last_request()
                .to_ascii_lowercase()
                .contains("authorization:"),
            "{}",
            server.last_request()
        );
    }
}
