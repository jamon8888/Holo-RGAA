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

/// The tool surface is a contract with every agent already wired to this
/// server, so growing it is a deliberate act: #166 adds `lint_static`.
#[test]
fn exposes_every_registered_agent_tool() {
    assert_eq!(
        ToolServer::tool_names(),
        [
            "analyze",
            "remediate",
            "igt",
            "audit_url",
            "get_audit_result",
            "list_criteria",
            "lint_static",
            "source_map",
            "verify_fix",
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

/// Issue #32: the substrate version is recorded on `AnalyzePageResult` but
/// used to stop at the MCP boundary, so a client could not tell which
/// Obscura produced a finding. These tests pin that it now crosses, and that
/// it crosses *additively*.
mod substrate_version {
    use rgaa_mcp::AnalyzeResponse;
    use rgaa_obscura::{AnalyzePageResult, IgtResult, IgtResults};

    fn result(version: Option<&str>, igt: bool) -> AnalyzePageResult {
        AnalyzePageResult {
            url: "https://example.test".into(),
            findings: Vec::new(),
            evidence: Vec::new(),
            errors: Vec::new(),
            completed: true,
            duration_ms: 7,
            igt: igt.then(|| IgtResults {
                keyboard: IgtResult {
                    status: "pass".into(),
                    issues: Vec::new(),
                    igt_elements: Vec::new(),
                    terminated_reason: None,
                },
            }),
            obscura_version: version.map(ToOwned::to_owned),
        }
    }

    fn json(version: Option<&str>, igt: bool) -> serde_json::Value {
        serde_json::to_value(AnalyzeResponse::from_result(result(version, igt))).expect("serialize")
    }

    #[test]
    fn the_flat_shape_carries_the_version_the_bridge_reported() {
        assert_eq!(
            json(Some("obscura 0.2.2"), false)["obscura_version"],
            "obscura 0.2.2"
        );
    }

    /// The nested shape is a different struct, so it needs its own proof —
    /// this is exactly the pair that drifted apart in the first place.
    #[test]
    fn the_nested_shape_carries_it_too() {
        let payload = json(Some("obscura 0.2.2"), true);
        assert!(payload.get("data").is_some(), "expected the nested shape");
        assert_eq!(payload["obscura_version"], "obscura 0.2.2");
    }

    /// Backward compatibility, and the reason the field is skipped rather
    /// than serialized as `null`: a bridge that reports no version must
    /// produce the payload clients already parse, key for key.
    #[test]
    fn an_absent_version_leaves_the_old_payload_untouched() {
        for igt in [false, true] {
            let payload = json(None, igt);
            assert!(
                payload.get("obscura_version").is_none(),
                "an absent version must not appear as a key (igt: {igt})"
            );
        }
    }

    /// The schema is what an MCP client generates its types from, so the
    /// field has to be registered there as well as serialized.
    #[test]
    fn the_output_schema_registers_the_field_as_optional() {
        let schema = serde_json::to_value(schemars::schema_for!(AnalyzeResponse)).expect("schema");
        let text = schema.to_string();
        assert!(
            text.contains("obscura_version"),
            "the analyze output schema must declare obscura_version: {text}"
        );
        // Optional: it must never be listed as required, or a client's
        // generated type turns an old payload into a parse error.
        assert!(
            !schema
                .pointer("/required")
                .and_then(|r| r.as_array())
                .is_some_and(|r| r.iter().any(|v| v == "obscura_version")),
            "obscura_version must stay optional"
        );
    }
}
