use crate::{FallbackBackend, HoloClient, HoloResponse, OllamaClient};
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

/// Explicit backend selection. There is deliberately no default: the
/// operator always chooses the primary route explicitly. A second route is
/// used only as an opt-in technical fallback ([`Self::Fallback`]) — never
/// activated implicitly, never a silent default.
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
    /// A primary route with a second route as a technical fallback: calls
    /// go to `primary` first, and only reach `secondary` when `primary`
    /// fails (after its own internal retries) — see [`FallbackBackend`].
    Fallback {
        primary: Box<BackendConfig>,
        secondary: Box<BackendConfig>,
    },
}

impl BackendConfig {
    /// Reads `RGAA_LLM_BACKEND` (`holo3` | `ollama`) plus that backend's
    /// own variables (`HOLO3_API_KEY`, or `OLLAMA_MODEL` and optional
    /// `OLLAMA_ENDPOINT`), then optionally `RGAA_LLM_FALLBACK_BACKEND` (same
    /// two values, same per-backend variables) for a second route: when
    /// set, the returned config is [`Self::Fallback`] wrapping both — when
    /// unset, it's the primary alone, exactly as before this existed. No
    /// key or endpoint is ever hardcoded; both routes are selected and
    /// configured entirely through environment variables.
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
        let primary = Self::single_from_env_with(backend, &var)?;
        match var("RGAA_LLM_FALLBACK_BACKEND") {
            Some(fallback_backend) => {
                let secondary = Self::single_from_env_with(&fallback_backend, &var)?;
                Ok(Self::Fallback {
                    primary: Box::new(primary),
                    secondary: Box::new(secondary),
                })
            }
            None => Ok(primary),
        }
    }

    /// Builds one non-fallback variant (`holo3` | `ollama`) from `backend`
    /// and `var`, shared by the primary and (when configured) the
    /// secondary route in [`Self::from_env_with`].
    fn single_from_env_with(
        backend: &str,
        var: &impl Fn(&str) -> Option<String>,
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
                message: format!("unknown backend `{other}` (expected `holo3` or `ollama`)"),
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
            Self::Fallback { primary, secondary } => {
                Box::new(FallbackBackend::new(primary.build()?, secondary.build()?))
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

    #[test]
    fn no_fallback_var_yields_the_primary_alone() {
        let cfg = BackendConfig::from_env_with("holo3", env(&[("HOLO3_API_KEY", "k")])).unwrap();
        assert_eq!(
            cfg,
            BackendConfig::Holo3 {
                api_key: "k".into()
            }
        );
    }

    #[test]
    fn fallback_var_set_wraps_primary_and_secondary() {
        let cfg = BackendConfig::from_env_with(
            "holo3",
            env(&[
                ("HOLO3_API_KEY", "primary-key"),
                ("RGAA_LLM_FALLBACK_BACKEND", "ollama"),
                ("OLLAMA_MODEL", "qwen2.5:7b-instruct"),
            ]),
        )
        .unwrap();
        assert_eq!(
            cfg,
            BackendConfig::Fallback {
                primary: Box::new(BackendConfig::Holo3 {
                    api_key: "primary-key".into()
                }),
                secondary: Box::new(BackendConfig::Ollama {
                    model: "qwen2.5:7b-instruct".into(),
                    endpoint: None
                }),
            }
        );
    }

    #[test]
    fn fallback_var_set_but_missing_its_own_vars_errors() {
        let err = BackendConfig::from_env_with(
            "holo3",
            env(&[
                ("HOLO3_API_KEY", "k"),
                ("RGAA_LLM_FALLBACK_BACKEND", "ollama"),
                // OLLAMA_MODEL deliberately missing.
            ]),
        )
        .unwrap_err();
        assert!(err.to_string().contains("OLLAMA_MODEL"));
    }

    #[test]
    fn no_hardcoded_keys_every_credential_comes_from_env() {
        // Both routes of a fallback config are built purely from the `var`
        // closure — nothing in `BackendConfig` supplies a credential on its
        // own, so an empty environment always fails closed rather than
        // silently using some default key.
        assert!(BackendConfig::from_env_with(
            "holo3",
            env(&[("RGAA_LLM_FALLBACK_BACKEND", "ollama")]),
        )
        .is_err());
    }

    #[test]
    fn build_fallback_yields_a_backend_named_after_the_primary() {
        let cfg = BackendConfig::Fallback {
            primary: Box::new(BackendConfig::Holo3 {
                api_key: "k".into(),
            }),
            secondary: Box::new(BackendConfig::Ollama {
                model: "m".into(),
                endpoint: None,
            }),
        };
        let backend = cfg.build().unwrap();
        assert_eq!(backend.name(), "holo3");
    }
}
