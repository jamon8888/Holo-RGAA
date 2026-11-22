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
//! Nothing here hardcodes a credential. Hosted providers with a stable
//! workspace default may provide a model; an unconfigured environment still
//! fails closed with a message naming the missing credential variable.

use crate::completion::{CompletionParams, LlmProvenance, ReasoningEffort, ResponseFormat};
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
    /// Model used when none is configured. MyIA is the current default;
    /// `holo3` retains its historical model for legacy deployments. Other
    /// providers require an explicit `RGAA_LLM_MODEL`.
    pub default_model: Option<&'static str>,
    /// Whether the endpoint is a local inference runtime. CPU inference of a
    /// 7B-14B model can take minutes per call, so local providers get a far
    /// longer default timeout than a hosted API — see
    /// [`LlmSettings::timeout`].
    pub local: bool,
}

/// Offer-level IA mode (PRD D1 / P-4), read from `RGAA_LLM_MODE`.
///
/// `off` and `byok` are the two Souverain modes (no third-party agent
/// required); `managed` is the Confort gateway. The default is `off` when no
/// provider is configured, `byok` otherwise.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LlmMode {
    /// No model call at all.
    Off,
    /// Client's own key, or a local model: Souverain.
    Byok,
    /// Instance gateway with prepaid credits: Confort.
    Managed,
}

impl LlmMode {
    /// Parses a configured value case-insensitively.
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "off" => Some(Self::Off),
            "byok" => Some(Self::Byok),
            "managed" => Some(Self::Managed),
            _ => None,
        }
    }

    /// Configuration token, as written in `RGAA_LLM_MODE`.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Byok => "byok",
            Self::Managed => "managed",
        }
    }

    /// Whether this is the Souverain offer.
    #[must_use]
    pub fn is_souverain(self) -> bool {
        !matches!(self, Self::Managed)
    }

    /// `false` in `off`: no request may leave the process.
    #[must_use]
    pub fn allows_model_calls(self) -> bool {
        !matches!(self, Self::Off)
    }

    /// Offer name shown by the CLI (D1).
    #[must_use]
    pub fn display_name(self) -> &'static str {
        if self.is_souverain() {
            "Souverain"
        } else {
            "Confort"
        }
    }
}

/// Hosts of agent publishers and of routers that can reach one (ticket #440).
///
/// Versioned here so the CLI, the agent and the raw transport all refuse the
/// same endpoints. Subdomains are covered; client-hosted OpenAI-compatible
/// endpoints, `api.mistral.ai` and local/private runtimes are deliberately not
/// listed. This is a product choice to confirm with the offer owner.
pub const FORBIDDEN_AGENT_HOSTS: &[&str] = &[
    "api.anthropic.com",
    "api.openai.com",
    "generativelanguage.googleapis.com",
    "openrouter.ai",
];

/// Lowercase host of an HTTP(S) endpoint, without userinfo, port or trailing
/// dot. `None` when the string carries no authority.
#[must_use]
pub fn endpoint_host(base_url: &str) -> Option<String> {
    let after_scheme = base_url
        .split_once("://")
        .map_or(base_url, |(_, rest)| rest);
    let authority = after_scheme.split(['/', '?', '#']).next()?;
    let hostport = authority.rsplit_once('@').map_or(authority, |(_, h)| h);
    let host = if let Some(inside) = hostport.strip_prefix('[') {
        inside.split_once(']').map_or(hostport, |(h, _)| h)
    } else {
        hostport.split_once(':').map_or(hostport, |(h, _)| h)
    };
    let host = host.trim().trim_end_matches('.').to_ascii_lowercase();
    (!host.is_empty()).then_some(host)
}

/// Whether `base_url` resolves to a versioned agent-publisher host, including
/// its subdomains but not lookalike suffixes.
#[must_use]
pub fn is_agent_publisher_endpoint(base_url: &str) -> bool {
    let Some(host) = endpoint_host(base_url) else {
        return false;
    };
    FORBIDDEN_AGENT_HOSTS.iter().any(|forbidden| {
        host == *forbidden
            || host
                .strip_suffix(forbidden)
                .is_some_and(|prefix| prefix.ends_with('.'))
    })
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
        name: "myia",
        base_url: "https://api.medium.text-generation-webui.myia.io/v1",
        key_var: "MYIA_API_KEY",
        default_model: Some("swift-1.5-27b"),
        requires_key: true,
        local: false,
    },
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
        default_model: Some("mistral-large-4"),
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

/// Whether any provider-selection variable is set (empty counts as unset).
/// `RGAA_LLM_MODEL` and the legacy per-provider variables count too: nobody
/// sets a model or a key unless they intend to reach a backend.
fn provider_is_configured(var: &impl Fn(&str) -> Option<String>) -> bool {
    const SELECTION_VARS: &[&str] = &[
        "RGAA_LLM_PROVIDER",
        "RGAA_LLM_MODEL",
        "RGAA_LLM_API_KEY",
        "MYIA_API_KEY",
        "MYIA_BASE_URL",
        "HOLO3_API_KEY",
        "HOLO3_BASE_URL",
        "HOLO3_MODEL",
    ];
    let is_set = |name: &str| var(name).is_some_and(|v| !v.trim().is_empty());
    SELECTION_VARS.iter().any(|name| is_set(name)) || PROVIDERS.iter().any(|p| is_set(p.key_var))
}

/// Resolves `RGAA_LLM_MODE` and whether a provider was configured.
///
/// # Errors
/// Returns [`RgaaError::Llm`] naming `RGAA_LLM_MODE` when it holds an unknown
/// value.
fn resolve_mode(var: &impl Fn(&str) -> Option<String>) -> Result<(LlmMode, bool), RgaaError> {
    let configured = provider_is_configured(var);
    let mode = match var("RGAA_LLM_MODE") {
        Some(raw) if !raw.trim().is_empty() => LlmMode::parse(&raw).ok_or_else(|| {
            config_error(format!(
                "unknown RGAA_LLM_MODE `{raw}` (expected off, byok or managed)"
            ))
        })?,
        _ if configured => LlmMode::Byok,
        _ => LlmMode::Off,
    };
    Ok((mode, configured))
}

/// Read-only view of the active offer, for `rgaa status` and audit headers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModeStatus {
    /// Souverain (`off` / `byok`) or Confort (`managed`).
    pub mode: LlmMode,
    /// Provider actually configured; `None` when nothing is configured.
    pub provider: Option<&'static str>,
    /// Effective LLM host; `None` when nothing is configured.
    pub host: Option<String>,
    /// Human-readable reason the mode was selected.
    pub reason: String,
}

impl ModeStatus {
    /// Resolves the mode from the process environment.
    ///
    /// # Errors
    /// See [`LlmSettings::from_env`], plus an unknown `RGAA_LLM_MODE`.
    pub fn from_env() -> Result<Self, RgaaError> {
        Self::from_env_with(|k| std::env::var(k).ok())
    }

    /// As [`Self::from_env`], reading through `var` instead of the process
    /// environment.
    ///
    /// # Errors
    /// See [`Self::from_env`].
    pub fn from_env_with(var: impl Fn(&str) -> Option<String>) -> Result<Self, RgaaError> {
        let (_, configured) = resolve_mode(&var)?;
        let settings = LlmSettings::from_env_with(&var)?;
        let mode = settings.mode;
        let reason = match (mode, configured) {
            (LlmMode::Off, false) => {
                "aucun fournisseur LLM configuré : aucun appel modèle (D1)".to_string()
            }
            (LlmMode::Off, true) => "RGAA_LLM_MODE=off : aucun appel modèle (D1)".to_string(),
            (LlmMode::Byok, _) => {
                "clé du client ou modèle local, aucun agent tiers requis (D1)".to_string()
            }
            (LlmMode::Managed, _) => {
                "passerelle managed de l'instance, crédits prépayés (D1, D4)".to_string()
            }
        };
        Ok(Self {
            mode,
            provider: configured.then_some(settings.provider.name),
            host: if configured {
                endpoint_host(&settings.base_url)
            } else {
                None
            },
            reason,
        })
    }
}

/// Fully resolved LLM settings for one route.
///
/// Build with [`Self::from_env`] (the whole process environment) or
/// [`Self::from_env_with`] (an explicit lookup, used by tests and by the
/// fallback route in `rgaa-holo`).
#[derive(Clone, PartialEq)]
pub struct LlmSettings {
    /// Offer-level mode this route was resolved under (D1, P-4).
    pub mode: LlmMode,
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
    /// Completion parameters every path on this route sends. Resolved here
    /// so the raw transport and the `rig` agent cannot drift apart — see
    /// [`CompletionParams`].
    pub params: CompletionParams,
}

impl std::fmt::Debug for LlmSettings {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LlmSettings")
            .field("mode", &self.mode.as_str())
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
            .field("params", &self.params)
            .finish()
    }
}

impl LlmSettings {
    /// Reads the primary route from the process environment.
    ///
    /// # Environment
    /// - `RGAA_LLM_PROVIDER` (default `myia`): a name from [`PROVIDERS`].
    /// - `RGAA_LLM_MODEL` (optional for providers with a default; legacy
    ///   `HOLO3_MODEL` on the legacy Holo3 route): the model id. MyIA defaults
    ///   to `swift-1.5-27b`; `qwen3.6-35b-a3b` is also accepted by that API.
    /// - `RGAA_LLM_MODEL_TACTICAL` / `RGAA_LLM_MODEL_REASONING` (optional):
    ///   per-tier overrides, each defaulting to `RGAA_LLM_MODEL`.
    /// - `RGAA_LLM_API_KEY`, or the provider's own `key_var` (legacy
    ///   `MYIA_API_KEY`; legacy `HOLO3_API_KEY` selects the historical Holo3
    ///   route only when no provider is selected): required unless the
    ///   provider is local.
    /// - `RGAA_LLM_BASE_URL` (optional; `MYIA_BASE_URL` on the MyIA route and
    ///   legacy `HOLO3_BASE_URL` on the Holo3 route): overrides the preset;
    ///   required for `custom`.
    /// - `RGAA_LLM_TIMEOUT_SECS` (optional): per-request timeout; defaults to
    ///   30s remote / 600s local.
    /// - `RGAA_LLM_TEMPERATURE` (optional, default
    ///   [`DEFAULT_TEMPERATURE`](crate::completion::DEFAULT_TEMPERATURE)).
    /// - `RGAA_LLM_MAX_TOKENS` (optional, default
    ///   [`DEFAULT_MAX_TOKENS`](crate::completion::DEFAULT_MAX_TOKENS)).
    /// - `RGAA_LLM_ENABLE_THINKING` (optional, `true` / `false` / `auto`;
    ///   `auto` switches thinking off on a local runtime and says nothing to
    ///   a hosted API).
    /// - `RGAA_LLM_RESPONSE_FORMAT` (optional, `none` / `json_object` /
    ///   `json_schema`; default `none`).
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
    /// The legacy `HOLO3_*` variables and generic `RGAA_LLM_API_KEY` apply to
    /// the primary route only. With no explicit provider, legacy Holo3
    /// variables select the Holo3 migration route unless `MYIA_API_KEY` is
    /// present. An explicitly selected provider remains authoritative; legacy
    /// Holo3 base URL/model values cannot override it. A fallback route must
    /// be configured explicitly, so a primary key is never silently sent to
    /// another provider.
    ///
    /// # Errors
    /// See [`Self::from_env`].
    pub fn from_env_prefixed(
        prefix: &str,
        var: &impl Fn(&str) -> Option<String>,
    ) -> Result<Self, RgaaError> {
        let is_primary = prefix.is_empty();
        let key = |suffix: &str| format!("RGAA_LLM_{prefix}{suffix}");
        // An empty value counts as unset, for *every* variable below: CI
        // secrets and `.env` templates routinely inject a variable with no
        // value, and treating that as a real setting turns a working
        // configuration into a startup failure (a blank provider read as
        // unknown, a blank base URL overriding the preset, a blank generic key
        // hiding the provider's own, a blank timeout failing to parse).
        let non_empty = |v: String| Some(v).filter(|v| !v.trim().is_empty());
        let get = |name: &str| var(name).and_then(non_empty);
        // `RGAA_LLM_MODE` is unprefixed: it is an offer-level setting, not a
        // per-route one. Both the primary and any fallback route obey it.
        let (mode, _) = resolve_mode(var)?;
        let provider_var = key("PROVIDER");
        let explicitly_selected_provider = get(&provider_var);
        let has_myia_key = get("MYIA_API_KEY").is_some();
        let legacy_holo3_route = is_primary
            && explicitly_selected_provider.is_none()
            && !has_myia_key
            && ["HOLO3_API_KEY", "HOLO3_BASE_URL", "HOLO3_MODEL"]
                .iter()
                .any(|name| get(name).is_some());
        let name = explicitly_selected_provider.unwrap_or_else(|| {
            if legacy_holo3_route {
                "holo3".to_string()
            } else {
                "myia".to_string()
            }
        });
        let provider = provider(&name).ok_or_else(|| {
            config_error(format!(
                "unknown {provider_var} `{name}` (expected one of: {})",
                provider_names()
            ))
        })?;
        // Provider-native URL/model settings are recognized only for their
        // matching provider. This includes explicit provider selection, but
        // never lets one provider's settings override another provider.
        let legacy_holo3 = is_primary && provider.name == "holo3";
        let myia_native = is_primary && provider.name == "myia";
        let holo3_setting = |name: &'static str| {
            if legacy_holo3 {
                get(name)
            } else {
                None
            }
        };
        let myia_setting = |name: &'static str| {
            if myia_native {
                get(name)
            } else {
                None
            }
        };

        let base_url_var = key("BASE_URL");
        let base_url = get(&base_url_var)
            .or_else(|| myia_setting("MYIA_BASE_URL"))
            .or_else(|| holo3_setting("HOLO3_BASE_URL"))
            .unwrap_or_else(|| provider.base_url.to_string());
        if base_url.is_empty() {
            return Err(config_error(format!(
                "{base_url_var} is not set (required by provider `custom`)"
            )));
        }
        // D1 / D4: `managed` is the instance gateway, which is always
        // configured explicitly. The fallback route has no gateway of its own.
        if mode == LlmMode::Managed && is_primary && get("RGAA_LLM_BASE_URL").is_none() {
            return Err(config_error(
                "RGAA_LLM_BASE_URL is required when RGAA_LLM_MODE=managed (instance gateway)",
            ));
        }
        // Ticket #440: the Souverain offer never sends data to an agent
        // publisher (or to a router that can reach one). Fail at resolution,
        // before any client is built, with a distinct error code.
        if mode.is_souverain() && is_agent_publisher_endpoint(&base_url) {
            return Err(RgaaError::Llm {
                message: format!(
                    "mode {} (Souverain) refuse l'endpoint {base_url} : éditeur d'agent ou routeur (liste versionnée dans rgaa-core)",
                    mode.as_str()
                ),
                code: Some("AGENT_PUBLISHER_FORBIDDEN".to_string()),
            });
        }

        let api_key_var = key("API_KEY");
        // Provider-native keys belong to the primary route only. A fallback
        // must use its own prefixed `RGAA_LLM_FALLBACK_API_KEY`; otherwise a
        // primary credential could be sent to a different host.
        let native_key = |name: &str| if is_primary { get(name) } else { None };
        let api_key = get(&api_key_var)
            .or_else(|| native_key(provider.key_var))
            .unwrap_or_default();
        // In `off` no request is ever sent, so a missing credential is not a
        // startup error: the CLI must still be able to report the mode.
        if provider.requires_key && api_key.is_empty() && mode.allows_model_calls() {
            let required_var = if is_primary {
                format!("{} or {api_key_var}", provider.key_var)
            } else {
                api_key_var.clone()
            };
            return Err(config_error(format!(
                "{required_var} is not set, required by provider `{}`",
                provider.name
            )));
        }

        let model_var = key("MODEL");
        let model = match get(&model_var)
            .or_else(|| holo3_setting("HOLO3_MODEL"))
            .or_else(|| provider.default_model.map(str::to_string))
        {
            Some(model) => model,
            None if !mode.allows_model_calls() => String::new(),
            None => {
                return Err(config_error(format!(
                    "{model_var} is not set (provider `{}` has no default model)",
                    provider.name
                )))
            }
        };

        // A blank tier variable falls back to the route's model rather than
        // being sent as an empty model identifier.
        let model_tactical = get(&key("MODEL_TACTICAL")).unwrap_or_else(|| model.clone());
        let model_reasoning = get(&key("MODEL_REASONING")).unwrap_or_else(|| model.clone());

        let timeout_var = key("TIMEOUT_SECS");
        let timeout_secs = match get(&timeout_var) {
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

        // Completion parameters: one resolution, read by every path.
        let temperature = match get(&key("TEMPERATURE")) {
            Some(raw) => {
                let parsed = raw
                    .trim()
                    .parse::<f64>()
                    .map_err(|_| config_error(format!("{} is not a number", key("TEMPERATURE"))))?;
                // `parse::<f64>` happily accepts `NaN`, `inf` and `-1`.
                // `serde_json` writes a non-finite float as `null`, so
                // `temperature: null` would go on the wire and the provenance
                // line would read `NaN`; a negative value earns a 400 on every
                // single call. MAX_TOKENS and TIMEOUT_SECS are already checked
                // here, so this one was the gap.
                if !parsed.is_finite() || parsed < 0.0 {
                    return Err(config_error(format!(
                        "{} is `{raw}` (expected a finite number >= 0)",
                        key("TEMPERATURE")
                    )));
                }
                parsed
            }
            None => crate::completion::DEFAULT_TEMPERATURE,
        };
        let max_tokens = match get(&key("MAX_TOKENS")) {
            Some(raw) => {
                let n = raw.trim().parse::<u32>().map_err(|_| {
                    config_error(format!("{} is not a whole number", key("MAX_TOKENS")))
                })?;
                if n == 0 {
                    return Err(config_error(format!(
                        "{} must be greater than 0",
                        key("MAX_TOKENS")
                    )));
                }
                n
            }
            None => crate::completion::DEFAULT_MAX_TOKENS,
        };
        // `auto` (and unset) means: switch thinking off on a local runtime,
        // where reasoning models are served and their thinking tokens ate the
        // completion budget, and say nothing to a hosted API, which would
        // reject the unknown body key outright.
        let thinking_var = key("ENABLE_THINKING");
        // Lowercased before matching: `RESPONSE_FORMAT` next door already does,
        // and two neighbouring variables that disagree about whether `Auto` is
        // valid is a trap rather than a policy.
        let thinking_raw = get(&thinking_var).map(|v| v.trim().to_ascii_lowercase());
        let enable_thinking = match thinking_raw.as_deref() {
            None | Some("auto") => provider.local.then_some(false),
            Some("true" | "1" | "yes" | "on") => Some(true),
            Some("false" | "0" | "no" | "off") => Some(false),
            Some(other) => {
                return Err(config_error(format!(
                    "{thinking_var} is `{other}` (expected true, false or auto)"
                )))
            }
        };
        let rf_var = key("RESPONSE_FORMAT");
        let response_format = match get(&rf_var) {
            Some(raw) => ResponseFormat::parse(&raw).ok_or_else(|| {
                config_error(format!(
                    "unknown {rf_var} `{raw}` (expected none, json_object or json_schema)"
                ))
            })?,
            None => ResponseFormat::None,
        };

        let effort_var = key("REASONING_EFFORT");
        let reasoning_effort = match get(&effort_var) {
            Some(raw) => Some(ReasoningEffort::parse(&raw).ok_or_else(|| {
                config_error(format!(
                    "{effort_var} is `{raw}` (expected low, medium or high; \
                     `none` is refused by Mistral Large 4)"
                ))
            })?),
            None => None,
        };

        Ok(Self {
            mode,
            provider,
            // A trailing slash would produce `/v1//chat/completions`.
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key,
            model,
            model_tactical,
            model_reasoning,
            timeout: std::time::Duration::from_secs(timeout_secs),
            params: CompletionParams {
                temperature,
                max_tokens,
                enable_thinking,
                response_format,
                reasoning_effort,
            },
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

    /// What a call on `model` through this route will record as having run
    /// with — see [`LlmProvenance`].
    #[must_use]
    pub fn provenance(&self, model: &str) -> LlmProvenance {
        LlmProvenance::new(
            self.provider.name,
            model,
            self.chat_completions_url(),
            &self.params,
        )
    }

    /// Requests per minute this route should be held to by default.
    ///
    /// A hosted API bills and throttles, so the historical 10/20 rpm stays.
    /// A self-hosted endpoint is a box the operator already owns: throttling
    /// it to 10 rpm turned a bake-off into an overnight job for no reason, so
    /// local routes default to `0` — unlimited, in
    /// `rgaa_agent::ratelimit::Ratelimiter`'s reading of it. An explicit
    /// `RGAA_TACTICAL_RPM` / `RGAA_REASONING_RPM` still wins.
    #[must_use]
    pub fn default_rpm(&self, hosted_default: u32) -> u32 {
        if self.provider.local {
            0
        } else {
            hosted_default
        }
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
    fn mistral_defaults_to_the_pinned_large_4_model() {
        let s = LlmSettings::from_env_with(env(&[
            ("RGAA_LLM_PROVIDER", "mistral"),
            ("MISTRAL_API_KEY", "k"),
        ]))
        .unwrap();
        // A versioned identifier, not `mistral-large-latest`: verdicts must
        // stay reproducible when Mistral moves the floating alias.
        assert_eq!(s.model, "mistral-large-4");
        assert_eq!(s.model_tactical, "mistral-large-4");
        assert_eq!(s.model_reasoning, "mistral-large-4");
        assert_eq!(
            s.chat_completions_url(),
            "https://api.mistral.ai/v1/chat/completions"
        );
    }

    /// Mistral Large 4 always reasons and defaults to `high`; the effort is
    /// the cost lever, so it must reach the wire on both LLM paths.
    #[test]
    fn reasoning_effort_reaches_the_request_body_on_both_paths() {
        let s = LlmSettings::from_env_with(env(&[
            ("RGAA_LLM_PROVIDER", "mistral"),
            ("MISTRAL_API_KEY", "k"),
            ("RGAA_LLM_REASONING_EFFORT", "Low"),
        ]))
        .unwrap();
        assert_eq!(s.params.extra_body()["reasoning_effort"], "low");
        assert_eq!(
            s.params.extra_body_without_response_format()["reasoning_effort"],
            "low"
        );
        assert!(s.provenance("m").summary().contains("reasoning_effort=low"));
    }

    #[test]
    fn reasoning_effort_unset_sends_nothing() {
        let s = LlmSettings::from_env_with(env(&[
            ("RGAA_LLM_PROVIDER", "mistral"),
            ("MISTRAL_API_KEY", "k"),
        ]))
        .unwrap();
        assert!(!s.params.extra_body().contains_key("reasoning_effort"));
    }

    /// `none` is refused by Mistral Large 4 (HTTP 400) and by NVIDIA's
    /// endpoint; fail at configuration time instead of on every call.
    #[test]
    fn reasoning_effort_none_or_unknown_is_rejected() {
        for bad in ["none", "max", "hgh"] {
            let err = LlmSettings::from_env_with(env(&[
                ("RGAA_LLM_PROVIDER", "mistral"),
                ("MISTRAL_API_KEY", "k"),
                ("RGAA_LLM_REASONING_EFFORT", bad),
            ]))
            .unwrap_err();
            assert!(
                err.to_string().contains("RGAA_LLM_REASONING_EFFORT"),
                "{bad}: {err}"
            );
        }
    }

    #[test]
    fn legacy_holo3_environment_stays_supported_and_myia_is_the_default() {
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

        let s = LlmSettings::from_env_with(env(&[("MYIA_API_KEY", "myia-key")])).unwrap();
        assert_eq!(s.provider.name, "myia");
        assert_eq!(
            s.base_url,
            "https://api.medium.text-generation-webui.myia.io/v1"
        );
        assert_eq!(s.model, "swift-1.5-27b");

        let err = LlmSettings::from_env_with(env(&[("RGAA_LLM_MODEL", "m")])).unwrap_err();
        assert!(err.to_string().contains("MYIA_API_KEY"), "{err}");
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
    fn myia_native_base_url_is_used_below_the_generic_override() {
        let native = LlmSettings::from_env_with(env(&[
            ("MYIA_API_KEY", "myia-key"),
            ("MYIA_BASE_URL", "https://myia-proxy.example/v1"),
        ]))
        .unwrap();
        assert_eq!(native.base_url, "https://myia-proxy.example/v1");

        let generic = LlmSettings::from_env_with(env(&[
            ("MYIA_API_KEY", "myia-key"),
            ("MYIA_BASE_URL", "https://myia-proxy.example/v1"),
            ("RGAA_LLM_BASE_URL", "https://generic.example/v1"),
        ]))
        .unwrap();
        assert_eq!(generic.base_url, "https://generic.example/v1");

        let other_provider = LlmSettings::from_env_with(env(&[
            ("RGAA_LLM_PROVIDER", "groq"),
            ("RGAA_LLM_API_KEY", "groq-key"),
            ("RGAA_LLM_MODEL", "llama"),
            ("MYIA_BASE_URL", "https://myia-proxy.example/v1"),
        ]))
        .unwrap();
        assert_eq!(other_provider.base_url, "https://api.groq.com/openai/v1");
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
    fn a_blank_tier_variable_falls_back_to_the_route_model() {
        // Deployment tooling routinely injects an empty variable; sending an
        // empty model identifier to the provider would be a 400 at best.
        let s = LlmSettings::from_env_with(env(&[
            ("RGAA_LLM_PROVIDER", "ollama"),
            ("RGAA_LLM_MODEL", "base"),
            ("RGAA_LLM_MODEL_TACTICAL", ""),
            ("RGAA_LLM_MODEL_REASONING", "   "),
        ]))
        .unwrap();
        assert_eq!(s.model_tactical, "base");
        assert_eq!(s.model_reasoning, "base");
    }

    #[test]
    fn per_tier_models_override_the_default() {
        let s = LlmSettings::from_env_with(env(&[
            ("RGAA_LLM_PROVIDER", "openrouter"),
            ("RGAA_LLM_MODE", "managed"),
            ("RGAA_LLM_BASE_URL", "https://openrouter.ai/api/v1"),
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

        let selected_myia = LlmSettings::from_env_with(env(&[
            ("RGAA_LLM_PROVIDER", "myia"),
            ("MYIA_API_KEY", "myia-key"),
            ("HOLO3_BASE_URL", "https://staging.hcompany.ai/v1"),
            ("HOLO3_MODEL", "holo3-1-35b-a3b"),
        ]))
        .unwrap();
        assert_eq!(selected_myia.provider.name, "myia");
        assert_eq!(
            selected_myia.base_url,
            "https://api.medium.text-generation-webui.myia.io/v1"
        );
        assert_eq!(selected_myia.model, "swift-1.5-27b");
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
            ("RGAA_LLM_MODE", "managed"),
            ("RGAA_LLM_BASE_URL", "https://api.openai.com/v1"),
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
            ("RGAA_LLM_MODE", "managed"),
            ("RGAA_LLM_BASE_URL", "https://api.openai.com/v1"),
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
            ("RGAA_LLM_MODE", "managed"),
            ("RGAA_LLM_BASE_URL", "https://api.openai.com/v1"),
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
    fn a_custom_fallback_route_never_inherits_the_primary_generic_key() {
        // `custom`'s own `key_var` IS the generic `RGAA_LLM_API_KEY`, so
        // without the primary-only guard the fallback route would send the
        // OpenAI key as a bearer token to an arbitrary host — over plain HTTP
        // here. CWE-522.
        let vars = env(&[
            ("RGAA_LLM_PROVIDER", "openai"),
            ("RGAA_LLM_MODE", "managed"),
            ("RGAA_LLM_BASE_URL", "https://api.openai.com/v1"),
            ("RGAA_LLM_API_KEY", "sk-primary-secret"),
            ("RGAA_LLM_MODEL", "gpt-4o-mini"),
            ("RGAA_LLM_FALLBACK_PROVIDER", "custom"),
            ("RGAA_LLM_FALLBACK_BASE_URL", "http://gpu-box:8000/v1"),
            ("RGAA_LLM_FALLBACK_MODEL", "Qwen/Qwen2.5-32B-Instruct"),
            // No RGAA_LLM_FALLBACK_API_KEY.
        ]);
        let primary = LlmSettings::from_env_prefixed("", &vars).unwrap();
        assert_eq!(primary.api_key, "sk-primary-secret");

        let fallback = LlmSettings::from_env_prefixed("FALLBACK_", &vars).unwrap();
        assert!(
            fallback.api_key.is_empty(),
            "primary key leaked to the custom fallback host: {:?}",
            fallback.api_key
        );
        assert_eq!(fallback.api_key_opt(), None);

        // A fallback key set explicitly is still honoured.
        let vars = env(&[
            ("RGAA_LLM_PROVIDER", "openai"),
            ("RGAA_LLM_MODE", "managed"),
            ("RGAA_LLM_BASE_URL", "https://api.openai.com/v1"),
            ("RGAA_LLM_API_KEY", "sk-primary-secret"),
            ("RGAA_LLM_MODEL", "gpt-4o-mini"),
            ("RGAA_LLM_FALLBACK_PROVIDER", "custom"),
            ("RGAA_LLM_FALLBACK_BASE_URL", "http://gpu-box:8000/v1"),
            ("RGAA_LLM_FALLBACK_MODEL", "m"),
            ("RGAA_LLM_FALLBACK_API_KEY", "own-key"),
        ]);
        let fallback = LlmSettings::from_env_prefixed("FALLBACK_", &vars).unwrap();
        assert_eq!(fallback.api_key, "own-key");
    }

    #[test]
    fn a_blank_value_is_unset_for_every_route_variable() {
        // Deployment tooling injects blank variables wholesale; none of them
        // may turn a valid configuration into a startup failure.
        let s = LlmSettings::from_env_with(env(&[
            ("RGAA_LLM_PROVIDER", ""),
            ("RGAA_LLM_BASE_URL", "  "),
            ("RGAA_LLM_API_KEY", ""),
            ("OPENAI_API_KEY", "native-key"),
            ("RGAA_LLM_MODEL", ""),
            ("RGAA_LLM_TIMEOUT_SECS", ""),
            ("HOLO3_API_KEY", "holo-key"),
        ]))
        .unwrap();
        // Blank provider plus a legacy Holo3 key → the migration route, not
        // "unknown provider ``".
        assert_eq!(s.provider.name, "holo3");
        // Blank base URL → the preset, not an empty endpoint.
        assert_eq!(s.base_url, "https://api.hcompany.ai/v1");
        // Blank generic key → the provider's native variable still wins.
        assert_eq!(s.api_key, "holo-key");
        // Blank model → holo3's default.
        assert_eq!(s.model, "holo3-1-35b-a3b");
        // Blank timeout → the locality default, not a parse error.
        assert_eq!(s.timeout.as_secs(), 30);
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
                // Neutral host: this test is about credentials, not the
                // Souverain refusal of agent-publisher hosts.
                ("RGAA_LLM_BASE_URL", "https://llm.example/v1"),
            ]))
            .unwrap_err();
            assert!(err.to_string().contains(p.key_var), "{}: {err}", p.name);
        }
    }

    #[test]
    fn completion_params_default_to_the_shared_constants() {
        let s = LlmSettings::from_env_with(env(&[("HOLO3_API_KEY", "k"), ("RGAA_LLM_MODEL", "m")]))
            .unwrap();
        assert_eq!(s.params.temperature, crate::completion::DEFAULT_TEMPERATURE);
        assert_eq!(s.params.max_tokens, crate::completion::DEFAULT_MAX_TOKENS);
        // Hosted: say nothing about thinking, send no response_format.
        assert_eq!(s.params.enable_thinking, None);
        assert_eq!(s.params.response_format, ResponseFormat::None);
        assert!(s.params.extra_body().is_empty());
    }

    #[test]
    fn a_local_runtime_switches_thinking_off_by_default() {
        for name in ["vllm", "ollama", "lmstudio"] {
            let s = LlmSettings::from_env_with(env(&[
                ("RGAA_LLM_PROVIDER", name),
                ("RGAA_LLM_MODEL", "qwen3:8b"),
            ]))
            .unwrap();
            assert_eq!(s.params.enable_thinking, Some(false), "{name}");
            assert_eq!(
                s.params.extra_body()["chat_template_kwargs"],
                serde_json::json!({"enable_thinking": false}),
                "{name}"
            );
        }
    }

    #[test]
    fn thinking_can_be_forced_either_way_or_back_to_auto() {
        let with = |v: &str| {
            LlmSettings::from_env_with(env(&[
                ("RGAA_LLM_PROVIDER", "vllm"),
                ("RGAA_LLM_MODEL", "m"),
                ("RGAA_LLM_ENABLE_THINKING", v),
            ]))
        };
        assert_eq!(with("true").unwrap().params.enable_thinking, Some(true));
        assert_eq!(with("false").unwrap().params.enable_thinking, Some(false));
        assert_eq!(with("auto").unwrap().params.enable_thinking, Some(false));
        let err = with("maybe").unwrap_err();
        assert!(err.to_string().contains("ENABLE_THINKING"), "{err}");
    }

    /// `parse::<f64>` takes `NaN`, `inf` and negatives without complaint.
    /// A non-finite float serializes to `null`, so `temperature: null` would
    /// reach the provider and the provenance line would read `NaN`; a
    /// negative earns a 400 on every call. Rejected at configuration time,
    /// like the other numeric settings beside it.
    #[test]
    fn a_temperature_that_cannot_go_on_the_wire_is_refused() {
        let with = |v: &str| {
            LlmSettings::from_env_with(env(&[
                ("RGAA_LLM_PROVIDER", "vllm"),
                ("RGAA_LLM_MODEL", "m"),
                ("RGAA_LLM_TEMPERATURE", v),
            ]))
        };
        for bad in ["NaN", "nan", "inf", "-inf", "-0.5", "-1"] {
            let err = with(bad).unwrap_err();
            assert!(
                err.to_string().contains("TEMPERATURE"),
                "{bad} should be refused, got {err}"
            );
        }
        for good in ["0", "0.0", "0.7", "2"] {
            assert!(with(good).is_ok(), "{good} should be accepted");
        }
    }

    /// `RESPONSE_FORMAT` lowercases its input, so `ENABLE_THINKING` beside it
    /// must too — otherwise `TRUE` and `Auto` fail startup while `JSON_OBJECT`
    /// works, and the difference is invisible until someone capitalises one.
    #[test]
    fn the_thinking_flag_ignores_case() {
        let with = |v: &str| {
            LlmSettings::from_env_with(env(&[
                ("RGAA_LLM_PROVIDER", "vllm"),
                ("RGAA_LLM_MODEL", "m"),
                ("RGAA_LLM_ENABLE_THINKING", v),
            ]))
        };
        assert_eq!(with("TRUE").unwrap().params.enable_thinking, Some(true));
        assert_eq!(with("False").unwrap().params.enable_thinking, Some(false));
        assert_eq!(with("Auto").unwrap().params.enable_thinking, Some(false));
        assert_eq!(with("  On  ").unwrap().params.enable_thinking, Some(true));
    }

    #[test]
    fn response_format_and_budget_are_configurable() {
        let s = LlmSettings::from_env_with(env(&[
            ("RGAA_LLM_PROVIDER", "vllm"),
            ("RGAA_LLM_MODEL", "m"),
            ("RGAA_LLM_RESPONSE_FORMAT", "json_schema"),
            ("RGAA_LLM_TEMPERATURE", "0.0"),
            ("RGAA_LLM_MAX_TOKENS", "256"),
        ]))
        .unwrap();
        assert_eq!(s.params.response_format, ResponseFormat::JsonSchema);
        assert_eq!(s.params.temperature, 0.0);
        assert_eq!(s.params.max_tokens, 256);
        assert_eq!(
            s.params.extra_body()["response_format"]["type"],
            "json_schema"
        );

        for (var, value) in [
            ("RGAA_LLM_RESPONSE_FORMAT", "yaml"),
            ("RGAA_LLM_MAX_TOKENS", "0"),
            ("RGAA_LLM_MAX_TOKENS", "lots"),
            ("RGAA_LLM_TEMPERATURE", "warm"),
        ] {
            assert!(
                LlmSettings::from_env_with(env(&[
                    ("RGAA_LLM_PROVIDER", "vllm"),
                    ("RGAA_LLM_MODEL", "m"),
                    (var, value),
                ]))
                .is_err(),
                "{var}={value} should fail closed"
            );
        }
    }

    #[test]
    fn hosted_rate_limits_are_dropped_for_a_self_hosted_endpoint() {
        let hosted =
            LlmSettings::from_env_with(env(&[("HOLO3_API_KEY", "k"), ("RGAA_LLM_MODEL", "m")]))
                .unwrap();
        assert_eq!(hosted.default_rpm(10), 10);
        assert_eq!(hosted.timeout.as_secs(), 30);

        let self_hosted = LlmSettings::from_env_with(env(&[
            ("RGAA_LLM_PROVIDER", "vllm"),
            ("RGAA_LLM_BASE_URL", "http://gpu-box:8000/v1"),
            ("RGAA_LLM_MODEL", "m"),
        ]))
        .unwrap();
        assert_eq!(
            self_hosted.default_rpm(10),
            0,
            "local routes are unthrottled"
        );
        assert_eq!(self_hosted.default_rpm(20), 0);
        assert_eq!(self_hosted.timeout.as_secs(), 600);
    }

    #[test]
    fn provenance_records_the_effective_route_and_params() {
        let s = LlmSettings::from_env_with(env(&[
            ("RGAA_LLM_PROVIDER", "ollama"),
            ("RGAA_LLM_BASE_URL", "http://gpu-box:11434/v1"),
            ("RGAA_LLM_MODEL", "qwen3:8b"),
        ]))
        .unwrap();
        let p = s.provenance(&s.model);
        assert_eq!(p.provider, "ollama");
        assert_eq!(p.model, "qwen3:8b");
        assert_eq!(p.endpoint, "http://gpu-box:11434/v1/chat/completions");
        assert_eq!(p.temperature, crate::completion::DEFAULT_TEMPERATURE);
        assert_eq!(p.max_tokens, crate::completion::DEFAULT_MAX_TOKENS);
        assert_eq!(p.enable_thinking, Some(false));
        assert_eq!(p.response_format, "none");
        assert!(p.summary().contains("thinking=off"), "{}", p.summary());
    }

    // Ticket #440 — the two offers are driven by RGAA_LLM_MODE (D1, P-4).
    // Expected values come from the coordinator's decision, not from the
    // implementation: off/byok => Souverain, managed => Confort.

    #[test]
    fn mode_defaults_to_off_then_byok_when_a_provider_is_configured() {
        let off = LlmSettings::from_env_with(env(&[])).unwrap();
        assert_eq!(off.mode, LlmMode::Off);
        assert_eq!(off.mode.display_name(), "Souverain");
        assert!(!off.mode.allows_model_calls());

        let byok = LlmSettings::from_env_with(env(&[
            ("RGAA_LLM_PROVIDER", "mistral"),
            ("MISTRAL_API_KEY", "k"),
        ]))
        .unwrap();
        assert_eq!(byok.mode, LlmMode::Byok);
        assert_eq!(byok.mode.display_name(), "Souverain");
        assert!(byok.mode.allows_model_calls());
    }

    #[test]
    fn managed_mode_requires_the_instance_gateway_url() {
        let err = LlmSettings::from_env_with(env(&[
            ("RGAA_LLM_PROVIDER", "mistral"),
            ("MISTRAL_API_KEY", "k"),
            ("RGAA_LLM_MODE", "managed"),
        ]))
        .unwrap_err();
        assert!(err.to_string().contains("RGAA_LLM_BASE_URL"), "{err}");

        let s = LlmSettings::from_env_with(env(&[
            ("RGAA_LLM_PROVIDER", "mistral"),
            ("MISTRAL_API_KEY", "k"),
            ("RGAA_LLM_MODE", "managed"),
            ("RGAA_LLM_BASE_URL", "https://gateway.rgaa.example/v1"),
        ]))
        .unwrap();
        assert_eq!(s.mode, LlmMode::Managed);
        assert_eq!(s.mode.display_name(), "Confort");
    }

    #[test]
    fn an_unknown_llm_mode_fails_closed() {
        for bad in ["bidon", "auto", "managed-v2"] {
            let err = LlmSettings::from_env_with(env(&[("RGAA_LLM_MODE", bad)])).unwrap_err();
            assert!(err.to_string().contains("RGAA_LLM_MODE"), "{bad}: {err}");
        }
    }

    #[test]
    fn souverain_mode_refuses_agent_publisher_hosts_and_subdomains() {
        // Versioned list from the coordinator's decision (#440): these are
        // agent publishers or routers that can reach one.
        for url in [
            "https://api.anthropic.com/v1",
            "https://api.openai.com/v1",
            "https://openrouter.ai/api/v1",
            "https://generativelanguage.googleapis.com/v1beta",
            "https://eu.api.openai.com/v1",
        ] {
            let err = LlmSettings::from_env_with(env(&[
                ("RGAA_LLM_PROVIDER", "custom"),
                ("RGAA_LLM_BASE_URL", url),
                ("RGAA_LLM_MODEL", "m"),
            ]))
            .unwrap_err();
            assert!(
                format!("{err:?}").contains("AGENT_PUBLISHER_FORBIDDEN"),
                "{url}: {err:?}"
            );
        }

        // The list is a suffix match on host labels, not a substring match:
        // a client-hosted endpoint whose name merely ends in the same text
        // is not an agent publisher.
        for url in [
            "https://api.mistral.ai/v1",
            "http://localhost:11434/v1",
            "http://10.0.0.5:8000/v1",
            "https://llm.interne.example/v1",
            "https://api.anthropic.com.evil.example/v1",
        ] {
            let s = LlmSettings::from_env_with(env(&[
                ("RGAA_LLM_PROVIDER", "custom"),
                ("RGAA_LLM_BASE_URL", url),
                ("RGAA_LLM_MODEL", "m"),
            ]))
            .unwrap();
            assert_eq!(s.base_url, url, "{url} was refused in Souverain mode");
            assert_eq!(s.mode, LlmMode::Byok);
        }
    }

    #[test]
    fn managed_mode_may_point_at_a_gateway_host() {
        // Confort declares flows to agent publishers (P-1): the refusal is a
        // Souverain guarantee only.
        let s = LlmSettings::from_env_with(env(&[
            ("RGAA_LLM_PROVIDER", "custom"),
            (
                "RGAA_LLM_BASE_URL",
                "https://api.anthropic.example-gateway.net/v1",
            ),
            ("RGAA_LLM_MODEL", "m"),
            ("RGAA_LLM_MODE", "managed"),
        ]))
        .unwrap();
        assert_eq!(s.mode, LlmMode::Managed);
    }

    #[test]
    fn mode_status_reports_mode_provider_host_and_reason() {
        let status = ModeStatus::from_env_with(env(&[])).unwrap();
        assert_eq!(status.mode, LlmMode::Off);
        assert_eq!(status.provider, None);
        assert_eq!(status.host, None);
        assert!(
            status.reason.contains("aucun fournisseur"),
            "{}",
            status.reason
        );

        let status = ModeStatus::from_env_with(env(&[
            ("RGAA_LLM_PROVIDER", "mistral"),
            ("MISTRAL_API_KEY", "k"),
        ]))
        .unwrap();
        assert_eq!(status.mode, LlmMode::Byok);
        assert_eq!(status.provider, Some("mistral"));
        assert_eq!(status.host.as_deref(), Some("api.mistral.ai"));
    }
}
