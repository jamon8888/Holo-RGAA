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

/// Automatic assessment of a criterion, independent of human verification.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AutomatedVerdict {
    /// Automatically assessed as conforming.
    Pass,
    /// Automatically assessed as nonconforming.
    Fail,
    /// Automatically assessed as not applicable.
    NotApplicable,
}

/// Mechanism supporting an automatic assessment.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum VerdictBasis {
    /// An axe-core result.
    Axe,
    /// A deterministic rule or probe.
    Deterministic,
    /// A browser interaction or observation.
    Browser,
    /// A model estimate requiring separate verification.
    ModelEstimate,
}

/// Human review recorded separately from the automatic assessment.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReviewEvent {
    /// Status assigned by the reviewer.
    pub status: CriterionStatus,
    /// Identity of the reviewer.
    pub author: String,
    /// Timestamp of the review.
    pub reviewed_at: String,
    /// Reason supporting the reviewed status.
    pub reason: String,
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
/// This gates **inapplicability and nothing else**, which is the asymmetry #203
/// recommendation 2 turns on: a model may not make a criterion conform by declaring the
/// tests nobody covered inapplicable, because that would rest a published conformance
/// claim on an unreproducible judgement — the same defect as a `Pass` that cannot fail,
/// reached by a longer route.
///
/// It deliberately does **not** gate `Pass`. A model verdict can still pass a test, as it
/// can today at criterion granularity, so `IaAssiste` criteria stay decidable. Whether an
/// unreproducible `Pass` should be allowed to close a test at all is a real question and a
/// wider one than recommendation 2 settled — it belongs with #181 (what default-Pass rests
/// on), not here, and changing it silently would alter every `IaAssiste` verdict.
#[must_use]
pub fn is_deterministic_source(source: &str) -> bool {
    matches!(source, "axe-core" | "gap-fix" | "manual" | "automated")
}

/// Derive a criterion's verdict from its per-test outcomes.
///
/// `expected_keys` is the criterion's **test keys** from the catalog
/// (`CatalogCriterion::tests` / `TestAccounting::test_keys`), not a count. Coverage is
/// checked key by key, so a criterion whose mechanisms reported on only some of its tests
/// — or reported a key that does not belong to it — cannot be passed on the strength of
/// what they did reach.
///
/// Taking a bare count here was wrong twice over. It let outcomes for unknown keys stand
/// in for real ones (`("1", Pass), ("2", Pass), ("99", Pass)` covering a 3-test criterion
/// whose test 3 was never reported), and the only count available in the catalog,
/// `total_test_count`, counts **sub-items** rather than tests — 20 for criterion 1.1,
/// which has 8 test keys — so a count-based check was unsatisfiable for every criterion
/// with sub-items.
///
/// The rule (#203 recommendations 1 and 2):
///
/// 1. no outcomes at all → `None`, and the caller keeps whatever the mechanism set;
/// 2. any test failed → `Fail`, whatever else is known;
/// 3. any test errored → the criterion is not decided: `NotTested`;
/// 4. any outcome still unresolved — `NeedsReview`, `NotTested`, or an inapplicability
///    only a model asserted → `NeedsReview`, even when another outcome passed the same
///    key. A `Pass` next to an open question does not close the question;
/// 5. every expected key settled, and at least one of them by a `Pass` → `Pass`;
/// 6. every expected key settled and all of them deterministically inapplicable →
///    `NotApplicable`;
/// 7. an expected key left unsettled, or no expected keys known at all → `NeedsReview`,
///    because a human still has to close it. Not knowing which tests exist is not grounds
///    for declaring them satisfied.
#[must_use]
pub fn reduce_test_outcomes(
    tests: &[TestOutcome],
    expected_keys: &[String],
) -> Option<CriterionStatus> {
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

    // Rule 4: an unresolved outcome blocks the criterion even if the same key also has a
    // Pass. Whoever raised the question is still owed an answer.
    let unresolved = tests.iter().any(|t| {
        matches!(
            t.status,
            CriterionStatus::NeedsReview | CriterionStatus::NotTested
        ) || (t.status == CriterionStatus::NotApplicable && !is_deterministic_source(&t.source))
    });
    if unresolved {
        return Some(CriterionStatus::NeedsReview);
    }

    let settled: std::collections::BTreeSet<&str> = tests
        .iter()
        .filter(|t| t.status == CriterionStatus::Pass || settled_na(t))
        .map(|t| t.test_key.as_str())
        .collect();

    if expected_keys.is_empty() || !expected_keys.iter().all(|k| settled.contains(k.as_str())) {
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
    /// Automatic verdict, absent for older or incomplete assessments.
    #[serde(default)]
    pub automated_verdict: Option<AutomatedVerdict>,
    /// Mechanisms supporting the automatic verdict.
    #[serde(default)]
    pub verdict_basis: Vec<VerdictBasis>,
    /// Evidence supporting the assessment.
    #[serde(default)]
    pub evidence: Vec<crate::EvidenceRef>,
    /// Calibration version applied to confidence, independent of its raw value.
    #[serde(default)]
    pub confidence_calibration_version: Option<String>,
    /// Whether a human review is required.
    #[serde(default)]
    pub review_required: bool,
    /// Reason a human review is required.
    #[serde(default)]
    pub review_reason: Option<String>,
    /// Verified status; a model estimate alone does not populate this field.
    #[serde(default)]
    pub verified_status: Option<CriterionStatus>,
    /// History of human reviews of this criterion.
    #[serde(default)]
    pub review_events: Vec<ReviewEvent>,
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
    /// True only after every page and criterion passes the completion gate.
    #[serde(default)]
    pub audit_complete: bool,
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

    const LEGACY_CRITERION_JSON: &str = r#"{"criterion_id":"1.1","title":"images","classification":"Deterministe","status":"pass","violations":[],"confidence":1.0,"justification":"alt present","source":"axe-core","citations":[],"considered_sources":[],"tests":[]}"#;
    const LEGACY_AUDIT_JSON: &str = r#"{"audit_id":"old","url":"https://example.test","pages":[],"total_criteria":0,"passed":0,"failed":0,"na":0,"overall_compliance":0.0,"taux_global":0.0,"coverage_percent":0.0,"etat_conformite":"non conforme","duration_ms":0}"#;

    #[test]
    fn automatic_verdicts_round_trip_with_stable_json_names() -> serde_json::Result<()> {
        for (verdict, json) in [
            (AutomatedVerdict::Pass, r#""pass""#),
            (AutomatedVerdict::Fail, r#""fail""#),
            (AutomatedVerdict::NotApplicable, r#""not_applicable""#),
        ] {
            assert_eq!(serde_json::to_string(&verdict)?, json);
            assert_eq!(serde_json::from_str::<AutomatedVerdict>(json)?, verdict);
        }
        Ok(())
    }

    #[test]
    fn verdict_bases_round_trip_with_stable_json_names() -> serde_json::Result<()> {
        for (basis, json) in [
            (VerdictBasis::Axe, r#""axe""#),
            (VerdictBasis::Deterministic, r#""deterministic""#),
            (VerdictBasis::Browser, r#""browser""#),
            (VerdictBasis::ModelEstimate, r#""model_estimate""#),
        ] {
            assert_eq!(serde_json::to_string(&basis)?, json);
            assert_eq!(serde_json::from_str::<VerdictBasis>(json)?, basis);
        }
        Ok(())
    }

    #[test]
    fn criterion_result_legacy_json_defaults_assessment_fields() -> serde_json::Result<()> {
        let decoded: CriterionResult = serde_json::from_str(LEGACY_CRITERION_JSON)?;
        assert_eq!(decoded.status, CriterionStatus::Pass);
        assert_eq!(decoded.automated_verdict, None);
        assert!(decoded.verdict_basis.is_empty());
        assert!(decoded.evidence.is_empty());
        assert_eq!(decoded.confidence_calibration_version, None);
        assert!(!decoded.review_required);
        assert_eq!(decoded.review_reason, None);
        assert_eq!(decoded.verified_status, None);
        assert!(decoded.review_events.is_empty());
        Ok(())
    }

    #[test]
    fn audit_result_legacy_json_defaults_to_incomplete() -> serde_json::Result<()> {
        let old_audit: AuditResult = serde_json::from_str(LEGACY_AUDIT_JSON)?;
        assert!(!old_audit.audit_complete);
        let decoded: AuditResult = serde_json::from_str(&serde_json::to_string(&old_audit)?)?;
        assert!(!decoded.audit_complete);
        Ok(())
    }

    #[test]
    fn human_review_event_round_trips() -> serde_json::Result<()> {
        let json = r#"{"status":"fail","author":"auditrice","reviewed_at":"2026-10-07T10:00:00Z","reason":"alternative absente"}"#;
        let review: ReviewEvent = serde_json::from_str(json)?;
        assert_eq!(review.status, CriterionStatus::Fail);
        assert_eq!(review.author, "auditrice");
        assert_eq!(review.reviewed_at, "2026-10-07T10:00:00Z");
        assert_eq!(review.reason, "alternative absente");
        assert_eq!(
            serde_json::from_str::<ReviewEvent>(&serde_json::to_string(&review)?)?,
            review
        );
        Ok(())
    }

    #[test]
    fn criterion_assessment_fields_round_trip_without_verifying_model_estimate(
    ) -> serde_json::Result<()> {
        let mut result: CriterionResult = serde_json::from_str(LEGACY_CRITERION_JSON)?;
        result.automated_verdict = Some(AutomatedVerdict::Pass);
        result.verdict_basis = vec![VerdictBasis::ModelEstimate];
        result.evidence = vec![crate::EvidenceRef {
            kind: "dom_snapshot".into(),
            hash: "sha256:abc".into(),
            location: Some("snapshots/page.html".into()),
        }];
        result.confidence_calibration_version = Some("v1".into());
        result.review_required = true;
        result.review_reason = Some("model estimate requires review".into());
        let decoded: CriterionResult = serde_json::from_str(&serde_json::to_string(&result)?)?;
        assert_eq!(decoded, result);
        assert_eq!(decoded.verified_status, None);
        Ok(())
    }

    #[test]
    fn verified_review_history_round_trips_without_replacing_automatic_verdict(
    ) -> serde_json::Result<()> {
        let mut result: CriterionResult = serde_json::from_str(LEGACY_CRITERION_JSON)?;
        result.automated_verdict = Some(AutomatedVerdict::Pass);
        result.verified_status = Some(CriterionStatus::Fail);
        result.review_events = vec![ReviewEvent {
            status: CriterionStatus::Fail,
            author: "auditrice".into(),
            reviewed_at: "2026-10-07T10:00:00Z".into(),
            reason: "alternative absente".into(),
        }];
        let decoded: CriterionResult = serde_json::from_str(&serde_json::to_string(&result)?)?;
        assert_eq!(decoded, result);
        assert_eq!(decoded.automated_verdict, Some(AutomatedVerdict::Pass));
        assert_eq!(decoded.verified_status, Some(CriterionStatus::Fail));
        assert_eq!(decoded.review_events[0].status, CriterionStatus::Fail);
        Ok(())
    }

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

    /// The criterion's test keys as the catalog carries them.
    fn keys(k: &[&str]) -> Vec<String> {
        k.iter().map(|s| (*s).to_string()).collect()
    }

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
        assert_eq!(reduce_test_outcomes(&[], &keys(&["1", "2", "3"])), None);
    }

    /// A failure is a failure whatever else is known, and whoever found it.
    #[test]
    fn one_failed_test_fails_the_criterion() {
        let tests = vec![
            outcome("1", CriterionStatus::Pass, "axe-core"),
            outcome("2", CriterionStatus::Fail, "agent"),
            outcome("3", CriterionStatus::Pass, "axe-core"),
        ];
        assert_eq!(
            reduce_test_outcomes(&tests, &keys(&["1", "2", "3"])),
            Some(CriterionStatus::Fail)
        );
    }

    #[test]
    fn every_test_passing_passes_the_criterion() {
        let tests = vec![
            outcome("1", CriterionStatus::Pass, "axe-core"),
            outcome("2", CriterionStatus::Pass, "gap-fix"),
        ];
        assert_eq!(
            reduce_test_outcomes(&tests, &keys(&["1", "2"])),
            Some(CriterionStatus::Pass)
        );
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
            reduce_test_outcomes(&tests, &keys(&["1", "2", "3"])),
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
        assert_eq!(
            reduce_test_outcomes(&tests, &keys(&["1", "2"])),
            Some(CriterionStatus::Pass)
        );
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
            reduce_test_outcomes(&tests, &keys(&["1", "2"])),
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
                reduce_test_outcomes(&tests, &keys(&["1", "2"])),
                Some(CriterionStatus::NeedsReview),
                "{source} must not settle a test either"
            );
        }
    }

    /// A model may still *fail* a test — the asymmetry is deliberate and one-directional.
    #[test]
    fn a_model_can_still_fail_a_test() {
        let tests = vec![outcome("1", CriterionStatus::Fail, "agent")];
        assert_eq!(
            reduce_test_outcomes(&tests, &keys(&["1"])),
            Some(CriterionStatus::Fail)
        );
    }

    #[test]
    fn every_test_deterministically_inapplicable_makes_the_criterion_inapplicable() {
        let tests = vec![
            outcome("1", CriterionStatus::NotApplicable, "axe-core"),
            outcome("2", CriterionStatus::NotApplicable, "automated"),
        ];
        assert_eq!(
            reduce_test_outcomes(&tests, &keys(&["1", "2"])),
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
            reduce_test_outcomes(&tests, &keys(&["1", "2"])),
            Some(CriterionStatus::NotTested)
        );
    }

    /// Not knowing which tests a criterion has is not grounds for declaring them all
    /// satisfied.
    #[test]
    fn an_unknown_test_set_cannot_yield_a_pass() {
        let tests = vec![outcome("1", CriterionStatus::Pass, "axe-core")];
        assert_eq!(
            reduce_test_outcomes(&tests, &keys(&[])),
            Some(CriterionStatus::NeedsReview)
        );
    }

    /// Reported by CodeRabbit and Sourcery on #209, and real: a count-based check let
    /// outcomes for keys the criterion does not have stand in for the ones it does.
    #[test]
    fn an_outcome_for_an_unknown_key_does_not_cover_a_real_one() {
        let tests = vec![
            outcome("1", CriterionStatus::Pass, "axe-core"),
            outcome("2", CriterionStatus::Pass, "axe-core"),
            outcome("99", CriterionStatus::Pass, "axe-core"),
        ];
        assert_eq!(
            reduce_test_outcomes(&tests, &keys(&["1", "2", "3"])),
            Some(CriterionStatus::NeedsReview),
            "test 3 was never reported; an outcome for key 99 does not stand in for it"
        );
    }

    /// Reported by CodeRabbit on #209, and real: a `Pass` on a key did not stop an
    /// unresolved outcome on that same key from being ignored.
    #[test]
    fn an_unresolved_outcome_blocks_the_pass_even_beside_a_pass_on_the_same_key() {
        for open_status in [CriterionStatus::NeedsReview, CriterionStatus::NotTested] {
            let tests = vec![
                outcome("1", CriterionStatus::Pass, "axe-core"),
                outcome("1", open_status.clone(), "agent"),
                outcome("2", CriterionStatus::Pass, "axe-core"),
            ];
            assert_eq!(
                reduce_test_outcomes(&tests, &keys(&["1", "2"])),
                Some(CriterionStatus::NeedsReview),
                "a {open_status:?} on key 1 is still owed an answer"
            );
        }

        // and a model-asserted inapplicability beside a Pass on the same key
        let tests = vec![
            outcome("1", CriterionStatus::Pass, "axe-core"),
            outcome("1", CriterionStatus::NotApplicable, "agent"),
            outcome("2", CriterionStatus::Pass, "axe-core"),
        ];
        assert_eq!(
            reduce_test_outcomes(&tests, &keys(&["1", "2"])),
            Some(CriterionStatus::NeedsReview)
        );
    }

    /// The bug neither reviewer caught. `total_test_count` counts **sub-items** (20 for
    /// criterion 1.1) while the criterion has 8 test keys, so any count-based coverage
    /// check was unsatisfiable for every criterion with sub-items. Keys, not counts.
    #[test]
    fn coverage_is_measured_against_keys_not_the_catalogs_sub_item_count() {
        let (_, one_one) = crate::catalog::RgaaCatalog::by_id("1.1").expect("1.1 exists");
        let expected = &one_one.test_accounting.test_keys;
        assert_eq!(expected.len(), 8, "1.1 has 8 test keys");
        assert_eq!(
            one_one.test_accounting.total, 20,
            "while total_test_count counts 20 sub-items — the two must not be conflated"
        );

        let tests: Vec<TestOutcome> = expected
            .iter()
            .map(|k| outcome(k, CriterionStatus::Pass, "axe-core"))
            .collect();
        assert_eq!(
            reduce_test_outcomes(&tests, expected),
            Some(CriterionStatus::Pass),
            "all 8 keys passing must pass the criterion, not demand 20"
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
            reduce_test_outcomes(&tests, &keys(&["1", "2"])),
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
