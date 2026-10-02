//! Integration tests for the HTTP/JSON-RPC transport.
//!
//! Each test spawns a minimal HTTP server and validates JSON-RPC dispatch
//! of the MCP tools over HTTP POST /mcp and SSE streaming.

use rgaa_mcp::ToolServer;
use serde_json::{json, Value};
use std::sync::Arc;

/// A minimal tool server for testing, reusing the real tools.
fn test_tool_server() -> ToolServer {
    ToolServer::new(
        // Use stub services for testing; the real ones need browser substrate
        Arc::new(rgaa_mcp::NoOpAnalyzeService),
        Arc::new(rgaa_mcp::RemediationServiceImpl::default()),
        Arc::new(rgaa_mcp::NoOpGuidedService),
        Arc::new(rgaa_mcp::OrchestrationService::new()),
        Arc::new(rgaa_mcp::NoOpStorageService),
    )
}

#[tokio::test]
async fn http_server_starts_and_health_check_works() {
    let state = rgaa_mcp_http::AppState::new(test_tool_server());
    let app = rgaa_mcp_http::app(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("failed to bind to loopback");
    let addr = listener.local_addr().expect("no local addr");

    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("server failed");
    });

    // Give the server a moment to start
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // Health check should return immediately without touching services
    let client = reqwest::Client::new();
    let resp = client
        .get(format!("http://{addr}/health"))
        .send()
        .await
        .expect("health request failed");
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.expect("invalid health JSON");
    assert_eq!(body.get("status").and_then(Value::as_str), Some("ok"));
}

#[tokio::test]
async fn jsonrpc_initialize_handshake_works() {
    let state = rgaa_mcp_http::AppState::new(test_tool_server());
    let app = rgaa_mcp_http::app(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("failed to bind");
    let addr = listener.local_addr().expect("no addr");

    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("server failed");
    });

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("http://{addr}/mcp"))
        .header("Content-Type", "application/json")
        .body(r#"{"jsonrpc":"2.0","id":1,"method":"initialize"}"#)
        .send()
        .await
        .expect("request failed");

    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.expect("invalid JSON-RPC response");

    // MCP 2.0 initialize response
    assert_eq!(body.get("jsonrpc").and_then(Value::as_str), Some("2.0"));
    assert!(
        body.get("result").is_some(),
        "initialize should return result"
    );
    assert_eq!(body.get("id").and_then(Value::as_u64), Some(1));
}

#[tokio::test]
async fn tools_list_announces_all_tools() {
    let state = rgaa_mcp_http::AppState::new(test_tool_server());
    let app = rgaa_mcp_http::app(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("failed to bind");
    let addr = listener.local_addr().expect("no addr");

    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("server failed");
    });

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("http://{addr}/mcp"))
        .header("Content-Type", "application/json")
        .body(r#"{"jsonrpc":"2.0","id":1,"method":"tools/list"}"#)
        .send()
        .await
        .expect("request failed");

    let body: Value = resp.json().await.expect("invalid JSON");
    let tools: Vec<Value> = body
        .get("result")
        .and_then(Value::as_object)
        .and_then(|obj| obj.get("tools").and_then(Value::as_array))
        .cloned()
        .unwrap_or_default();

    let tool_names: Vec<&str> = tools
        .iter()
        .filter_map(|t| t.get("name").and_then(Value::as_str))
        .collect();

    // Phase 1 tools must be in the list
    assert!(
        tool_names.contains(&"lint_static"),
        "lint_static not in tools/list"
    );
    assert!(
        tool_names.contains(&"verify_fix"),
        "verify_fix not in tools/list"
    );
    assert!(
        tool_names.contains(&"source_map"),
        "source_map not in tools/list"
    );

    // Existing tools must still be there
    assert!(tool_names.contains(&"analyze"), "analyze not in tools/list");
    assert!(
        tool_names.contains(&"remediate"),
        "remediate not in tools/list"
    );
}

#[tokio::test]
async fn tools_call_lint_static_with_no_args_returns_error() {
    let state = rgaa_mcp_http::AppState::new(test_tool_server());
    let app = rgaa_mcp_http::app(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("failed to bind");
    let addr = listener.local_addr().expect("no addr");

    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("server failed");
    });

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("http://{addr}/mcp"))
        .header("Content-Type", "application/json")
        .body(r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"lint_static","arguments":{}}}"#)
        .send()
        .await
        .expect("request failed");

    let body: Value = resp.json().await.expect("invalid JSON");

    // lint_static requires sources or source_files, so it should error
    assert!(
        body.get("error").is_some(),
        "lint_static should error without sources"
    );
    assert_eq!(body.get("id").and_then(Value::as_u64), Some(2));
}

#[tokio::test]
async fn tools_call_returns_json_rpc_error_for_unknown_tool() {
    let state = rgaa_mcp_http::AppState::new(test_tool_server());
    let app = rgaa_mcp_http::app(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("failed to bind");
    let addr = listener.local_addr().expect("no addr");

    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("server failed");
    });

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("http://{addr}/mcp"))
        .header("Content-Type", "application/json")
        .body(r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"nonexistent_tool","arguments":{}}}"#)
        .send()
        .await
        .expect("request failed");

    let body: Value = resp.json().await.expect("invalid JSON");

    // Should be a JSON-RPC error
    assert!(body.get("error").is_some());
    assert_eq!(body.get("id").and_then(Value::as_u64), Some(3));
}

#[tokio::test]
async fn parse_error_returns_json_rpc_parse_error() {
    let state = rgaa_mcp_http::AppState::new(test_tool_server());
    let app = rgaa_mcp_http::app(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("failed to bind");
    let addr = listener.local_addr().expect("no addr");

    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("server failed");
    });

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("http://{addr}/mcp"))
        .header("Content-Type", "application/json")
        .body("not valid json")
        .send()
        .await
        .expect("request failed");

    let body: Value = resp.json().await.expect("invalid JSON");

    // Should be a parse error (no id, since we couldn't parse the id)
    let error_code = body
        .get("error")
        .and_then(|e| e.get("code").and_then(Value::as_i64));
    assert_eq!(
        error_code,
        Some(-32700),
        "expected JSON-RPC parse error code"
    );
}
