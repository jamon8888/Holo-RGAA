//! Security contract tests.
//!
//! Asserts: no secrets in logs, no cookie values in serialized requests,
//! remote upload opt-in, request-size limits, and safe error messages.

use rgaa_core::{AuditBundle, AuditConfig};
use rgaa_remediation::{adapter_for, Framework, RemediationIssue, RemediationPolicy};
use serde_json::json;

#[test]
fn no_secrets_in_serialized_proposal() {
    let proposal = rgaa_remediation::PatchProposal::new(
        "p-sec",
        vec!["f-sec".into()],
        "diff content",
        vec!["src/App.tsx".into()],
        "rationale",
        vec!["risk".into()],
        vec!["cargo test".into()],
        "effect",
    );
    let json_str = serde_json::to_string(&proposal).unwrap();

    // No API keys, tokens, passwords, or cookies in serialized output
    let lower = json_str.to_lowercase();
    assert!(
        !lower.contains("password"),
        "proposal JSON contains 'password'"
    );
    assert!(
        !lower.contains("api_key"),
        "proposal JSON contains 'api_key'"
    );
    assert!(!lower.contains("secret"), "proposal JSON contains 'secret'");
    assert!(!lower.contains("cookie"), "proposal JSON contains 'cookie'");
    assert!(!lower.contains("bearer"), "proposal JSON contains 'bearer'");
    assert!(
        !lower.contains("authorization"),
        "proposal JSON contains 'authorization'"
    );
}

#[test]
fn no_secrets_in_serialized_finding() {
    let finding = rgaa_core::Finding::new("f-no-leak");
    let json_str = serde_json::to_string(&finding).unwrap();
    let lower = json_str.to_lowercase();
    assert!(
        !lower.contains("password"),
        "finding JSON contains 'password'"
    );
    assert!(
        !lower.contains("api_key"),
        "finding JSON contains 'api_key'"
    );
    assert!(
        !lower.contains("secret_key"),
        "finding JSON contains 'secret_key'"
    );
}

#[test]
fn no_secrets_in_serialized_bundle() {
    let bundle = AuditBundle::new("audit-sec", "https://example.test", AuditConfig::default());
    let json_str = serde_json::to_string(&bundle).unwrap();
    let lower = json_str.to_lowercase();
    assert!(
        !lower.contains("password"),
        "bundle JSON contains 'password'"
    );
    assert!(!lower.contains("api_key"), "bundle JSON contains 'api_key'");
    assert!(!lower.contains("secret"), "bundle JSON contains 'secret'");
}

#[test]
fn remote_remediation_opt_in_required() {
    // Policy with remote disabled should block remote usage
    let policy = RemediationPolicy {
        allow_remote_ai: false,
        use_remote_ai: true,
        ..Default::default()
    };
    let issue = RemediationIssue {
        id: "remote-block".into(),
        rule: "image-alt".into(),
        element_html: "import React from \"react\"; <img src=\"x\">".into(),
        page_url: "https://example.test".into(),
        source_locations: vec![rgaa_remediation::SourceLocation {
            file: "src/App.tsx".into(),
            line: 1,
            column: Some(1),
        }],
        summary: "missing alt".into(),
        remediation: "add alt".into(),
        criteria: vec!["RGAA-1.1".into()],
        framework: Some(Framework::React),
    };
    let adapter = adapter_for(Framework::React);
    let outcomes = rgaa_remediation::remediate(&[issue], &policy, adapter).expect("batch");
    match &outcomes[0] {
        rgaa_remediation::RemediationOutcome::Error(e) => {
            assert_eq!(e.code, rgaa_remediation::RemediationErrorCode::PolicyDenied);
        }
        _ => panic!("expected policy denied error for remote remediation"),
    }
}

#[test]
fn batch_size_limits_enforced() {
    let policy = RemediationPolicy::default();
    let adapter = adapter_for(Framework::React);

    // Empty batch rejected
    assert!(rgaa_remediation::remediate(&[], &policy, adapter).is_err());

    // 26 issues rejected
    let issues: Vec<_> = (0..26)
        .map(|i| RemediationIssue {
            id: format!("i-{i}"),
            rule: "image-alt".into(),
            element_html: "import React from \"react\"; <img src=\"x\">".into(),
            page_url: "https://example.test".into(),
            source_locations: vec![rgaa_remediation::SourceLocation {
                file: "src/App.tsx".into(),
                line: 1,
                column: Some(1),
            }],
            summary: "missing alt".into(),
            remediation: "add alt".into(),
            criteria: vec!["RGAA-1.1".into()],
            framework: Some(Framework::React),
        })
        .collect();
    assert!(rgaa_remediation::remediate(&issues, &policy, adapter).is_err());

    // 25 issues accepted
    let issues_25: Vec<_> = (0..25)
        .map(|i| RemediationIssue {
            id: format!("i-{i}"),
            rule: "image-alt".into(),
            element_html: "import React from \"react\"; <img src=\"x\">".into(),
            page_url: "https://example.test".into(),
            source_locations: vec![rgaa_remediation::SourceLocation {
                file: "src/App.tsx".into(),
                line: 1,
                column: Some(1),
            }],
            summary: "missing alt".into(),
            remediation: "add alt".into(),
            criteria: vec!["RGAA-1.1".into()],
            framework: Some(Framework::React),
        })
        .collect();
    assert!(rgaa_remediation::remediate(&issues_25, &policy, adapter).is_ok());
}

#[test]
fn error_messages_are_safe() {
    let policy = RemediationPolicy::default();
    let adapter = adapter_for(Framework::React);

    // Invalid issue should produce safe error message (no file paths, no stack traces)
    let bad_issue = RemediationIssue {
        id: "".into(),
        rule: "image-alt".into(),
        element_html: "img".into(),
        page_url: "https://example.test".into(),
        source_locations: vec![],
        summary: "missing alt".into(),
        remediation: "add alt".into(),
        criteria: vec![],
        framework: Some(Framework::React),
    };
    let outcomes = rgaa_remediation::remediate(&[bad_issue], &policy, adapter).expect("batch");
    match &outcomes[0] {
        rgaa_remediation::RemediationOutcome::Error(e) => {
            // Error message should not leak internal paths
            assert!(!e.message.contains("/home/"), "error contains file path");
            assert!(
                !e.message.contains("\\Users\\"),
                "error contains Windows path"
            );
            assert!(!e.message.contains("src/"), "error contains source path");
            // Error code should be a valid variant
            assert_ne!(e.code, rgaa_remediation::RemediationErrorCode::NeedsReview);
        }
        _ => panic!("expected error for invalid issue"),
    }
}

#[test]
fn approval_token_is_bound_to_proposal() {
    let proposal = rgaa_remediation::PatchProposal::new(
        "p-bind",
        vec!["f-bind".into()],
        "diff",
        vec!["file.tsx".into()],
        "why",
        vec![],
        vec![],
        "effect",
    );
    let token = proposal.approval_token();

    // Correct token works
    let mut p = proposal.clone();
    p.approve("reviewer", &token).expect("valid token");

    // Wrong token rejected
    let mut p2 = proposal.clone();
    assert!(p2.approve("reviewer", "wrong-token").is_err());

    // Modified proposal: approve succeeds (token matches stored hash) but ensure_approved fails
    let mut p3 = proposal.clone();
    let token3 = p3.approval_token();
    p3.diff = "tampered".into();
    // approve() only checks token against stored proposal_hash, so it succeeds
    p3.approve("reviewer", &token3).expect("token matches");
    // But ensure_approved() recomputes hash and detects tampering
    assert!(
        p3.ensure_approved().is_err(),
        "tampered proposal should fail ensure_approved"
    );
}

#[test]
fn deserialized_approval_bypass_is_blocked() {
    let proposal = rgaa_remediation::PatchProposal::new(
        "p-bypass",
        vec!["f-bypass".into()],
        "diff",
        vec!["file.tsx".into()],
        "why",
        vec![],
        vec![],
        "effect",
    );
    let mut payload = serde_json::to_value(&proposal).unwrap();
    // Try to inject "NotRequired" state to bypass approval
    payload["approval"] = json!("NotRequired");
    let restored: rgaa_remediation::PatchProposal =
        serde_json::from_value(payload).expect("deserialize");
    assert!(restored.requires_approval());
    assert!(restored.ensure_approved().is_err());
}
