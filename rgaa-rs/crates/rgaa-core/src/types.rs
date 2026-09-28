use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Classification {
    Deterministe,
    IaAssiste,
    Manuel,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CriterionStatus {
    Pass,
    Fail,
    NotApplicable,
    Error,
    NeedsReview,
    NotTested,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ConformityStatus {
    Conforme,
    NonConforme,
    NonApplicable,
    NonTeste,
}

impl From<CriterionStatus> for ConformityStatus {
    fn from(status: CriterionStatus) -> Self {
        match status {
            CriterionStatus::Pass => ConformityStatus::Conforme,
            CriterionStatus::Fail => ConformityStatus::NonConforme,
            CriterionStatus::NotApplicable => ConformityStatus::NonApplicable,
            CriterionStatus::NeedsReview | CriterionStatus::NotTested => ConformityStatus::NonTeste,
            CriterionStatus::Error => ConformityStatus::NonConforme,
        }
    }
}

/// The outcome of one RGAA **test** within a criterion.
///
/// RGAA conformance is defined test by test: a criterion conforms only when all of its
/// tests pass. Until #203 the workspace modelled results at criterion granularity only,
/// so every mechanism's finding was flattened to one verdict per criterion and the
/// catalog's `test_keys` were read by nothing.
///
/// This does not replace [`CriterionResult::status`]. The atomic criterion verdict is
/// what an opposable report must state; per-test outcomes are how it is *justified*.
/// See `reduce_test_outcomes`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TestOutcome {
    /// Test identity as the RGAA reference numbers it within the criterion — the same
    /// keys the catalog ships in `automatable_criteres.json`.
    pub test_key: String,
    pub status: CriterionStatus,
    /// Mechanism that decided this test: `axe-core`, `gap-fix`, `agent`, `manual`…
    /// Load-bearing, not informational: only a deterministic source may establish that
    /// a test is inapplicable (see [`is_deterministic_source`]).
    pub source: String,
    /// What the mechanism saw, when it has something to show.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence: Option<String>,
}

/// Whether `source` is reproducible evidence, as opposed to a model's judgement.
///
/// This gates one thing only, and it is the asymmetry #203 recommendation 2 turns on: a
/// model may **fail** a test and may **flag** one for review, but it may not make a
/// criterion conform by declaring the tests nobody covered inapplicable. Allowing that
/// would rest a published conformance claim on an unreproducible judgement — the same
/// defect as a `Pass` that cannot fail, reached by a longer route.
#[must_use]
pub fn is_deterministic_source(source: &str) -> bool {
    matches!(source, "axe-core" | "gap-fix" | "manual" | "automated")
}

/// Derive a criterion's verdict from its per-test outcomes.
///
/// `total_tests` is the criterion's test count from the catalog, so a criterion whose
/// mechanisms reported on only some of its tests cannot be passed on the strength of the
/// ones they did reach.
///
/// The rule (#203 recommendations 1 and 2):
///
/// 1. no outcomes at all → `None`, and the caller keeps whatever the mechanism set;
/// 2. any test failed → `Fail`, whatever else is known;
/// 3. any test errored → the criterion is not decided: `NotTested`;
/// 4. every test accounted for, each either `Pass` or deterministically
///    `NotApplicable` → `Pass`;
/// 5. every test deterministically `NotApplicable` → `NotApplicable`;
/// 6. anything else — a test left unreported, or one only a model called inapplicable —
///    → `NeedsReview`, because a human still has to close it.
#[must_use]
pub fn reduce_test_outcomes(tests: &[TestOutcome], total_tests: usize) -> Option<CriterionStatus> {
    if tests.is_empty() {
        return None;
    }

    if tests.iter().any(|t| t.status == CriterionStatus::Fail) {
        return Some(CriterionStatus::Fail);
    }
    if tests.iter().any(|t| t.status == CriterionStatus::Error) {
        return Some(CriterionStatus::NotTested);
    }

    // An inapplicable test only counts as settled when a reproducible mechanism said so.
    let settled_na = |t: &TestOutcome| {
        t.status == CriterionStatus::NotApplicable && is_deterministic_source(&t.source)
    };

    let mut reported: std::collections::BTreeSet<&str> = std::collections::BTreeSet::new();
    for t in tests {
        if t.status == CriterionStatus::Pass || settled_na(t) {
            reported.insert(t.test_key.as_str());
        }
    }

    // `total_tests == 0` means the catalog has no test count for this criterion; not
    // knowing how many tests exist is not grounds for declaring them all satisfied.
    if total_tests == 0 || reported.len() < total_tests {
        return Some(CriterionStatus::NeedsReview);
    }

    if tests.iter().all(settled_na) {
        return Some(CriterionStatus::NotApplicable);
    }
    Some(CriterionStatus::Pass)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CriterionResult {
    pub criterion_id: String,
    pub title: String,
    pub classification: Classification,
    pub status: CriterionStatus,
    pub violations: Vec<Violation>,
    pub confidence: Option<f64>,
    pub justification: Option<String>,
    pub source: String,
    /// Sources backing this verdict, when it relied on retrieved documents
    /// (see [`crate::Citation`]). Empty for verdicts reached without
    /// retrieval — deterministic rules, manual review, "not tested" — which
    /// stay valid with no citations at all. Defaults to empty on
    /// deserialize so results persisted before this field existed still
    /// load.
    #[serde(default)]
    pub citations: Vec<crate::Citation>,
    /// Every source that produced a candidate verdict for this criterion
    /// before merge precedence picked a winner, in the order they were
    /// considered (see `rgaa_orchestrator::merge`). Keeps an overwrite
    /// auditable after the fact: a `Pass` from `axe-core` that outranked an
    /// `agent` `needs_review` still records both here. Empty for results that
    /// never went through a merge (a single source, or a catalog fallback).
    /// Defaults to empty on deserialize, and is omitted when empty, so
    /// results persisted before this field existed still load.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub considered_sources: Vec<String>,
    /// Per-test outcomes backing this verdict, when a mechanism knew which RGAA test it
    /// answered (#203). Empty for a mechanism that only speaks at criterion granularity,
    /// in which case [`Self::status`] is whatever that mechanism set.
    ///
    /// When it is non-empty, [`Self::status`] is *derived* from it — see
    /// [`reduce_test_outcomes`] — so the two cannot disagree. Defaults to empty on
    /// deserialize and is omitted when empty, so results persisted before this field
    /// existed still load and unchanged reports stay byte-identical.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tests: Vec<TestOutcome>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Violation {
    pub rule_id: String,
    pub impact: String,
    pub description: String,
    pub nodes_affected: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PageResult {
    pub url: String,
    pub title: Option<String>,
    pub criteria: Vec<CriterionResult>,
    pub compliance_rate: f64,
    pub crawl_depth: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AuditResult {
    pub audit_id: String,
    pub url: String,
    pub pages: Vec<PageResult>,
    pub total_criteria: usize,
    pub passed: usize,
    pub failed: usize,
    pub na: usize,
    pub overall_compliance: f64,
    pub taux_global: f64,
    pub coverage_percent: f64,
    pub etat_conformite: String,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrawlConfig {
    pub max_pages: usize,
    pub max_depth: u32,
    pub respect_robots: bool,
    pub sample_mode: bool,
}

impl Default for CrawlConfig {
    fn default() -> Self {
        Self {
            max_pages: 50,
            max_depth: 5,
            respect_robots: true,
            sample_mode: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn criterion_statuses_have_stable_json_names() {
        let statuses = [
            (CriterionStatus::Pass, "pass"),
            (CriterionStatus::Fail, "fail"),
            (CriterionStatus::NotApplicable, "not_applicable"),
            (CriterionStatus::Error, "error"),
            (CriterionStatus::NeedsReview, "needs_review"),
            (CriterionStatus::NotTested, "not_tested"),
        ];

        for (status, expected) in statuses {
            assert_eq!(
                serde_json::to_string(&status).unwrap(),
                format!("\"{expected}\"")
            );
        }

        assert!(serde_json::from_str::<CriterionStatus>("\"na\"").is_err());
    }

    #[test]
    fn test_status_mapping() {
        assert_eq!(
            ConformityStatus::from(CriterionStatus::Pass),
            ConformityStatus::Conforme
        );
        assert_eq!(
            ConformityStatus::from(CriterionStatus::Fail),
            ConformityStatus::NonConforme
        );
        assert_eq!(
            ConformityStatus::from(CriterionStatus::NotApplicable),
            ConformityStatus::NonApplicable
        );
        assert_eq!(
            ConformityStatus::from(CriterionStatus::NeedsReview),
            ConformityStatus::NonTeste
        );
        assert_eq!(
            ConformityStatus::from(CriterionStatus::NotTested),
            ConformityStatus::NonTeste
        );
        assert_eq!(
            ConformityStatus::from(CriterionStatus::Error),
            ConformityStatus::NonConforme
        );
    }

    fn sample_result(citations: Vec<crate::Citation>) -> CriterionResult {
        CriterionResult {
            criterion_id: "1.1.1".into(),
            title: "Image alt".into(),
            classification: Classification::IaAssiste,
            status: CriterionStatus::Fail,
            violations: vec![],
            confidence: Some(0.9),
            justification: Some("missing alt".into()),
            source: "agent".into(),
            citations,
            considered_sources: vec![],
            tests: vec![],
        }
    }

    #[test]
    fn result_without_citations_serializes_and_stays_valid() {
        let result = sample_result(vec![]);
        let json = serde_json::to_value(&result).unwrap();
        assert_eq!(json["citations"], serde_json::json!([]));

        let decoded: CriterionResult = serde_json::from_value(json).unwrap();
        assert_eq!(decoded, result);
        assert!(decoded.citations.is_empty());
    }

    #[test]
    fn result_with_typed_citations_round_trips() {
        let result = sample_result(vec![
            crate::Citation::referentiel("1.1.1", "2024.1"),
            crate::Citation::crawl("https://example.org/", "2025-01-01T00:00:00Z", "sha256:x"),
        ]);
        let json = serde_json::to_string(&result).unwrap();
        let decoded: CriterionResult = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, result);
        assert_eq!(decoded.citations.len(), 2);
    }

    #[test]
    fn legacy_result_json_without_citations_field_still_deserializes() {
        // Serialized before this field existed — must keep loading with an
        // empty citations list rather than failing.
        let legacy = serde_json::json!({
            "criterion_id": "1.1.1",
            "title": "Image alt",
            "classification": "IaAssiste",
            "status": "fail",
            "violations": [],
            "confidence": 0.9,
            "justification": "missing alt",
            "source": "agent"
        });
        let decoded: CriterionResult = serde_json::from_value(legacy).unwrap();
        assert!(decoded.citations.is_empty());
    }

    // ------------------------------------------------------------------ #203

    fn outcome(test_key: &str, status: CriterionStatus, source: &str) -> TestOutcome {
        TestOutcome {
            test_key: test_key.into(),
            status,
            source: source.into(),
            evidence: None,
        }
    }

    /// No per-test data means the mechanism only spoke at criterion granularity, and its
    /// verdict stands untouched. This is what keeps every existing mechanism working.
    #[test]
    fn no_test_outcomes_leaves_the_criterion_verdict_alone() {
        assert_eq!(reduce_test_outcomes(&[], 3), None);
    }

    /// A failure is a failure whatever else is known, and whoever found it.
    #[test]
    fn one_failed_test_fails_the_criterion() {
        let tests = vec![
            outcome("1", CriterionStatus::Pass, "axe-core"),
            outcome("2", CriterionStatus::Fail, "agent"),
            outcome("3", CriterionStatus::Pass, "axe-core"),
        ];
        assert_eq!(reduce_test_outcomes(&tests, 3), Some(CriterionStatus::Fail));
    }

    #[test]
    fn every_test_passing_passes_the_criterion() {
        let tests = vec![
            outcome("1", CriterionStatus::Pass, "axe-core"),
            outcome("2", CriterionStatus::Pass, "gap-fix"),
        ];
        assert_eq!(reduce_test_outcomes(&tests, 2), Some(CriterionStatus::Pass));
    }

    /// The core of recommendation 1: a criterion with three tests, two of them passing
    /// and the third never reported, is not conformant. Passing it on the strength of
    /// the tests the mechanisms happened to reach is #199's defect at test granularity.
    #[test]
    fn a_test_nobody_reported_blocks_the_pass() {
        let tests = vec![
            outcome("1", CriterionStatus::Pass, "axe-core"),
            outcome("2", CriterionStatus::Pass, "axe-core"),
        ];
        assert_eq!(
            reduce_test_outcomes(&tests, 3),
            Some(CriterionStatus::NeedsReview),
            "test 3 was never reported, so a human still has to close the criterion"
        );
    }

    /// Recommendation 2. A deterministic query may establish that a test does not apply,
    /// which lets the criterion conform on the remaining ones.
    #[test]
    fn deterministic_inapplicability_can_settle_a_test() {
        let tests = vec![
            outcome("1", CriterionStatus::Pass, "axe-core"),
            outcome("2", CriterionStatus::NotApplicable, "automated"),
        ];
        assert_eq!(reduce_test_outcomes(&tests, 2), Some(CriterionStatus::Pass));
    }

    /// Recommendation 2, the half that matters. A model saying a test does not apply is
    /// a judgement nobody can reproduce, so it must not buy a conformance claim. The
    /// criterion goes to review instead.
    #[test]
    fn a_model_cannot_settle_a_test_by_calling_it_inapplicable() {
        let tests = vec![
            outcome("1", CriterionStatus::Pass, "axe-core"),
            outcome("2", CriterionStatus::NotApplicable, "agent"),
        ];
        assert_eq!(
            reduce_test_outcomes(&tests, 2),
            Some(CriterionStatus::NeedsReview),
            "an LLM may fail or flag a test, never declare it out of scope to reach a Pass"
        );

        // and the same through the batch and error paths
        for source in ["agent-batch", "agent-error"] {
            let tests = vec![
                outcome("1", CriterionStatus::Pass, "axe-core"),
                outcome("2", CriterionStatus::NotApplicable, source),
            ];
            assert_eq!(
                reduce_test_outcomes(&tests, 2),
                Some(CriterionStatus::NeedsReview),
                "{source} must not settle a test either"
            );
        }
    }

    /// A model may still *fail* a test — the asymmetry is deliberate and one-directional.
    #[test]
    fn a_model_can_still_fail_a_test() {
        let tests = vec![outcome("1", CriterionStatus::Fail, "agent")];
        assert_eq!(reduce_test_outcomes(&tests, 1), Some(CriterionStatus::Fail));
    }

    #[test]
    fn every_test_deterministically_inapplicable_makes_the_criterion_inapplicable() {
        let tests = vec![
            outcome("1", CriterionStatus::NotApplicable, "axe-core"),
            outcome("2", CriterionStatus::NotApplicable, "automated"),
        ];
        assert_eq!(
            reduce_test_outcomes(&tests, 2),
            Some(CriterionStatus::NotApplicable)
        );
    }

    /// An errored test means the criterion was not decided. It must not read as
    /// conformance, and it must not read as a failure either.
    #[test]
    fn an_errored_test_leaves_the_criterion_not_tested() {
        let tests = vec![
            outcome("1", CriterionStatus::Pass, "axe-core"),
            outcome("2", CriterionStatus::Error, "agent"),
        ];
        assert_eq!(
            reduce_test_outcomes(&tests, 2),
            Some(CriterionStatus::NotTested)
        );
    }

    /// Not knowing how many tests a criterion has is not grounds for declaring them all
    /// satisfied.
    #[test]
    fn an_unknown_test_count_cannot_yield_a_pass() {
        let tests = vec![outcome("1", CriterionStatus::Pass, "axe-core")];
        assert_eq!(
            reduce_test_outcomes(&tests, 0),
            Some(CriterionStatus::NeedsReview)
        );
    }

    /// Two mechanisms reporting the same test must not count as two tests covered.
    #[test]
    fn duplicate_reports_of_one_test_do_not_cover_the_others() {
        let tests = vec![
            outcome("1", CriterionStatus::Pass, "axe-core"),
            outcome("1", CriterionStatus::Pass, "gap-fix"),
        ];
        assert_eq!(
            reduce_test_outcomes(&tests, 2),
            Some(CriterionStatus::NeedsReview),
            "test 1 reported twice still leaves test 2 unreported"
        );
    }

    #[test]
    fn only_reproducible_sources_are_deterministic() {
        for source in ["axe-core", "gap-fix", "manual", "automated"] {
            assert!(is_deterministic_source(source), "{source}");
        }
        for source in [
            "agent",
            "agent-batch",
            "agent-error",
            "partially-automatable",
        ] {
            assert!(!is_deterministic_source(source), "{source}");
        }
    }

    /// Per-test outcomes ride alongside the atomic verdict; a result written before the
    /// field existed still loads, and one without tests still serialises unchanged.
    #[test]
    fn legacy_result_json_without_tests_field_still_deserializes() {
        let json = r#"{
            "criterion_id": "1.1",
            "title": "t",
            "classification": "Deterministe",
            "status": "pass",
            "violations": [],
            "confidence": null,
            "justification": null,
            "source": "axe-core"
        }"#;
        let r: CriterionResult = serde_json::from_str(json).expect("legacy JSON must load");
        assert!(r.tests.is_empty());
        assert!(!serde_json::to_string(&r).unwrap().contains("\"tests\""));
    }

    #[test]
    fn test_outcomes_round_trip() {
        let mut r = sample_result(vec![]);
        r.tests = vec![TestOutcome {
            test_key: "3".into(),
            status: CriterionStatus::Fail,
            source: "axe-core".into(),
            evidence: Some("image-alt on 2 nodes".into()),
        }];
        let json = serde_json::to_string(&r).unwrap();
        let back: CriterionResult = serde_json::from_str(&json).unwrap();
        assert_eq!(back.tests, r.tests);
        assert_eq!(
            back.tests[0].evidence.as_deref(),
            Some("image-alt on 2 nodes")
        );
    }
}
