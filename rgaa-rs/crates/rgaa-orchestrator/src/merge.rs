//! Explicit merge precedence for the per-criterion verdicts produced by the
//! audit's several sources.
//!
//! A criterion can be reached by more than one mechanism in the same run:
//! axe-core, the gap-fix snippets, and the agentic evaluation all key their
//! results by criterion id. Merging them by successive `HashMap::extend` calls
//! made the *last* source win on every collision, which made the LLM
//! authoritative over deterministic evidence — including when the LLM call had
//! failed. This module replaces that accident of ordering with a rule that is
//! written down and tested.
//!
//! # The rule
//!
//! 1. **An errored evaluation is not evidence.** A candidate whose source is
//!    an error path (`agent-error`) or whose status is [`CriterionStatus::Error`]
//!    carries no information about the criterion, so it can never outrank a
//!    candidate that reached a verdict. When *every* candidate errored the
//!    criterion is reported [`CriterionStatus::NotTested`] — excluded from the
//!    conformity rate — never `NeedsReview`, which would read as "a human
//!    should look at this" when in fact nothing was measured.
//! 2. **Deterministic evidence outranks an LLM verdict.** A verdict from
//!    `axe-core`, `gap-fix` or `manual` is reproducible and falsifiable; an
//!    `agent` verdict is neither. Where both speak, the deterministic one wins.
//! 3. **Within one rank, the more conservative verdict wins**: `Fail` over
//!    `NeedsReview` over `Pass` over `NotApplicable`. Two mechanisms of equal
//!    standing disagreeing is not a licence to report the friendlier answer.
//! 4. **Ties keep the first candidate**, so the output does not depend on the
//!    iteration order of any map.
//!
//! Every candidate's source is recorded on the winner in
//! [`CriterionResult::considered_sources`], so an overwrite stays auditable
//! after the fact.
//!
//! # Deliberately out of scope
//!
//! Rule 2 is right where the deterministic mechanism covers the whole
//! criterion. Where it covers only part of one, the honest answer is neither
//! source's verdict alone but a per-test result; that is the test-level
//! granularity work, not this module's.

use std::collections::HashMap;

use rgaa_core::types::{CriterionResult, CriterionStatus, TestOutcome, VerdictBasis};

/// Sources whose verdicts are reproducible evidence about the criterion.
const DETERMINISTIC_SOURCES: [&str; 3] = ["axe-core", "gap-fix", "manual"];

/// Sources that mark a mechanism having failed rather than having concluded.
const ERROR_SOURCES: [&str; 1] = ["agent-error"];

/// Whether `result` records a mechanism that failed instead of concluding.
fn is_errored(result: &CriterionResult) -> bool {
    ERROR_SOURCES.contains(&result.source.as_str()) || result.status == CriterionStatus::Error
}

/// How much standing `result` has as evidence about its criterion. Higher wins.
fn evidence_rank(result: &CriterionResult) -> u8 {
    if is_errored(result) {
        0
    } else if DETERMINISTIC_SOURCES.contains(&result.source.as_str()) {
        2
    } else {
        1
    }
}

/// Tie-break within one [`evidence_rank`]: the more conservative verdict wins.
fn conservatism(status: &CriterionStatus) -> u8 {
    match status {
        CriterionStatus::Fail => 4,
        CriterionStatus::NeedsReview => 3,
        CriterionStatus::Pass => 2,
        CriterionStatus::NotApplicable => 1,
        CriterionStatus::Error | CriterionStatus::NotTested => 0,
    }
}

/// Pick the verdict that stands for one criterion out of every candidate
/// produced for it, in the order the sources were consulted.
///
/// Returns `None` for no candidates, so callers can fall through to the
/// catalog's own default for an untouched criterion.
#[must_use]
pub fn merge_candidates(candidates: Vec<CriterionResult>) -> Option<CriterionResult> {
    let considered: Vec<String> = candidates.iter().map(|c| c.source.clone()).collect();

    // Keep the estimate independently from the result that wins verified-status
    // precedence. The prediction can disagree with deterministic evidence and
    // both facts must remain visible to reports and later review.
    let model_assessment = candidates
        .iter()
        .find(|candidate| {
            candidate.automated_verdict.is_some()
                && candidate
                    .verdict_basis
                    .contains(&VerdictBasis::ModelEstimate)
        })
        .cloned();
    let all_tests: Vec<TestOutcome> = candidates
        .iter()
        .flat_map(|candidate| candidate.tests.iter().cloned())
        .collect();
    let mut all_basis = Vec::new();
    let mut all_evidence = Vec::new();
    for candidate in &candidates {
        for basis in &candidate.verdict_basis {
            if !all_basis.contains(basis) {
                all_basis.push(*basis);
            }
        }
        for evidence in &candidate.evidence {
            if !all_evidence.contains(evidence) {
                all_evidence.push(evidence.clone());
            }
        }
    }

    let (_, mut winner) = candidates.into_iter().enumerate().reduce(|best, next| {
        let key = |(i, r): &(usize, CriterionResult)| {
            (
                evidence_rank(r),
                conservatism(&r.status),
                std::cmp::Reverse(*i),
            )
        };
        if key(&next) > key(&best) {
            next
        } else {
            best
        }
    })?;

    // Only reachable when every candidate errored: report that nothing was
    // measured rather than asking a human to review a verdict that does not
    // exist.
    if is_errored(&winner) {
        winner.status = CriterionStatus::NotTested;
        winner.violations.clear();
        winner.confidence = None;
        winner.justification = Some(format!(
            "Not tested — every mechanism for this criterion errored. Last error: {}",
            winner.justification.as_deref().unwrap_or("unknown")
        ));
    }

    winner.considered_sources = considered;
    winner.tests = all_tests;
    winner.verdict_basis = all_basis;
    winner.evidence = all_evidence;
    let source_basis = match winner.source.as_str() {
        "axe-core" => Some(VerdictBasis::Axe),
        "gap-fix" | "manual" | "automated" => Some(VerdictBasis::Deterministic),
        _ => None,
    };
    if let Some(basis) = source_basis {
        if !winner.verdict_basis.contains(&basis) {
            winner.verdict_basis.push(basis);
        }
    }
    if let Some(assessment) = model_assessment {
        winner.automated_verdict = assessment.automated_verdict;
        winner.raw_confidence = assessment.raw_confidence;
        winner.confidence = assessment.confidence;
        winner.confidence_calibration_version = assessment.confidence_calibration_version.clone();
        winner.review_required |= assessment.review_required;
        if assessment.review_reason.is_some() {
            winner.review_reason = assessment.review_reason.clone();
        }
    }
    if DETERMINISTIC_SOURCES.contains(&winner.source.as_str())
        && matches!(
            winner.status,
            CriterionStatus::Pass | CriterionStatus::Fail | CriterionStatus::NotApplicable
        )
    {
        winner.verified_status = Some(winner.status.clone());
    }
    Some(winner)
}

/// Merge every `(criterion_id, result)` a page's sources produced into one map,
/// applying [`merge_candidates`] to each criterion more than one source reached.
///
/// `candidates` is consumed in order — chain the per-source maps in the order
/// the sources were consulted — which fixes the order of
/// [`CriterionResult::considered_sources`] but, unlike the `extend` chain this
/// replaces, never the winner. Takes a flat iterator rather than a `Vec` of maps
/// because the sources do not agree on a map type (`AxeMapper` returns an
/// `IndexMap`, the others a `HashMap`).
#[must_use]
pub fn merge_results(
    candidates: impl IntoIterator<Item = (String, CriterionResult)>,
) -> HashMap<String, CriterionResult> {
    let mut grouped: HashMap<String, Vec<CriterionResult>> = HashMap::new();
    for (criterion_id, result) in candidates {
        grouped.entry(criterion_id).or_default().push(result);
    }

    grouped
        .into_iter()
        .filter_map(|(criterion_id, results)| {
            merge_candidates(results).map(|winner| (criterion_id, winner))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rgaa_core::types::Classification;

    fn result(source: &str, status: CriterionStatus) -> CriterionResult {
        CriterionResult {
            criterion_id: "1.2".into(),
            title: "Chaque image de décoration est-elle correctement ignorée ?".into(),
            classification: Classification::Deterministe,
            status,
            violations: vec![],
            confidence: None,
            raw_confidence: None,
            justification: Some(format!("from {source}")),
            source: source.into(),
            citations: vec![],
            considered_sources: vec![],
            tests: vec![],
            automated_verdict: None,
            verdict_basis: Vec::new(),
            evidence: Vec::new(),
            confidence_calibration_version: None,
            review_required: false,
            review_reason: None,
            verified_status: None,
            review_events: Vec::new(),
        }
    }

    /// The collision that produced 30 overwritten criteria in the parisprivate
    /// run: axe reached a verdict, the agent asked for review, and `extend`
    /// order handed it to the agent.
    #[test]
    fn axe_pass_outranks_agent_needs_review() {
        let merged = merge_candidates(vec![
            result("axe-core", CriterionStatus::Pass),
            result("agent", CriterionStatus::NeedsReview),
        ])
        .expect("two candidates merge to one");

        assert_eq!(merged.source, "axe-core");
        assert_eq!(merged.status, CriterionStatus::Pass);
        assert_eq!(merged.considered_sources, vec!["axe-core", "agent"]);
    }

    /// An LLM `Pass` must not erase an observed violation.
    #[test]
    fn axe_fail_outranks_agent_pass() {
        let merged = merge_candidates(vec![
            result("axe-core", CriterionStatus::Fail),
            result("agent", CriterionStatus::Pass),
        ])
        .expect("two candidates merge to one");

        assert_eq!(merged.source, "axe-core");
        assert_eq!(merged.status, CriterionStatus::Fail);
    }

    #[test]
    fn deterministic_fail_keeps_conflicting_model_prediction_separate() {
        let mut model = result("agent-estimate", CriterionStatus::NeedsReview);
        model.automated_verdict = Some(rgaa_core::AutomatedVerdict::Pass);
        model.raw_confidence = Some(0.73);
        model.confidence = Some(0.61);
        model.confidence_calibration_version = Some("calibration-2026-10".into());
        model.verdict_basis = vec![VerdictBasis::ModelEstimate];
        model.review_required = true;
        model.tests.push(TestOutcome {
            test_key: "1".into(),
            status: CriterionStatus::Pass,
            source: "agent-estimate".into(),
            evidence: Some("model observation".into()),
        });
        let mut deterministic = result("axe-core", CriterionStatus::Fail);
        deterministic.verdict_basis = vec![VerdictBasis::Axe];
        deterministic.tests.push(TestOutcome {
            test_key: "1".into(),
            status: CriterionStatus::Fail,
            source: "axe-core".into(),
            evidence: Some("axe violation".into()),
        });

        let merged = merge_candidates(vec![deterministic, model]).expect("results merge");

        assert_eq!(merged.status, CriterionStatus::Fail);
        assert_eq!(merged.verified_status, Some(CriterionStatus::Fail));
        assert_eq!(
            merged.automated_verdict,
            Some(rgaa_core::AutomatedVerdict::Pass)
        );
        assert_eq!(merged.raw_confidence, Some(0.73));
        assert_eq!(merged.confidence, Some(0.61));
        assert_eq!(
            merged.confidence_calibration_version.as_deref(),
            Some("calibration-2026-10")
        );
        assert!(merged.verdict_basis.contains(&VerdictBasis::Axe));
        assert!(merged.verdict_basis.contains(&VerdictBasis::ModelEstimate));
        assert!(merged.review_required);
        assert_eq!(merged.tests.len(), 2);
    }

    #[test]
    fn gap_fix_fail_outranks_agent_pass() {
        let merged = merge_candidates(vec![
            result("gap-fix", CriterionStatus::Fail),
            result("agent", CriterionStatus::Pass),
        ])
        .expect("two candidates merge to one");

        assert_eq!(merged.source, "gap-fix");
        assert_eq!(merged.status, CriterionStatus::Fail);
    }

    /// The 7 criteria (1.2, 6.1, 8.10, 9.3, 11.1, 12.6, 13.4) whose
    /// deterministic result a *failed* LLM call erased.
    #[test]
    fn agent_error_never_overwrites_any_source() {
        for (source, status) in [
            ("axe-core", CriterionStatus::Pass),
            ("axe-core", CriterionStatus::Fail),
            ("gap-fix", CriterionStatus::Fail),
            ("manual", CriterionStatus::NeedsReview),
            ("agent", CriterionStatus::Pass),
        ] {
            let merged = merge_candidates(vec![
                result(source, status.clone()),
                result("agent-error", CriterionStatus::NeedsReview),
            ])
            .expect("two candidates merge to one");

            assert_eq!(merged.source, source, "{source} must survive agent-error");
            assert_eq!(merged.status, status, "{source} verdict must survive");
            assert_eq!(merged.considered_sources, vec![source, "agent-error"]);
        }
    }

    /// An error standing alone is not something a human can review — nothing
    /// was measured, so the criterion is `NotTested` and leaves the rate alone.
    #[test]
    fn lone_agent_error_is_not_tested_not_needs_review() {
        let merged = merge_candidates(vec![result("agent-error", CriterionStatus::NeedsReview)])
            .expect("one candidate merges to itself");

        assert_eq!(merged.status, CriterionStatus::NotTested);
        assert_eq!(merged.source, "agent-error");
        assert!(
            merged
                .justification
                .as_deref()
                .is_some_and(|j| j.contains("every mechanism") && j.contains("from agent-error")),
            "the error must stay readable in the justification: {:?}",
            merged.justification
        );
    }

    /// `Error` status is an error path whatever the source label says.
    #[test]
    fn error_status_is_treated_as_errored_regardless_of_source() {
        let merged = merge_candidates(vec![
            result("agent", CriterionStatus::Error),
            result("axe-core", CriterionStatus::Pass),
        ])
        .expect("two candidates merge to one");

        assert_eq!(merged.source, "axe-core");
        assert_eq!(merged.status, CriterionStatus::Pass);
    }

    /// Two mechanisms of equal standing disagreeing must not resolve to the
    /// friendlier answer.
    #[test]
    fn within_one_rank_the_conservative_verdict_wins() {
        let merged = merge_candidates(vec![
            result("axe-core", CriterionStatus::Pass),
            result("gap-fix", CriterionStatus::Fail),
        ])
        .expect("two candidates merge to one");
        assert_eq!(merged.status, CriterionStatus::Fail);

        // and in the other input order
        let merged = merge_candidates(vec![
            result("gap-fix", CriterionStatus::Fail),
            result("axe-core", CriterionStatus::Pass),
        ])
        .expect("two candidates merge to one");
        assert_eq!(merged.status, CriterionStatus::Fail);
    }

    /// Equal rank and equal conservatism: the first candidate stands, so the
    /// winner never depends on map iteration order.
    #[test]
    fn ties_keep_the_first_candidate() {
        let merged = merge_candidates(vec![
            result("axe-core", CriterionStatus::Pass),
            result("gap-fix", CriterionStatus::Pass),
        ])
        .expect("two candidates merge to one");

        assert_eq!(merged.source, "axe-core");
    }

    #[test]
    fn no_candidates_leaves_the_criterion_to_the_catalog_fallback() {
        assert!(merge_candidates(vec![]).is_none());
    }

    #[test]
    fn merge_results_keeps_uncontested_criteria_untouched() {
        let axe = HashMap::from([("1.1".to_string(), result("axe-core", CriterionStatus::Fail))]);
        let agent = HashMap::from([("3.1".to_string(), result("agent", CriterionStatus::Pass))]);

        let merged = merge_results(axe.into_iter().chain(agent));

        assert_eq!(merged.len(), 2);
        assert_eq!(merged["1.1"].source, "axe-core");
        assert_eq!(merged["1.1"].considered_sources, vec!["axe-core"]);
        assert_eq!(merged["3.1"].source, "agent");
    }

    #[test]
    fn merge_results_resolves_collisions_across_sources() {
        let axe = HashMap::from([("1.2".to_string(), result("axe-core", CriterionStatus::Pass))]);
        let agent = HashMap::from([(
            "1.2".to_string(),
            result("agent-error", CriterionStatus::NeedsReview),
        )]);

        let merged = merge_results(axe.into_iter().chain(agent));

        assert_eq!(merged.len(), 1);
        assert_eq!(merged["1.2"].source, "axe-core");
        assert_eq!(merged["1.2"].status, CriterionStatus::Pass);
        assert_eq!(
            merged["1.2"].considered_sources,
            vec!["axe-core", "agent-error"]
        );
    }
}
