use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use chrono::{TimeZone, Utc};
use rgaa_cli::commands::review::{apply_review, run, ReviewArgs, ReviewError, ReviewStatus};
use rgaa_cli::commands::CommonArgs;
use rgaa_core::{
    AuditResult, AutomatedVerdict, Classification, CriterionResult, CriterionStatus, EvidenceRef,
    PageResult, ReviewEvent, VerdictBasis,
};

static TEST_DIRECTORY_SEQUENCE: AtomicU64 = AtomicU64::new(0);

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        let sequence = TEST_DIRECTORY_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "rgaa-cli-review-test-{}-{sequence}",
            std::process::id()
        ));
        fs::create_dir_all(&path).expect("temporary test directory should be created");
        Self(path)
    }

    fn path(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn criterion(id: &str) -> CriterionResult {
    CriterionResult {
        criterion_id: id.to_owned(),
        title: format!("Critère {id}"),
        classification: Classification::IaAssiste,
        status: CriterionStatus::NeedsReview,
        violations: vec![],
        raw_confidence: Some(0.82),
        confidence: Some(0.71),
        justification: Some("prediction rationale".to_owned()),
        source: "agent-estimate".to_owned(),
        citations: vec![],
        considered_sources: vec!["agent-estimate".to_owned()],
        tests: vec![],
        automated_verdict: Some(AutomatedVerdict::Pass),
        verdict_basis: vec![VerdictBasis::ModelEstimate],
        evidence: vec![EvidenceRef::new("dom_snapshot", "sha256:abc")],
        confidence_calibration_version: Some("calibration-v1".to_owned()),
        review_required: true,
        review_reason: Some("manual confirmation required".to_owned()),
        verified_status: None,
        review_events: vec![],
    }
}

fn audit(page_urls: &[&str], criterion_ids: &[&str]) -> AuditResult {
    let pages = page_urls
        .iter()
        .enumerate()
        .map(|(index, url)| PageResult {
            url: (*url).to_owned(),
            title: Some(format!("Page {index}")),
            criteria: criterion_ids.iter().map(|id| criterion(id)).collect(),
            compliance_rate: 0.0,
            crawl_depth: index as u32,
        })
        .collect();
    AuditResult {
        audit_id: "audit-1".to_owned(),
        url: page_urls
            .first()
            .copied()
            .unwrap_or("https://example.test")
            .to_owned(),
        pages,
        total_criteria: criterion_ids.len() * page_urls.len(),
        passed: 0,
        failed: 0,
        na: 0,
        overall_compliance: 0.0,
        taux_global: 0.0,
        coverage_percent: 0.0,
        automatic_verdict_coverage_percent: 0.0,
        test_evidence_coverage_percent: 0.0,
        verified_compliance_percent: 0.0,
        etat_conformite: "incomplete".to_owned(),
        duration_ms: 1,
        audit_complete: true,
    }
}

fn args(input: &Path, criterion: &str, status: ReviewStatus) -> ReviewArgs {
    ReviewArgs {
        common: CommonArgs {
            config: None,
            output: None,
            format: None,
            audit_id: None,
            log_file: None,
        },
        input: input.to_path_buf(),
        criterion: criterion.to_owned(),
        status,
        author: "reviewer".to_owned(),
        reason: "verified against the source page".to_owned(),
        url: None,
    }
}

#[test]
fn applying_review_preserves_prediction_confidence_and_evidence() {
    let mut result = criterion("1.1");
    let prediction = result.automated_verdict;
    let raw_confidence = result.raw_confidence;
    let confidence = result.confidence;
    let basis = result.verdict_basis.clone();
    let evidence = result.evidence.clone();
    let source = result.source.clone();
    let reviewed_at = Utc.with_ymd_and_hms(2026, 10, 7, 12, 34, 56).unwrap();

    apply_review(
        &mut result,
        CriterionStatus::Fail,
        "  reviewer  ",
        reviewed_at,
        "  missing description  ",
    )
    .unwrap();

    assert_eq!(result.automated_verdict, prediction);
    assert_eq!(result.raw_confidence, raw_confidence);
    assert_eq!(result.confidence, confidence);
    assert_eq!(result.verdict_basis, basis);
    assert_eq!(result.evidence, evidence);
    assert_eq!(result.source, source);
    assert_eq!(result.verified_status, Some(CriterionStatus::Fail));
    assert_eq!(result.status, CriterionStatus::Fail);
    assert_eq!(
        result.review_events,
        vec![ReviewEvent {
            status: CriterionStatus::Fail,
            author: "reviewer".to_owned(),
            reviewed_at: "2026-10-07T12:34:56Z".to_owned(),
            reason: "missing description".to_owned(),
        }]
    );
}

#[test]
fn second_review_appends_to_history_without_replacing_the_prediction() {
    let mut result = criterion("1.1");
    let prediction = result.automated_verdict;
    let first = Utc.with_ymd_and_hms(2026, 10, 7, 12, 0, 0).unwrap();
    let second = Utc.with_ymd_and_hms(2026, 10, 8, 12, 0, 0).unwrap();

    apply_review(&mut result, CriterionStatus::Pass, "A", first, "confirmed").unwrap();
    apply_review(
        &mut result,
        CriterionStatus::Fail,
        "B",
        second,
        "new evidence",
    )
    .unwrap();

    assert_eq!(result.automated_verdict, prediction);
    assert_eq!(result.verified_status, Some(CriterionStatus::Fail));
    assert_eq!(result.review_events.len(), 2);
    assert_eq!(result.review_events[0].status, CriterionStatus::Pass);
    assert_eq!(result.review_events[1].status, CriterionStatus::Fail);
}

#[test]
fn apply_review_rejects_empty_author_reason_and_unsupported_status() {
    let when = Utc::now();
    let mut result = criterion("1.1");
    assert_eq!(
        apply_review(&mut result, CriterionStatus::Pass, "  ", when, "reason"),
        Err(ReviewError::EmptyAuthor)
    );
    assert_eq!(
        apply_review(&mut result, CriterionStatus::Pass, "reviewer", when, " \n "),
        Err(ReviewError::EmptyReason)
    );
    assert_eq!(
        apply_review(
            &mut result,
            CriterionStatus::NeedsReview,
            "reviewer",
            when,
            "reason"
        ),
        Err(ReviewError::UnsupportedStatus)
    );
    assert!(result.review_events.is_empty());
    assert_eq!(result.verified_status, None);
}

#[test]
fn clap_rejects_unsupported_review_status() {
    use clap::ValueEnum;
    assert!(ReviewStatus::from_str("needs_review", true).is_err());
    assert!(ReviewStatus::from_str("not_applicable", true).is_ok());
}

#[test]
fn command_rejects_unknown_criterion_and_missing_target() {
    let directory = TestDirectory::new();
    let input = directory.path("audit.json");
    fs::write(
        &input,
        serde_json::to_vec(&audit(&["https://a.test"], &["1.1"])).unwrap(),
    )
    .unwrap();

    let unknown = args(&input, "99.99", ReviewStatus::Pass);
    assert_eq!(run(unknown).unwrap_err().exit_code(), 2);

    let mut missing = args(&input, "1.2", ReviewStatus::Pass);
    missing.author = "reviewer".to_owned();
    assert_eq!(run(missing).unwrap_err().exit_code(), 2);
}

#[test]
fn command_requires_url_for_ambiguous_multi_page_criterion() {
    let directory = TestDirectory::new();
    let input = directory.path("multi.json");
    let original = audit(&["https://a.test", "https://b.test"], &["1.1"]);
    fs::write(&input, serde_json::to_vec(&original).unwrap()).unwrap();

    assert_eq!(
        run(args(&input, "1.1", ReviewStatus::Fail))
            .unwrap_err()
            .exit_code(),
        2
    );
    let unchanged: AuditResult = serde_json::from_slice(&fs::read(&input).unwrap()).unwrap();
    assert_eq!(unchanged, original);
}

#[test]
fn command_targets_exact_page_and_rejects_duplicate_criterion_entries() {
    let directory = TestDirectory::new();
    let input = directory.path("multi.json");
    let mut original = audit(&["https://a.test", "https://b.test"], &["1.1"]);
    fs::write(&input, serde_json::to_vec(&original).unwrap()).unwrap();

    let mut targeted = args(&input, "1.1", ReviewStatus::Fail);
    targeted.url = Some("https://b.test".to_owned());
    run(targeted).unwrap();
    let reviewed: AuditResult = serde_json::from_slice(&fs::read(&input).unwrap()).unwrap();
    assert_eq!(reviewed.pages[0].criteria[0].verified_status, None);
    assert_eq!(
        reviewed.pages[1].criteria[0].verified_status,
        Some(CriterionStatus::Fail)
    );
    assert_eq!(
        reviewed.pages[1].criteria[0].automated_verdict,
        Some(AutomatedVerdict::Pass)
    );

    original.pages[0].criteria.push(criterion("1.1"));
    fs::write(&input, serde_json::to_vec(&original).unwrap()).unwrap();
    let mut duplicate = args(&input, "1.1", ReviewStatus::Pass);
    duplicate.url = Some("https://a.test".to_owned());
    assert_eq!(run(duplicate).unwrap_err().exit_code(), 2);
}

#[test]
fn command_writes_output_or_atomically_replaces_input() {
    let directory = TestDirectory::new();
    let input = directory.path("input.json");
    let output = directory.path("output.json");
    let original = audit(&["https://a.test"], &["1.1"]);
    let original_bytes = serde_json::to_vec(&original).unwrap();
    fs::write(&input, &original_bytes).unwrap();

    let mut to_output = args(&input, "1.1", ReviewStatus::NotApplicable);
    to_output.common.output = Some(output.clone());
    run(to_output).unwrap();
    assert_eq!(fs::read(&input).unwrap(), original_bytes);
    let output_audit: AuditResult = serde_json::from_slice(&fs::read(&output).unwrap()).unwrap();
    assert_eq!(
        output_audit.pages[0].criteria[0].verified_status,
        Some(CriterionStatus::NotApplicable)
    );

    run(args(&input, "1.1", ReviewStatus::Pass)).unwrap();
    let replaced: AuditResult = serde_json::from_slice(&fs::read(&input).unwrap()).unwrap();
    assert_eq!(
        replaced.pages[0].criteria[0].verified_status,
        Some(CriterionStatus::Pass)
    );
    assert_eq!(replaced.pages[0].criteria[0].review_events.len(), 1);
    assert!(fs::read_dir(&directory.0).unwrap().all(|entry| {
        !entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .ends_with(".tmp")
    }));
}

#[test]
fn command_refreshes_status_aggregates_without_changing_automatic_coverage() {
    let directory = TestDirectory::new();
    let input = directory.path("review-metrics.json");
    let mut original = audit(&["https://a.test"], &["1.1", "1.2", "1.3"]);
    original.pages[0].criteria[1].status = CriterionStatus::Pass;
    original.pages[0].criteria[1].verified_status = Some(CriterionStatus::Pass);
    original.pages[0].criteria[2].status = CriterionStatus::NotApplicable;
    original.pages[0].criteria[2].verified_status = Some(CriterionStatus::NotApplicable);
    original.pages[0].compliance_rate = 12.0;
    original.passed = 7;
    original.failed = 8;
    original.na = 9;
    original.overall_compliance = 10.0;
    original.taux_global = 11.0;
    original.coverage_percent = 12.0;
    original.automatic_verdict_coverage_percent = 73.0;
    original.test_evidence_coverage_percent = 64.0;
    original.verified_compliance_percent = 14.0;
    original.etat_conformite = "stale".to_owned();
    original.audit_complete = true;
    fs::write(&input, serde_json::to_vec(&original).unwrap()).unwrap();

    run(args(&input, "1.1", ReviewStatus::Fail)).unwrap();
    let reviewed: AuditResult = serde_json::from_slice(&fs::read(&input).unwrap()).unwrap();
    let all_criteria: Vec<_> = reviewed
        .pages
        .iter()
        .flat_map(|page| page.criteria.iter().cloned())
        .collect();
    let expected_metrics = rgaa_report::compute_metrics(&all_criteria, &rgaa_report::RGAA_41);
    let expected_audit_metrics = rgaa_report::compute_audit_metrics(&reviewed.pages);

    assert_eq!((reviewed.passed, reviewed.failed, reviewed.na), (1, 1, 1));
    assert_eq!(
        reviewed.pages[0].compliance_rate,
        rgaa_report::compliance_rate(&reviewed.pages[0].criteria)
    );
    assert_eq!(
        reviewed.overall_compliance,
        rgaa_report::compliance_rate(&all_criteria)
    );
    assert_eq!(reviewed.taux_global, expected_metrics.taux_global);
    assert_eq!(reviewed.coverage_percent, expected_metrics.coverage_percent);
    assert_eq!(reviewed.etat_conformite, expected_metrics.etat_conformite);
    assert_eq!(
        reviewed.verified_compliance_percent,
        expected_audit_metrics.verified_compliance_percent
    );
    assert_eq!(reviewed.automatic_verdict_coverage_percent, 73.0);
    assert_eq!(reviewed.test_evidence_coverage_percent, 64.0);
    assert_eq!(reviewed.audit_complete, original.audit_complete);
    assert_eq!(reviewed.total_criteria, original.total_criteria);
    assert_eq!(
        reviewed.pages[0].criteria[0].automated_verdict,
        original.pages[0].criteria[0].automated_verdict
    );
    assert_eq!(
        reviewed.pages[0].criteria[0].raw_confidence,
        original.pages[0].criteria[0].raw_confidence
    );
    assert_eq!(
        reviewed.pages[0].criteria[0].confidence,
        original.pages[0].criteria[0].confidence
    );
    assert_eq!(
        reviewed.pages[0].criteria[0].evidence,
        original.pages[0].criteria[0].evidence
    );
}

#[cfg(unix)]
#[test]
fn in_place_replacement_preserves_restrictive_file_permissions() {
    use std::os::unix::fs::PermissionsExt;

    let directory = TestDirectory::new();
    let input = directory.path("private-audit.json");
    fs::write(
        &input,
        serde_json::to_vec(&audit(&["https://a.test"], &["1.1"])).unwrap(),
    )
    .unwrap();
    fs::set_permissions(&input, fs::Permissions::from_mode(0o600)).unwrap();

    run(args(&input, "1.1", ReviewStatus::Fail)).unwrap();

    let mode = fs::metadata(&input).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o600);
}

#[test]
fn command_rejects_malformed_json_and_invalid_author_or_reason() {
    let directory = TestDirectory::new();
    let input = directory.path("audit.json");
    fs::write(&input, b"{").unwrap();
    assert_eq!(
        run(args(&input, "1.1", ReviewStatus::Pass))
            .unwrap_err()
            .exit_code(),
        2
    );

    fs::write(
        &input,
        serde_json::to_vec(&audit(&["https://a.test"], &["1.1"])).unwrap(),
    )
    .unwrap();
    let mut empty_author = args(&input, "1.1", ReviewStatus::Pass);
    empty_author.author = "  ".to_owned();
    assert_eq!(run(empty_author).unwrap_err().exit_code(), 2);
    let mut empty_reason = args(&input, "1.1", ReviewStatus::Pass);
    empty_reason.reason = "\t".to_owned();
    assert_eq!(run(empty_reason).unwrap_err().exit_code(), 2);
}
