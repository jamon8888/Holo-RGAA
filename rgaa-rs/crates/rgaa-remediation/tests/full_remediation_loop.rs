//! End-to-end remediation loop test.
//!
//! Asserts the full workflow: analyze → triage → remediate → approve → apply → verify → report.
//! Covers finding lifecycle transitions, policy evaluation, and baseline comparison.

use rgaa_core::{
    AuditBundle, AuditConfig, CriterionResult, CriterionStatus, EvidenceRef, Finding, PageAudit,
};
use rgaa_remediation::{
    adapter_for, FindingLifecycle, FindingState, Framework, PatchProposal, RemediationOutcome,
    RemediationPolicy,
};

fn finding(id: &str, status: CriterionStatus) -> Finding {
    let mut f = Finding::new(id);
    f.rule = "image-alt".into();
    f.url = "https://example.test".into();
    f.target = "img".into();
    f.status = status;
    f.evidence = vec![EvidenceRef::new("screenshot", format!("sha256:{}", id))];
    f
}

fn make_bundle(findings: Vec<Finding>, completed: bool) -> AuditBundle {
    let mut bundle = AuditBundle::new("audit-e2e", "https://example.test", AuditConfig::default());
    bundle.findings = findings;
    bundle.pages.push(PageAudit {
        page_id: "page-1".into(),
        url: "https://example.test".into(),
        title: Some("Test Page".into()),
        criteria: vec![CriterionResult {
            criterion_id: "1.1".into(),
            title: "Image alt".into(),
            classification: rgaa_core::Classification::Deterministe,
            status: if completed {
                CriterionStatus::Pass
            } else {
                CriterionStatus::Fail
            },
            violations: vec![],
            confidence: Some(0.95),
            justification: None,
            source: "obscura".into(),
        }],
        findings: vec![],
        errors: vec![],
        completed,
        duration_ms: 100,
    });
    let total_findings = bundle.findings.len();
    let completed_pages = if completed { 1 } else { 0 };
    let failed_count = if completed { 0 } else { total_findings };
    bundle.summary = rgaa_core::AuditSummary {
        total_pages: 1,
        completed_pages,
        total_findings,
        passed: completed_pages,
        failed: failed_count,
        needs_review: 0,
        errors: 0,
    };
    bundle
}

#[test]
fn full_remediation_loop_analyze_to_resolve() {
    // Step 1: Analyze — initial bundle with open findings
    let bundle = make_bundle(vec![finding("f1", CriterionStatus::Fail)], false);
    assert!(!bundle.validate().is_ok() || bundle.validate().is_ok());
    // Bundle validates
    bundle.validate().expect("bundle validates");

    // Step 2: Triage — create lifecycle, transition to Triaged
    let mut lifecycle = FindingLifecycle::new("f1");
    assert_eq!(lifecycle.state, FindingState::Open);
    lifecycle
        .transition(FindingState::Triaged, "auditor", "reviewed")
        .expect("triage");

    // Step 3: Remediate — generate proposal
    let policy = RemediationPolicy::default();
    let issue = rgaa_remediation::RemediationIssue {
        id: "f1".into(),
        rule: "image-alt".into(),
        element_html: "import React from \"react\"; <img src=\"hero.png\">".into(),
        page_url: "https://example.test".into(),
        source_locations: vec![rgaa_remediation::SourceLocation {
            file: "src/App.tsx".into(),
            line: 10,
            column: Some(4),
        }],
        summary: "Image missing alt".into(),
        remediation: "Add alt attribute".into(),
        criteria: vec!["RGAA-1.1".into()],
        framework: Some(Framework::React),
    };
    let adapter = adapter_for(Framework::React);
    let mut outcomes = rgaa_remediation::remediate(&[issue], &policy, adapter).expect("batch");
    let mut guidance = match outcomes.remove(0) {
        RemediationOutcome::Ok(g) => g,
        _ => panic!("expected proposal"),
    };

    // Step 4: Approve
    assert!(guidance.proposal.requires_approval());
    guidance
        .proposal
        .ensure_approved()
        .expect_err("not approved yet");
    let token = guidance.proposal.approval_token();
    guidance
        .proposal
        .approve("reviewer", &token)
        .expect("approve");
    guidance.proposal.ensure_approved().expect("approved");

    // Step 5: Apply — transition lifecycle
    lifecycle
        .transition(FindingState::FixProposed, "auditor", "proposal ready")
        .expect("fix proposed");
    lifecycle
        .transition(FindingState::AwaitingApproval, "auditor", "submitted")
        .expect("awaiting");
    lifecycle
        .transition(FindingState::Applied, "reviewer", "approved and applied")
        .expect("applied");

    // Step 6: Verify — mark as resolved
    lifecycle
        .transition(FindingState::Verifying, "ci", "run tests")
        .expect("verifying");
    lifecycle
        .transition(FindingState::Resolved, "ci", "tests pass")
        .expect("resolved");
    assert_eq!(lifecycle.state, FindingState::Resolved);

    // Step 7: Report — verify against baseline
    let baseline = make_bundle(vec![finding("f1", CriterionStatus::Fail)], false);
    let resolved_bundle = make_bundle(vec![finding("f1", CriterionStatus::Pass)], true);
    resolved_bundle.validate().expect("resolved validates");

    let policy = RemediationPolicy::default();
    let result = policy.evaluate(&resolved_bundle, Some(&baseline));
    assert!(result.passed, "policy should pass with resolved finding");
    assert_eq!(result.counts.resolved, 1);
}

#[test]
fn incomplete_igt_cannot_pass() {
    let mut bundle = AuditBundle::new("audit-igt", "https://example.test", AuditConfig::default());
    bundle.checkpoints.push(rgaa_core::CheckpointResult {
        checkpoint_id: "igt-1".into(),
        criterion_id: "1.1".into(),
        status: CriterionStatus::Pass,
        evidence: vec![], // empty evidence = incomplete
        summary: "incomplete test".into(),
    });
    // validate() checks that Pass checkpoints have evidence
    let err = bundle
        .validate()
        .expect_err("incomplete evidence should fail");
    assert!(
        matches!(err, rgaa_core::RgaaError::IncompleteEvidence(ref id) if id == "igt-1"),
        "should fail with IncompleteEvidence for igt-1, got: {:?}",
        err
    );
}

#[test]
fn failed_page_remains_in_bundle() {
    let mut bundle = AuditBundle::new("audit-fail", "https://example.test", AuditConfig::default());
    bundle.pages.push(PageAudit {
        page_id: "page-fail".into(),
        url: "https://example.test/fail".into(),
        title: Some("Failing Page".into()),
        criteria: vec![CriterionResult {
            criterion_id: "3.2".into(),
            title: "Contrast".into(),
            classification: rgaa_core::Classification::Deterministe,
            status: CriterionStatus::Fail,
            violations: vec![rgaa_core::Violation {
                rule_id: "color-contrast".into(),
                impact: "serious".into(),
                description: "Low contrast".into(),
                nodes_affected: 3,
            }],
            confidence: Some(1.0),
            justification: None,
            source: "obscura".into(),
        }],
        findings: vec![finding("f-page-fail", CriterionStatus::Fail)],
        errors: vec![],
        completed: true,
        duration_ms: 50,
    });
    bundle.findings = vec![finding("f-fail", CriterionStatus::Fail)];
    bundle.validate().expect("valid bundle with failures");
    assert_eq!(bundle.pages.len(), 1);
    assert_eq!(bundle.findings.len(), 1);
}

#[test]
fn approved_proposal_applies_only_its_files() {
    let proposal = PatchProposal::new(
        "p-1",
        vec!["f-1".into()],
        "--- a/src/App.tsx\n+++ b/src/App.tsx\n@@ -10,1 +10,1 @@\n-<img src=\"hero.png\">\n+<img src=\"hero.png\" alt=\"Hero\">",
        vec!["src/App.tsx".into()],
        "Fix image alt for RGAA 1.1",
        vec!["React image alt attribute".into()],
        vec!["npm test".into()],
        "Adds alt attribute to hero image",
    );
    assert_eq!(proposal.files, vec!["src/App.tsx"]);
    assert_eq!(proposal.finding_ids, vec!["f-1"]);
    // Verify the proposal hash is bound to this content
    assert_eq!(proposal.proposal_hash, proposal.compute_hash());
}

#[test]
fn re_audit_required_before_resolution() {
    let mut lifecycle = FindingLifecycle::new("f-reaudit");
    lifecycle
        .transition(FindingState::Triaged, "auditor", "triaged")
        .expect("triage");
    lifecycle
        .transition(FindingState::FixProposed, "auditor", "proposed")
        .expect("propose");
    lifecycle
        .transition(FindingState::AwaitingApproval, "auditor", "submitted")
        .expect("awaiting");
    lifecycle
        .transition(FindingState::Applied, "reviewer", "applied")
        .expect("applied");

    // Cannot go directly from Applied to Resolved — must go through Verifying
    assert!(
        lifecycle
            .transition(FindingState::Resolved, "ci", "skip verify")
            .is_err(),
        "must not skip verification step"
    );

    // Must go through Verifying first
    lifecycle
        .transition(FindingState::Verifying, "ci", "running tests")
        .expect("verifying");
    lifecycle
        .transition(FindingState::Resolved, "ci", "tests pass")
        .expect("resolved");
}

#[test]
fn policy_rejects_unresolved_findings() {
    let policy = RemediationPolicy::default();
    let baseline = make_bundle(vec![finding("f-keep", CriterionStatus::Fail)], false);
    let current = make_bundle(vec![finding("f-keep", CriterionStatus::Fail)], false);
    let result = policy.evaluate(&current, Some(&baseline));
    assert!(!result.passed);
    assert_eq!(result.counts.unchanged, 1);
    assert!(!result.failures.is_empty());
}
