//! The one place completion parameters are defined.
//!
//! Before this module the two LLM code paths disagreed: `rgaa-holo`'s raw
//! transport hardcoded `temperature: 0.1` / `max_tokens: 512` while the
//! `rig`-based agent sent neither, so the provider's own defaults applied and
//! two runs of the same bake-off were not comparable. [`CompletionParams`] is
//! now resolved once, next to the provider route (see
//! [`LlmSettings`](crate::LlmSettings)), and *both* paths read it.
//!
//! It also carries the two knobs an OpenAI-*compatible* server needs that the
//! hosted API never did:
//!
//! * [`CompletionParams::enable_thinking`] — reasoning models served by vLLM
//!   or Ollama emit thinking tokens that are billed against `max_tokens`,
//!   which truncated the verdict JSON and tripped the circuit breaker.
//! * [`CompletionParams::response_format`] — JSON-object or JSON-schema
//!   constrained decoding, which the design doc promised and the transport
//!   never sent.

use serde::{Deserialize, Serialize};

/// Sampling temperature used by every path.
///
/// 0.1 rather than the agent config's dead 0.3: a bake-off compares verdicts
/// across models, so the sampler must contribute as little variance as it can.
pub const DEFAULT_TEMPERATURE: f64 = 0.1;

/// Completion budget used by every path.
///
/// 4096 rather than the transport's hardcoded 512. The agent path asks for a
/// batch of five verdicts, each with a French justification, in one reply;
/// 512 truncated that reply, and a truncated reply parses as *no* verdicts.
/// A single-verdict call simply stops well short of the ceiling.
pub const DEFAULT_MAX_TOKENS: u32 = 4096;

/// How the server is asked to constrain its output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ResponseFormat {
    /// Send no `response_format` at all — the historical behaviour, and the
    /// default: a server that does not know the field answers 400, and the
    /// extractor in `rgaa-holo` copes with fenced and prose-wrapped JSON
    /// anyway.
    #[default]
    None,
    /// `{"type": "json_object"}` — valid JSON, shape unconstrained.
    JsonObject,
    /// `{"type": "json_schema", ...}` carrying [`verdict_schema`] — supported
    /// by vLLM (guided decoding), recent Ollama and OpenAI.
    JsonSchema,
}

impl ResponseFormat {
    /// Parses the `RGAA_LLM_RESPONSE_FORMAT` spelling.
    #[must_use]
    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "none" | "off" | "" => Some(Self::None),
            "json_object" | "json" => Some(Self::JsonObject),
            "json_schema" | "schema" => Some(Self::JsonSchema),
            _ => None,
        }
    }

    /// The `response_format` value to put in the request body, or `None` when
    /// nothing should be sent.
    #[must_use]
    pub fn body_value(self) -> Option<serde_json::Value> {
        match self {
            Self::None => None,
            Self::JsonObject => Some(serde_json::json!({"type": "json_object"})),
            Self::JsonSchema => Some(serde_json::json!({
                "type": "json_schema",
                "json_schema": {
                    "name": "rgaa_verdict",
                    "strict": true,
                    "schema": verdict_schema(),
                }
            })),
        }
    }

    /// Name used in logs and provenance.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::JsonObject => "json_object",
            Self::JsonSchema => "json_schema",
        }
    }
}

/// JSON schema of the single-criterion verdict every backend is asked for —
/// the wire shape of `rgaa_holo::HoloResponse`.
///
/// Kept here rather than in `rgaa-holo` so `response_format` can be resolved
/// at configuration time, before any backend exists.
#[must_use]
pub fn verdict_schema() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["verdict", "confidence", "justification"],
        "properties": {
            "verdict": {"type": "string", "enum": ["pass", "fail", "na"]},
            "confidence": {"type": "number", "minimum": 0.0, "maximum": 1.0},
            "justification": {"type": "string"},
        }
    })
}

/// Completion parameters shared by the raw transport and the `rig` agent.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CompletionParams {
    /// Sampling temperature. Default [`DEFAULT_TEMPERATURE`].
    pub temperature: f64,
    /// Completion budget. Default [`DEFAULT_MAX_TOKENS`].
    pub max_tokens: u32,
    /// Whether the server should run its thinking phase.
    ///
    /// `None` means *say nothing*: a hosted API that does not know the field
    /// rejects the whole request, so the keys are only emitted when there is
    /// a reason to. The default is `Some(false)` for a local runtime (vLLM,
    /// Ollama, LM Studio, `custom`), which is where reasoning models are
    /// served and where the thinking tokens ate the completion budget, and
    /// `None` for a hosted provider. `RGAA_LLM_ENABLE_THINKING=true|false`
    /// forces it either way; `auto` restores the per-provider default.
    pub enable_thinking: Option<bool>,
    /// Output constraint. Default [`ResponseFormat::None`].
    pub response_format: ResponseFormat,
}

impl Default for CompletionParams {
    fn default() -> Self {
        Self {
            temperature: DEFAULT_TEMPERATURE,
            max_tokens: DEFAULT_MAX_TOKENS,
            enable_thinking: None,
            response_format: ResponseFormat::None,
        }
    }
}

impl CompletionParams {
    /// Request-body keys that switch the server's thinking phase on or off.
    ///
    /// There is no standard for this, so all three spellings in circulation
    /// are sent together and whichever the server understands wins:
    ///
    /// * `chat_template_kwargs.enable_thinking` — vLLM and SGLang, consumed
    ///   by the Qwen3 / DeepSeek-R1 chat templates.
    /// * `think` — Ollama.
    /// * `enable_thinking` — top-level, accepted by several forks.
    ///
    /// Servers that do not know a key ignore it; the ones that would *reject*
    /// it are the hosted APIs, which is why [`Self::enable_thinking`] is
    /// `None` for them and this returns an empty map.
    #[must_use]
    pub fn thinking_body(&self) -> serde_json::Map<String, serde_json::Value> {
        let mut map = serde_json::Map::new();
        if let Some(on) = self.enable_thinking {
            map.insert(
                "chat_template_kwargs".to_string(),
                serde_json::json!({"enable_thinking": on}),
            );
            map.insert("think".to_string(), serde_json::Value::Bool(on));
            map.insert("enable_thinking".to_string(), serde_json::Value::Bool(on));
        }
        map
    }

    /// Everything beyond `model`/`messages`/`temperature`/`max_tokens` that
    /// belongs in the request body: the thinking keys plus `response_format`.
    /// Empty when nothing extra is configured.
    #[must_use]
    pub fn extra_body(&self) -> serde_json::Map<String, serde_json::Value> {
        let mut map = self.thinking_body();
        if let Some(rf) = self.response_format.body_value() {
            map.insert("response_format".to_string(), rf);
        }
        map
    }

    /// As [`Self::extra_body`], without `response_format`.
    ///
    /// The `rig` agent path uses this: it asks one prompt for an *array* of
    /// five verdicts and another for a single object, from the same agent,
    /// and a `response_format` is fixed per agent — constraining it to the
    /// single-verdict schema would make the batch prompt unanswerable. The
    /// thinking keys have no such conflict.
    #[must_use]
    pub fn extra_body_without_response_format(&self) -> serde_json::Map<String, serde_json::Value> {
        self.thinking_body()
    }
}

/// The parameters a call *actually* ran with, recorded next to the verdict
/// so a bake-off result can be audited after the fact.
///
/// Built by whichever backend made the call, from the same
/// [`CompletionParams`] it sent — not from the configuration a reader might
/// re-resolve later, which may have moved on.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LlmProvenance {
    /// Provider preset the route resolved to (`vllm`, `ollama`, `holo3`, …).
    pub provider: String,
    /// Model identifier sent on the wire.
    pub model: String,
    /// Endpoint the request went to, so two runs of the same model on
    /// different boxes stay distinguishable.
    pub endpoint: String,
    /// Effective sampling temperature.
    pub temperature: f64,
    /// Effective completion budget.
    pub max_tokens: u32,
    /// Effective thinking flag; `None` when the request said nothing about
    /// it and the server's own default applied.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enable_thinking: Option<bool>,
    /// Effective output constraint (`none`, `json_object`, `json_schema`).
    pub response_format: String,
}

impl LlmProvenance {
    /// Records `params` as sent by `provider`/`model` at `endpoint`.
    #[must_use]
    pub fn new(
        provider: impl Into<String>,
        model: impl Into<String>,
        endpoint: impl Into<String>,
        params: &CompletionParams,
    ) -> Self {
        Self {
            provider: provider.into(),
            model: model.into(),
            endpoint: endpoint.into(),
            temperature: params.temperature,
            max_tokens: params.max_tokens,
            enable_thinking: params.enable_thinking,
            response_format: params.response_format.as_str().to_string(),
        }
    }

    /// One-line rendering for logs and for a report footer.
    #[must_use]
    pub fn summary(&self) -> String {
        let thinking = match self.enable_thinking {
            Some(true) => "thinking=on",
            Some(false) => "thinking=off",
            None => "thinking=default",
        };
        format!(
            "{}/{} temp={} max_tokens={} {thinking} response_format={}",
            self.provider, self.model, self.temperature, self.max_tokens, self.response_format
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_the_single_source_of_truth() {
        let p = CompletionParams::default();
        assert_eq!(p.temperature, DEFAULT_TEMPERATURE);
        assert_eq!(p.max_tokens, DEFAULT_MAX_TOKENS);
        assert_eq!(p.enable_thinking, None);
        assert_eq!(p.response_format, ResponseFormat::None);
        // Nothing extra is sent unless something was configured, so a hosted
        // API that rejects unknown body keys keeps working untouched.
        assert!(p.extra_body().is_empty());
    }

    #[test]
    fn thinking_body_sends_every_spelling() {
        let p = CompletionParams {
            enable_thinking: Some(false),
            ..Default::default()
        };
        let body = p.thinking_body();
        assert_eq!(
            body["chat_template_kwargs"],
            serde_json::json!({"enable_thinking": false})
        );
        assert_eq!(body["think"], serde_json::json!(false));
        assert_eq!(body["enable_thinking"], serde_json::json!(false));
    }

    #[test]
    fn response_format_shapes() {
        assert!(ResponseFormat::None.body_value().is_none());
        assert_eq!(
            ResponseFormat::JsonObject.body_value().unwrap(),
            serde_json::json!({"type": "json_object"})
        );
        let schema = ResponseFormat::JsonSchema.body_value().unwrap();
        assert_eq!(schema["type"], "json_schema");
        assert_eq!(schema["json_schema"]["name"], "rgaa_verdict");
        assert_eq!(
            schema["json_schema"]["schema"]["properties"]["verdict"]["enum"],
            serde_json::json!(["pass", "fail", "na"])
        );
    }

    #[test]
    fn response_format_parses_its_spellings() {
        assert_eq!(
            ResponseFormat::parse("json"),
            Some(ResponseFormat::JsonObject)
        );
        assert_eq!(
            ResponseFormat::parse("  JSON_SCHEMA "),
            Some(ResponseFormat::JsonSchema)
        );
        assert_eq!(ResponseFormat::parse("off"), Some(ResponseFormat::None));
        assert_eq!(ResponseFormat::parse("yaml"), None);
    }

    #[test]
    fn the_rig_path_drops_response_format_but_keeps_thinking() {
        let p = CompletionParams {
            enable_thinking: Some(false),
            response_format: ResponseFormat::JsonSchema,
            ..Default::default()
        };
        assert!(p.extra_body().contains_key("response_format"));
        let rig = p.extra_body_without_response_format();
        assert!(!rig.contains_key("response_format"));
        assert!(rig.contains_key("chat_template_kwargs"));
    }
}
