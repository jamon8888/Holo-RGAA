use crate::{ChatBackend, FallbackBackend, HoloResponse};
use async_trait::async_trait;
use rgaa_core::{LlmSettings, RgaaError};

/// A chat-completion backend able to return an RGAA verdict.
///
/// Every OpenAI-compatible provider goes through [`ChatBackend`]; callers
/// hold a `Box<dyn LlmBackend>` and record [`name`](Self::name) alongside
/// every verdict they persist.
#[async_trait]
pub trait LlmBackend: Send + Sync {
    /// Stable identifier of the backend (`"holo3"`, `"ollama"`, `"groq"`, …),
    /// surfaced in results.
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

/// Which model tier a backend should be built on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tier {
    /// [`LlmSettings::model`] — the route's default model.
    #[default]
    Default,
    /// [`LlmSettings::model_tactical`] — fast and cheap, most criteria.
    Tactical,
    /// [`LlmSettings::model_reasoning`] — slower and stronger, hard criteria.
    Reasoning,
}

impl Tier {
    fn model_of(self, settings: &LlmSettings) -> String {
        match self {
            Self::Default => settings.model.clone(),
            Self::Tactical => settings.model_tactical.clone(),
            Self::Reasoning => settings.model_reasoning.clone(),
        }
    }
}

/// Explicit backend selection, resolved from the shared provider table
/// ([`rgaa_core::PROVIDERS`]).
///
/// There is deliberately no hardcoded credential and no guessed model: an
/// unconfigured environment fails closed. A second route is used only as an
/// opt-in technical fallback ([`Self::Fallback`]) — never activated
/// implicitly, never a silent default.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackendConfig {
    /// A single provider route.
    Single(LlmSettings),
    /// A primary route with a second route as a technical fallback: calls
    /// go to `primary` first, and only reach `secondary` when `primary`
    /// fails (after its own internal retries) — see [`FallbackBackend`].
    Fallback {
        primary: Box<BackendConfig>,
        secondary: Box<BackendConfig>,
    },
}

impl BackendConfig {
    /// Reads the primary route from `RGAA_LLM_PROVIDER` and its companion
    /// variables (see [`LlmSettings::from_env`]), then — when
    /// `RGAA_LLM_FALLBACK_PROVIDER` is set — a second, independently
    /// configured route from the `RGAA_LLM_FALLBACK_*` variables, returning
    /// [`Self::Fallback`] wrapping both. When it is unset the primary route
    /// is returned alone.
    ///
    /// # Errors
    /// Returns [`RgaaError::Llm`] naming the missing variable when either
    /// route cannot be resolved.
    pub fn from_env() -> Result<Self, RgaaError> {
        Self::from_env_with(|k| std::env::var(k).ok())
    }

    /// As [`Self::from_env`], reading through `var` instead of the process
    /// environment.
    ///
    /// # Errors
    /// See [`Self::from_env`].
    pub fn from_env_with(var: impl Fn(&str) -> Option<String>) -> Result<Self, RgaaError> {
        let primary = Self::Single(LlmSettings::from_env_prefixed("", &var)?);
        match var("RGAA_LLM_FALLBACK_PROVIDER") {
            Some(_) => Ok(Self::Fallback {
                primary: Box::new(primary),
                secondary: Box::new(Self::Single(LlmSettings::from_env_prefixed(
                    "FALLBACK_",
                    &var,
                )?)),
            }),
            None => Ok(primary),
        }
    }

    /// Builds the backend on each route's default model.
    ///
    /// # Errors
    /// Returns [`RgaaError::Llm`] if an HTTP client cannot be built.
    pub fn build(self) -> Result<Box<dyn LlmBackend>, RgaaError> {
        self.build_tier(Tier::Default)
    }

    /// Builds the backend on `tier`'s model, applying the same tier to both
    /// routes of a [`Self::Fallback`].
    ///
    /// # Errors
    /// Returns [`RgaaError::Llm`] if an HTTP client cannot be built.
    pub fn build_tier(self, tier: Tier) -> Result<Box<dyn LlmBackend>, RgaaError> {
        Ok(match self {
            Self::Single(settings) => {
                let model = tier.model_of(&settings);
                Box::new(ChatBackend::with_model(&settings, model)?)
            }
            Self::Fallback { primary, secondary } => Box::new(FallbackBackend::new(
                primary.build_tier(tier)?,
                secondary.build_tier(tier)?,
            )),
        })
    }

    /// The settings of the primary route — the one every call is attempted
    /// on first.
    #[must_use]
    pub fn primary_settings(&self) -> &LlmSettings {
        match self {
            Self::Single(s) => s,
            Self::Fallback { primary, .. } => primary.primary_settings(),
        }
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
    fn holo3_stays_the_default_provider_and_needs_its_key() {
        let cfg = BackendConfig::from_env_with(env(&[
            ("HOLO3_API_KEY", "k"),
            ("HOLO3_MODEL", "holo3-1-35b-a3b"),
        ]))
        .unwrap();
        let s = cfg.primary_settings();
        assert_eq!(s.provider.name, "holo3");
        assert_eq!(s.api_key, "k");

        let err = BackendConfig::from_env_with(env(&[("HOLO3_MODEL", "m")])).unwrap_err();
        assert!(err.to_string().contains("HOLO3_API_KEY"), "{err}");
    }

    #[test]
    fn ollama_config_needs_a_model_and_takes_an_optional_endpoint() {
        let cfg = BackendConfig::from_env_with(env(&[
            ("RGAA_LLM_PROVIDER", "OLLAMA"),
            ("RGAA_LLM_MODEL", "qwen2.5:7b-instruct"),
        ]))
        .unwrap();
        assert_eq!(
            cfg.primary_settings().chat_completions_url(),
            "http://localhost:11434/v1/chat/completions"
        );

        let cfg = BackendConfig::from_env_with(env(&[
            ("RGAA_LLM_PROVIDER", "ollama"),
            ("RGAA_LLM_MODEL", "m"),
            ("RGAA_LLM_BASE_URL", "http://gpu-box:11434/v1"),
        ]))
        .unwrap();
        assert_eq!(
            cfg.primary_settings().chat_completions_url(),
            "http://gpu-box:11434/v1/chat/completions"
        );

        let err =
            BackendConfig::from_env_with(env(&[("RGAA_LLM_PROVIDER", "ollama")])).unwrap_err();
        assert!(err.to_string().contains("RGAA_LLM_MODEL"), "{err}");
    }

    #[test]
    fn any_openai_compatible_provider_is_selectable() {
        for (name, expected_url) in [
            ("openai", "https://api.openai.com/v1/chat/completions"),
            (
                "openrouter",
                "https://openrouter.ai/api/v1/chat/completions",
            ),
            ("groq", "https://api.groq.com/openai/v1/chat/completions"),
            ("mistral", "https://api.mistral.ai/v1/chat/completions"),
        ] {
            let cfg = BackendConfig::from_env_with(env(&[
                ("RGAA_LLM_PROVIDER", name),
                ("RGAA_LLM_API_KEY", "k"),
                ("RGAA_LLM_MODEL", "m"),
            ]))
            .unwrap();
            assert_eq!(cfg.primary_settings().chat_completions_url(), expected_url);
            assert_eq!(cfg.build().unwrap().name(), name);
        }
    }

    #[test]
    fn unknown_backend_is_rejected_not_defaulted() {
        let err =
            BackendConfig::from_env_with(env(&[("RGAA_LLM_PROVIDER", "openai-ish")])).unwrap_err();
        assert!(err.to_string().contains("openai-ish"), "{err}");
    }

    #[test]
    fn build_yields_named_backends() {
        let holo = BackendConfig::from_env_with(env(&[
            ("HOLO3_API_KEY", "k"),
            ("HOLO3_MODEL", "holo3-1-35b-a3b"),
        ]))
        .unwrap()
        .build()
        .unwrap();
        assert_eq!(holo.name(), "holo3");
        assert_eq!(holo.model(), "holo3-1-35b-a3b");

        let ollama = BackendConfig::from_env_with(env(&[
            ("RGAA_LLM_PROVIDER", "ollama"),
            ("RGAA_LLM_MODEL", "m"),
        ]))
        .unwrap()
        .build()
        .unwrap();
        assert_eq!(ollama.name(), "ollama");
        assert_eq!(ollama.model(), "m");
    }

    #[test]
    fn build_tier_picks_that_tiers_model() {
        let cfg = BackendConfig::from_env_with(env(&[
            ("RGAA_LLM_PROVIDER", "ollama"),
            ("RGAA_LLM_MODEL", "base"),
            ("RGAA_LLM_MODEL_TACTICAL", "small"),
            ("RGAA_LLM_MODEL_REASONING", "big"),
        ]))
        .unwrap();
        assert_eq!(
            cfg.clone().build_tier(Tier::Tactical).unwrap().model(),
            "small"
        );
        assert_eq!(
            cfg.clone().build_tier(Tier::Reasoning).unwrap().model(),
            "big"
        );
        assert_eq!(cfg.build_tier(Tier::Default).unwrap().model(), "base");
    }

    #[test]
    fn no_fallback_var_yields_the_primary_alone() {
        let cfg =
            BackendConfig::from_env_with(env(&[("HOLO3_API_KEY", "k"), ("HOLO3_MODEL", "m")]))
                .unwrap();
        assert!(matches!(cfg, BackendConfig::Single(_)));
    }

    #[test]
    fn fallback_var_set_wraps_primary_and_secondary() {
        let cfg = BackendConfig::from_env_with(env(&[
            ("RGAA_LLM_PROVIDER", "openai"),
            ("RGAA_LLM_API_KEY", "primary-key"),
            ("RGAA_LLM_MODEL", "gpt-4o-mini"),
            ("RGAA_LLM_FALLBACK_PROVIDER", "ollama"),
            ("RGAA_LLM_FALLBACK_MODEL", "qwen2.5:7b-instruct"),
        ]))
        .unwrap();
        let BackendConfig::Fallback { primary, secondary } = &cfg else {
            panic!("expected a fallback config, got {cfg:?}");
        };
        assert_eq!(primary.primary_settings().provider.name, "openai");
        assert_eq!(secondary.primary_settings().provider.name, "ollama");
        assert_eq!(secondary.primary_settings().model, "qwen2.5:7b-instruct");
        // A fallback backend is named after the route calls are tried on.
        assert_eq!(cfg.build().unwrap().name(), "openai");
    }

    #[test]
    fn fallback_var_set_but_missing_its_own_vars_errors() {
        let err = BackendConfig::from_env_with(env(&[
            ("HOLO3_API_KEY", "k"),
            ("HOLO3_MODEL", "m"),
            ("RGAA_LLM_FALLBACK_PROVIDER", "ollama"),
            // RGAA_LLM_FALLBACK_MODEL deliberately missing.
        ]))
        .unwrap_err();
        assert!(err.to_string().contains("RGAA_LLM_FALLBACK_MODEL"), "{err}");
    }

    #[test]
    fn no_hardcoded_keys_every_credential_comes_from_env() {
        // Both routes are built purely from the `var` closure — nothing in
        // `BackendConfig` supplies a credential on its own, so an empty
        // environment always fails closed rather than silently using some
        // default key.
        assert!(
            BackendConfig::from_env_with(env(&[("RGAA_LLM_FALLBACK_PROVIDER", "ollama")])).is_err()
        );
    }

    #[test]
    fn debug_never_leaks_a_key() {
        let cfg = BackendConfig::from_env_with(env(&[
            ("RGAA_LLM_PROVIDER", "openai"),
            ("RGAA_LLM_API_KEY", "sk-super-secret"),
            ("RGAA_LLM_MODEL", "gpt-4o-mini"),
            ("RGAA_LLM_FALLBACK_PROVIDER", "groq"),
            ("RGAA_LLM_FALLBACK_API_KEY", "gsk-also-secret"),
            ("RGAA_LLM_FALLBACK_MODEL", "llama-3.1-8b-instant"),
        ]))
        .unwrap();
        let dbg = format!("{cfg:?}");
        assert!(!dbg.contains("sk-super-secret"), "{dbg}");
        assert!(!dbg.contains("gsk-also-secret"), "{dbg}");
    }
}
