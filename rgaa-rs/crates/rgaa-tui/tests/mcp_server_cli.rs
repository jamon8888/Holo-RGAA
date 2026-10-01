//! End-to-end check of `rgaa mcp-server` (ticket #160): the real binary is
//! spawned, probed, driven through one HTTP tool call, and told to stop with
//! SIGTERM. Unix only — the test asserts on signal handling.
#![cfg(unix)]

use std::process::{Command, Stdio};
use std::time::Duration;

/// Picks a port by binding and releasing it. Racy in principle; in practice
/// the kernel does not immediately hand the same ephemeral port to another
/// process, and the alternative (parsing the child's log) couples the test
/// to the log format.
fn free_port() -> u16 {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind probe");
    listener.local_addr().expect("probe addr").port()
}

async fn wait_for_health(port: u16) -> serde_json::Value {
    let url = format!("http://127.0.0.1:{port}/health");
    let deadline = tokio::time::Instant::now() + Duration::from_secs(30);
    loop {
        if let Ok(resp) = reqwest::get(&url).await {
            if resp.status().is_success() {
                return resp.json().await.expect("health json");
            }
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "rgaa mcp-server never became healthy on port {port}"
        );
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

#[tokio::test]
async fn mcp_server_serves_health_and_tools_then_exits_zero_on_sigterm() {
    let port = free_port();
    let mut child = Command::new(env!("CARGO_BIN_EXE_rgaa"))
        .args(["mcp-server", "--port", &port.to_string()])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn rgaa mcp-server");

    let health = wait_for_health(port).await;
    assert_eq!(health["status"], serde_json::json!("ok"));
    assert!(
        health["version"].as_str().is_some_and(|v| !v.is_empty()),
        "health must report a version: {health}"
    );

    // `list_criteria` is the one tool that needs neither a browser substrate
    // nor the network, so this stays a transport test rather than an audit.
    let body: serde_json::Value = reqwest::Client::new()
        .post(format!("http://127.0.0.1:{port}/mcp"))
        .json(&serde_json::json!({
            "jsonrpc":"2.0","id":1,"method":"tools/call",
            "params":{"name":"list_criteria","arguments":{}}
        }))
        .send()
        .await
        .expect("tools/call")
        .json()
        .await
        .expect("tools/call json");
    assert_eq!(
        body["result"]["structuredContent"]["criteria"]
            .as_array()
            .map(Vec::len),
        Some(106),
        "unexpected tools/call response: {body}"
    );

    let killed = Command::new("kill")
        .args(["-TERM", &child.id().to_string()])
        .status()
        .expect("send SIGTERM");
    assert!(killed.success(), "could not signal the server");

    let status = tokio::task::spawn_blocking(move || child.wait())
        .await
        .expect("join wait")
        .expect("wait for exit");
    assert_eq!(
        status.code(),
        Some(0),
        "SIGTERM must be a clean shutdown, got {status:?}"
    );
}
