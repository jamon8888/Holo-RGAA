//! Provider-neutral LLM configuration, resolved from the environment.
//!
//! Every backend this workspace talks to speaks the OpenAI chat-completions
//! wire format (`POST {base_url}/chat/completions`), so a provider is fully
//! described by three things: a base URL, which environment variable carries
//! its key, and whether a key is required at all. [`PROVIDERS`] is that
//! table; [`LlmSettings::from_env`] turns it plus the environment into the
//! concrete settings both [`rgaa-agent`](../../rgaa_agent) (through `rig`)
//! and [`rgaa-holo`](../../rgaa_holo) (through its own transport) consume.
//!
//! Nothing here hardcodes a credential and nothing guesses a model: an
//! unconfigured environment fails closed with a message naming the missing
//! variable.

use crate::error::RgaaError;

/// One OpenAI-compatible provider preset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Provider {
    /// Stable identifier accepted by `RGAA_LLM_PROVIDER` and recorded
    /// alongside verdicts for provenance.
    pub name: &'static str,
    /// Base URL of the OpenAI-compatible API, without `/chat/completions`.
    /// Empty for [`custom`](PROVIDERS), which requires `RGAA_LLM_BASE_URL`.
    pub base_url: &'static str,
    /// Provider-native variable holding the API key, accepted in addition to
    /// the generic `RGAA_LLM_API_KEY`.
    pub key_var: &'static str,
    /// Whether a key is mandatory. Local runtimes (Ollama, LM Studio, vLLM)
    /// accept requests without one.
    pub requires_key: bool,
    /// Model used when none is configured. Only `holo3` carries one: it is
    /// the historical default this workspace shipped with, kept so a
    /// deployment that only sets `HOLO3_API_KEY` keeps working. Every other
    /// provider serves a catalog no preset can sensibly guess from, so it
    /// requires an explicit `RGAA_LLM_MODEL`.
    pub default_model: Option<&'static str>,
    /// Whether the endpoint is a local inference runtime. CPU inference of a
    /// 7B-14B model can take minutes per call, so local providers get a far
    /// longer default timeout than a hosted API — see
    /// [`LlmSettings::timeout`].
    pub local: bool,
}

/// Default request timeout for a hosted API.
const REMOTE_TIMEOUT_SECS: u64 = 30;
/// Default request timeout for a local inference runtime.
const LOCAL_TIMEOUT_SECS: u64 = 600;

/// Every provider preset known to the workspace.
///
/// Anthropic is deliberately absent: its Messages API is not OpenAI-compatible,
/// so reach Claude models through `openrouter` instead.
pub const PROVIDERS: &[Provider] = &[
    Provider {
        name: "holo3",
        base_url: "https://api.hcompany.ai/v1",
        key_var: "HOLO3_API_KEY",
        default_model: Some("holo3-1-35b-a3b"),
        requires_key: true,
        local: false,
    },
    Provider {
        name: "openai",
        base_url: "https://api.openai.com/v1",
        key_var: "OPENAI_API_KEY",
        default_model: None,
        requires_key: true,
        local: false,
    },
    Provider {
        name: "openrouter",
        base_url: "https://openrouter.ai/api/v1",
        key_var: "OPENROUTER_API_KEY",
        default_model: None,
        requires_key: true,
        local: false,
    },
    Provider {
        name: "groq",
        base_url: "https://api.groq.com/openai/v1",
        key_var: "GROQ_API_KEY",
        default_model: None,
        requires_key: true,
        local: false,
    },
    Provider {
        name: "mistral",
        base_url: "https://api.mistral.ai/v1",
        key_var: "MISTRAL_API_KEY",
        default_model: None,
        requires_key: true,
        local: false,
    },
    Provider {
        name: "deepseek",
        base_url: "https://api.deepseek.com/v1",
        key_var: "DEEPSEEK_API_KEY",
        default_model: None,
        requires_key: true,
        local: false,
    },
    Provider {
        name: "together",
        base_url: "https://api.together.xyz/v1",
        key_var: "TOGETHER_API_KEY",
        default_model: None,
        requires_key: true,
        local: false,
    },
    Provider {
        name: "xai",
        base_url: "https://api.x.ai/v1",
        key_var: "XAI_API_KEY",
        default_model: None,
        requires_key: true,
        local: false,
    },
    Provider {
        name: "ollama",
        base_url: "http://localhost:11434/v1",
        key_var: "OLLAMA_API_KEY",
        default_model: None,
        requires_key: false,
        local: true,
    },
    Provider {
        name: "lmstudio",
        base_url: "http://localhost:1234/v1",
        key_var: "LMSTUDIO_API_KEY",
        default_model: None,
        requires_key: false,
        local: true,
    },
    Provider {
        name: "vllm",
        base_url: "http://localhost:8000/v1",
        key_var: "VLLM_API_KEY",
        default_model: None,
        requires_key: false,
        local: true,
    },
    Provider {
        name: "custom",
        base_url: "",
        key_var: "RGAA_LLM_API_KEY",
        default_model: None,
        requires_key: false,
        local: true,
    },
];

/// Looks up a preset by name, case-insensitively.
#[must_use]
pub fn provider(name: &str) -> Option<&'static Provider> {
    let name = name.trim().to_ascii_lowercase();
    PROVIDERS.iter().find(|p| p.name == name)
}

/// Comma-separated list of accepted provider names, for error messages.
#[must_use]
pub fn provider_names() -> String {
    PROVIDERS
        .iter()
        .map(|p| p.name)
        .collect::<Vec<_>>()
        .join(", ")
}

fn config_error(message: impl Into<String>) -> RgaaError {
    RgaaError::Llm {
        message: message.into(),
        code: Some("BACKEND_NOT_CONFIGURED".to_string()),
    }
}

/// Fully resolved LLM settings for one route.
///
/// Build with [`Self::from_env`] (the whole process environment) or
/// [`Self::from_env_with`] (an explicit lookup, used by tests and by the
/// fallback route in `rgaa-holo`).
#[derive(Clone, PartialEq, Eq)]
pub struct LlmSettings {
    /// Preset this was resolved from; `name` is recorded with verdicts.
    pub provider: &'static Provider,
    /// Effective base URL, after any `RGAA_LLM_BASE_URL` override.
    pub base_url: String,
    /// API key; empty when the provider does not require one.
    pub api_key: String,
    /// Model used when a tier has no model of its own.
    pub model: String,
    /// Model for the cheap/fast tier (most criteria).
    pub model_tactical: String,
    /// Model for the reasoning tier (visual and hard criteria).
    pub model_reasoning: String,
    /// Per-request timeout for this route.
    pub timeout: std::time::Duration,
}

impl std::fmt::Debug for LlmSettings {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LlmSettings")
            .field("provider", &self.provider.name)
            .field("base_url", &self.base_url)
            .field(
                "api_key",
                &if self.api_key.is_empty() {
                    "<none>"
                } else {
                    "<redacted>"
                },
            )
            .field("model", &self.model)
            .field("model_tactical", &self.model_tactical)
            .field("model_reasoning", &self.model_reasoning)
            .field("timeout", &self.timeout)
            .finish()
    }
}

impl LlmSettings {
    /// Reads the primary route from the process environment.
    ///
    /// # Environment
    /// - `RGAA_LLM_PROVIDER` (default `holo3`): a name from [`PROVIDERS`].
    /// - `RGAA_LLM_MODEL` (required, legacy `HOLO3_MODEL`): the model id.
    /// - `RGAA_LLM_MODEL_TACTICAL` / `RGAA_LLM_MODEL_REASONING` (optional):
    ///   per-tier overrides, each defaulting to `RGAA_LLM_MODEL`.
    /// - `RGAA_LLM_API_KEY`, or the provider's own `key_var` (legacy
    ///   `HOLO3_API_KEY`): required unless the provider is local.
    /// - `RGAA_LLM_BASE_URL` (optional, legacy `HOLO3_BASE_URL`): overrides
    ///   the preset; required when the provider is `custom`.
    ///
    /// # Errors
    /// Returns [`RgaaError::Llm`] naming the offending variable when the
    /// provider is unknown, a required key is absent, no model is set, or
    /// `custom` is selected without a base URL.
    pub fn from_env() -> Result<Self, RgaaError> {
        Self::from_env_with(|k| std::env::var(k).ok())
    }

    /// As [`Self::from_env`], but resolves a *named route* whose variables
    /// carry `prefix` after `RGAA_LLM_` — `""` for the primary route
    /// (`RGAA_LLM_PROVIDER`, `RGAA_LLM_MODEL`, …) and e.g. `"FALLBACK_"`
    /// for a second, independently-configured route
    /// (`RGAA_LLM_FALLBACK_PROVIDER`, `RGAA_LLM_FALLBACK_MODEL`, …).
    ///
    /// The legacy `HOLO3_*` variables and the generic `RGAA_LLM_API_KEY`
    /// apply to the primary route only: a fallback route must be configured
    /// explicitly, so a key meant for the primary provider is never silently
    /// sent to a different one.
    ///
    /// # Errors
    /// See [`Self::from_env`].
    pub fn from_env_prefixed(
        prefix: &str,
        var: &impl Fn(&str) -> Option<String>,
    ) -> Result<Self, RgaaError> {
        let is_primary = prefix.is_empty();
        let key = |suffix: &str| format!("RGAA_LLM_{prefix}{suffix}");
        // Legacy `HOLO3_*` fallback, primary route only.
        let legacy = |name: &'static str| if is_primary { var(name) } else { None };

        let provider_var = key("PROVIDER");
        let name = var(&provider_var).unwrap_or_else(|| "holo3".to_string());
        let provider = provider(&name).ok_or_else(|| {
            config_error(format!(
                "unknown {provider_var} `{name}` (expected one of: {})",
                provider_names()
            ))
        })?;

        let base_url_var = key("BASE_URL");
        let base_url = var(&base_url_var)
            .or_else(|| legacy("HOLO3_BASE_URL"))
            .unwrap_or_else(|| provider.base_url.to_string());
        if base_url.is_empty() {
            return Err(config_error(format!(
                "{base_url_var} is not set (required by provider `custom`)"
            )));
        }

        let api_key_var = key("API_KEY");
        let api_key = var(&api_key_var)
            .or_else(|| var(provider.key_var))
            .unwrap_or_default();
        if provider.requires_key && api_key.is_empty() {
            return Err(config_error(format!(
                "{} is not set (or {api_key_var}), required by provider `{}`",
                provider.key_var, provider.name
            )));
        }

        let model_var = key("MODEL");
        // An empty value counts as unset: CI secrets and `.env` templates
        // routinely inject a variable with no value, and failing on that
        // would be indistinguishable from a real misconfiguration.
        let non_empty = |v: String| Some(v).filter(|v| !v.trim().is_empty());
        let model = var(&model_var)
            .and_then(non_empty)
            .or_else(|| legacy("HOLO3_MODEL").and_then(non_empty))
            .or_else(|| provider.default_model.map(str::to_string))
            .ok_or_else(|| {
                config_error(format!(
                    "{model_var} is not set (provider `{}` has no default model)",
                    provider.name
                ))
            })?;

        let model_tactical = var(&key("MODEL_TACTICAL")).unwrap_or_else(|| model.clone());
        let model_reasoning = var(&key("MODEL_REASONING")).unwrap_or_else(|| model.clone());

        let timeout_var = key("TIMEOUT_SECS");
        let timeout_secs = match var(&timeout_var) {
            Some(raw) => raw.trim().parse::<u64>().map_err(|_| {
                config_error(format!("{timeout_var} is not a whole number of seconds"))
            })?,
            None if provider.local => LOCAL_TIMEOUT_SECS,
            None => REMOTE_TIMEOUT_SECS,
        };
        if timeout_secs == 0 {
            return Err(config_error(format!(
                "{timeout_var} must be greater than 0"
            )));
        }

        Ok(Self {
            provider,
            // A trailing slash would produce `/v1//chat/completions`.
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key,
            model,
            model_tactical,
            model_reasoning,
            timeout: std::time::Duration::from_secs(timeout_secs),
        })
    }

    /// As [`Self::from_env`], reading through `var` instead of the process
    /// environment.
    pub fn from_env_with(var: impl Fn(&str) -> Option<String>) -> Result<Self, RgaaError> {
        Self::from_env_prefixed("", &var)
    }

    /// Full chat-completions URL — what a raw HTTP transport posts to.
    /// `rig` wants [`base_url`](Self::base_url) instead, and appends the
    /// path itself.
    #[must_use]
    pub fn chat_completions_url(&self) -> String {
        format!("{}/chat/completions", self.base_url)
    }

    /// `None` when the provider needs no key, so a transport can skip the
    /// `Authorization` header entirely rather than send an empty bearer.
    #[must_use]
    pub fn api_key_opt(&self) -> Option<String> {
        if self.api_key.is_empty() {
            None
        } else {
            Some(self.api_key.clone())
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
    fn defaults_to_holo3_and_requires_its_key() {
        let s = LlmSettings::from_env_with(env(&[
            ("HOLO3_API_KEY", "k"),
            ("RGAA_LLM_MODEL", "holo3-1-35b-a3b"),
        ]))
        .unwrap();
        assert_eq!(s.provider.name, "holo3");
        assert_eq!(s.base_url, "https://api.hcompany.ai/v1");
        assert_eq!(
            s.chat_completions_url(),
            "https://api.hcompany.ai/v1/chat/completions"
        );

        let err = LlmSettings::from_env_with(env(&[("RGAA_LLM_MODEL", "m")])).unwrap_err();
        assert!(err.to_string().contains("HOLO3_API_KEY"), "{err}");
    }

    #[test]
    fn generic_key_var_works_for_every_provider() {
        let s = LlmSettings::from_env_with(env(&[
            ("RGAA_LLM_PROVIDER", "groq"),
            ("RGAA_LLM_API_KEY", "gsk-x"),
            ("RGAA_LLM_MODEL", "llama-3.3-70b-versatile"),
        ]))
        .unwrap();
        assert_eq!(s.base_url, "https://api.groq.com/openai/v1");
        assert_eq!(s.api_key, "gsk-x");
    }

    #[test]
    fn local_providers_need_no_key() {
        let s = LlmSettings::from_env_with(env(&[
            ("RGAA_LLM_PROVIDER", "ollama"),
            ("RGAA_LLM_MODEL", "qwen2.5:14b-instruct"),
        ]))
        .unwrap();
        assert!(s.api_key.is_empty());
        assert_eq!(s.api_key_opt(), None);
        assert_eq!(
            s.chat_completions_url(),
            "http://localhost:11434/v1/chat/completions"
        );
    }

    #[test]
    fn custom_provider_requires_an_explicit_base_url() {
        let err = LlmSettings::from_env_with(env(&[
            ("RGAA_LLM_PROVIDER", "custom"),
            ("RGAA_LLM_MODEL", "m"),
        ]))
        .unwrap_err();
        assert!(err.to_string().contains("RGAA_LLM_BASE_URL"), "{err}");

        let s = LlmSettings::from_env_with(env(&[
            ("RGAA_LLM_PROVIDER", "custom"),
            ("RGAA_LLM_BASE_URL", "http://gpu-box:8000/v1/"),
            ("RGAA_LLM_MODEL", "m"),
        ]))
        .unwrap();
        // Trailing slash trimmed, so the joined URL stays well-formed.
        assert_eq!(
            s.chat_completions_url(),
            "http://gpu-box:8000/v1/chat/completions"
        );
    }

    #[test]
    fn model_is_required_and_tiers_default_to_it() {
        // An empty value is treated as unset, and Ollama has no default.
        let err = LlmSettings::from_env_with(env(&[
            ("RGAA_LLM_PROVIDER", "ollama"),
            ("RGAA_LLM_MODEL", "  "),
        ]))
        .unwrap_err();
        assert!(
            err.to_string().contains("RGAA_LLM_MODEL is not set"),
            "{err}"
        );

        let s = LlmSettings::from_env_with(env(&[
            ("RGAA_LLM_PROVIDER", "ollama"),
            ("RGAA_LLM_MODEL", "base"),
        ]))
        .unwrap();
        assert_eq!(s.model_tactical, "base");
        assert_eq!(s.model_reasoning, "base");
    }

    #[test]
    fn holo3_keeps_its_historical_default_model() {
        // A deployment that only ever set HOLO3_API_KEY keeps working.
        let s = LlmSettings::from_env_with(env(&[("HOLO3_API_KEY", "k")])).unwrap();
        assert_eq!(s.model, "holo3-1-35b-a3b");

        // …including when CI injects the variable with an empty value.
        let s = LlmSettings::from_env_with(env(&[
            ("HOLO3_API_KEY", "k"),
            ("HOLO3_MODEL", ""),
            ("RGAA_LLM_MODEL", ""),
        ]))
        .unwrap();
        assert_eq!(s.model, "holo3-1-35b-a3b");
    }

    #[test]
    fn every_other_provider_demands_an_explicit_model() {
        for p in PROVIDERS.iter().filter(|p| p.default_model.is_none()) {
            let err = LlmSettings::from_env_with(env(&[
                ("RGAA_LLM_PROVIDER", p.name),
                ("RGAA_LLM_API_KEY", "k"),
                ("RGAA_LLM_BASE_URL", "http://example.invalid/v1"),
            ]))
            .unwrap_err();
            assert!(
                err.to_string().contains("RGAA_LLM_MODEL"),
                "{}: {err}",
                p.name
            );
        }
    }

    #[test]
    fn per_tier_models_override_the_default() {
        let s = LlmSettings::from_env_with(env(&[
            ("RGAA_LLM_PROVIDER", "openrouter"),
            ("RGAA_LLM_API_KEY", "k"),
            ("RGAA_LLM_MODEL", "mistralai/mistral-small"),
            ("RGAA_LLM_MODEL_TACTICAL", "mistralai/ministral-8b"),
            ("RGAA_LLM_MODEL_REASONING", "anthropic/claude-sonnet-4.5"),
        ]))
        .unwrap();
        assert_eq!(s.model_tactical, "mistralai/ministral-8b");
        assert_eq!(s.model_reasoning, "anthropic/claude-sonnet-4.5");
        assert_eq!(s.model, "mistralai/mistral-small");
    }

    #[test]
    fn legacy_holo3_vars_still_configure_the_app() {
        let s = LlmSettings::from_env_with(env(&[
            ("HOLO3_API_KEY", "k"),
            ("HOLO3_BASE_URL", "https://staging.hcompany.ai/v1"),
            ("HOLO3_MODEL", "holo3-1-35b-a3b"),
        ]))
        .unwrap();
        assert_eq!(s.base_url, "https://staging.hcompany.ai/v1");
        assert_eq!(s.model, "holo3-1-35b-a3b");
    }

    #[test]
    fn unknown_provider_is_rejected_not_defaulted() {
        let err =
            LlmSettings::from_env_with(env(&[("RGAA_LLM_PROVIDER", "anthropic")])).unwrap_err();
        assert!(err.to_string().contains("anthropic"), "{err}");
        assert!(err.to_string().contains("openrouter"), "{err}");
    }

    #[test]
    fn timeouts_default_by_locality_and_are_overridable() {
        let remote = LlmSettings::from_env_with(env(&[
            ("RGAA_LLM_PROVIDER", "openai"),
            ("RGAA_LLM_API_KEY", "k"),
            ("RGAA_LLM_MODEL", "gpt-4o-mini"),
        ]))
        .unwrap();
        assert_eq!(remote.timeout.as_secs(), 30);

        // Local CPU inference needs minutes, not the hosted API's 30s.
        let local = LlmSettings::from_env_with(env(&[
            ("RGAA_LLM_PROVIDER", "ollama"),
            ("RGAA_LLM_MODEL", "qwen2.5:14b-instruct"),
        ]))
        .unwrap();
        assert_eq!(local.timeout.as_secs(), 600);

        let overridden = LlmSettings::from_env_with(env(&[
            ("RGAA_LLM_PROVIDER", "ollama"),
            ("RGAA_LLM_MODEL", "m"),
            ("RGAA_LLM_TIMEOUT_SECS", "90"),
        ]))
        .unwrap();
        assert_eq!(overridden.timeout.as_secs(), 90);
    }

    #[test]
    fn an_unusable_timeout_is_rejected_not_ignored() {
        for bad in ["0", "abc", "-5"] {
            let err = LlmSettings::from_env_with(env(&[
                ("RGAA_LLM_PROVIDER", "ollama"),
                ("RGAA_LLM_MODEL", "m"),
                ("RGAA_LLM_TIMEOUT_SECS", bad),
            ]))
            .unwrap_err();
            assert!(
                err.to_string().contains("RGAA_LLM_TIMEOUT_SECS"),
                "{bad}: {err}"
            );
        }
    }

    #[test]
    fn provider_lookup_is_case_insensitive() {
        assert_eq!(provider(" OpenAI ").unwrap().name, "openai");
        assert!(provider("nope").is_none());
    }

    #[test]
    fn debug_never_leaks_the_key() {
        let s = LlmSettings::from_env_with(env(&[
            ("RGAA_LLM_PROVIDER", "openai"),
            ("RGAA_LLM_API_KEY", "sk-super-secret"),
            ("RGAA_LLM_MODEL", "gpt-4o-mini"),
        ]))
        .unwrap();
        let dbg = format!("{s:?}");
        assert!(!dbg.contains("sk-super-secret"), "{dbg}");
        assert!(dbg.contains("<redacted>"), "{dbg}");
    }

    #[test]
    fn a_prefixed_route_reads_only_its_own_variables() {
        let vars = env(&[
            ("RGAA_LLM_PROVIDER", "openai"),
            ("RGAA_LLM_API_KEY", "primary-key"),
            ("RGAA_LLM_MODEL", "gpt-4o-mini"),
            ("RGAA_LLM_FALLBACK_PROVIDER", "ollama"),
            ("RGAA_LLM_FALLBACK_MODEL", "qwen2.5:14b-instruct"),
        ]);

        let primary = LlmSettings::from_env_prefixed("", &vars).unwrap();
        assert_eq!(primary.provider.name, "openai");
        assert_eq!(primary.model, "gpt-4o-mini");

        let fallback = LlmSettings::from_env_prefixed("FALLBACK_", &vars).unwrap();
        assert_eq!(fallback.provider.name, "ollama");
        assert_eq!(fallback.model, "qwen2.5:14b-instruct");
        // The primary's key never leaks onto the fallback route.
        assert!(fallback.api_key.is_empty());
    }

    #[test]
    fn a_prefixed_route_ignores_the_legacy_holo3_vars() {
        // `HOLO3_MODEL` configures the primary route, never a second one:
        // an unconfigured fallback must fail closed rather than inherit it.
        let vars = env(&[
            ("HOLO3_API_KEY", "k"),
            ("HOLO3_MODEL", "holo3-1-35b-a3b"),
            ("RGAA_LLM_FALLBACK_PROVIDER", "ollama"),
        ]);
        assert!(LlmSettings::from_env_prefixed("", &vars).is_ok());
        let err = LlmSettings::from_env_prefixed("FALLBACK_", &vars).unwrap_err();
        assert!(err.to_string().contains("RGAA_LLM_FALLBACK_MODEL"), "{err}");
    }

    #[test]
    fn no_preset_carries_a_credential() {
        // Every key comes from the environment: an empty one always fails
        // closed for remote providers rather than silently using a default.
        for p in PROVIDERS.iter().filter(|p| p.requires_key) {
            let err = LlmSettings::from_env_with(env(&[
                ("RGAA_LLM_PROVIDER", p.name),
                ("RGAA_LLM_MODEL", "m"),
            ]))
            .unwrap_err();
            assert!(err.to_string().contains(p.key_var), "{}: {err}", p.name);
        }
    }
}
