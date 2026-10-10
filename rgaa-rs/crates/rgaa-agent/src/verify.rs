use rgaa_core::{
    AutomatedVerdict, Criterion, CriterionResult, CriterionStatus, EvidenceRef, TestOutcome,
    TestRoutePlan, VerdictBasis,
};
use rgaa_holo::HoloResponse;
use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

const CALIBRATION_JSON: &str = include_str!("../data/verdict-calibration.json");
const WILSON_Z_95: f64 = 1.959_963_984_540_054;

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
/// The model value is retained as raw confidence. A calibrated confidence is
/// populated only when the built-in held-out calibration table has an eligible
/// bin; either way, an estimate remains unresolved and requires review.
#[must_use]
pub fn map_automatic_response(
    criteria: &[Criterion],
    response_json: &str,
) -> HashMap<String, CriterionResult> {
    let Some(items) = parse_automatic_json_array(response_json) else {
        return unresolved_automatic_results(criteria, "malformed automatic estimate JSON");
    };
    let mut results = HashMap::with_capacity(criteria.len());
    for criterion in criteria {
        let mut matches = items
            .iter()
            .filter(|item| automatic_criterion_id(item) == Some(criterion.id));
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

// All supplied aliases must identify the same criterion. A conflicting or
// malformed alias cannot be used to fill more than one requested result.
fn automatic_criterion_id(item: &serde_json::Value) -> Option<&str> {
    let mut canonical = None;
    for key in ["criterion_id", "criterion", "id"] {
        if let Some(value) = item.get(key) {
            let id = value.as_str()?;
            if canonical.is_some_and(|existing| existing != id) {
                return None;
            }
            canonical = Some(id);
        }
    }
    canonical
}

#[derive(serde::Deserialize)]
struct AutomaticResponse {
    #[serde(default)]
    tests: serde_json::Value,
    #[serde(alias = "automated_verdict", alias = "status")]
    verdict: String,
    confidence: serde_json::Value,
    #[serde(default, alias = "reasoning", alias = "explanation")]
    justification: String,
    // Required by the wire contract, but cannot waive review of an estimate.
    #[serde(default = "default_review_required")]
    _review_required: bool,
    #[serde(default)]
    evidence: serde_json::Value,
}

/// Accept a raw JSON array or one embedded in a fenced/prose-wrapped reply.
/// The scanner respects quoted strings and escapes, and only returns a fully
/// parsed array; truncated model output remains unresolved.
fn parse_automatic_json_array(text: &str) -> Option<Vec<serde_json::Value>> {
    if let Ok(items) = serde_json::from_str::<Vec<serde_json::Value>>(text.trim()) {
        return Some(items);
    }

    let mut start = None;
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;

    for (index, character) in text.char_indices() {
        if in_string {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '"' {
                in_string = false;
            }
            continue;
        }

        match character {
            '"' => in_string = true,
            '[' => {
                if depth == 0 {
                    start = Some(index);
                }
                depth += 1;
            }
            ']' if depth > 0 => {
                depth -= 1;
                if depth == 0 {
                    let array = &text[start?..index + character.len_utf8()];
                    if let Ok(items) = serde_json::from_str::<Vec<serde_json::Value>>(array) {
                        return Some(items);
                    }
                    start = None;
                }
            }
            _ => {}
        }
    }

    None
}

#[derive(serde::Deserialize)]
struct AutomaticTestResponse {
    #[serde(alias = "test_id", alias = "id")]
    test_key: String,
    #[serde(alias = "status")]
    verdict: String,
    #[serde(alias = "reasoning", alias = "explanation", alias = "evidence")]
    justification: String,
}

fn default_review_required() -> bool {
    true
}

fn estimate_verdict(value: &str) -> Option<AutomatedVerdict> {
    match value
        .trim()
        .to_ascii_lowercase()
        .replace([' ', '-'], "_")
        .as_str()
    {
        "pass" | "conforme" | "passed" => Some(AutomatedVerdict::Pass),
        "fail" | "non_conforme" | "failed" => Some(AutomatedVerdict::Fail),
        _ => None,
    }
}

fn validated_estimate(
    criterion: &Criterion,
    response: AutomaticResponse,
) -> Option<CriterionResult> {
    let verdict = estimate_verdict(&response.verdict)?;
    let confidence = response
        .confidence
        .as_f64()
        .or_else(|| response.confidence.as_str()?.trim().parse::<f64>().ok())
        .filter(|value| value.is_finite() && (0.0..=1.0).contains(value));
    let justification = if response.justification.trim().is_empty() {
        "Le modèle a fourni un verdict agrégé sans justification exploitable ; revue humaine requise."
            .to_owned()
    } else {
        response.justification
    };
    let routes = TestRoutePlan::builtin();
    let expected: Vec<_> = routes
        .routes()
        .iter()
        .filter(|route| route.criterion_id == criterion.id && route.fallback == "holo_estimate")
        .collect();
    let mut seen = HashSet::with_capacity(expected.len());
    let mut tests = Vec::with_capacity(expected.len());
    let mut any_fail = false;
    let mut invalid_breakdown = false;
    let test_values = match response.tests {
        serde_json::Value::Array(values) => values,
        object @ serde_json::Value::Object(_) => vec![object],
        _ => Vec::new(),
    };
    for value in test_values {
        let Ok(test) = serde_json::from_value::<AutomaticTestResponse>(value) else {
            invalid_breakdown = true;
            continue;
        };
        if !seen.insert(test.test_key.clone())
            || !routes
                .for_test(criterion.id, &test.test_key)
                .is_some_and(|route| route.fallback == "holo_estimate")
        {
            invalid_breakdown = true;
            continue;
        }
        let Some(test_verdict) = estimate_verdict(&test.verdict) else {
            invalid_breakdown = true;
            continue;
        };
        any_fail |= test_verdict == AutomatedVerdict::Fail;
        tests.push(TestOutcome {
            test_key: test.test_key,
            status: match test_verdict {
                AutomatedVerdict::Pass => CriterionStatus::Pass,
                _ => CriterionStatus::Fail,
            },
            source: "agent-estimate".to_owned(),
            evidence: (!test.justification.trim().is_empty()).then_some(test.justification),
        });
    }
    // A sound aggregate prediction is useful even when the model omits or
    // contradicts individual sub-tests. Keep only a complete, consistent
    // test breakdown; otherwise retain the criterion verdict for human review
    // and leave test-level coverage visibly incomplete.
    let complete_test_breakdown = !invalid_breakdown
        && !expected.is_empty()
        && tests.len() == expected.len()
        && (verdict == AutomatedVerdict::Fail) == any_fail;
    if !complete_test_breakdown {
        tests.clear();
    }
    if complete_test_breakdown {
        // Canonical route order makes reordered model responses reproducible.
        tests.sort_by_key(|test| {
            expected
                .iter()
                .position(|route| route.test_key == test.test_key)
        });
    }
    // Model replies sometimes return evidence as prose strings instead of
    // auditable {kind, hash} references. Keep only valid references; never
    // promote prose into evidence or discard an otherwise usable verdict.
    let evidence_values = match response.evidence {
        serde_json::Value::Array(values) => values,
        object @ serde_json::Value::Object(_) => vec![object],
        _ => Vec::new(),
    };
    let evidence: Vec<EvidenceRef> = evidence_values
        .into_iter()
        .filter_map(|value| serde_json::from_value(value).ok())
        .filter(|reference: &EvidenceRef| {
            !reference.kind.trim().is_empty() && !reference.hash.trim().is_empty()
        })
        .collect();
    let reason = if evidence.is_empty() {
        "model estimate requires human review: no auditable evidence references were supplied"
    } else if !complete_test_breakdown {
        "model estimate requires human review: test-level breakdown is incomplete or inconsistent"
    } else {
        "model estimate requires independent review of the supplied evidence references"
    };
    let mut result = unresolved_automatic_result(criterion, reason);
    result.source = "agent-estimate".to_owned();
    result.automated_verdict = Some(verdict);
    result.verdict_basis = vec![VerdictBasis::ModelEstimate];
    result.tests = tests;
    result.raw_confidence = confidence;
    if confidence.is_some() {
        if let Some(table) = builtin_calibration() {
            apply_calibration(&mut result, table);
        }
    }
    result.justification = Some(justification);
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
        raw_confidence: None,
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

/// A versioned table mapping model-confidence ranges to held-out correctness.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CalibrationTable {
    /// Identifier recorded on results only when a calibration bin was applied.
    pub version: String,
    /// Last date on which this table may be used, in `YYYY-MM-DD` format.
    pub valid_until: String,
    bins: Vec<CalibrationBin>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct CalibrationBin {
    criterion_family: String,
    min_raw_confidence: f64,
    max_raw_confidence: f64,
    sample_count: usize,
    correct_count: usize,
    false_pass_count: usize,
    false_fail_count: usize,
    accuracy: f64,
}

/// Error raised when a calibration manifest is malformed, stale, or internally
/// inconsistent.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
#[error("invalid confidence calibration manifest: {0}")]
pub struct CalibrationError(String);

impl CalibrationTable {
    /// Parse and validate a calibration manifest against today's date.
    pub fn from_json(raw: &str) -> Result<Self, CalibrationError> {
        let today = chrono::Utc::now().date_naive();
        Self::from_json_at(raw, today)
    }

    fn from_json_at(raw: &str, today: chrono::NaiveDate) -> Result<Self, CalibrationError> {
        let table: Self =
            serde_json::from_str(raw).map_err(|error| CalibrationError(error.to_string()))?;
        if table.version.trim().is_empty() {
            return Err(CalibrationError("version must not be empty".into()));
        }
        let valid_until = chrono::NaiveDate::parse_from_str(&table.valid_until, "%Y-%m-%d")
            .map_err(|error| CalibrationError(format!("invalid valid_until date: {error}")))?;
        if valid_until < today {
            return Err(CalibrationError("calibration manifest is stale".into()));
        }

        let mut previous: Option<(&str, f64)> = None;
        for bin in &table.bins {
            if bin.criterion_family.trim().is_empty()
                || !bin.min_raw_confidence.is_finite()
                || !bin.max_raw_confidence.is_finite()
                || bin.min_raw_confidence < 0.0
                || bin.max_raw_confidence > 1.0
                || bin.min_raw_confidence >= bin.max_raw_confidence
            {
                return Err(CalibrationError(
                    "invalid confidence range or criterion family".into(),
                ));
            }
            let reconciled_count = bin
                .false_pass_count
                .checked_add(bin.false_fail_count)
                .and_then(|errors| errors.checked_add(bin.correct_count));
            if bin.sample_count == 0
                || bin.correct_count > bin.sample_count
                || reconciled_count != Some(bin.sample_count)
            {
                return Err(CalibrationError("sample counts do not reconcile".into()));
            }
            let computed_accuracy = bin.correct_count as f64 / bin.sample_count as f64;
            if !bin.accuracy.is_finite()
                || !(0.0..=1.0).contains(&bin.accuracy)
                || (bin.accuracy - computed_accuracy).abs() > 1e-9
            {
                return Err(CalibrationError(
                    "accuracy does not match labeled counts".into(),
                ));
            }
            if let Some((previous_family, previous_max)) = previous {
                if bin.criterion_family.as_str() < previous_family
                    || (bin.criterion_family == previous_family
                        && bin.min_raw_confidence < previous_max)
                {
                    return Err(CalibrationError(
                        "bins must be sorted and may not overlap within a criterion family".into(),
                    ));
                }
            }
            previous = Some((&bin.criterion_family, bin.max_raw_confidence));
        }
        Ok(table)
    }
}

/// Return a conservative 95% Wilson lower confidence bound for an eligible
/// criterion-family/raw-confidence bin. Bins need at least 30 held-out labels.
#[must_use]
pub fn calibrate_confidence(
    criterion_id: &str,
    raw_confidence: f64,
    table: &CalibrationTable,
) -> Option<f64> {
    if !raw_confidence.is_finite() || !(0.0..=1.0).contains(&raw_confidence) {
        return None;
    }
    let family = criterion_id.split('.').next()?;
    let bin = table.bins.iter().find(|bin| {
        bin.criterion_family == family
            && raw_confidence >= bin.min_raw_confidence
            && (raw_confidence < bin.max_raw_confidence
                || (bin.max_raw_confidence == 1.0 && raw_confidence == 1.0))
    })?;
    if bin.sample_count < 30 {
        return None;
    }
    let n = bin.sample_count as f64;
    let p = bin.accuracy;
    let z2 = WILSON_Z_95 * WILSON_Z_95;
    let denominator = 1.0 + z2 / n;
    let center = p + z2 / (2.0 * n);
    let margin = WILSON_Z_95 * ((p * (1.0 - p) / n + z2 / (4.0 * n * n)).sqrt());
    Some(((center - margin) / denominator).clamp(0.0, 1.0))
}

fn apply_calibration(result: &mut CriterionResult, table: &CalibrationTable) {
    result.confidence = result
        .raw_confidence
        .and_then(|raw| calibrate_confidence(&result.criterion_id, raw, table));
    result.confidence_calibration_version = result.confidence.map(|_| table.version.clone());
}

fn builtin_calibration() -> Option<&'static CalibrationTable> {
    static TABLE: OnceLock<Option<CalibrationTable>> = OnceLock::new();
    TABLE
        .get_or_init(|| CalibrationTable::from_json(CALIBRATION_JSON).ok())
        .as_ref()
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

    fn calibration_json(sample_count: usize, correct_count: usize, accuracy: f64) -> String {
        let errors = sample_count - correct_count;
        format!(
            r#"{{"version":"eval-v1","valid_until":"2099-12-31","bins":[{{"criterion_family":"4","min_raw_confidence":0.7,"max_raw_confidence":0.9,"sample_count":{sample_count},"correct_count":{correct_count},"false_pass_count":{errors},"false_fail_count":0,"accuracy":{accuracy}}}]}}"#
        )
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
            assert_eq!(result.raw_confidence, Some(0.72));
            assert_eq!(result.confidence, None);
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
    fn invalid_test_breakdowns_preserve_only_the_aggregate_estimate() {
        for tests in [
            json!([{"test_key":"1","verdict":"pass","justification":"ok"}]),
            json!([{"test_key":"1","verdict":"pass","justification":"ok"},{"test_key":"1","verdict":"pass","justification":"ok"},{"test_key":"3","verdict":"pass","justification":"ok"}]),
            json!([{"test_key":"1","verdict":"pass","justification":"ok"},{"test_key":"2","verdict":"pass","justification":"ok"},{"test_key":"99","verdict":"pass","justification":"ok"}]),
        ] {
            let mut item = response("pass");
            item["tests"] = tests;
            assert_aggregate_only(&mapped(json!([item])));
        }
        let mut item = response("pass");
        item["tests"][0]["verdict"] = json!("not_applicable");
        assert_aggregate_only(&mapped(json!([item])));
    }

    #[test]
    fn inconsistent_tests_and_invalid_confidence_do_not_discard_the_aggregate() {
        let mut item = response("pass");
        item["tests"][0]["verdict"] = json!("fail");
        assert_aggregate_only(&mapped(json!([item])));
        for confidence in [json!(-0.1), json!(1.1), json!("NaN"), json!("invalid")] {
            let mut item = response("pass");
            item["confidence"] = confidence;
            let result = mapped(json!([item]));
            assert_eq!(result.automated_verdict, Some(AutomatedVerdict::Pass));
            assert_eq!(result.raw_confidence, None);
            assert_eq!(result.confidence, None);
            assert_eq!(result.verified_status, None);
            assert!(result.review_required);
        }
    }

    fn assert_aggregate_only(result: &CriterionResult) {
        assert_eq!(result.automated_verdict, Some(AutomatedVerdict::Pass));
        assert_eq!(result.status, CriterionStatus::NeedsReview);
        assert_eq!(result.verified_status, None);
        assert!(result.review_required);
        assert!(result.tests.is_empty());
    }

    #[test]
    fn fenced_response_preserves_strings_with_brackets_and_escaped_quotes() {
        let mut item = response("pass");
        item["justification"] = json!("observed [media] with \"caption\"");
        let text = format!("Result:\n```json\n{}\n```", json!([item]));
        let criteria = vec![RgaaCriteria::find("4.2").unwrap().clone()];
        let results = map_automatic_response(&criteria, &text);
        assert_eq!(
            results["4.2"].automated_verdict,
            Some(AutomatedVerdict::Pass)
        );
        assert_eq!(results["4.2"].tests.len(), 3);
    }

    #[test]
    fn aliases_and_numeric_string_confidence_preserve_aggregate_only_results() {
        for key in ["criterion_id", "criterion", "id"] {
            let mut item =
                json!({"status":" PASSED ", "confidence":"0.72", "reasoning":"estimate"});
            item[key] = json!("4.2");
            let result = mapped(json!([item]));
            assert_aggregate_only(&result);
            assert_eq!(result.raw_confidence, Some(0.72));
        }
    }

    #[test]
    fn conflicting_criterion_aliases_cannot_fill_multiple_results() {
        let criteria = vec![
            RgaaCriteria::find("4.2").unwrap().clone(),
            RgaaCriteria::find("4.4").unwrap().clone(),
        ];
        let mut item = response("pass");
        item["id"] = json!("4.4");
        let results = map_automatic_response(&criteria, &json!([item]).to_string());
        assert_unresolved(&results["4.2"]);
        assert_unresolved(&results["4.4"]);
        let mut item = response("pass");
        item["id"] = json!("4.2");
        assert_eq!(mapped(json!([item])).tests.len(), 3);
    }

    #[test]
    fn complete_breakdown_with_duplicate_or_invalid_extra_test_is_incomplete() {
        for extra in [
            json!({"test_key":"1", "verdict":"fail", "justification":"contradiction"}),
            json!({"test_key":"1", "verdict":"pass", "justification":"duplicate"}),
            json!({"test_key":"99", "verdict":"pass", "justification":"unknown"}),
            json!("malformed test"),
        ] {
            let mut item = response("pass");
            item["tests"].as_array_mut().unwrap().push(extra);
            assert_aggregate_only(&mapped(json!([item])));
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

    #[test]
    fn calibration_uses_the_wilson_lower_bound_and_held_out_threshold() {
        let table = CalibrationTable::from_json(&calibration_json(100, 71, 0.71)).unwrap();
        let calibrated = calibrate_confidence("4.2", 0.8, &table).unwrap();
        assert!(calibrated > 0.0 && calibrated < 0.71);
        assert_eq!(calibrate_confidence("13.7", 0.8, &table), None);

        let undersized =
            CalibrationTable::from_json(&calibration_json(29, 20, 20.0 / 29.0)).unwrap();
        assert_eq!(calibrate_confidence("4.2", 0.8, &undersized), None);
    }

    #[test]
    fn applied_calibration_keeps_raw_value_and_human_review_requirement() {
        let table = CalibrationTable::from_json(&calibration_json(100, 71, 0.71)).unwrap();
        let mut result = mapped(json!([response("pass")]));
        apply_calibration(&mut result, &table);
        assert_eq!(result.raw_confidence, Some(0.72));
        assert!(result.confidence.unwrap() > 0.0);
        assert_eq!(
            result.confidence_calibration_version.as_deref(),
            Some("eval-v1")
        );
        assert!(result.review_required);
        assert_eq!(result.verified_status, None);
    }

    #[test]
    fn calibration_rejects_stale_malformed_conflicting_and_out_of_range_data() {
        let today = chrono::NaiveDate::from_ymd_opt(2026, 10, 7).unwrap();
        let stale = r#"{"version":"old","valid_until":"2026-10-06","bins":[]}"#;
        assert!(CalibrationTable::from_json_at(stale, today).is_err());
        assert!(CalibrationTable::from_json("not json").is_err());

        let duplicate_or_overlapping = r#"{"version":"v1","valid_until":"2099-12-31","bins":[
          {"criterion_family":"4","min_raw_confidence":0.5,"max_raw_confidence":0.8,"sample_count":30,"correct_count":24,"false_pass_count":3,"false_fail_count":3,"accuracy":0.8},
          {"criterion_family":"4","min_raw_confidence":0.7,"max_raw_confidence":0.9,"sample_count":30,"correct_count":24,"false_pass_count":3,"false_fail_count":3,"accuracy":0.8}] }"#;
        assert!(CalibrationTable::from_json(duplicate_or_overlapping).is_err());

        let invalid_range = r#"{"version":"v1","valid_until":"2099-12-31","bins":[{"criterion_family":"4","min_raw_confidence":-0.1,"max_raw_confidence":0.9,"sample_count":30,"correct_count":24,"false_pass_count":3,"false_fail_count":3,"accuracy":0.8}]}"#;
        assert!(CalibrationTable::from_json(invalid_range).is_err());

        let invalid_accuracy = calibration_json(30, 24, 0.81);
        assert!(CalibrationTable::from_json(&invalid_accuracy).is_err());
    }

    #[test]
    fn calibration_manifest_requires_sorted_non_overlapping_bins() {
        let out_of_order = r#"{"version":"v1","valid_until":"2099-12-31","bins":[
          {"criterion_family":"4","min_raw_confidence":0.7,"max_raw_confidence":0.9,"sample_count":30,"correct_count":24,"false_pass_count":3,"false_fail_count":3,"accuracy":0.8},
          {"criterion_family":"3","min_raw_confidence":0.1,"max_raw_confidence":0.4,"sample_count":30,"correct_count":24,"false_pass_count":3,"false_fail_count":3,"accuracy":0.8}] }"#;
        assert!(CalibrationTable::from_json(out_of_order).is_err());
    }
}
