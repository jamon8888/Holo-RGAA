use crate::{HoloClient, HoloResponse, OllamaClient};
use async_trait::async_trait;
use rgaa_core::RgaaError;

/// A chat-completion backend able to return an RGAA verdict.
///
/// Both the remote Holo3 API ([`HoloClient`]) and a local Ollama server
/// ([`OllamaClient`]) implement it; callers hold a `Box<dyn LlmBackend>` and
/// record [`name`](Self::name) alongside every verdict they persist.
#[async_trait]
pub trait LlmBackend: Send + Sync {
    /// Stable identifier of the backend (`"holo3"`, `"ollama"`), surfaced in results.
    fn name(&self) -> &'static str;

    /// Model identifier the backend sends, for provenance.
    fn model(&self) -> &str;

    async fn evaluate(&self, prompt: &str) -> Result<HoloResponse, RgaaError>;

    async fn evaluate_multimodal(
        &self,
        prompt: &str,
        image_base64: Option<&str>,
    ) -> Result<HoloResponse, RgaaError>;
}

/// Explicit backend selection. There is deliberately no default and no
/// fallback from one backend to the other: the operator chooses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackendConfig {
    Holo3 {
        api_key: String,
    },
    Ollama {
        model: String,
        /// Full chat-completions endpoint; `None` uses [`OllamaClient::DEFAULT_ENDPOINT`].
        endpoint: Option<String>,
    },
}

impl BackendConfig {
    /// Reads `RGAA_LLM_BACKEND` (`holo3` | `ollama`) plus the backend's own
    /// variables: `HOLO3_API_KEY`, or `OLLAMA_MODEL` and optional `OLLAMA_ENDPOINT`.
    pub fn from_env() -> Result<Self, RgaaError> {
        let backend = std::env::var("RGAA_LLM_BACKEND").map_err(|_| RgaaError::Llm {
            message: "RGAA_LLM_BACKEND is not set (expected `holo3` or `ollama`)".to_string(),
            code: Some("BACKEND_NOT_CONFIGURED".to_string()),
        })?;
        Self::from_env_with(&backend, |k| std::env::var(k).ok())
    }

    fn from_env_with(
        backend: &str,
        var: impl Fn(&str) -> Option<String>,
    ) -> Result<Self, RgaaError> {
        let missing = |name: &str| RgaaError::Llm {
            message: format!("{name} is not set"),
            code: Some("BACKEND_NOT_CONFIGURED".to_string()),
        };
        match backend.trim().to_ascii_lowercase().as_str() {
            "holo3" => Ok(Self::Holo3 {
                api_key: var("HOLO3_API_KEY").ok_or_else(|| missing("HOLO3_API_KEY"))?,
            }),
            "ollama" => Ok(Self::Ollama {
                model: var("OLLAMA_MODEL").ok_or_else(|| missing("OLLAMA_MODEL"))?,
                endpoint: var("OLLAMA_ENDPOINT"),
            }),
            other => Err(RgaaError::Llm {
                message: format!(
                    "unknown RGAA_LLM_BACKEND `{other}` (expected `holo3` or `ollama`)"
                ),
                code: Some("BACKEND_NOT_CONFIGURED".to_string()),
            }),
        }
    }

    pub fn build(self) -> Result<Box<dyn LlmBackend>, RgaaError> {
        Ok(match self {
            Self::Holo3 { api_key } => Box::new(HoloClient::new(api_key)?),
            Self::Ollama { model, endpoint } => {
                let client = OllamaClient::new(model)?;
                Box::new(match endpoint {
                    Some(e) => client.with_endpoint(e),
                    None => client,
                })
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn env(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let map: HashMap<String, String> = pairs
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect();
        move |k| map.get(k).cloned()
    }

    #[test]
    fn holo3_config_needs_api_key() {
        let cfg = BackendConfig::from_env_with("holo3", env(&[("HOLO3_API_KEY", "k")])).unwrap();
        assert_eq!(
            cfg,
            BackendConfig::Holo3 {
                api_key: "k".into()
            }
        );
        assert!(BackendConfig::from_env_with("holo3", env(&[])).is_err());
    }

    #[test]
    fn ollama_config_needs_model_and_takes_optional_endpoint() {
        let cfg =
            BackendConfig::from_env_with("OLLAMA", env(&[("OLLAMA_MODEL", "qwen2.5:7b-instruct")]))
                .unwrap();
        assert_eq!(
            cfg,
            BackendConfig::Ollama {
                model: "qwen2.5:7b-instruct".into(),
                endpoint: None
            }
        );
        let cfg = BackendConfig::from_env_with(
            "ollama",
            env(&[
                ("OLLAMA_MODEL", "m"),
                (
                    "OLLAMA_ENDPOINT",
                    "http://gpu-box:11434/v1/chat/completions",
                ),
            ]),
        )
        .unwrap();
        assert!(matches!(
            cfg,
            BackendConfig::Ollama {
                endpoint: Some(_),
                ..
            }
        ));
        assert!(BackendConfig::from_env_with("ollama", env(&[])).is_err());
    }

    #[test]
    fn unknown_backend_is_rejected_not_defaulted() {
        let err = BackendConfig::from_env_with("openai", env(&[])).unwrap_err();
        assert!(err.to_string().contains("openai"));
    }

    #[test]
    fn build_yields_named_backends() {
        let holo = BackendConfig::Holo3 {
            api_key: "k".into(),
        }
        .build()
        .unwrap();
        assert_eq!(holo.name(), "holo3");
        let ollama = BackendConfig::Ollama {
            model: "m".into(),
            endpoint: None,
        }
        .build()
        .unwrap();
        assert_eq!(ollama.name(), "ollama");
        assert_eq!(ollama.model(), "m");
    }
}
