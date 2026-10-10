use rgaa_core::provider::LlmSettings;
use rgaa_core::{CompletionParams, LlmProvenance};
use rig_core::http_client::ReqwestClient;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Runtime configuration for the RGAA agentic evaluator.
///
/// Construct via [`AgentConfig::default`] for local runs or
/// [`AgentConfig::from_env`] to read the provider, models and credentials
/// from the environment — see [`LlmSettings`] for the variables involved.
#[derive(Clone, Deserialize)]
pub struct AgentConfig {
    /// Provider name the settings were resolved from (`myia`, `openai`,
    /// `ollama`, …), recorded for provenance.
    #[serde(default = "default_provider")]
    pub provider: String,
    /// Base URL of the OpenAI-compatible API, without `/chat/completions`.
    pub base_url: String,
    /// API key (redacted in Debug/Serialize). Empty for local providers.
    pub api_key: String,
    /// Default model identifier, used by any tier without its own model.
    pub model: String,
    /// Model for the tactical (fast, cheap) tier. Defaults to [`Self::model`].
    #[serde(default)]
    pub model_tactical: String,
    /// Model for the reasoning (slow, capable) tier. Defaults to [`Self::model`].
    #[serde(default)]
    pub model_reasoning: String,
    /// Per-request timeout applied to the HTTP client both the evaluator and
    /// the verifier talk through. Resolved from `RGAA_LLM_TIMEOUT_SECS`,
    /// defaulting to 30s remote / 600s local (see [`LlmSettings::timeout`]).
    #[serde(default = "default_timeout")]
    pub timeout: Duration,
    /// Filesystem path used by LanceDB for memory and vector storage.
    pub lancedb_path: String,
    /// Embedding backend used for memory and vector retrieval.
    pub embedding_backend: EmbeddingBackendConfig,
    /// Expected embedding dimensionality. Must match the chosen model.
    pub embedding_dimensions: usize,
    /// Conversation-memory retention policy.
    pub memory_retention: MemoryRetention,
    /// Maximum agentic turns per criterion evaluation.
    pub max_turns: usize,
    /// Completion parameters sent on every call this agent makes.
    ///
    /// Replaces the `max_tokens`/`temperature` fields that used to sit here
    /// and reach nothing: the `rig` agent sent neither, so the provider's own
    /// defaults applied while `rgaa-holo`'s transport sent a second,
    /// different pair of its own. Both paths now read this one value — see
    /// [`CompletionParams`].
    #[serde(default)]
    pub params: CompletionParams,
    /// Requests per minute for the tactical (fast) model tier.
    #[serde(default = "default_tactical_rpm")]
    pub tactical_rpm: u32,
    /// Requests per minute for the reasoning (slow) model tier.
    #[serde(default = "default_reasoning_rpm")]
    pub reasoning_rpm: u32,
    /// Maximum concurrent criterion evaluations (LLM calls in flight).
    /// Defaults to `max(1, min(tactical_rpm / 15, 16))` per perf design doc.
    /// Set `RGAA_AGENT_CONCURRENCY` to override.
    #[serde(default = "default_agent_concurrency")]
    pub agent_concurrency: usize,
}

fn default_tactical_rpm() -> u32 {
    10
}

fn default_reasoning_rpm() -> u32 {
    20
}

fn default_provider() -> String {
    "myia".to_string()
}

fn default_timeout() -> Duration {
    Duration::from_secs(30)
}

/// Concurrency for an unthrottled (self-hosted) route.
///
/// `rpm == 0` means "no rate limit" to
/// [`Ratelimiter`](crate::ratelimit::Ratelimiter), but the RPM formula below
/// reads it as "almost no requests" and would pin a GPU box to one call at a
/// time — the opposite of what dropping the limit was for. A fixed, modest
/// default instead: enough to keep a single box busy, not enough to bury it.
/// `RGAA_AGENT_CONCURRENCY` overrides it.
const UNTHROTTLED_CONCURRENCY: usize = 4;

/// `max(1, min(rpm / 15, 16))` — the perf design doc's formula, applied to
/// whichever tactical RPM is actually configured, with `0` (unlimited)
/// handled separately as [`UNTHROTTLED_CONCURRENCY`].
fn agent_concurrency_for(rpm: u32) -> usize {
    if rpm == 0 {
        return UNTHROTTLED_CONCURRENCY;
    }
    ((rpm / 15).clamp(1, 16)) as usize
}

fn default_agent_concurrency() -> usize {
    agent_concurrency_for(default_tactical_rpm())
}

impl std::fmt::Debug for AgentConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AgentConfig")
            .field("provider", &self.provider)
            .field("base_url", &self.base_url)
            .field("api_key", &"<redacted>")
            .field("model", &self.model)
            .field("model_tactical", &self.model_tactical)
            .field("model_reasoning", &self.model_reasoning)
            .field("timeout", &self.timeout)
            .field("lancedb_path", &self.lancedb_path)
            .field("embedding_backend", &self.embedding_backend)
            .field("embedding_dimensions", &self.embedding_dimensions)
            .field("memory_retention", &self.memory_retention)
            .field("max_turns", &self.max_turns)
            .field("params", &self.params)
            .field("tactical_rpm", &self.tactical_rpm)
            .field("reasoning_rpm", &self.reasoning_rpm)
            .finish()
    }
}

impl Serialize for AgentConfig {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        #[derive(Serialize)]
        struct AgentConfigNoKey<'a> {
            provider: &'a str,
            base_url: &'a str,
            model: &'a str,
            model_tactical: &'a str,
            model_reasoning: &'a str,
            timeout_secs: u64,
            lancedb_path: &'a str,
            embedding_backend: &'a EmbeddingBackendConfig,
            embedding_dimensions: usize,
            memory_retention: &'a MemoryRetention,
            max_turns: usize,
            params: &'a CompletionParams,
            tactical_rpm: u32,
            reasoning_rpm: u32,
        }

        let no_key = AgentConfigNoKey {
            provider: &self.provider,
            base_url: &self.base_url,
            model: &self.model,
            model_tactical: self.model_tactical(),
            model_reasoning: self.model_reasoning(),
            timeout_secs: self.timeout.as_secs(),
            lancedb_path: &self.lancedb_path,
            embedding_backend: &self.embedding_backend,
            embedding_dimensions: self.embedding_dimensions,
            memory_retention: &self.memory_retention,
            max_turns: self.max_turns,
            params: &self.params,
            tactical_rpm: self.tactical_rpm,
            reasoning_rpm: self.reasoning_rpm,
        };
        no_key.serialize(serializer)
    }
}

/// Embedding backend selection.
///
/// Only [`EmbeddingBackendConfig::FastEmbed`] is implemented; bring your own
/// backend by extending [`crate::embeddings`] and adding a variant here.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EmbeddingBackendConfig {
    /// On-device embeddings via `fastembed` (`all-MiniLM-L6-v2` by default).
    FastEmbed {
        /// Model name understood by `fastembed`.
        model_name: String,
    },
}

/// Conversation-memory retention policy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MemoryRetention {
    /// Messages live only for the duration of a single audit.
    PerAudit,
    /// Messages persist across audits.
    Persistent,
    /// Short-term messages expire after a TTL; matching long-term patterns persist.
    Hybrid {
        /// Time-to-live for short-term messages.
        short_term_ttl: Duration,
        /// Glob pattern selecting long-term message keys.
        long_term_pattern: String,
    },
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            provider: default_provider(),
            base_url: "https://api.medium.text-generation-webui.myia.io/v1".into(),
            api_key: String::new(),
            model: "swift-1.5-27b".into(),
            model_tactical: String::new(),
            model_reasoning: String::new(),
            timeout: default_timeout(),
            lancedb_path: "./data/lancedb".into(),
            embedding_backend: EmbeddingBackendConfig::FastEmbed {
                model_name: "all-MiniLM-L6-v2".into(),
            },
            embedding_dimensions: 384,
            memory_retention: MemoryRetention::Hybrid {
                short_term_ttl: Duration::from_secs(7 * 24 * 60 * 60),
                long_term_pattern: "findings-*".into(),
            },
            max_turns: 10,
            params: CompletionParams::default(),
            tactical_rpm: default_tactical_rpm(),
            reasoning_rpm: default_reasoning_rpm(),
            agent_concurrency: default_agent_concurrency(),
        }
    }
}

impl AgentConfig {
    /// Builds configuration from environment variables.
    ///
    /// The provider, models and credentials come from [`LlmSettings::from_env`]
    /// — one table of OpenAI-compatible providers shared with `rgaa-holo`, so
    /// `RGAA_LLM_PROVIDER=groq` (or `ollama`, `openrouter`, `custom`, …)
    /// switches the whole app without a recompile. MyIA is the default;
    /// legacy `HOLO3_*` variables select the historical Holo3 route only
    /// when no provider has been explicitly selected.
    ///
    /// # Environment Variables
    /// - `RGAA_LLM_PROVIDER` (optional, default `myia`)
    /// - `RGAA_LLM_MODEL` (optional for providers with a default; MyIA uses
    ///   `swift-1.5-27b`, with `qwen3.6-35b-a3b` also accepted)
    /// - `RGAA_LLM_MODEL_TACTICAL` / `RGAA_LLM_MODEL_REASONING` (optional,
    ///   each defaulting to `RGAA_LLM_MODEL`)
    /// - `RGAA_LLM_API_KEY` or the provider's own key variable (`MYIA_API_KEY`
    ///   for MyIA; legacy `HOLO3_API_KEY` selects Holo3 only when the provider
    ///   is not explicitly set)
    /// - `RGAA_LLM_BASE_URL` (optional; legacy `HOLO3_BASE_URL` applies only
    ///   to the legacy Holo3 route)
    /// - `LANCEDB_PATH` (optional): LanceDB storage path. Defaults to
    ///   `./data/lancedb`.
    /// - `RGAA_TACTICAL_RPM` (optional): Tactical model requests per minute.
    ///   Defaults to 10 against a hosted API and to `0` — unlimited —
    ///   against a self-hosted one (`ollama`, `vllm`, `lmstudio`, `custom`).
    /// - `RGAA_REASONING_RPM` (optional): Reasoning model requests per minute.
    ///   Defaults to 20, or `0` against a self-hosted endpoint.
    /// - `RGAA_LLM_TEMPERATURE`, `RGAA_LLM_MAX_TOKENS`,
    ///   `RGAA_LLM_ENABLE_THINKING`, `RGAA_LLM_RESPONSE_FORMAT` (optional):
    ///   the completion parameters, resolved by
    ///   [`LlmSettings`] and shared with `rgaa-holo`.
    /// - `RGAA_AGENT_CONCURRENCY` (optional): Maximum concurrent criterion
    ///   evaluations. Defaults to `max(1, min(tactical_rpm / 15, 16))` using
    ///   the `RGAA_TACTICAL_RPM` value resolved above. A value of `0` is
    ///   ignored in favour of that default.
    ///
    /// # Errors
    /// Returns [`crate::error::AgentError::Config`] naming the missing or
    /// invalid variable when the LLM route cannot be resolved.
    pub fn from_env() -> Result<Self, crate::error::AgentError> {
        let llm =
            LlmSettings::from_env().map_err(|e| crate::error::AgentError::Config(e.to_string()))?;
        Ok(Self::from_llm_settings(llm))
    }

    /// Builds configuration from already-resolved [`LlmSettings`], leaving
    /// every non-LLM knob at its default except those with their own
    /// environment variables (`LANCEDB_PATH`, the two RPM limits).
    pub fn from_llm_settings(llm: LlmSettings) -> Self {
        // Parsed before the struct literal so the concurrency fallback can be
        // derived from it. The *default* now depends on the route: a hosted
        // API throttles and bills, a box the operator owns does neither, so
        // a self-hosted endpoint runs unthrottled unless told otherwise.
        let tactical_rpm = env_u32("RGAA_TACTICAL_RPM", llm.default_rpm(default_tactical_rpm()));
        let reasoning_rpm = env_u32(
            "RGAA_REASONING_RPM",
            llm.default_rpm(default_reasoning_rpm()),
        );
        Self {
            provider: llm.provider.name.to_string(),
            base_url: llm.base_url,
            api_key: llm.api_key,
            model: llm.model,
            model_tactical: llm.model_tactical,
            model_reasoning: llm.model_reasoning,
            timeout: llm.timeout,
            lancedb_path: std::env::var("LANCEDB_PATH").unwrap_or_else(|_| "./data/lancedb".into()),
            params: llm.params,
            tactical_rpm,
            reasoning_rpm,
            // Zero is dropped rather than honoured: it would reach
            // `buffer_unordered(0)`, which never polls its source stream and
            // leaves an audit pending forever. Falling back matches how the
            // RPM variables treat an unparseable value.
            agent_concurrency: std::env::var("RGAA_AGENT_CONCURRENCY")
                .ok()
                .and_then(|v| v.parse::<usize>().ok())
                .filter(|&v| v > 0)
                // Derived from the RPM actually configured above, not from the
                // hardcoded default: raising RGAA_TACTICAL_RPM must raise the
                // parallelism the formula documents.
                .unwrap_or_else(|| agent_concurrency_for(tactical_rpm)),
            ..Default::default()
        }
    }

    /// Model for the tactical tier, falling back to [`Self::model`] when no
    /// tier-specific model was configured.
    #[must_use]
    pub fn model_tactical(&self) -> &str {
        if self.model_tactical.is_empty() {
            &self.model
        } else {
            &self.model_tactical
        }
    }

    /// Model for the reasoning tier, falling back to [`Self::model`] when no
    /// tier-specific model was configured.
    #[must_use]
    pub fn model_reasoning(&self) -> &str {
        if self.model_reasoning.is_empty() {
            &self.model
        } else {
            &self.model_reasoning
        }
    }

    /// Builds the HTTP backend the `rig` clients run on, carrying
    /// [`Self::timeout`]. `rig`'s default client has no timeout of its own,
    /// so without this a slow local backend would hang past the configured
    /// limit and a hung remote one would never be cut off.
    ///
    /// Built from `rig`'s own re-exported reqwest (`ReqwestClient`), not the
    /// workspace's: the workspace is on reqwest 0.12 and rig on 0.13, and
    /// only rig's own type implements the `HttpClientExt` its client builder
    /// requires.
    ///
    /// # Errors
    /// Returns [`crate::error::AgentError::Config`] if the HTTP client cannot
    /// be built (e.g. TLS initialization failure).
    pub fn http_client(&self) -> Result<ReqwestClient, crate::error::AgentError> {
        ReqwestClient::builder()
            .timeout(self.timeout)
            .build()
            .map_err(|e| crate::error::AgentError::Config(format!("HTTP client init failed: {e}")))
    }

    /// What a call on `model` through this configuration records as having
    /// run with: the effective model, temperature, token budget and thinking
    /// flag, so a bake-off run stays auditable after the fact.
    #[must_use]
    pub fn provenance(&self, model: &str) -> LlmProvenance {
        LlmProvenance::new(
            self.provider.clone(),
            model,
            format!("{}/chat/completions", self.base_url.trim_end_matches('/')),
            &self.params,
        )
    }

    /// True when both tiers resolve to the same model — the single-model
    /// case, where callers can build one client instead of two.
    #[must_use]
    pub fn tiers_share_one_model(&self) -> bool {
        self.model_tactical() == self.model_reasoning()
    }
}

fn env_u32(name: &str, default: u32) -> u32 {
    std::env::var(name)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings(tactical: &str, reasoning: &str) -> AgentConfig {
        AgentConfig {
            model: "base".into(),
            model_tactical: tactical.into(),
            model_reasoning: reasoning.into(),
            ..Default::default()
        }
    }

    #[test]
    fn empty_tier_models_fall_back_to_the_default_model() {
        let c = settings("", "");
        assert_eq!(c.model_tactical(), "base");
        assert_eq!(c.model_reasoning(), "base");
        assert!(c.tiers_share_one_model());
    }

    #[test]
    fn configured_tier_models_win() {
        let c = settings("small", "big");
        assert_eq!(c.model_tactical(), "small");
        assert_eq!(c.model_reasoning(), "big");
        assert!(!c.tiers_share_one_model());
    }

    #[test]
    fn one_tier_can_be_overridden_alone() {
        let c = settings("", "big");
        assert_eq!(c.model_tactical(), "base");
        assert_eq!(c.model_reasoning(), "big");
        assert!(!c.tiers_share_one_model());
    }

    #[test]
    fn serialization_omits_the_api_key_and_resolves_tiers() {
        let c = AgentConfig {
            api_key: "sk-super-secret".into(),
            model: "base".into(),
            model_tactical: String::new(),
            ..Default::default()
        };
        let json = serde_json::to_string(&c).unwrap();
        assert!(!json.contains("sk-super-secret"), "{json}");
        assert!(!json.contains("api_key"), "{json}");
        // The empty tier is serialized resolved, not blank.
        assert!(json.contains(r#""model_tactical":"base""#), "{json}");
    }

    #[test]
    fn debug_redacts_the_api_key() {
        let c = AgentConfig {
            api_key: "sk-super-secret".into(),
            ..Default::default()
        };
        assert!(!format!("{c:?}").contains("sk-super-secret"));
    }

    #[test]
    fn from_llm_settings_carries_the_resolved_timeout() {
        // A local provider's long default must survive into the agent config,
        // otherwise the advertised 600s never reaches the HTTP client.
        let llm = LlmSettings::from_env_with(|k| {
            match k {
                "RGAA_LLM_PROVIDER" => Some("ollama"),
                "RGAA_LLM_MODEL" => Some("qwen2.5:14b-instruct"),
                _ => None,
            }
            .map(str::to_string)
        })
        .unwrap();
        let c = AgentConfig::from_llm_settings(llm);
        assert_eq!(c.timeout.as_secs(), 600);
        assert!(c.http_client().is_ok());
    }

    #[test]
    fn from_llm_settings_carries_provider_and_models() {
        let llm = LlmSettings::from_env_with(|k| {
            match k {
                "RGAA_LLM_PROVIDER" => Some("groq"),
                "RGAA_LLM_API_KEY" => Some("k"),
                "RGAA_LLM_MODEL" => Some("llama-3.3-70b-versatile"),
                "RGAA_LLM_MODEL_TACTICAL" => Some("llama-3.1-8b-instant"),
                _ => None,
            }
            .map(str::to_string)
        })
        .unwrap();
        let c = AgentConfig::from_llm_settings(llm);
        assert_eq!(c.provider, "groq");
        assert_eq!(c.base_url, "https://api.groq.com/openai/v1");
        assert_eq!(c.model_tactical(), "llama-3.1-8b-instant");
        assert_eq!(c.model_reasoning(), "llama-3.3-70b-versatile");
    }

    #[test]
    fn concurrency_follows_the_configured_rpm_not_the_default() {
        // The formula is documented as max(1, min(rpm / 15, 16)). Deriving it
        // from the hardcoded default instead pinned every deployment to 1.
        assert_eq!(agent_concurrency_for(10), 1);
        assert_eq!(agent_concurrency_for(60), 4);
        assert_eq!(agent_concurrency_for(300), 16);
        // Clamped at the top; `0` is unlimited and handled separately
        // (see `an_unthrottled_route_does_not_collapse_to_one_call_at_a_time`).
        assert_eq!(agent_concurrency_for(1), 1);
        assert_eq!(agent_concurrency_for(100_000), 16);
        assert_eq!(default_agent_concurrency(), agent_concurrency_for(10));
    }

    #[test]
    fn completion_params_come_from_the_resolved_route() {
        // #193: the agent used to carry its own dead 4096/0.3 while the
        // transport hardcoded 512/0.1. There is one source now, and this is
        // the agent side of it.
        let llm = LlmSettings::from_env_with(|k| {
            match k {
                "RGAA_LLM_PROVIDER" => Some("vllm"),
                "RGAA_LLM_MODEL" => Some("qwen3-8b"),
                "RGAA_LLM_TEMPERATURE" => Some("0.05"),
                "RGAA_LLM_MAX_TOKENS" => Some("1234"),
                _ => None,
            }
            .map(str::to_string)
        })
        .unwrap();
        let c = AgentConfig::from_llm_settings(llm);
        assert_eq!(c.params.temperature, 0.05);
        assert_eq!(c.params.max_tokens, 1234);
        assert_eq!(c.params.enable_thinking, Some(false));

        // The default config carries the same shared constants, not a
        // second opinion of its own.
        let d = AgentConfig::default();
        assert_eq!(d.params.temperature, rgaa_core::DEFAULT_TEMPERATURE);
        assert_eq!(d.params.max_tokens, rgaa_core::DEFAULT_MAX_TOKENS);
    }

    #[test]
    fn provenance_reports_the_effective_route() {
        let c = AgentConfig {
            provider: "vllm".into(),
            base_url: "http://gpu-box:8000/v1/".into(),
            model: "qwen3-8b".into(),
            ..Default::default()
        };
        let p = c.provenance(c.model_tactical());
        assert_eq!(p.provider, "vllm");
        assert_eq!(p.model, "qwen3-8b");
        // The trailing slash must not produce `/v1//chat/completions`.
        assert_eq!(p.endpoint, "http://gpu-box:8000/v1/chat/completions");
        assert_eq!(p.max_tokens, rgaa_core::DEFAULT_MAX_TOKENS);
    }

    #[test]
    fn an_unthrottled_route_does_not_collapse_to_one_call_at_a_time() {
        // 0 rpm means unlimited to the rate limiter; reading it through the
        // rpm/15 formula would have pinned a self-hosted box to 1.
        assert_eq!(agent_concurrency_for(0), UNTHROTTLED_CONCURRENCY);
        const { assert!(UNTHROTTLED_CONCURRENCY > 1) };
    }

    #[test]
    fn concurrency_is_never_zero() {
        // `buffer_unordered(0)` never polls its source stream, so a zero here
        // hangs an audit instead of failing it.
        assert!(agent_concurrency_for(0) >= 1);
        assert!(default_agent_concurrency() >= 1);
    }
}
