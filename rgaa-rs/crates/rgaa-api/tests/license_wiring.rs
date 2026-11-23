//! Licence middleware wiring of the REST app (#87): which routes are gated,
//! in what order relative to API-key auth, CORS and the resilience stack, and
//! the per-URL metering of batch creation.
//!
//! Goes through `build_app_with_api_keys` — the same router `build_app`
//! returns, with only the API-key lookup swapped for a stub — so no Postgres
//! is needed and none of these tests skips. The stub storage's pool is
//! `connect_lazy` on an unreachable address: it never connects, and a test
//! that accidentally reached the database would fail, not pass.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use axum::body::Body;
use axum::extract::State;
use axum::http::{header, Method, Request, StatusCode};
use axum::response::Response;
use axum::routing::post;
use axum::Router;
use chrono::{DateTime, Duration as ChronoDuration, TimeZone, Utc};
use serde_json::{json, Value};
use tower::ServiceExt;

use rgaa_api::batch::{system_clock, BatchRunner, BatchService, InMemoryBatchStore};
use rgaa_api::routes::ApiKeyValidator;
use rgaa_api::{apply_resilience, build_app_with_api_keys, classify_route, AppState};
use rgaa_core::{AuditBundle, AuditConfig, AuditResult, CrawlConfig};
use rgaa_license::middleware::{enforce, Classification, LicenseLayerState};
use rgaa_license::{
    Feature, GateConfig, LicenseClaims, LicenseFile, LicenseGate, LicenseIssuer, ManualClock, Tier,
    UsageCounter,
};
use rgaa_orchestrator::Orchestrator;
use rgaa_storage::{AuditSummary, Storage, StorageError};

const KEY: &str = "test-key";

// ---- fixtures ------------------------------------------------------------

struct Keys;

#[async_trait::async_trait]
impl ApiKeyValidator for Keys {
    async fn is_valid(&self, key: &str) -> bool {
        key == KEY
    }
}

/// Storage that answers from nothing. Its pool is lazy and points nowhere.
struct StubStorage {
    pool: sqlx::PgPool,
}

#[async_trait::async_trait]
impl Storage for StubStorage {
    fn pool(&self) -> &sqlx::PgPool {
        &self.pool
    }
    async fn save_audit(&self, audit: &AuditResult) -> Result<String, StorageError> {
        Ok(audit.audit_id.clone())
    }
    async fn get_audit(&self, _id: &str) -> Result<Option<AuditResult>, StorageError> {
        Ok(None)
    }
    async fn list_audits(
        &self,
        _limit: usize,
        _offset: usize,
    ) -> Result<Vec<AuditSummary>, StorageError> {
        Ok(vec![])
    }
    async fn delete_audit(&self, _id: &str) -> Result<(), StorageError> {
        Ok(())
    }
    async fn save_audit_log(
        &self,
        _audit_id: &str,
        _action: &str,
        _details: Option<Value>,
    ) -> Result<String, StorageError> {
        Ok(String::new())
    }
    async fn put_bundle(&self, _bundle: &AuditBundle) -> Result<(), StorageError> {
        Ok(())
    }
    async fn get_bundle_by_audit_id(
        &self,
        _audit_id: &str,
    ) -> Result<Option<AuditBundle>, StorageError> {
        Ok(None)
    }
}

/// Accepts a batch and audits nothing: the batch state machine is covered
/// by `batch_endpoints.rs`; here only admission matters.
struct IdleRunner;

#[async_trait::async_trait]
impl BatchRunner for IdleRunner {
    async fn run(
        &self,
        _urls: Vec<String>,
        _config: CrawlConfig,
        _on_url: rgaa_orchestrator::BatchObserver,
    ) -> Result<(), String> {
        Ok(())
    }
}

fn t0() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, 19, 12, 0, 0)
        .single()
        .expect("valid date")
}

enum Licence {
    None,
    Garbage,
    Tier(Tier, Option<u32>),
}

fn gate(licence: Licence) -> Arc<LicenseGate> {
    let issuer = LicenseIssuer::generate().expect("issuer");
    let gate = Arc::new(LicenseGate::new(
        issuer.verifier(),
        Arc::new(UsageCounter::in_memory()),
        Arc::new(ManualClock::new(t0())),
        GateConfig {
            upgrade_url: Some("https://example.test/upgrade".into()),
            ..GateConfig::default()
        },
    ));
    let token = match licence {
        Licence::None => return gate,
        Licence::Garbage => "v4.public.not-a-token".to_string(),
        Licence::Tier(tier, quota) => issuer
            .sign(&LicenseClaims {
                sub: "org-1".into(),
                tier,
                iat: t0() - ChronoDuration::days(1),
                exp: t0() + ChronoDuration::days(365),
                nbf: None,
                quota,
            })
            .expect("sign"),
    };
    gate.install(Some(&LicenseFile {
        token,
        last_validated: t0(),
    }));
    gate
}

fn app(gate: Arc<LicenseGate>) -> Router {
    let pool = sqlx::postgres::PgPoolOptions::new()
        .acquire_timeout(Duration::from_millis(200))
        .connect_lazy("postgres://nobody@127.0.0.1:1/none")
        .expect("lazy pool");
    let storage: Arc<dyn Storage> = Arc::new(StubStorage { pool });
    let state = AppState {
        orchestrator: Arc::new(Orchestrator::with_storage(storage.clone())),
        storage,
        // Deliberately unrestricted: `build_app` must replace it with the
        // app's gate, or the batch tests below would never see a 429.
        batches: BatchService::new(
            Arc::new(InMemoryBatchStore::new()),
            Arc::new(IdleRunner),
            system_clock(),
            Arc::new(LicenseGate::unrestricted()),
        ),
    };
    build_app_with_api_keys(state, gate, Arc::new(Keys))
}

fn bundle_body() -> Value {
    json!({ "bundle": AuditBundle::new("a-1", "https://example.test", AuditConfig::default()) })
}

fn batch_body(n: usize) -> Value {
    let urls: Vec<String> = (0..n).map(|i| format!("https://site{i}.test")).collect();
    json!({ "urls": urls })
}

async fn send(
    app: &Router,
    method: Method,
    path: &str,
    key: Option<&str>,
    body: Option<Value>,
) -> Response {
    let mut req = Request::builder().method(method).uri(path);
    if let Some(key) = key {
        req = req.header(header::AUTHORIZATION, format!("Bearer {key}"));
    }
    let req = match body {
        Some(b) => req
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(b.to_string())),
        None => req.body(Body::empty()),
    }
    .expect("request");
    app.clone().oneshot(req).await.expect("response")
}

fn hdr<'a>(r: &'a Response, name: &str) -> Option<&'a str> {
    r.headers().get(name).and_then(|v| v.to_str().ok())
}

async fn json_body(r: Response) -> Value {
    let bytes = axum::body::to_bytes(r.into_body(), 1 << 20)
        .await
        .expect("body");
    serde_json::from_slice(&bytes).expect("json body")
}

// ---- exemptions and ordering ---------------------------------------------

#[tokio::test]
async fn health_and_criteria_stay_open_with_an_invalid_licence() {
    let app = app(gate(Licence::Garbage));
    for path in ["/health", "/criteria"] {
        let r = send(&app, Method::GET, path, None, None).await;
        assert_eq!(r.status(), StatusCode::OK, "{path}");
        assert!(hdr(&r, "x-license-tier").is_none(), "{path} is not gated");
    }
}

#[tokio::test]
async fn invalid_licence_refuses_v1_with_401() {
    let app = app(gate(Licence::Garbage));
    let r = send(&app, Method::GET, "/v1/audit-bundles", Some(KEY), None).await;
    assert_eq!(r.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(json_body(r).await["code"], "license_invalid");
}

#[tokio::test]
async fn api_key_auth_runs_before_the_licence_check() {
    // Free would refuse every route below with 402; without a key the caller
    // must get auth's bare 401 instead, and learn nothing about the licence.
    let app = app(gate(Licence::None));
    let cases = [
        (Method::POST, "/v1/audit-bundles", Some(bundle_body())),
        (Method::GET, "/v1/audit-bundles", None),
        (Method::POST, "/v1/policy/evaluate", Some(bundle_body())),
        (Method::POST, "/v1/batches", Some(batch_body(1))),
        (Method::GET, "/v1/batches/x", None),
    ];
    for (method, path, body) in cases {
        for key in [None, Some("wrong")] {
            let r = send(&app, method.clone(), path, key, body.clone()).await;
            assert_eq!(r.status(), StatusCode::UNAUTHORIZED, "{method} {path}");
            assert!(hdr(&r, "x-license-tier").is_none(), "{method} {path}");
        }
    }
}

// ---- tiers ---------------------------------------------------------------

#[tokio::test]
async fn free_tier_gets_402_feature_not_in_tier() {
    let app = app(gate(Licence::None));
    let r = send(
        &app,
        Method::POST,
        "/v1/audit-bundles",
        Some(KEY),
        Some(bundle_body()),
    )
    .await;
    assert_eq!(r.status(), StatusCode::PAYMENT_REQUIRED);
    assert_eq!(hdr(&r, "x-license-tier"), Some("free"));
    let body = json_body(r).await;
    assert_eq!(body["code"], "feature_not_in_tier");
    assert_eq!(body["required_tier"], "professional");
    assert_eq!(body["upgrade_url"], "https://example.test/upgrade");

    let r = send(
        &app,
        Method::POST,
        "/v1/batches",
        Some(KEY),
        Some(batch_body(1)),
    )
    .await;
    assert_eq!(r.status(), StatusCode::PAYMENT_REQUIRED);
}

#[tokio::test]
async fn professional_passes_and_gets_licence_headers() {
    let gate = gate(Licence::Tier(Tier::Professional, None));
    let app = app(gate.clone());

    let r = send(
        &app,
        Method::POST,
        "/v1/audit-bundles",
        Some(KEY),
        Some(bundle_body()),
    )
    .await;
    assert_eq!(r.status(), StatusCode::OK);
    assert_eq!(hdr(&r, "x-license-tier"), Some("professional"));
    assert_eq!(hdr(&r, "x-license-status"), Some("valid"));
    assert_eq!(hdr(&r, "x-ratelimit-limit"), Some("500"));
    assert_eq!(hdr(&r, "x-ratelimit-remaining"), Some("499"));
    assert!(hdr(&r, "x-ratelimit-reset").is_some());

    // Reads are access checks: headers, no consumption.
    let r = send(&app, Method::GET, "/v1/audit-bundles", Some(KEY), None).await;
    assert_eq!(r.status(), StatusCode::OK);
    assert_eq!(hdr(&r, "x-ratelimit-remaining"), Some("499"));
    assert_eq!(gate.snapshot().rate.remaining, 499);
}

#[tokio::test]
async fn exhausted_quota_is_429_with_retry_after() {
    let app = app(gate(Licence::Tier(Tier::Professional, Some(1))));
    let post = || {
        send(
            &app,
            Method::POST,
            "/v1/policy/evaluate",
            Some(KEY),
            Some(bundle_body()),
        )
    };
    assert_eq!(post().await.status(), StatusCode::OK);
    let r = post().await;
    assert_eq!(r.status(), StatusCode::TOO_MANY_REQUESTS);
    let retry: u64 = hdr(&r, "retry-after")
        .and_then(|v| v.parse().ok())
        .expect("numeric Retry-After");
    assert!(retry > 0);
    assert_eq!(json_body(r).await["code"], "quota_exceeded");
}

// ---- batches -------------------------------------------------------------

#[tokio::test]
async fn batch_beyond_remaining_quota_is_refused_whole() {
    let gate = gate(Licence::Tier(Tier::Professional, Some(5)));
    let app = app(gate.clone());

    let r = send(
        &app,
        Method::POST,
        "/v1/batches",
        Some(KEY),
        Some(batch_body(20)),
    )
    .await;
    assert_eq!(r.status(), StatusCode::TOO_MANY_REQUESTS);
    assert!(hdr(&r, "retry-after").is_some());
    assert_eq!(json_body(r).await["code"], "quota_exceeded");
    assert_eq!(gate.snapshot().rate.remaining, 5, "nothing consumed");

    // Exactly the remaining budget fits, and the response reports the
    // post-consumption count — not the layer's pre-handler peek.
    let r = send(
        &app,
        Method::POST,
        "/v1/batches",
        Some(KEY),
        Some(batch_body(5)),
    )
    .await;
    assert_eq!(r.status(), StatusCode::ACCEPTED);
    assert_eq!(hdr(&r, "x-ratelimit-remaining"), Some("0"));
    assert_eq!(gate.snapshot().rate.remaining, 0);
}

#[tokio::test]
async fn malformed_batch_costs_nothing() {
    let gate = gate(Licence::Tier(Tier::Professional, Some(5)));
    let app = app(gate.clone());
    let r = send(
        &app,
        Method::POST,
        "/v1/batches",
        Some(KEY),
        Some(batch_body(0)),
    )
    .await;
    assert_eq!(r.status(), StatusCode::BAD_REQUEST);
    assert_eq!(gate.snapshot().rate.remaining, 5);
}

// ---- CORS ----------------------------------------------------------------

#[tokio::test]
async fn cors_exposes_licence_headers_even_on_refusals() {
    let app = app(gate(Licence::None));

    let preflight = Request::builder()
        .method(Method::OPTIONS)
        .uri("/v1/audit-bundles")
        .header(header::ORIGIN, "https://client.test")
        .header(header::ACCESS_CONTROL_REQUEST_METHOD, "POST")
        .body(Body::empty())
        .expect("request");
    let r = app.clone().oneshot(preflight).await.expect("response");
    assert!(r.status().is_success(), "preflight: {}", r.status());
    assert_eq!(hdr(&r, "access-control-allow-origin"), Some("*"));

    let req = Request::builder()
        .method(Method::POST)
        .uri("/v1/audit-bundles")
        .header(header::ORIGIN, "https://client.test")
        .header(header::AUTHORIZATION, format!("Bearer {KEY}"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(bundle_body().to_string()))
        .expect("request");
    let r = app.clone().oneshot(req).await.expect("response");
    assert_eq!(r.status(), StatusCode::PAYMENT_REQUIRED);
    assert_eq!(hdr(&r, "access-control-allow-origin"), Some("*"));
    let exposed = hdr(&r, "access-control-expose-headers")
        .expect("expose-headers")
        .to_ascii_lowercase();
    for name in rgaa_license::gate::headers::EXPOSED {
        assert!(exposed.contains(name), "{name} not exposed: {exposed}");
    }
}

// ---- resilience ----------------------------------------------------------

async fn slow(State(started): State<Arc<tokio::sync::Notify>>) -> &'static str {
    started.notify_one();
    tokio::time::sleep(Duration::from_millis(400)).await;
    "ok"
}

/// The licence layer must wrap the resilience stack, not sit inside it.
/// With one concurrency slot held by an admitted request, a second request
/// over quota must get the licence's 429: inside the stack it would be shed
/// as 503 first, and would have taken a slot to be refused.
#[tokio::test(flavor = "multi_thread")]
async fn quota_refusal_is_429_not_503_under_the_resilience_stack() {
    let gate = gate(Licence::Tier(Tier::Professional, Some(1)));
    let started = Arc::new(tokio::sync::Notify::new());
    let router = Router::new()
        .route("/v1/audit-bundles", post(slow))
        .with_state(started.clone());
    let router = apply_resilience(router, 1, Duration::from_secs(5)).route_layer(
        axum::middleware::from_fn_with_state(LicenseLayerState::new(gate, classify_route), enforce),
    );

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let addr: SocketAddr = listener.local_addr().expect("addr");
    tokio::spawn(async move {
        let _ = axum::serve(listener, router).await;
    });
    let url = format!("http://{addr}/v1/audit-bundles");
    let client = reqwest::Client::new();

    let first = tokio::spawn({
        let (client, url) = (client.clone(), url.clone());
        async move { client.post(url).send().await.expect("first").status() }
    });
    // Fails instead of hanging if the first request never reaches the handler.
    if tokio::time::timeout(Duration::from_secs(10), started.notified())
        .await
        .is_err()
    {
        panic!("first request never reached the handler: {:?}", first.await);
    }

    let second = client.post(&url).send().await.expect("second");
    assert_eq!(second.status(), reqwest::StatusCode::TOO_MANY_REQUESTS);
    assert!(second.headers().get("retry-after").is_some());
    assert_eq!(first.await.expect("join"), reqwest::StatusCode::OK);
}

// ---- classification --------------------------------------------------------

#[test]
fn every_v1_route_has_the_agreed_classification() {
    use Classification::{Access, Metered};
    use Feature::{Batch, FullAudit, Remediation};
    let cases = [
        (Method::POST, "/v1/audit-bundles", Metered(FullAudit, 1)),
        (Method::GET, "/v1/audit-bundles", Access(FullAudit)),
        (Method::GET, "/v1/audit-bundles/a-1", Access(FullAudit)),
        (Method::DELETE, "/v1/audit-bundles/a-1", Access(FullAudit)),
        (Method::GET, "/v1/findings", Access(FullAudit)),
        (Method::POST, "/v1/policy/evaluate", Metered(Remediation, 1)),
        (Method::POST, "/v1/batches", Access(Batch)),
        (Method::GET, "/v1/batches/b-1", Access(Batch)),
        (Method::GET, "/v1/batches/b-1/results", Access(Batch)),
        // Unlisted routes fail closed.
        (Method::POST, "/v1/something-new", Metered(FullAudit, 1)),
        (Method::GET, "/v1/audit-bundlesX", Metered(FullAudit, 1)),
    ];
    for (method, path, expected) in cases {
        assert_eq!(classify_route(&method, path), expected, "{method} {path}");
    }
}
