use rgaa_core::CriterionStatus;
use rgaa_holo::HoloResponse;

/// Minimum confidence for a verdict to be accepted without human review.
///
/// Below this threshold the criterion is escalated to [`CriterionStatus::NeedsReview`].
pub const CONFIDENCE_THRESHOLD: f64 = 0.6;

/// Map a HoloResponse to a CriterionStatus, applying confidence threshold.
///
/// - confidence < 0.6 → NeedsReview (human reviews low-confidence verdicts)
/// - verdict "pass"/"conforme" + confidence >= 0.6 → Pass
/// - verdict "fail"/"non_conforme" + confidence >= 0.6 → Fail
/// - verdict "na"/"non_applicable" + confidence >= 0.6 → NotApplicable
/// - unknown verdict → NeedsReview
pub fn map_verdict(response: &HoloResponse) -> CriterionStatus {
    if response.confidence < CONFIDENCE_THRESHOLD {
        return CriterionStatus::NeedsReview;
    }

    match response.verdict.as_str() {
        "pass" | "conforme" => CriterionStatus::Pass,
        "fail" | "non_conforme" => CriterionStatus::Fail,
        "na" | "non_applicable" => CriterionStatus::NotApplicable,
        _ => CriterionStatus::NeedsReview,
    }
}

/// Evidence trace for a single action during act→verify loop.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ActionTrace {
    /// Tool name that was invoked.
    pub tool: String,
    /// Optional reference ID.
    pub ref_id: Option<String>,
    /// Optional key identifier.
    pub key: Option<String>,
    /// Optional text associated with the action.
    pub text: Option<String>,
    /// Optional resulting focused element selector.
    pub resulting_focused_element: Option<String>,
    /// Timestamp of the action in milliseconds since epoch.
    pub timestamp_ms: u64,
}

/// Structured evidence for a criterion evaluation.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CriterionEvidence {
    /// Optional base64-encoded screenshot.
    pub screenshot: Option<String>,
    /// Sequence of actions taken during evaluation.
    pub actions_taken: Vec<ActionTrace>,
    /// Optional snapshot of the page context at evaluation time.
    pub page_context_snapshot: Option<String>,
}
