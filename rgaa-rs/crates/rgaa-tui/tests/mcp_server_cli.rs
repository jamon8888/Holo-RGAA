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

    // Bounded poll, not `child.wait()`.
    //
    // The regression this test guards against is a shutdown that never
    // completes. An unbounded wait turns that regression into a job that
    // hangs until the CI limit — hours of a runner, and a timeout message
    // that says nothing about the cause — instead of a test failure that
    // names it. A guard against a hang must not hang.
    //
    // Ownership of `child` stays here so the process can be killed and
    // reaped on timeout; `spawn_blocking` could not be cancelled once
    // started, and would leak the child into the rest of the run.
    let deadline = std::time::Instant::now() + Duration::from_secs(20);
    let status = loop {
        match child.try_wait().expect("poll the server process") {
            Some(status) => break status,
            None if std::time::Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                panic!("the server did not exit within 20s of SIGTERM; the drain hung");
            }
            None => tokio::time::sleep(Duration::from_millis(50)).await,
        }
    };
    assert_eq!(
        status.code(),
        Some(0),
        "SIGTERM must be a clean shutdown, got {status:?}"
    );
}
