//! A VCR-style recording/replay backend — ticket #130.
//!
//! [`CassetteBackend`] implements [`LlmBackend`] but has no HTTP client
//! field at all: it can only answer from an in-memory [`Cassette`] loaded
//! ahead of time, so a run against it is reproducible in CI without ever
//! touching a real provider. A prompt with no matching recorded entry is a
//! hard error ([`RgaaError::Llm`] with code `CASSETTE_MISS`), never a
//! silent fallback to a live call.

use crate::{HoloResponse, LlmBackend};
use async_trait::async_trait;
use rgaa_core::RgaaError;
use serde::{Deserialize, Serialize};

/// One recorded prompt → response exchange, plus the metadata a baseline
/// harness needs for a cost/quality report (ticket #130): how long the
/// original call took and, when the provider reported it, how many tokens
/// it used.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CassetteEntry {
    /// Stable fingerprint of the recorded prompt — see [`prompt_fingerprint`].
    pub prompt_fingerprint: String,
    pub response: HoloResponse,
    /// Recorded latency, in milliseconds.
    #[serde(default)]
    pub duration_ms: u64,
    /// Recorded token usage, when the provider reported it.
    #[serde(default)]
    pub tokens: Option<u32>,
}

/// A set of recorded exchanges, serializable to/from JSON for checking
/// into version control or writing out from a real (live) recording run.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Cassette {
    pub entries: Vec<CassetteEntry>,
}

impl Cassette {
    pub fn new() -> Self {
        Self::default()
    }

    /// Parses a cassette from its JSON serialization (see [`Self::to_json`]).
    ///
    /// # Errors
    /// Returns [`RgaaError::Llm`] if `json` doesn't parse.
    pub fn from_json(json: &str) -> Result<Self, RgaaError> {
        serde_json::from_str(json).map_err(|e| RgaaError::Llm {
            message: format!("failed to parse cassette JSON: {e}"),
            code: Some("CASSETTE_PARSE_ERROR".to_string()),
        })
    }

    /// # Errors
    /// Returns [`RgaaError::Llm`] if serialization fails (should not happen
    /// for this type).
    pub fn to_json(&self) -> Result<String, RgaaError> {
        serde_json::to_string_pretty(self).map_err(|e| RgaaError::Llm {
            message: format!("failed to serialize cassette: {e}"),
            code: Some("CASSETTE_SERIALIZE_ERROR".to_string()),
        })
    }

    /// Records (or overwrites, if `prompt` was already recorded) one
    /// exchange.
    pub fn record(
        &mut self,
        prompt: &str,
        response: HoloResponse,
        duration_ms: u64,
        tokens: Option<u32>,
    ) {
        let fingerprint = prompt_fingerprint(prompt);
        self.entries.retain(|e| e.prompt_fingerprint != fingerprint);
        self.entries.push(CassetteEntry {
            prompt_fingerprint: fingerprint,
            response,
            duration_ms,
            tokens,
        });
    }

    fn find(&self, prompt: &str) -> Option<&CassetteEntry> {
        let fingerprint = prompt_fingerprint(prompt);
        self.entries
            .iter()
            .find(|e| e.prompt_fingerprint == fingerprint)
    }
}

/// A stable FNV-1a fingerprint of `prompt`, used as the cassette lookup
/// key instead of the raw (often huge) prompt text — deterministic and
/// dependency-free, same construction as [`rgaa_core::FindingFingerprint`].
pub fn prompt_fingerprint(prompt: &str) -> String {
    let mut hash = 0xcbf29ce484222325_u64;
    for &byte in prompt.as_bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("rgaa-cassette-v1-{hash:016x}")
}

/// Replays a [`Cassette`] — never makes a network call, by construction
/// (there is no HTTP client field on this type at all). Used by CI runs
/// and the baseline harness to reproduce a prior evaluation run exactly.
pub struct CassetteBackend {
    name: &'static str,
    model: String,
    cassette: Cassette,
}

impl CassetteBackend {
    pub fn new(name: &'static str, model: impl Into<String>, cassette: Cassette) -> Self {
        Self {
            name,
            model: model.into(),
            cassette,
        }
    }

    /// The full recorded [`CassetteEntry`] for `prompt`, including its
    /// `duration_ms`/`tokens` metadata — the baseline harness's cost
    /// numbers come from here, not from [`LlmBackend::evaluate`], which
    /// only returns the [`HoloResponse`] the trait requires.
    pub fn entry_for(&self, prompt: &str) -> Option<&CassetteEntry> {
        self.cassette.find(prompt)
    }
}

#[async_trait]
impl LlmBackend for CassetteBackend {
    fn name(&self) -> &'static str {
        self.name
    }

    fn model(&self) -> &str {
        &self.model
    }

    async fn evaluate(&self, prompt: &str) -> Result<HoloResponse, RgaaError> {
        self.cassette
            .find(prompt)
            .map(|e| e.response.clone())
            .ok_or_else(|| RgaaError::Llm {
                message: format!(
                    "no cassette entry for prompt (fingerprint {}); cassette replay never falls \
                     back to a live call",
                    prompt_fingerprint(prompt)
                ),
                code: Some("CASSETTE_MISS".to_string()),
            })
    }

    async fn evaluate_multimodal(
        &self,
        prompt: &str,
        _image_base64: Option<&str>,
    ) -> Result<HoloResponse, RgaaError> {
        // The cassette key is the text prompt only — a recorded exchange
        // covers both call shapes, since the image (if any) was already
        // part of the original recorded call.
        self.evaluate(prompt).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_response(verdict: &str) -> HoloResponse {
        HoloResponse {
            verdict: verdict.to_string(),
            confidence: 0.9,
            justification: "recorded".to_string(),
        }
    }

    #[tokio::test]
    async fn replays_a_recorded_prompt_exactly() {
        let mut cassette = Cassette::new();
        cassette.record(
            "evaluate criterion 1.1",
            sample_response("fail"),
            120,
            Some(340),
        );

        let backend = CassetteBackend::new("cassette", "holo3-1-35b-a3b", cassette);
        let response = backend.evaluate("evaluate criterion 1.1").await.unwrap();
        assert_eq!(response.verdict, "fail");
        assert_eq!(response.confidence, 0.9);
    }

    #[tokio::test]
    async fn unrecorded_prompt_is_a_hard_error_not_a_live_fallback() {
        let backend = CassetteBackend::new("cassette", "m", Cassette::new());
        let err = backend.evaluate("never recorded").await.unwrap_err();
        assert!(
            err.to_string().contains("no cassette entry")
                || matches!(&err, RgaaError::Llm { code: Some(c), .. } if c == "CASSETTE_MISS")
        );
    }

    #[tokio::test]
    async fn round_trip_through_json_replays_identically() {
        // Simulates the CI use case: a cassette recorded once (e.g. by a
        // live run with real credentials, elsewhere) is checked in as
        // JSON, reloaded fresh here with zero network access, and replays
        // byte-for-byte the same responses.
        let mut original = Cassette::new();
        original.record("prompt A", sample_response("pass"), 100, Some(50));
        original.record("prompt B", sample_response("fail"), 200, None);
        let json = original.to_json().unwrap();

        let reloaded = Cassette::from_json(&json).unwrap();
        assert_eq!(reloaded, original);

        let backend_a = CassetteBackend::new("cassette", "m", reloaded.clone());
        let backend_b = CassetteBackend::new("cassette", "m", reloaded);
        let response_a = backend_a.evaluate("prompt A").await.unwrap();
        let response_b = backend_b.evaluate("prompt A").await.unwrap();
        assert_eq!(response_a.verdict, response_b.verdict);
        assert_eq!(response_a.verdict, "pass");
    }

    #[test]
    fn recording_the_same_prompt_twice_overwrites_not_duplicates() {
        let mut cassette = Cassette::new();
        cassette.record("prompt", sample_response("pass"), 100, None);
        cassette.record("prompt", sample_response("fail"), 150, None);
        assert_eq!(cassette.entries.len(), 1);
        assert_eq!(cassette.entries[0].response.verdict, "fail");
    }

    #[test]
    fn entry_for_exposes_recorded_cost_metadata() {
        let mut cassette = Cassette::new();
        cassette.record("prompt", sample_response("pass"), 250, Some(120));
        let backend = CassetteBackend::new("cassette", "m", cassette);
        let entry = backend.entry_for("prompt").unwrap();
        assert_eq!(entry.duration_ms, 250);
        assert_eq!(entry.tokens, Some(120));
    }
}
