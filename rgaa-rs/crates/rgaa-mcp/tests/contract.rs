use rgaa_mcp::server::{
    AnalyzeService, GuidedService, NoOpStorageService, OrchestrationService, RemediationService,
    RemediationServiceImpl,
};
use rgaa_mcp::{
    AnalyzeConfigInput, AnalyzeRequest, ApprovalStateDto, CookieInput, GuidedTestRequest,
    LazyObscuraBridge, McpFailure, ObscuraAnalyzeService, RemediationRequest, RemediationResponse,
    ToolServer,
};
use rgaa_remediation::{RemediationIssue, RemediationOutcome, SourceLocation};
use rmcp::handler::server::wrapper::Parameters;
use schemars::schema_for;
use std::sync::Arc;

#[test]
fn exposes_exactly_three_agent_tools() {
    assert_eq!(
        ToolServer::tool_names(),
        [
            "analyze",
            "remediate",
            "igt",
            "audit_url",
            "get_audit_result"
        ]
    );
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
fn output_schemas_are_typed_not_unconstrained_json() {
    let analyze = serde_json::to_value(schema_for!(rgaa_mcp::AnalyzeResponse)).unwrap();
    let remediate = serde_json::to_value(schema_for!(RemediationResponse)).unwrap();
    let igt = serde_json::to_value(schema_for!(rgaa_mcp::GuidedTestResponse)).unwrap();

    let findings = &analyze["properties"]["findings"]["items"];
    let outcomes = &remediate["properties"]["outcomes"]["items"];
    let evidence = &igt["properties"]["evidence"]["items"];

    for items in [findings, outcomes, evidence] {
        assert_ne!(
            *items,
            serde_json::json!({}),
            "output item schema must not be an unconstrained serde_json::Value"
        );
    }
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
            cookies: vec![CookieInput {
                name: "session".into(),
                value: "super-secret-value".into(),
                domain: "example.test".into(),
                path: None,
                same_site: None,
                r#secure: None,
                http_only: None,
                expires: None,
            }],
            ..Default::default()
        },
        viewport_width: None,
        viewport_height: None,
    };
    let json = serde_json::to_string(&request).expect("serialize");
    assert!(!json.contains("super-secret"));
    assert!(!json.contains("secret-value"));
}

#[test]
fn remediation_keeps_one_outcome_per_issue() {
    let service = RemediationServiceImpl::default();
    let valid = valid_issue("valid", "image-alt");
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

#[test]
fn approval_state_and_token_are_surfaced_in_response_dto() {
    let service = RemediationServiceImpl::default();
    let outcomes = service
        .remediate(vec![valid_issue("approval", "image-alt")])
        .expect("batch");
    let dto: rgaa_mcp::RemediationOutcomeDto =
        rgaa_mcp::RemediationOutcomeDto::from(outcomes.into_iter().next().unwrap());
    match dto {
        rgaa_mcp::RemediationOutcomeDto::Ok {
            proposal, issue_id, ..
        } => {
            assert_eq!(issue_id, "approval");
            assert_eq!(proposal.approval_state, ApprovalStateDto::Required);
            assert!(proposal.approval_token.starts_with("rgaa-approval-v1-"));
        }
        rgaa_mcp::RemediationOutcomeDto::Error { .. } => panic!("expected an ok proposal"),
    }
}

#[tokio::test]
async fn analyze_handler_preserves_invalid_input_code() {
    let server = test_server();
    let result = server
        .analyze(Parameters(AnalyzeRequest {
            url: "file:///etc/passwd".into(),
            config: Default::default(),
            viewport_width: None,
            viewport_height: None,
        }))
        .await;
    let err = unwrap_err(result, "must reject non-http URL");
    assert_eq!(err.data.as_ref().unwrap()["code"], "INVALID_INPUT");
    assert!(err.message.contains("INVALID_INPUT"));
}

#[tokio::test]
async fn analyze_handler_distinguishes_execution_failure_and_redacts_secrets() {
    struct Failing;
    impl AnalyzeService for Failing {
        fn analyze(
            &self,
            _request: rgaa_obscura::AnalyzeRequest,
        ) -> std::pin::Pin<
            Box<
                dyn std::future::Future<
                        Output = Result<rgaa_obscura::AnalyzePageResult, McpFailure>,
                    > + Send
                    + '_,
            >,
        > {
            Box::pin(async { Err(McpFailure::execution("cookie=secret123")) })
        }
    }
    let server = ToolServer::new(
        Arc::new(Failing),
        Arc::new(RemediationServiceImpl::default()),
        Arc::new(PanickingGuided),
        Arc::new(OrchestrationService::new()),
        Arc::new(NoOpStorageService),
    );
    let result = server
        .analyze(Parameters(AnalyzeRequest {
            url: "https://example.test".into(),
            config: Default::default(),
            viewport_width: None,
            viewport_height: None,
        }))
        .await;
    let err = unwrap_err(result, "service failed");
    assert_eq!(err.data.as_ref().unwrap()["code"], "EXECUTION_FAILED");
    assert!(!err.message.contains("secret123"));
    assert!(err.message.contains("[REDACTED]"));
}

#[test]
fn remediate_handler_rejects_mismatched_outcomes() {
    struct Mismatched;
    impl RemediationService for Mismatched {
        fn remediate(
            &self,
            _issues: Vec<RemediationIssue>,
        ) -> Result<Vec<RemediationOutcome>, McpFailure> {
            Ok(vec![])
        }
    }
    let server = ToolServer::new(
        Arc::new(PanickingAnalyze),
        Arc::new(Mismatched),
        Arc::new(PanickingGuided),
        Arc::new(OrchestrationService::new()),
        Arc::new(NoOpStorageService),
    );
    let result = server.remediate(Parameters(RemediationRequest {
        issues: vec![valid_issue_input("one")],
    }));
    let err = unwrap_err(result, "mismatched outcomes must be rejected");
    assert_eq!(err.data.as_ref().unwrap()["code"], "INCOMPLETE_RESULT");
}

#[test]
fn remediate_handler_rejects_empty_batch_with_invalid_input() {
    let server = test_server();
    let result = server.remediate(Parameters(RemediationRequest { issues: vec![] }));
    let err = unwrap_err(result, "empty batch must be rejected");
    assert_eq!(err.data.as_ref().unwrap()["code"], "INVALID_INPUT");
}

#[tokio::test]
async fn analyze_service_returns_typed_unavailable_error_without_browser() {
    let bridge = Arc::new(LazyObscuraBridge::new(
        rgaa_obscura::ObscuraBridge::with_binary_path("/nonexistent/obscura-binary".into()),
    ));
    let service = ObscuraAnalyzeService::new(bridge);
    let request = rgaa_obscura::AnalyzeRequest {
        url: "https://example.test".into(),
        config: rgaa_obscura::AnalyzeConfig::default(),
    };
    let error = service
        .analyze(request)
        .await
        .expect_err("no browser available");
    assert_eq!(error.code(), "UNSUPPORTED_CONFIGURATION");
}

fn unwrap_err<T>(result: Result<T, rmcp::ErrorData>, message: &str) -> rmcp::ErrorData {
    match result {
        Err(err) => err,
        Ok(_) => panic!("{message}"),
    }
}

fn valid_issue(id: &str, rule: &str) -> RemediationIssue {
    RemediationIssue {
        id: id.into(),
        rule: rule.into(),
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
    }
}

fn valid_issue_input(id: &str) -> rgaa_mcp::RemediationIssueInput {
    rgaa_mcp::RemediationIssueInput {
        id: id.into(),
        rule: "image-alt".into(),
        element_html: "import React from \"react\"; <img src=\"hero.png\">".into(),
        page_url: "https://example.test".into(),
        source_locations: vec![rgaa_mcp::SourceLocationInput {
            file: "src/App.tsx".into(),
            line: 1,
            column: None,
        }],
        summary: "missing alternative text".into(),
        remediation: "add alt".into(),
        criteria: vec!["RGAA-1.1".into()],
        framework: Some(rgaa_mcp::FrameworkInput::React),
    }
}

fn test_server() -> ToolServer {
    ToolServer::new(
        Arc::new(PanickingAnalyze),
        Arc::new(RemediationServiceImpl::default()),
        Arc::new(PanickingGuided),
        Arc::new(OrchestrationService::new()),
        Arc::new(NoOpStorageService),
    )
}

struct PanickingAnalyze;
impl AnalyzeService for PanickingAnalyze {
    fn analyze(
        &self,
        _request: rgaa_obscura::AnalyzeRequest,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = Result<rgaa_obscura::AnalyzePageResult, McpFailure>>
                + Send
                + '_,
        >,
    > {
        Box::pin(async { Err(McpFailure::execution("unexpected analyze call")) })
    }
}

struct PanickingGuided;
impl GuidedService for PanickingGuided {
    fn run(
        &self,
        _test: rgaa_obscura::GuidedTest,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = Result<rgaa_obscura::GuidedRunResult, McpFailure>>
                + Send
                + '_,
        >,
    > {
        Box::pin(async { Err(McpFailure::execution("unexpected guided call")) })
    }
}
