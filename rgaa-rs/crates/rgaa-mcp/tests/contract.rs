use rgaa_mcp::server::{RemediationService, RemediationServiceImpl};
use rgaa_mcp::{
    AnalyzeConfigInput, AnalyzeRequest, CookieReferenceInput, GuidedTestRequest,
    RemediationRequest, ScreenshotPolicyInput, ToolServer,
};
use rgaa_remediation::{RemediationIssue, RemediationOutcome, SourceLocation};
use schemars::schema_for;

#[test]
fn exposes_exactly_three_agent_tools() {
    assert_eq!(ToolServer::tool_names(), ["analyze", "remediate", "igt"]);
}

#[test]
fn schemas_are_objects_with_required_fields() {
    let analyze = serde_json::to_value(schema_for!(AnalyzeRequest)).expect("schema");
    let remediate = serde_json::to_value(schema_for!(RemediationRequest)).expect("schema");
    let igt = serde_json::to_value(schema_for!(GuidedTestRequest)).expect("schema");
    assert_eq!(analyze["type"], "object");
    assert!(analyze["required"]
        .as_array()
        .unwrap()
        .iter()
        .any(|v| v == "url"));
    assert!(remediate["required"]
        .as_array()
        .unwrap()
        .iter()
        .any(|v| v == "issues"));
    assert!(igt["required"]
        .as_array()
        .unwrap()
        .iter()
        .any(|v| v == "test"));
}

#[test]
fn remediation_batch_bounds_are_enforced() {
    assert!(RemediationRequest::validate_issue_count(0).is_err());
    assert!(RemediationRequest::validate_issue_count(26).is_err());
    assert!(RemediationRequest::validate_issue_count(1).is_ok());
    assert!(RemediationRequest::validate_issue_count(25).is_ok());
}

#[test]
fn malformed_inputs_have_stable_codes() {
    let error = AnalyzeRequest::malformed("file:///etc/passwd").expect_err("must reject");
    assert_eq!(error.code(), "INVALID_INPUT");
}

#[test]
fn serialized_inputs_never_contain_cookie_values() {
    let request = AnalyzeRequest {
        url: "https://example.test".into(),
        config: AnalyzeConfigInput {
            cookie_references: vec![CookieReferenceInput {
                name: "session".into(),
                domain: None,
            }],
            screenshot_policy: ScreenshotPolicyInput::None,
            ..Default::default()
        },
    };
    let json = serde_json::to_string(&request).expect("serialize");
    assert!(!json.contains("super-secret"));
    assert!(!json.contains("RGAA_COOKIE_SESSION"));
}

#[test]
fn remediation_keeps_one_outcome_per_issue() {
    let service = RemediationServiceImpl::default();
    let valid = RemediationIssue {
        id: "valid".into(),
        rule: "image-alt".into(),
        element_html: "import React from \"react\"; <img src=\"hero.png\">".into(),
        page_url: "https://example.test".into(),
        source_locations: vec![SourceLocation {
            file: "src/App.tsx".into(),
            line: 1,
            column: None,
        }],
        summary: "missing alternative text".into(),
        remediation: "add alt".into(),
        criteria: vec!["RGAA-1.1".into()],
        framework: Some(rgaa_remediation::Framework::React),
    };
    let invalid = RemediationIssue {
        id: "invalid".into(),
        rule: String::new(),
        ..valid.clone()
    };
    let outcomes = service.remediate(vec![valid, invalid]).expect("batch");
    assert_eq!(outcomes.len(), 2);
    assert!(
        matches!(&outcomes[0], RemediationOutcome::Ok(guidance) if guidance.issue_id == "valid")
    );
    assert!(
        matches!(&outcomes[1], RemediationOutcome::Error(error) if error.issue_id == "invalid")
    );
}
