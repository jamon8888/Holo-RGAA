//! Batch REST endpoints (#167): create, poll per-URL status, read the
//! aggregated result, and stop serving a batch once it has expired.
//!
//! Real HTTP against a real `axum::serve` router on an ephemeral port — the
//! routes go through the same extractors, status codes and JSON bodies a
//! client sees. The audits themselves come from a scripted runner: what is
//! under test is the batch state machine and its HTTP surface, not the
//! browser pipeline.

use chrono::{DateTime, Duration as ChronoDuration, Utc};
use rgaa_api::batch::{
    batch_router, BatchRunner, BatchService, Clock, InMemoryBatchStore, MAX_BATCH_URLS,
};
use rgaa_core::{AuditResult, CrawlConfig};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

fn audit(url: &str, taux: f64) -> AuditResult {
    AuditResult {
        audit_id: format!("audit-for-{url}"),
        url: url.to_string(),
        pages: vec![],
        total_criteria: 106,
        passed: 50,
        failed: 10,
        na: 46,
        overall_compliance: taux,
        taux_global: taux,
        coverage_percent: 56.6,
        etat_conformite: "partielle".to_string(),
        duration_ms: 1,
        audit_complete: false,
    }
}

/// Replays a fixed outcome per URL. When `gate` is present, one outcome is
/// released per message the test sends — which is how a half-finished batch
/// is observed deterministically, without sleeping and hoping.
struct ScriptedRunner {
    outcomes: HashMap<String, Result<AuditResult, String>>,
    gate: Option<tokio::sync::Mutex<tokio::sync::mpsc::Receiver<()>>>,
    run_error: Option<String>,
}

#[async_trait::async_trait]
impl BatchRunner for ScriptedRunner {
    async fn run(
        &self,
        urls: Vec<String>,
        _config: CrawlConfig,
        on_url: rgaa_orchestrator::BatchObserver,
    ) -> Result<(), String> {
        if let Some(error) = &self.run_error {
            return Err(error.clone());
        }
        for url in urls {
            if let Some(gate) = &self.gate {
                gate.lock().await.recv().await;
            }
            match self.outcomes.get(&url) {
                Some(Ok(result)) => on_url(&url, Ok(result)),
                Some(Err(e)) => on_url(&url, Err(e.as_str())),
                None => on_url(&url, Err("no scripted outcome")),
            }
        }
        Ok(())
    }
}

/// Test clock: readable by the service, movable by the test.
#[derive(Clone)]
struct TestClock(Arc<std::sync::Mutex<DateTime<Utc>>>);

impl TestClock {
    fn new() -> Self {
        Self(Arc::new(std::sync::Mutex::new(Utc::now())))
    }

    fn advance(&self, by: ChronoDuration) {
        let mut now = self.0.lock().unwrap();
        *now += by;
    }

    fn as_clock(&self) -> Clock {
        let inner = Arc::clone(&self.0);
        Arc::new(move || *inner.lock().unwrap())
    }
}

async fn spawn(runner: ScriptedRunner, clock: Clock) -> SocketAddr {
    let service = BatchService::new(Arc::new(InMemoryBatchStore::new()), Arc::new(runner), clock);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let addr = listener.local_addr().expect("addr");
    tokio::spawn(async move {
        let _ = axum::serve(listener, batch_router(service)).await;
    });
    addr
}

async fn create(addr: SocketAddr, urls: &[&str]) -> reqwest::Response {
    reqwest::Client::new()
        .post(format!("http://{addr}/v1/batches"))
        .json(&json!({ "urls": urls }))
        .send()
        .await
        .expect("create batch")
}

async fn status(addr: SocketAddr, id: &str) -> reqwest::Response {
    reqwest::get(format!("http://{addr}/v1/batches/{id}"))
        .await
        .expect("get status")
}

async fn results(addr: SocketAddr, id: &str) -> reqwest::Response {
    reqwest::get(format!("http://{addr}/v1/batches/{id}/results"))
        .await
        .expect("get results")
}

/// Polls status until `predicate` holds. The batch runs on a detached task,
/// so a bare read right after a tick is inherently racy; a bounded poll
/// keeps the assertion about the endpoint's eventual answer, and still
/// fails loudly if the state never arrives.
async fn poll_until(addr: SocketAddr, id: &str, predicate: impl Fn(&Value) -> bool) -> Value {
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    loop {
        let body: Value = status(addr, id).await.json().await.expect("status json");
        if predicate(&body) {
            return body;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "batch never reached the expected state; last status: {body}"
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

fn scripted(outcomes: Vec<(&str, Result<AuditResult, String>)>) -> ScriptedRunner {
    ScriptedRunner {
        outcomes: outcomes
            .into_iter()
            .map(|(u, o)| (u.to_string(), o))
            .collect(),
        gate: None,
        run_error: None,
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn posting_urls_returns_an_accepted_batch_id() {
    let clock = TestClock::new();
    let addr = spawn(
        scripted(vec![("https://a.test", Ok(audit("https://a.test", 80.0)))]),
        clock.as_clock(),
    )
    .await;

    let response = create(addr, &["https://a.test"]).await;
    assert_eq!(response.status(), 202);

    let body: Value = response.json().await.expect("create json");
    assert!(!body["batch_id"].as_str().expect("batch_id").is_empty());
    assert_eq!(body["url_count"], 1);
    assert_eq!(body["state"], "running");
    assert!(body["expires_at"].is_string());
}

#[tokio::test(flavor = "multi_thread")]
async fn status_reports_per_url_progress_as_each_url_finishes() {
    let (tx, rx) = tokio::sync::mpsc::channel(4);
    let clock = TestClock::new();
    let runner = ScriptedRunner {
        outcomes: [
            (
                "https://a.test".to_string(),
                Ok(audit("https://a.test", 90.0)),
            ),
            (
                "https://b.test".to_string(),
                Ok(audit("https://b.test", 70.0)),
            ),
        ]
        .into_iter()
        .collect(),
        gate: Some(tokio::sync::Mutex::new(rx)),
        run_error: None,
    };
    let addr = spawn(runner, clock.as_clock()).await;

    let body: Value = create(addr, &["https://a.test", "https://b.test"])
        .await
        .json()
        .await
        .expect("create json");
    let id = body["batch_id"].as_str().expect("batch_id").to_string();

    // Nothing released yet: both URLs must still be pending, and the batch
    // must not claim to be finished.
    let initial: Value = status(addr, &id).await.json().await.expect("status json");
    assert_eq!(initial["state"], "running");
    assert_eq!(initial["pending"], 2);
    assert_eq!(initial["total"], 2);

    tx.send(()).await.expect("release first url");
    let half = poll_until(addr, &id, |b| b["completed"] == 1).await;
    assert_eq!(half["state"], "running");
    assert_eq!(half["pending"], 1);
    let first = &half["urls"][0];
    assert_eq!(first["url"], "https://a.test");
    assert_eq!(first["state"], "completed");
    assert_eq!(first["audit_id"], "audit-for-https://a.test");
    assert_eq!(first["taux_global"], 90.0);

    tx.send(()).await.expect("release second url");
    let done = poll_until(addr, &id, |b| b["state"] == "completed").await;
    assert_eq!(done["completed"], 2);
    assert_eq!(done["pending"], 0);
}

#[tokio::test(flavor = "multi_thread")]
async fn final_results_are_refused_while_the_batch_is_still_running() {
    let (tx, rx) = tokio::sync::mpsc::channel(4);
    let clock = TestClock::new();
    let runner = ScriptedRunner {
        outcomes: [(
            "https://a.test".to_string(),
            Ok(audit("https://a.test", 42.0)),
        )]
        .into_iter()
        .collect(),
        gate: Some(tokio::sync::Mutex::new(rx)),
        run_error: None,
    };
    let addr = spawn(runner, clock.as_clock()).await;

    let body: Value = create(addr, &["https://a.test"])
        .await
        .json()
        .await
        .expect("create json");
    let id = body["batch_id"].as_str().expect("batch_id").to_string();

    // A partial average read as final is the failure mode this guards.
    assert_eq!(results(addr, &id).await.status(), 409);

    tx.send(()).await.expect("release url");
    poll_until(addr, &id, |b| b["state"] == "completed").await;
    assert_eq!(results(addr, &id).await.status(), 200);
}

#[tokio::test(flavor = "multi_thread")]
async fn final_results_aggregate_only_the_urls_that_produced_an_audit() {
    let clock = TestClock::new();
    let addr = spawn(
        scripted(vec![
            ("https://a.test", Ok(audit("https://a.test", 80.0))),
            ("https://b.test", Ok(audit("https://b.test", 60.0))),
            ("https://c.test", Err("navigation timed out".to_string())),
        ]),
        clock.as_clock(),
    )
    .await;

    let body: Value = create(
        addr,
        &["https://a.test", "https://b.test", "https://c.test"],
    )
    .await
    .json()
    .await
    .expect("create json");
    let id = body["batch_id"].as_str().expect("batch_id").to_string();
    poll_until(addr, &id, |b| b["state"] == "completed").await;

    let aggregated: Value = results(addr, &id).await.json().await.expect("results json");
    assert_eq!(aggregated["audited"], 2);
    assert_eq!(aggregated["failed"], 1);
    // The failed URL must not be averaged in as a zero.
    assert_eq!(aggregated["taux_global_moyen"], 70.0);
    let failed = aggregated["urls"]
        .as_array()
        .expect("urls")
        .iter()
        .find(|u| u["url"] == "https://c.test")
        .expect("failed url present");
    assert_eq!(failed["state"], "failed");
    assert_eq!(failed["error"], "navigation timed out");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_batch_stops_being_readable_24_hours_after_creation() {
    let clock = TestClock::new();
    let addr = spawn(
        scripted(vec![("https://a.test", Ok(audit("https://a.test", 80.0)))]),
        clock.as_clock(),
    )
    .await;

    let body: Value = create(addr, &["https://a.test"])
        .await
        .json()
        .await
        .expect("create json");
    let id = body["batch_id"].as_str().expect("batch_id").to_string();
    poll_until(addr, &id, |b| b["state"] == "completed").await;

    clock.advance(ChronoDuration::hours(23));
    assert_eq!(status(addr, &id).await.status(), 200);
    assert_eq!(results(addr, &id).await.status(), 200);

    clock.advance(ChronoDuration::hours(1));
    assert_eq!(status(addr, &id).await.status(), 410);
    assert_eq!(results(addr, &id).await.status(), 410);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_run_that_fails_before_fan_out_settles_the_batch_instead_of_hanging() {
    let clock = TestClock::new();
    let runner = ScriptedRunner {
        outcomes: HashMap::new(),
        gate: None,
        run_error: Some("obscura server failed to start".to_string()),
    };
    let addr = spawn(runner, clock.as_clock()).await;

    let body: Value = create(addr, &["https://a.test"])
        .await
        .json()
        .await
        .expect("create json");
    let id = body["batch_id"].as_str().expect("batch_id").to_string();

    let done = poll_until(addr, &id, |b| b["state"] == "completed").await;
    assert_eq!(done["failed"], 1);
    assert_eq!(done["urls"][0]["error"], "obscura server failed to start");

    let aggregated: Value = results(addr, &id).await.json().await.expect("results json");
    assert!(aggregated["taux_global_moyen"].is_null());
}

#[tokio::test(flavor = "multi_thread")]
async fn an_unknown_batch_id_is_a_not_found_rather_than_an_error() {
    let clock = TestClock::new();
    let addr = spawn(scripted(vec![]), clock.as_clock()).await;
    assert_eq!(status(addr, "no-such-batch").await.status(), 404);
    assert_eq!(results(addr, "no-such-batch").await.status(), 404);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_batch_with_no_urls_or_too_many_urls_is_rejected() {
    let clock = TestClock::new();
    let addr = spawn(scripted(vec![]), clock.as_clock()).await;

    assert_eq!(create(addr, &[]).await.status(), 400);

    let too_many: Vec<String> = (0..=MAX_BATCH_URLS)
        .map(|i| format!("https://{i}.test"))
        .collect();
    let response = reqwest::Client::new()
        .post(format!("http://{addr}/v1/batches"))
        .json(&json!({ "urls": too_many }))
        .send()
        .await
        .expect("create batch");
    assert_eq!(response.status(), 400);
}
