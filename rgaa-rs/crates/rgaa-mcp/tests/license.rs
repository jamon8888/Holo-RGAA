//! Per-tool licence check on the stdio path (#87): the tool server itself
//! refuses, with no HTTP layer involved. Every gate is built here with an
//! injected clock and in-memory counter: no env, no files, no network.

use std::sync::Arc;

use chrono::{TimeZone, Utc};
use rgaa_license::{
    Denial, Feature, GateConfig, LicenseGate, LicenseVerifier, ManualClock, UsageCounter,
};
use rgaa_mcp::{
    tool_license, AnalyzeRequest, AnalyzeService, GuidedService, McpFailure, NoOpStorageService,
    OrchestrationService, RemediationServiceImpl, ToolLicense, ToolServer,
};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::ServiceExt;
use serde_json::{json, Value};
use std::pin::Pin;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

struct PanickingAnalyze;

impl AnalyzeService for PanickingAnalyze {
    fn analyze(
        &self,
        _request: rgaa_obscura::AnalyzeRequest,
    ) -> Pin<
        Box<
            dyn std::future::Future<Output = Result<rgaa_obscura::AnalyzePageResult, McpFailure>>
                + Send
                + '_,
        >,
    > {
        panic!("a refused call must not reach the analyzer")
    }
}

struct PanickingGuided;

impl GuidedService for PanickingGuided {
    fn run(
        &self,
        _test: rgaa_obscura::GuidedTest,
    ) -> Pin<
        Box<
            dyn std::future::Future<Output = Result<rgaa_obscura::GuidedRunResult, McpFailure>>
                + Send
                + '_,
        >,
    > {
        panic!("a refused call must not reach the guided runner")
    }
}

/// An unlicensed (Free) gate. `enforce_quota` as given.
fn free_gate(enforce_quota: bool) -> Arc<LicenseGate> {
    let clock = Arc::new(ManualClock::new(
        Utc.with_ymd_and_hms(2026, 11, 19, 12, 0, 0).unwrap(),
    ));
    Arc::new(LicenseGate::new(
        LicenseVerifier::new(vec![]),
        Arc::new(UsageCounter::in_memory()),
        clock,
        GateConfig {
            enforce_quota,
            upgrade_url: Some("https://example.test/upgrade".into()),
            ..GateConfig::default()
        },
    ))
}

fn server(gate: Arc<LicenseGate>) -> ToolServer {
    ToolServer::new(
        Arc::new(PanickingAnalyze),
        Arc::new(RemediationServiceImpl::default()),
        Arc::new(PanickingGuided),
        Arc::new(OrchestrationService::new()),
        Arc::new(NoOpStorageService),
    )
    .with_license(gate)
}

fn lint_request() -> rgaa_mcp::LintStaticRequest {
    serde_json::from_value(json!({
        "sources": [{"path": "a.html", "content": "<img src=x>"}]
    }))
    .unwrap()
}

/// The denial the gate would give for `feature`, for comparing against.
fn expected_denial(feature: Feature) -> Box<Denial> {
    free_gate(false).check(feature, 1).unwrap_err()
}

#[test]
fn every_registered_tool_has_a_licence_class() {
    for name in ToolServer::tool_names() {
        assert!(
            tool_license(name).is_some(),
            "`{name}` has no licence class"
        );
    }
    assert_eq!(tool_license("list_criteria"), Some(ToolLicense::Exempt));
    assert_eq!(tool_license("initialize"), Some(ToolLicense::Exempt));
    assert_eq!(tool_license("tools/list"), Some(ToolLicense::Exempt));
    assert_eq!(
        tool_license("get_audit_result"),
        Some(ToolLicense::Access(Feature::FullAudit))
    );
    assert_eq!(
        tool_license("lint_static"),
        Some(ToolLicense::Metered(Feature::StaticLint, 1))
    );
    for name in ["analyze", "audit_url"] {
        assert_eq!(
            tool_license(name),
            Some(ToolLicense::Metered(Feature::FullAudit, 1))
        );
    }
    assert_eq!(
        tool_license("igt"),
        Some(ToolLicense::Metered(Feature::Igt, 1))
    );
    for name in ["remediate", "source_map", "verify_fix"] {
        assert_eq!(
            tool_license(name),
            Some(ToolLicense::Metered(Feature::Remediation, 1))
        );
    }
    assert_eq!(tool_license("no_such_tool"), None);
}

#[tokio::test]
async fn free_tier_refusal_is_the_denials_jsonrpc_error() {
    let server = server(free_gate(false));
    let error = server
        .analyze(Parameters(
            AnalyzeRequest::malformed("https://example.test").unwrap(),
        ))
        .await
        .err()
        .expect("analyze must be refused on Free");
    let expected = expected_denial(Feature::FullAudit).jsonrpc_error();
    assert_eq!(error.code.0, -32002);
    assert_eq!(json!(error.code.0), expected["code"]);
    assert_eq!(error.message, expected["message"].as_str().unwrap());
    assert_eq!(error.data.as_ref(), Some(&expected["data"]));
    let data = error.data.unwrap();
    assert_eq!(data["code"], "feature_not_in_tier");
    assert_eq!(data["required_tier"], "professional");
    assert_eq!(data["upgrade_url"], "https://example.test/upgrade");

    // Free still lints, and the catalogue is never gated.
    assert!(server.lint_static(Parameters(lint_request())).is_ok());
    assert!(server.list_criteria().is_ok());
}

#[tokio::test]
async fn quota_refusal_is_minus_32003_and_refusals_consume_nothing() {
    let gate = free_gate(true);
    let server = server(Arc::clone(&gate));
    // Refused tier calls first: they must not eat into the 10 free units.
    for _ in 0..5 {
        let e = server
            .remediate(Parameters(
                serde_json::from_value(json!({"issues": []})).unwrap(),
            ))
            .err()
            .expect("remediation is not in Free");
        assert_eq!(e.code.0, -32002);
    }
    for i in 0..10 {
        assert!(
            server.lint_static(Parameters(lint_request())).is_ok(),
            "call {i} of the 10 free units was refused"
        );
    }
    let e = server
        .lint_static(Parameters(lint_request()))
        .err()
        .expect("the 11th call is over quota");
    assert_eq!(e.code.0, -32003);
    let data = e.data.expect("quota data");
    assert_eq!(data["code"], "quota_exceeded");
    assert_eq!(data["remaining"], 0);
    assert!(data["retry_after"].as_u64().is_some_and(|s| s > 0));
}

/// Through rmcp's own stdio dispatch (`#[tool_router]`), over an in-memory
/// pipe: the refusal must reach the client as the licence error, not as a
/// generic execution failure.
#[tokio::test]
async fn stdio_dispatch_carries_the_licence_error() {
    let (client, server_io) = tokio::io::duplex(64 * 1024);
    let (server_read, server_write) = tokio::io::split(server_io);
    let running = tokio::spawn(async move {
        let service = server(free_gate(false))
            .serve((server_read, server_write))
            .await
            .expect("serve");
        let _ = service.waiting().await;
    });

    let (client_read, mut client_write) = tokio::io::split(client);
    let mut lines = BufReader::new(client_read).lines();
    let send = |msg: Value| {
        let mut line = msg.to_string();
        line.push('\n');
        line
    };
    for msg in [
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{
            "protocolVersion":"2025-03-26","capabilities":{},
            "clientInfo":{"name":"test","version":"0"}}}),
        json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
        json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{
            "name":"analyze","arguments":{"url":"https://example.test"}}}),
        json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{
            "name":"lint_static","arguments":{"sources":[{"path":"a.html","content":"<img src=x>"}]}}}),
    ] {
        client_write
            .write_all(send(msg).as_bytes())
            .await
            .expect("write");
    }

    let mut analyze = None;
    let mut lint = None;
    while analyze.is_none() || lint.is_none() {
        let line = tokio::time::timeout(std::time::Duration::from_secs(10), lines.next_line())
            .await
            .expect("stdio server answered in time")
            .expect("read")
            .expect("stream open");
        let msg: Value = serde_json::from_str(&line).expect("json line");
        match msg["id"].as_u64() {
            Some(2) => analyze = Some(msg),
            Some(3) => lint = Some(msg),
            _ => {}
        }
    }
    let analyze = analyze.unwrap();
    let expected = expected_denial(Feature::FullAudit).jsonrpc_error();
    assert_eq!(analyze["error"], expected, "{analyze}");
    let lint = lint.unwrap();
    assert!(lint["error"].is_null(), "{lint}");
    assert!(lint["result"].is_object(), "{lint}");

    drop(client_write);
    running.abort();
}
