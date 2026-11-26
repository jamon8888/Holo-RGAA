//! Licence enforcement over the HTTP transport (#87). Gates are injected
//! (in-memory counter, manual clock, no env, no files); the only network is a
//! listener on 127.0.0.1:0, so nothing here can skip.

use std::net::SocketAddr;
use std::sync::Arc;

use chrono::{TimeZone, Utc};
use rgaa_license::{
    GateConfig, LicenseError, LicenseGate, LicenseVerifier, ManualClock, UsageCounter,
};
use rgaa_mcp::{NoOpStorageService, OrchestrationService, RemediationServiceImpl, ToolServer};
use rgaa_mcp_http::app_with_license;
use serde_json::{json, Value};

fn free_gate() -> Arc<LicenseGate> {
    let clock = Arc::new(ManualClock::new(
        Utc.with_ymd_and_hms(2026, 11, 19, 12, 0, 0).unwrap(),
    ));
    Arc::new(LicenseGate::new(
        LicenseVerifier::new(vec![]),
        Arc::new(UsageCounter::in_memory()),
        clock,
        GateConfig {
            enforce_quota: true,
            upgrade_url: Some("https://example.test/upgrade".into()),
            ..GateConfig::default()
        },
    ))
}

fn tool_server() -> ToolServer {
    // The lazy bridge never starts a browser: every analyze call below is
    // refused before reaching it.
    let bridge = Arc::new(rgaa_mcp::LazyObscuraBridge::new(
        rgaa_obscura::ObscuraBridge::default(),
    ));
    ToolServer::new(
        Arc::new(rgaa_mcp::ObscuraAnalyzeService::new(bridge.clone())),
        Arc::new(RemediationServiceImpl::default()),
        Arc::new(rgaa_mcp::ObscuraGuidedService::new(bridge)),
        Arc::new(OrchestrationService::new()),
        Arc::new(NoOpStorageService),
    )
}

async fn spawn(gate: Arc<LicenseGate>, origins: Option<&str>) -> SocketAddr {
    let app = app_with_license(tool_server(), gate, origins, None);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let addr = listener.local_addr().expect("addr");
    tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });
    addr
}

fn call(id: u64, name: &str, arguments: Value) -> Value {
    json!({"jsonrpc":"2.0","id":id,"method":"tools/call",
           "params":{"name":name,"arguments":arguments}})
}

fn lint(id: u64) -> Value {
    call(
        id,
        "lint_static",
        json!({"sources":[{"path":"a.html","content":"<img src=x>"}]}),
    )
}

fn analyze(id: u64) -> Value {
    call(id, "analyze", json!({"url":"https://example.test"}))
}

async fn post(addr: SocketAddr, body: &Value) -> reqwest::Response {
    reqwest::Client::new()
        .post(format!("http://{addr}/mcp"))
        .json(body)
        .send()
        .await
        .expect("post")
}

fn header<'a>(resp: &'a reqwest::Response, name: &str) -> Option<&'a str> {
    resp.headers().get(name).and_then(|v| v.to_str().ok())
}

#[tokio::test]
async fn free_tier_lints_but_analyze_is_402_with_the_licence_error() {
    let addr = spawn(free_gate(), None).await;

    let ok = post(addr, &lint(1)).await;
    assert_eq!(ok.status().as_u16(), 200);
    assert_eq!(header(&ok, "x-license-tier"), Some("free"));
    assert_eq!(header(&ok, "x-license-status"), Some("unlicensed"));
    assert_eq!(header(&ok, "x-ratelimit-limit"), Some("10"));
    assert_eq!(header(&ok, "x-ratelimit-remaining"), Some("9"));
    let body: Value = ok.json().await.expect("json");
    assert_eq!(body["result"]["structuredContent"]["error_count"], 1);

    let refused = post(addr, &analyze(2)).await;
    assert_eq!(refused.status().as_u16(), 402);
    assert_eq!(header(&refused, "retry-after"), None);
    let body: Value = refused.json().await.expect("json");
    assert_eq!(body["jsonrpc"], "2.0");
    assert_eq!(body["id"], 2);
    assert!(body.get("result").is_none(), "{body}");
    assert_eq!(body["error"]["code"], -32002);
    assert_eq!(body["error"]["data"]["code"], "feature_not_in_tier");
    assert_eq!(body["error"]["data"]["required_tier"], "professional");
    assert_eq!(
        body["error"]["data"]["upgrade_url"],
        "https://example.test/upgrade"
    );
}

#[tokio::test]
async fn exhausted_quota_is_429_with_retry_after_and_refusals_consume_nothing() {
    let addr = spawn(free_gate(), None).await;

    // Tier refusals first: none of them may eat into the 10 free units.
    for id in 0..5 {
        assert_eq!(post(addr, &analyze(id)).await.status().as_u16(), 402);
    }
    for id in 0..10 {
        let resp = post(addr, &lint(id)).await;
        assert_eq!(resp.status().as_u16(), 200, "free unit {id} refused");
        let left = (9 - id).to_string();
        assert_eq!(header(&resp, "x-ratelimit-remaining"), Some(left.as_str()));
    }

    let over = post(addr, &lint(99)).await;
    assert_eq!(over.status().as_u16(), 429);
    let retry: u64 = header(&over, "retry-after")
        .expect("Retry-After on a quota refusal")
        .parse()
        .expect("seconds");
    assert!(retry > 0);
    assert_eq!(header(&over, "x-ratelimit-remaining"), Some("0"));
    let body: Value = over.json().await.expect("json");
    assert_eq!(body["id"], 99);
    assert_eq!(body["error"]["code"], -32003);
    assert_eq!(body["error"]["data"]["code"], "quota_exceeded");
    assert_eq!(body["error"]["data"]["retry_after"], retry);

    // A refused quota call does not consume either: still exactly 429.
    assert_eq!(post(addr, &lint(100)).await.status().as_u16(), 429);

    // The catalogue stays exempt, quota or not.
    let list = post(addr, &call(101, "list_criteria", json!({}))).await;
    assert_eq!(list.status().as_u16(), 200);
    assert_eq!(header(&list, "x-license-tier"), None);
}

#[tokio::test]
async fn health_answers_with_a_broken_licence_while_tools_get_401() {
    let gate = free_gate();
    gate.install_error(LicenseError::InvalidSignature);
    let addr = spawn(gate, None).await;

    let health = reqwest::get(format!("http://{addr}/health"))
        .await
        .expect("get /health");
    assert_eq!(health.status().as_u16(), 200);

    let refused = post(addr, &lint(1)).await;
    assert_eq!(refused.status().as_u16(), 401);
    let body: Value = refused.json().await.expect("json");
    assert_eq!(body["error"]["code"], -32002);
    assert_eq!(body["error"]["data"]["code"], "license_invalid");

    // The event stream is never metered.
    let sse = reqwest::Client::new()
        .get(format!("http://{addr}/mcp/events"))
        .send()
        .await
        .expect("sse");
    assert_eq!(sse.status().as_u16(), 200);
}

#[tokio::test]
async fn an_unknown_origin_is_403_before_the_licence_is_looked_at() {
    let gate = free_gate();
    gate.install_error(LicenseError::InvalidSignature);
    let addr = spawn(gate, Some("https://plugin.example")).await;
    let resp = reqwest::Client::new()
        .post(format!("http://{addr}/mcp"))
        .header("origin", "https://evil.example")
        .json(&analyze(1))
        .send()
        .await
        .expect("post");
    assert_eq!(resp.status().as_u16(), 403);
    assert_eq!(header(&resp, "x-license-tier"), None);
}

#[tokio::test]
async fn cors_exposes_the_licence_and_rate_limit_headers_by_name() {
    let addr = spawn(free_gate(), Some("https://plugin.example")).await;
    let resp = reqwest::Client::new()
        .post(format!("http://{addr}/mcp"))
        .header("origin", "https://plugin.example")
        .json(&lint(1))
        .send()
        .await
        .expect("post");
    assert_eq!(resp.status().as_u16(), 200);
    let exposed = header(&resp, "access-control-expose-headers")
        .expect("expose-headers on an allowed cross-origin response")
        .to_ascii_lowercase();
    assert!(!exposed.contains('*'), "{exposed}");
    for name in rgaa_license::gate::headers::EXPOSED {
        assert!(exposed.contains(name), "{name} not exposed: {exposed}");
    }
}
