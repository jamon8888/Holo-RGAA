use rgaa_core::CriterionStatus;
use rgaa_holo::HoloResponse;

pub const CONFIDENCE_THRESHOLD: f64 = 0.6;

/// Map a HoloResponse to a CriterionStatus, applying confidence threshold.
///
/// - confidence < 0.6 → NeedsReview (human reviews low-confidence verdicts)
/// - verdict "pass"/"conforme" + confidence >= 0.6 → Pass
/// - verdict "fail"/"non_conforme" + confidence >= 0.6 → Fail
/// - unknown verdict → NeedsReview
pub fn map_verdict(response: HoloResponse) -> CriterionStatus {
    if response.confidence < CONFIDENCE_THRESHOLD {
        return CriterionStatus::NeedsReview;
    }

    match response.verdict.as_str() {
        "pass" | "conforme" => CriterionStatus::Pass,
        "fail" | "non_conforme" => CriterionStatus::Fail,
        _ => CriterionStatus::NeedsReview,
    }
}

/// Evidence trace for a single action during act→verify loop.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ActionTrace {
    pub tool: String,
    pub ref_id: Option<String>,
    pub key: Option<String>,
    pub text: Option<String>,
    pub resulting_focused_element: Option<String>,
    pub timestamp_ms: u64,
}

/// Structured evidence for a criterion evaluation.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CriterionEvidence {
    pub screenshot: Option<String>,
    pub actions_taken: Vec<ActionTrace>,
    pub page_context_snapshot: Option<String>,
}
