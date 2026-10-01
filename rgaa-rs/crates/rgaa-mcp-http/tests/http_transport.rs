//! HTTP transport seams for rgaa-mcp-http (ticket #159):
//! POST /mcp JSON-RPC, GET /mcp/events SSE, CORS.

use futures::StreamExt;
use rgaa_mcp::{
    AnalyzeService, GuidedService, McpFailure, NoOpStorageService, OrchestrationService,
    RemediationServiceImpl, ToolServer,
};
use rgaa_mcp_http::{app, app_with_cors, serve_until, AppState};
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

struct StubAnalyze {
    delay: Duration,
}

impl AnalyzeService for StubAnalyze {
    fn analyze(
        &self,
        request: rgaa_obscura::AnalyzeRequest,
    ) -> Pin<
        Box<
            dyn std::future::Future<Output = Result<rgaa_obscura::AnalyzePageResult, McpFailure>>
                + Send
                + '_,
        >,
    > {
        Box::pin(async move {
            tokio::time::sleep(self.delay).await;
            Ok(rgaa_obscura::AnalyzePageResult {
                url: request.url,
                findings: Vec::new(),
                evidence: Vec::new(),
                errors: Vec::new(),
                completed: true,
                duration_ms: 1,
                igt: None,
                obscura_version: None,
            })
        })
    }
}

struct StubGuided;

impl GuidedService for StubGuided {
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
        Box::pin(async { Err(McpFailure::unsupported("guided stub")) })
    }
}

fn test_server(delay: Duration) -> ToolServer {
    ToolServer::new(
        Arc::new(StubAnalyze { delay }),
        Arc::new(RemediationServiceImpl::default()),
        Arc::new(StubGuided),
        Arc::new(OrchestrationService::new()),
        Arc::new(NoOpStorageService),
    )
}

async fn spawn(delay: Duration) -> (SocketAddr, tokio::task::JoinHandle<()>) {
    let state = AppState::new(test_server(delay));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let addr = listener.local_addr().expect("addr");
    let handle = tokio::spawn(async move {
        let _ = axum::serve(listener, app(state)).await;
    });
    (addr, handle)
}

async fn rpc(addr: SocketAddr, body: serde_json::Value) -> serde_json::Value {
    let client = reqwest::Client::new();
    let resp = client
        .post(format!("http://{addr}/mcp"))
        .json(&body)
        .send()
        .await
        .expect("post")
        .error_for_status()
        .expect("status")
        .json::<serde_json::Value>()
        .await
        .expect("json");
    resp
}

#[tokio::test]
async fn tools_list_returns_every_registered_tool() {
    let (addr, _h) = spawn(Duration::ZERO).await;
    let resp = rpc(
        addr,
        serde_json::json!({"jsonrpc":"2.0","id":1,"method":"tools/list"}),
    )
    .await;
    let tools = resp["result"]["tools"].as_array().expect("tools array");
    let mut names: Vec<&str> = tools.iter().filter_map(|t| t["name"].as_str()).collect();
    names.sort_unstable();
    assert_eq!(
        names,
        vec![
            "analyze",
            "audit_url",
            "get_audit_result",
            "igt",
            "lint_static",
            "list_criteria",
            "remediate",
            "source_map",
            "verify_fix",
        ]
    );
    assert!(tools[0]["inputSchema"].is_object());
}

/// The HTTP transport dispatches tools through a hand-written match, so a tool
/// registered on `ToolServer` is still unreachable here until an arm is added.
/// `tools/list` advertising it is not evidence that calling it works — this test
/// is.
#[tokio::test]
async fn lint_static_is_callable_over_http_and_not_only_listed() {
    let (addr, _h) = spawn(Duration::ZERO).await;
    let resp = rpc(
        addr,
        serde_json::json!({
            "jsonrpc":"2.0","id":7,"method":"tools/call",
            "params":{"name":"lint_static","arguments":{
                "profile":"wcag-2.1-aa",
                "sources":[{"path":"widget.tsx","content":"<div><img src={u} /></div>"}]
            }}
        }),
    )
    .await;
    let structured = &resp["result"]["structuredContent"];
    assert_eq!(structured["error_count"], 1);
    let finding = &structured["findings"][0];
    assert_eq!(finding["rule"], "img-alt");
    assert_eq!(finding["file"], "widget.tsx");
    assert_eq!(finding["line"], 1);
    assert_eq!(finding["references"][0]["framework"], "WCAG 2.1 AA");
    assert!(finding["fix_hint"]["suggestion"].is_string());
}

/// An unusable request must come back as a JSON-RPC error, not as an empty
/// report that reads like a clean file.
#[tokio::test]
async fn lint_static_rejects_a_request_with_no_sources() {
    let (addr, _h) = spawn(Duration::ZERO).await;
    let resp = rpc(
        addr,
        serde_json::json!({
            "jsonrpc":"2.0","id":8,"method":"tools/call",
            "params":{"name":"lint_static","arguments":{}}
        }),
    )
    .await;
    assert!(resp["error"].is_object(), "{resp}");
}

#[tokio::test]
async fn call_list_criteria_returns_106_criteria() {
    let (addr, _h) = spawn(Duration::ZERO).await;
    let resp = rpc(
        addr,
        serde_json::json!({
            "jsonrpc":"2.0","id":2,"method":"tools/call",
            "params":{"name":"list_criteria","arguments":{}}
        }),
    )
    .await;
    let criteria = resp["result"]["structuredContent"]["criteria"]
        .as_array()
        .expect("criteria");
    assert_eq!(criteria.len(), 106);
    assert_eq!(resp["result"]["isError"], serde_json::json!(false));
}

#[tokio::test]
async fn invalid_json_body_returns_parse_error() {
    let (addr, _h) = spawn(Duration::ZERO).await;
    let client = reqwest::Client::new();
    let resp = client
        .post(format!("http://{addr}/mcp"))
        .header("content-type", "application/json")
        .body("{not json")
        .send()
        .await
        .expect("post");
    // HTTP 200 with JSON-RPC error envelope, or 4xx — both acceptable;
    // what matters is a parse-error code surfaces.
    let body: serde_json::Value = resp.json().await.expect("json");
    let code = body["error"]["code"].as_i64().unwrap_or(0);
    assert_eq!(code, -32700, "expected PARSE_ERROR, got {body}");
}

#[tokio::test]
async fn unknown_method_returns_method_not_found() {
    let (addr, _h) = spawn(Duration::ZERO).await;
    let resp = rpc(
        addr,
        serde_json::json!({"jsonrpc":"2.0","id":3,"method":"no/such"}),
    )
    .await;
    assert_eq!(resp["error"]["code"], serde_json::json!(-32601));
}

/// With no allowlist configured, an unknown origin gets no CORS grant.
///
/// This used to assert the opposite — that any origin was allowed. That
/// default is what let a page open in the user's browser POST to the MCP
/// endpoint and drive `tools/call`, so the expectation is inverted rather
/// than the behaviour restored.
#[tokio::test]
async fn cors_denies_an_unconfigured_origin() {
    let (addr, _h) = spawn(Duration::ZERO).await;
    let client = reqwest::Client::new();
    let resp = client
        .post(format!("http://{addr}/mcp"))
        .header("origin", "https://plugin.example")
        .json(&serde_json::json!({"jsonrpc":"2.0","id":1,"method":"tools/list"}))
        .send()
        .await
        .expect("post");
    let acao = resp
        .headers()
        .get("access-control-allow-origin")
        .map(|v| v.to_str().unwrap_or("").to_string());
    assert!(
        acao.is_none(),
        "an unconfigured origin was granted CORS access: {acao:?}"
    );
}

#[tokio::test]
async fn sse_emits_progress_during_analyze() {
    let (addr, _h) = spawn(Duration::from_millis(300)).await;
    let client = reqwest::Client::new();

    let sse = client
        .get(format!("http://{addr}/mcp/events"))
        .header("accept", "text/event-stream")
        .send()
        .await
        .expect("sse connect");
    assert_eq!(sse.status().as_u16(), 200);
    let mut stream = sse.bytes_stream();

    let call = tokio::spawn({
        let body = serde_json::json!({
            "jsonrpc":"2.0","id":9,"method":"tools/call",
            "params":{"name":"analyze","arguments":{"url":"https://example.test"}}
        });
        async move {
            let client = reqwest::Client::new();
            client
                .post(format!("http://{addr}/mcp"))
                .json(&body)
                .send()
                .await
                .expect("analyze post")
                .json::<serde_json::Value>()
                .await
                .expect("analyze json")
        }
    });

    let mut buf = String::new();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while !buf.contains("tool_completed") {
        let chunk = tokio::time::timeout_at(deadline, stream.next())
            .await
            .expect("sse timeout")
            .expect("sse stream ended early")
            .expect("sse chunk");
        buf.push_str(&String::from_utf8_lossy(&chunk));
        assert!(
            buf.len() < 64 * 1024,
            "SSE buffer runaway: {}",
            &buf[..buf.len().min(500)]
        );
    }
    assert!(buf.contains("tool_started"), "missing tool_started: {buf}");
    assert!(buf.contains("analyze"), "missing tool name: {buf}");

    let resp = call.await.expect("join");
    assert_eq!(
        resp["result"]["structuredContent"]["url"],
        serde_json::json!("https://example.test")
    );
}

/// `tools/list` is generated by the macro router but `tools/call` dispatches
/// through a hand-written match, so a tool can be advertised over HTTP and
/// still be unreachable. This exercises the second half for `source_map`.
#[tokio::test]
async fn call_source_map_maps_a_finding_over_http() {
    let (addr, _h) = spawn(Duration::ZERO).await;
    let root = format!(
        "{}/../rgaa-mcp/tests/fixtures/source_map/vanilla-site",
        env!("CARGO_MANIFEST_DIR")
    );
    // `source_root` is confined to RGAA_SOURCE_MAP_ROOTS, defaulting to the
    // working directory — which for this test binary is the mcp-http crate,
    // not the sibling crate holding the fixtures. An operator running the
    // server from somewhere other than the project they are auditing has to
    // set this too, so the test configures it the way a deployment would
    // rather than widening the default.
    std::env::set_var("RGAA_SOURCE_MAP_ROOTS", &root);
    let resp = rpc(
        addr,
        serde_json::json!({
            "jsonrpc":"2.0","id":11,"method":"tools/call",
            "params":{"name":"source_map","arguments":{
                "source_root": root,
                "findings":[
                    {"id":"a","html":"<img src=\"/img/logo-mairie.svg\" alt=\"Mairie de Villeneuve\">"},
                    {"id":"b","selector":"body > main > div:nth-child(2) > span"}
                ]
            }}
        }),
    )
    .await;
    let structured = &resp["result"]["structuredContent"];
    assert_eq!(resp["result"]["isError"], serde_json::json!(false));
    assert_eq!(
        structured["mapped"][0]["source_location"]["file"],
        serde_json::json!("index.html")
    );
    assert_eq!(
        structured["mapped"][0]["source_location"]["line"],
        serde_json::json!(9)
    );
    assert_eq!(
        structured["unmappable"][0]["reason"],
        serde_json::json!("no_distinguishing_literal")
    );
}

/// A tool that is only registered on the stdio router is invisible here: the
/// HTTP transport dispatches `tools/call` through its own hand-written
/// `match`, so a new arm is what actually makes it reachable.
#[tokio::test]
async fn verify_fix_is_reachable_over_the_http_transport() {
    let (addr, _h) = spawn(Duration::ZERO).await;
    let reference = serde_json::json!({
        "schema_version": "1.0",
        "audit_id": "audit-ref",
        "url": "https://example.test",
        "config": {"max_pages": 50, "max_depth": 5, "respect_robots": true, "sample_mode": false},
        "pages": [],
        "findings": [{
            "id": "f-alt",
            "rule": "image-alt",
            "criterion_id": "1.1",
            "url": "https://example.test",
            "target": "#hero img",
            "component_path": null,
            "evidence": [],
            "status": "fail",
            "severity": null,
            "description": null,
            "remediation": null
        }],
        "checkpoints": [],
        "summary": {"total_pages": 0, "completed_pages": 0, "total_findings": 1,
                    "passed": 0, "failed": 1, "needs_review": 0, "na": 0, "errors": 0}
    });
    let resp = rpc(
        addr,
        serde_json::json!({
            "jsonrpc":"2.0","id":9,"method":"tools/call",
            "params":{"name":"verify_fix","arguments":{
                "reference_audit": reference,
                "files": [{"path": "src/Hero.tsx", "url": "https://example.test"}]
            }}
        }),
    )
    .await;
    assert_eq!(
        resp["result"]["isError"],
        serde_json::json!(false),
        "{resp}"
    );
    let content = &resp["result"]["structuredContent"];
    // The stub analyzer reports nothing, so the reference finding is gone.
    assert_eq!(
        content["fixed"][0]["finding"]["id"],
        serde_json::json!("f-alt"),
        "{content}"
    );
    assert!(content["remaining"].as_array().unwrap().is_empty());
    assert!(content["new"].as_array().unwrap().is_empty());
}

/// Starts the server with a caller-triggered shutdown instead of a signal.
/// Returns the address, the trigger, and the join handle for the serve loop.
async fn spawn_with_shutdown(
    delay: Duration,
) -> (
    SocketAddr,
    tokio::sync::oneshot::Sender<()>,
    tokio::task::JoinHandle<std::io::Result<()>>,
) {
    let state = AppState::new(test_server(delay));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let addr = listener.local_addr().expect("addr");
    let (trigger, wait) = tokio::sync::oneshot::channel::<()>();
    let stopping = state.clone();
    let router = app_with_cors(state, None);
    let handle = tokio::spawn(async move {
        serve_until(listener, router, async move {
            let _ = wait.await;
            stopping.begin_shutdown();
        })
        .await
    });
    (addr, trigger, handle)
}

#[tokio::test]
async fn health_reports_a_status_and_the_server_version() {
    let (addr, _h) = spawn(Duration::ZERO).await;
    let resp = reqwest::get(format!("http://{addr}/health"))
        .await
        .expect("get /health");
    assert_eq!(resp.status().as_u16(), 200);
    let body: serde_json::Value = resp.json().await.expect("json");
    assert_eq!(body["status"], serde_json::json!("ok"));
    assert_eq!(
        body["version"],
        serde_json::json!(env!("CARGO_PKG_VERSION")),
        "version must be the built binary's, not a hardcoded string"
    );
}

/// An open SSE stream never completes on its own, so before the streams were
/// wired to the shutdown flag a single attached client held the drain — and
/// the process — open indefinitely.
#[tokio::test]
async fn an_attached_sse_client_does_not_block_the_drain() {
    let (addr, trigger, server) = spawn_with_shutdown(Duration::ZERO).await;

    let sse = reqwest::Client::new()
        .get(format!("http://{addr}/mcp/events"))
        .header("accept", "text/event-stream")
        .send()
        .await
        .expect("sse connect");
    assert_eq!(sse.status().as_u16(), 200);
    let _stream = sse.bytes_stream();

    trigger.send(()).expect("trigger shutdown");
    let served = tokio::time::timeout(Duration::from_secs(5), server)
        .await
        .expect("an open SSE stream kept the server from shutting down")
        .expect("join server");
    assert!(served.is_ok(), "serve loop errored: {served:?}");
}

/// The drain is the whole point of handling SIGTERM: a plain exit would cut
/// the connection of whoever is mid-audit and lose the result.
#[tokio::test]
async fn shutdown_waits_for_an_in_flight_tool_call_to_answer() {
    let (addr, trigger, server) = spawn_with_shutdown(Duration::from_millis(800)).await;

    let call = tokio::spawn(async move {
        rpc(
            addr,
            serde_json::json!({
                "jsonrpc":"2.0","id":11,"method":"tools/call",
                "params":{"name":"analyze","arguments":{"url":"https://inflight.test"}}
            }),
        )
        .await
    });

    // Long enough for the request to have reached the handler and be sitting
    // in the stubbed 800 ms analyze.
    tokio::time::sleep(Duration::from_millis(200)).await;
    trigger.send(()).expect("trigger shutdown");
    assert!(
        !server.is_finished(),
        "server returned before the in-flight call could answer"
    );

    let resp = call.await.expect("join call");
    assert_eq!(
        resp["result"]["structuredContent"]["url"],
        serde_json::json!("https://inflight.test"),
        "the in-flight call was cut off by shutdown"
    );

    let served = tokio::time::timeout(Duration::from_secs(5), server)
        .await
        .expect("server did not return after the drain")
        .expect("join server");
    assert!(served.is_ok(), "serve loop errored: {served:?}");
}

/// Issue #198: request-side authorization.
///
/// CORS decides what a browser may *read*; it does not decide what the
/// server *runs*. A simple cross-origin `text/plain` POST skips preflight
/// entirely, so before this guard an attacking page could make the server
/// run `analyze` against a URL of its choosing and only lose the response.
mod request_side_authorization {
    use super::{test_server, Duration};
    use rgaa_mcp_http::{app_with_auth, AppState};
    use std::net::SocketAddr;

    async fn spawn_guarded(
        origins: Option<&str>,
        token: Option<&str>,
    ) -> (SocketAddr, tokio::task::JoinHandle<()>) {
        let state = AppState::new(test_server(Duration::ZERO));
        let app = app_with_auth(state, origins, token);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind");
        let addr = listener.local_addr().expect("addr");
        let handle = tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });
        (addr, handle)
    }

    fn tools_call() -> serde_json::Value {
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {"name": "list_criteria", "arguments": {}}
        })
    }

    /// The attack from the issue, verbatim: a browser-simple POST that needs
    /// no preflight. It must be refused *before* the tool runs.
    #[tokio::test]
    async fn a_simple_text_plain_post_from_an_unknown_origin_does_not_run_the_tool() {
        let (addr, _h) = spawn_guarded(Some("https://plugin.example"), None).await;
        let resp = reqwest::Client::new()
            .post(format!("http://{addr}/mcp"))
            .header("origin", "https://evil.example")
            .header("content-type", "text/plain;charset=UTF-8")
            .body(tools_call().to_string())
            .send()
            .await
            .expect("post");
        assert_eq!(resp.status(), reqwest::StatusCode::FORBIDDEN);
        let body: serde_json::Value = resp.json().await.expect("json body");
        assert!(
            body.get("error").is_some(),
            "a refusal must still be readable as JSON-RPC: {body}"
        );
        assert!(
            body.get("result").is_none(),
            "the tool must not have run: {body}"
        );
    }

    #[tokio::test]
    async fn an_allowlisted_origin_still_reaches_its_tool() {
        let (addr, _h) = spawn_guarded(Some("https://plugin.example"), None).await;
        let resp = reqwest::Client::new()
            .post(format!("http://{addr}/mcp"))
            .header("origin", "https://plugin.example")
            .json(&tools_call())
            .send()
            .await
            .expect("post");
        assert_eq!(resp.status(), reqwest::StatusCode::OK);
        let body: serde_json::Value = resp.json().await.expect("json body");
        assert!(body["result"]["structuredContent"]["criteria"].is_array());
    }

    /// A direct client (CLI, curl, another service) sends no `Origin`. It
    /// was never constrained by CORS and must keep working when no token is
    /// configured, or this change breaks every existing deployment.
    #[tokio::test]
    async fn a_direct_client_with_no_origin_keeps_working() {
        let (addr, _h) = spawn_guarded(None, None).await;
        let resp = reqwest::Client::new()
            .post(format!("http://{addr}/mcp"))
            .json(&tools_call())
            .send()
            .await
            .expect("post");
        assert_eq!(resp.status(), reqwest::StatusCode::OK);
    }

    #[tokio::test]
    async fn a_configured_token_is_required_and_sufficient() {
        let (addr, _h) = spawn_guarded(None, Some("s3cret")).await;
        let client = reqwest::Client::new();

        let refused = client
            .post(format!("http://{addr}/mcp"))
            .json(&tools_call())
            .send()
            .await
            .expect("post");
        assert_eq!(refused.status(), reqwest::StatusCode::UNAUTHORIZED);

        let wrong = client
            .post(format!("http://{addr}/mcp"))
            .header("authorization", "Bearer nope")
            .json(&tools_call())
            .send()
            .await
            .expect("post");
        assert_eq!(wrong.status(), reqwest::StatusCode::UNAUTHORIZED);

        let accepted = client
            .post(format!("http://{addr}/mcp"))
            .header("authorization", "Bearer s3cret")
            .json(&tools_call())
            .send()
            .await
            .expect("post");
        assert_eq!(accepted.status(), reqwest::StatusCode::OK);
    }

    /// The SSE stream leaks audit progress, so it is behind the same guard
    /// as the JSON-RPC endpoint rather than open beside it.
    #[tokio::test]
    async fn the_event_stream_is_guarded_too() {
        let (addr, _h) = spawn_guarded(None, Some("s3cret")).await;
        let resp = reqwest::Client::new()
            .get(format!("http://{addr}/mcp/events"))
            .send()
            .await
            .expect("get");
        assert_eq!(resp.status(), reqwest::StatusCode::UNAUTHORIZED);
    }

    /// `/health` stays open: a supervisor must be able to tell the process
    /// is alive without being handed a credential.
    #[tokio::test]
    async fn health_stays_reachable_without_a_credential() {
        let (addr, _h) = spawn_guarded(Some("https://plugin.example"), Some("s3cret")).await;
        let resp = reqwest::Client::new()
            .get(format!("http://{addr}/health"))
            .send()
            .await
            .expect("get");
        assert_eq!(resp.status(), reqwest::StatusCode::OK);
    }
}
