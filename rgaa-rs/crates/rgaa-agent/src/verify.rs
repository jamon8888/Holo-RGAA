use rgaa_core::{
    AutomatedVerdict, Criterion, CriterionResult, CriterionStatus, EvidenceRef, TestOutcome,
    TestRoutePlan, VerdictBasis,
};
use rgaa_holo::HoloResponse;
use std::collections::{HashMap, HashSet};

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

/// Map a JSON array of structured estimates onto exactly the requested criteria.
///
/// Invalid, absent or duplicate answers remain unresolved. Model estimates never
/// establish a verified status, including when the model claims inapplicability.
/// Confidence is the raw model value; no calibration or threshold is applied.
#[must_use]
pub fn map_automatic_response(
    criteria: &[Criterion],
    response_json: &str,
) -> HashMap<String, CriterionResult> {
    let Ok(items) = serde_json::from_str::<Vec<serde_json::Value>>(response_json) else {
        return unresolved_automatic_results(criteria, "malformed automatic estimate JSON");
    };
    let mut results = HashMap::with_capacity(criteria.len());
    for criterion in criteria {
        let mut matches = items.iter().filter(|item| {
            item.get("criterion_id").and_then(serde_json::Value::as_str) == Some(criterion.id)
        });
        let response = matches.next().filter(|_| matches.next().is_none());
        let result = response
            .and_then(|item| serde_json::from_value::<AutomaticResponse>(item.clone()).ok())
            .and_then(|response| validated_estimate(criterion, response))
            .unwrap_or_else(|| {
                unresolved_automatic_result(
                    criterion,
                    "missing, duplicate or invalid automatic estimate",
                )
            });
        results.insert(criterion.id.to_owned(), result);
    }
    results
}

#[derive(serde::Deserialize)]
struct AutomaticResponse {
    tests: Vec<AutomaticTestResponse>,
    verdict: String,
    justification: String,
    confidence: f64,
    // Required by the wire contract, but cannot waive review of an estimate.
    #[serde(rename = "review_required")]
    _review_required: bool,
    #[serde(default)]
    evidence: Vec<EvidenceRef>,
}

#[derive(serde::Deserialize)]
struct AutomaticTestResponse {
    test_key: String,
    verdict: String,
    justification: String,
}

fn estimate_verdict(value: &str) -> Option<AutomatedVerdict> {
    match value {
        "pass" => Some(AutomatedVerdict::Pass),
        "fail" => Some(AutomatedVerdict::Fail),
        _ => None,
    }
}

fn validated_estimate(
    criterion: &Criterion,
    response: AutomaticResponse,
) -> Option<CriterionResult> {
    let verdict = estimate_verdict(&response.verdict)?;
    if !response.confidence.is_finite()
        || !(0.0..=1.0).contains(&response.confidence)
        || response.justification.trim().is_empty()
    {
        return None;
    }
    let routes = TestRoutePlan::builtin();
    let expected: Vec<_> = routes
        .routes()
        .iter()
        .filter(|route| route.criterion_id == criterion.id && route.fallback == "holo_estimate")
        .collect();
    if expected.is_empty() || response.tests.len() != expected.len() {
        return None;
    }
    let mut seen = HashSet::with_capacity(expected.len());
    let mut tests = Vec::with_capacity(expected.len());
    let mut any_fail = false;
    for test in response.tests {
        if !seen.insert(test.test_key.clone())
            || !routes
                .for_test(criterion.id, &test.test_key)
                .is_some_and(|route| route.fallback == "holo_estimate")
            || test.justification.trim().is_empty()
        {
            return None;
        }
        let test_verdict = estimate_verdict(&test.verdict)?;
        any_fail |= test_verdict == AutomatedVerdict::Fail;
        tests.push(TestOutcome {
            test_key: test.test_key,
            status: match test_verdict {
                AutomatedVerdict::Pass => CriterionStatus::Pass,
                _ => CriterionStatus::Fail,
            },
            source: "agent-estimate".to_owned(),
            evidence: Some(test.justification),
        });
    }
    if (verdict == AutomatedVerdict::Fail) != any_fail {
        return None;
    }
    // Canonical route order makes reordered model responses reproducible.
    tests.sort_by_key(|test| {
        expected
            .iter()
            .position(|route| route.test_key == test.test_key)
    });
    let evidence: Vec<_> = response
        .evidence
        .into_iter()
        .filter(|reference| !reference.kind.trim().is_empty() && !reference.hash.trim().is_empty())
        .collect();
    let reason = if evidence.is_empty() {
        "model estimate has an evidence gap: no auditable evidence references were supplied"
    } else {
        "model estimate requires independent review of the supplied evidence references"
    };
    let mut result = unresolved_automatic_result(criterion, reason);
    result.source = "agent-estimate".to_owned();
    result.automated_verdict = Some(verdict);
    result.verdict_basis = vec![VerdictBasis::ModelEstimate];
    result.tests = tests;
    result.confidence = Some(response.confidence);
    result.justification = Some(response.justification);
    result.evidence = evidence;
    result.review_reason = Some(reason.to_owned());
    Some(result)
}

pub(crate) fn unresolved_automatic_results(
    criteria: &[Criterion],
    reason: &str,
) -> HashMap<String, CriterionResult> {
    criteria
        .iter()
        .map(|criterion| {
            (
                criterion.id.to_owned(),
                unresolved_automatic_result(criterion, reason),
            )
        })
        .collect()
}

fn unresolved_automatic_result(criterion: &Criterion, reason: &str) -> CriterionResult {
    CriterionResult {
        criterion_id: criterion.id.to_owned(),
        title: criterion.title.clone(),
        classification: criterion.classification,
        status: CriterionStatus::NeedsReview,
        violations: Vec::new(),
        confidence: None,
        justification: Some(reason.to_owned()),
        source: "agent-estimate-incomplete".to_owned(),
        citations: Vec::new(),
        considered_sources: Vec::new(),
        tests: Vec::new(),
        automated_verdict: None,
        verdict_basis: Vec::new(),
        evidence: Vec::new(),
        confidence_calibration_version: None,
        review_required: true,
        review_reason: Some(format!(
            "{reason}; evidence gap: no auditable evidence references are available"
        )),
        verified_status: None,
        review_events: Vec::new(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use rgaa_core::{AutomatedVerdict, RgaaCriteria, VerdictBasis};
    use serde_json::{json, Value};

    fn response(verdict: &str) -> Value {
        json!({"criterion_id":"4.2", "tests":[
            {"test_key":"1","verdict":verdict,"justification":"observed media"},
            {"test_key":"2","verdict":verdict,"justification":"observed media"},
            {"test_key":"3","verdict":verdict,"justification":"observed media"}
        ], "verdict":verdict, "justification":"observed media",
           "confidence":0.72, "review_required":false})
    }

    fn mapped(value: Value) -> rgaa_core::CriterionResult {
        let criteria = vec![RgaaCriteria::find("4.2").unwrap().clone()];
        map_automatic_response(&criteria, &value.to_string())
            .remove("4.2")
            .unwrap()
    }

    fn assert_unresolved(result: &rgaa_core::CriterionResult) {
        assert_eq!(result.status, CriterionStatus::NeedsReview);
        assert_eq!(result.automated_verdict, None);
        assert_eq!(result.verified_status, None);
        assert!(result.review_required);
        assert!(result
            .review_reason
            .as_deref()
            .unwrap()
            .contains("evidence gap"));
    }

    #[test]
    fn automatic_pass_and_fail_remain_unverified_estimates() {
        for (verdict, expected) in [
            ("pass", AutomatedVerdict::Pass),
            ("fail", AutomatedVerdict::Fail),
        ] {
            let result = mapped(json!([response(verdict)]));
            assert_eq!(result.automated_verdict, Some(expected));
            assert_eq!(result.status, CriterionStatus::NeedsReview);
            assert_eq!(result.verified_status, None);
            assert_eq!(result.verdict_basis, vec![VerdictBasis::ModelEstimate]);
            assert_eq!(result.confidence, Some(0.72));
            assert_eq!(result.confidence_calibration_version, None);
            assert!(result.review_required);
            assert!(result
                .review_reason
                .as_deref()
                .unwrap()
                .contains("evidence"));
            assert_eq!(
                result
                    .tests
                    .iter()
                    .map(|t| t.test_key.as_str())
                    .collect::<Vec<_>>(),
                vec!["1", "2", "3"]
            );
            assert!(result.tests.iter().all(|t| t.source == "agent-estimate"));
        }
    }

    #[test]
    fn invalid_or_not_applicable_verdict_cannot_establish_conformance() {
        for verdict in ["unknown", "not_applicable", "na"] {
            assert_unresolved(&mapped(json!([response(verdict)])));
        }
    }

    #[test]
    fn missing_and_duplicate_criterion_ids_remain_unresolved() {
        let mut missing = response("pass");
        missing.as_object_mut().unwrap().remove("criterion_id");
        for value in [
            json!([missing]),
            json!([response("pass"), response("pass")]),
            json!([]),
        ] {
            assert_unresolved(&mapped(value));
        }
    }

    #[test]
    fn malformed_json_retains_every_requested_id() {
        let criteria = vec![RgaaCriteria::find("4.2").unwrap().clone()];
        for text in ["not json", "[{", "{}"] {
            let results = map_automatic_response(&criteria, text);
            assert_eq!(results.len(), 1);
            assert_unresolved(&results["4.2"]);
        }
    }

    #[test]
    fn incomplete_batch_keeps_valid_estimates_and_unresolved_ids() {
        let criteria = vec![
            RgaaCriteria::find("4.2").unwrap().clone(),
            RgaaCriteria::find("4.4").unwrap().clone(),
        ];
        let results = map_automatic_response(&criteria, &json!([response("fail")]).to_string());
        assert_eq!(results.len(), 2);
        assert_eq!(
            results["4.2"].automated_verdict,
            Some(AutomatedVerdict::Fail)
        );
        assert_unresolved(&results["4.4"]);
    }

    #[test]
    fn missing_duplicate_unknown_or_invalid_test_keys_are_rejected() {
        for tests in [
            json!([{"test_key":"1","verdict":"pass","justification":"ok"}]),
            json!([{"test_key":"1","verdict":"pass","justification":"ok"},{"test_key":"1","verdict":"pass","justification":"ok"},{"test_key":"3","verdict":"pass","justification":"ok"}]),
            json!([{"test_key":"1","verdict":"pass","justification":"ok"},{"test_key":"2","verdict":"pass","justification":"ok"},{"test_key":"99","verdict":"pass","justification":"ok"}]),
        ] {
            let mut item = response("pass");
            item["tests"] = tests;
            assert_unresolved(&mapped(json!([item])));
        }
        let mut item = response("pass");
        item["tests"][0]["verdict"] = json!("not_applicable");
        assert_unresolved(&mapped(json!([item])));
    }

    #[test]
    fn inconsistent_aggregate_and_invalid_confidence_are_rejected() {
        let mut item = response("pass");
        item["tests"][0]["verdict"] = json!("fail");
        assert_unresolved(&mapped(json!([item])));
        for confidence in [-0.1, 1.1] {
            let mut item = response("pass");
            item["confidence"] = json!(confidence);
            assert_unresolved(&mapped(json!([item])));
        }
    }

    #[test]
    fn evidence_references_are_preserved_without_verifying_the_model() {
        let mut item = response("fail");
        item["evidence"] =
            json!([{"kind":"dom_snapshot","hash":"sha256:abc","location":"page.json"}]);
        let result = mapped(json!([item]));
        assert_eq!(result.evidence.len(), 1);
        assert_eq!(result.evidence[0].hash, "sha256:abc");
        assert_eq!(result.verified_status, None);
        assert!(result.review_required);
    }
}
