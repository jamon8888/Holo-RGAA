//! Per-call cost counters (#122).
//!
//! Every model call the agent makes emits one `tracing` event on the
//! [`TARGET`] target carrying tokens, latency and tool-call counts for the
//! criteria that call evaluated, so a cost drift is visible in production
//! without re-running an audit. Only counts and identifiers are recorded:
//! never the prompt, the page content, the reply or any credential.
//!
//! Nothing here exports anywhere. A subscriber (a JSON log layer today, an
//! OpenTelemetry layer later) picks the events up by target.

use rig_agent::agent::PromptResponse;
use rig_core::message::{AssistantContent, Message};
use std::time::Duration;

/// `tracing` target every cost event is emitted on; filter on it to route or
/// drop them, e.g. `RUST_LOG=rgaa_agent::metrics=info`.
pub const TARGET: &str = "rgaa_agent::metrics";

/// What one agent run cost. A run is one `prompt` call: it may span several
/// model calls when the model reaches for a tool.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CallMetrics {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub total_tokens: u64,
    /// Completion requests the run made (1 + one per tool round).
    pub model_calls: u64,
    /// Tool invocations the model requested across the run.
    pub tool_calls: u64,
    pub latency_ms: u64,
}

impl CallMetrics {
    /// Reads the counters off a finished run.
    ///
    /// `total_tokens` falls back to input + output when the provider reports
    /// no total, so the figure is never silently zero for a call that cost
    /// something. A provider that reports no usage at all yields zeros, which
    /// is the honest reading rather than an estimate.
    pub fn from_response(response: &PromptResponse, elapsed: Duration) -> Self {
        let tool_calls = response
            .messages
            .iter()
            .flatten()
            .map(|m| match m {
                Message::Assistant { content, .. } => content
                    .iter()
                    .filter(|c| matches!(c, AssistantContent::ToolCall(_)))
                    .count() as u64,
                _ => 0,
            })
            .sum();
        Self::from_parts(
            response.usage.input_tokens,
            response.usage.output_tokens,
            response.usage.total_tokens,
            response.completion_calls.len() as u64,
            tool_calls,
            elapsed,
        )
    }

    pub fn from_parts(
        input_tokens: u64,
        output_tokens: u64,
        total_tokens: u64,
        model_calls: u64,
        tool_calls: u64,
        elapsed: Duration,
    ) -> Self {
        Self {
            input_tokens,
            output_tokens,
            total_tokens: if total_tokens == 0 {
                input_tokens.saturating_add(output_tokens)
            } else {
                total_tokens
            },
            model_calls,
            tool_calls,
            latency_ms: u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX),
        }
    }

    /// Emits the cost event for a call that returned a reply.
    pub fn emit_ok(&self, criteria: &str, tier: &str) {
        tracing::info!(
            target: "rgaa_agent::metrics",
            criteria,
            tier,
            outcome = "ok",
            latency_ms = self.latency_ms,
            model_calls = self.model_calls,
            tool_calls = self.tool_calls,
            input_tokens = self.input_tokens,
            output_tokens = self.output_tokens,
            total_tokens = self.total_tokens,
            "model call cost"
        );
    }
}

/// Emits the cost event for a call that failed: only the latency is known,
/// the provider reported no usage.
pub fn emit_error(criteria: &str, tier: &str, elapsed: Duration) {
    tracing::info!(
        target: "rgaa_agent::metrics",
        criteria,
        tier,
        outcome = "error",
        latency_ms = u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX),
        "model call cost"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn total_falls_back_to_input_plus_output() {
        let m = CallMetrics::from_parts(120, 30, 0, 1, 0, Duration::from_millis(250));
        assert_eq!(m.total_tokens, 150);
        assert_eq!(m.latency_ms, 250);
    }

    #[test]
    fn a_reported_total_is_kept() {
        let m = CallMetrics::from_parts(120, 30, 400, 2, 1, Duration::ZERO);
        assert_eq!(m.total_tokens, 400);
        assert_eq!((m.model_calls, m.tool_calls), (2, 1));
    }

    #[test]
    fn no_usage_reported_stays_zero() {
        let m = CallMetrics::from_parts(0, 0, 0, 1, 0, Duration::from_secs(1));
        assert_eq!(m.total_tokens, 0);
        assert_eq!(m.latency_ms, 1000);
    }

    #[test]
    fn latency_saturates_instead_of_wrapping() {
        let m = CallMetrics::from_parts(0, 0, 0, 0, 0, Duration::MAX);
        assert_eq!(m.latency_ms, u64::MAX);
    }
}
