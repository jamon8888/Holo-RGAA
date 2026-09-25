//! HTTP transport seams for rgaa-mcp-http (ticket #159):
//! POST /mcp JSON-RPC, GET /mcp/events SSE, CORS.

use futures::StreamExt;
use rgaa_mcp::{
    AnalyzeService, GuidedService, McpFailure, NoOpStorageService, OrchestrationService,
    RemediationServiceImpl, ToolServer,
};
use rgaa_mcp_http::{app, AppState};
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
async fn tools_list_returns_six_tools() {
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
            "list_criteria",
            "remediate"
        ]
    );
    assert!(tools[0]["inputSchema"].is_object());
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

#[tokio::test]
async fn cors_reflects_plugin_origin() {
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
        acao.as_deref() == Some("*") || acao.as_deref() == Some("https://plugin.example"),
        "missing/incorrect CORS header, got {acao:?}"
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
