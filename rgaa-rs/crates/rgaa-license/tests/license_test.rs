//! Gate, middleware and usage-sync behaviour. Every test injects its clock,
//! keys and paths: no network, no secrets, no env vars, so nothing here can
//! skip in CI.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration as StdDuration;

use async_trait::async_trait;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::middleware::from_fn_with_state;
use axum::routing::{get, post};
use axum::Router;
use chrono::{DateTime, Duration, TimeZone, Utc};
use rgaa_license::middleware::{enforce, Classification, LicenseLayerState};
use rgaa_license::{
    Clock, Feature, GateConfig, LicenseClaims, LicenseError, LicenseFile, LicenseGate,
    LicenseIssuer, LicenseStatus, ManualClock, ReporterConfig, Tier, UsageCounter, UsageEvent,
    UsageReporter, UsageSink,
};
use tower::ServiceExt;

fn t0() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, 19, 12, 0, 0).unwrap()
}

struct Fixture {
    issuer: LicenseIssuer,
    clock: Arc<ManualClock>,
    gate: Arc<LicenseGate>,
}

fn fixture(config: GateConfig) -> Fixture {
    let issuer = LicenseIssuer::generate().unwrap();
    let clock = Arc::new(ManualClock::new(t0()));
    let gate = Arc::new(LicenseGate::new(
        issuer.verifier(),
        Arc::new(UsageCounter::in_memory()),
        clock.clone(),
        config,
    ));
    Fixture {
        issuer,
        clock,
        gate,
    }
}

fn claims(tier: Tier) -> LicenseClaims {
    LicenseClaims {
        sub: "org-1".into(),
        tier,
        iat: t0() - Duration::days(1),
        exp: t0() + Duration::days(365),
        nbf: None,
        quota: None,
    }
}

impl Fixture {
    fn install(&self, claims: &LicenseClaims, last_validated: DateTime<Utc>) {
        let token = self.issuer.sign(claims).unwrap();
        self.gate.install(Some(&LicenseFile {
            token,
            last_validated,
        }));
    }
}

fn with_grace() -> GateConfig {
    GateConfig {
        grace: Some(Duration::days(7)),
        ..GateConfig::default()
    }
}

// ---- tier gating -------------------------------------------------------

#[test]
fn unlicensed_is_free_and_lint_works() {
    let f = fixture(GateConfig::default());
    let ok = f.gate.check(Feature::StaticLint, 1).unwrap();
    assert_eq!(ok.tier, Tier::Free);
    assert_eq!(ok.status, LicenseStatus::Unlicensed);

    let denied = f.gate.check(Feature::FullAudit, 1).unwrap_err();
    assert_eq!(denied.http_status(), 402);
    assert_eq!(denied.error.code(), "feature_not_in_tier");
    assert_eq!(denied.retry_after_secs(), None);
}

#[test]
fn professional_unlocks_everything() {
    let f = fixture(GateConfig::default());
    f.install(&claims(Tier::Professional), t0());
    for feature in Feature::ALL {
        assert!(f.gate.check(feature, 1).is_ok(), "{feature}");
    }
}

#[test]
fn bad_token_refuses_even_free_features_with_401() {
    let f = fixture(GateConfig::default());
    f.gate.install(Some(&LicenseFile {
        token: "v4.public.not-really".into(),
        last_validated: t0(),
    }));
    let denied = f.gate.check(Feature::StaticLint, 1).unwrap_err();
    assert_eq!(denied.http_status(), 401);
    assert_eq!(denied.error.code(), "license_invalid");
}

#[test]
fn token_signed_by_another_key_is_401() {
    let f = fixture(GateConfig::default());
    let stranger = LicenseIssuer::generate().unwrap();
    f.gate.install(Some(&LicenseFile {
        token: stranger.sign(&claims(Tier::Team)).unwrap(),
        last_validated: t0(),
    }));
    let denied = f.gate.check(Feature::FullAudit, 1).unwrap_err();
    assert_eq!(denied.error, LicenseError::InvalidSignature);
    assert_eq!(denied.http_status(), 401);
}

#[test]
fn not_yet_valid_is_401() {
    let f = fixture(GateConfig::default());
    let mut c = claims(Tier::Team);
    c.nbf = Some(t0() + Duration::hours(1));
    f.install(&c, t0());
    assert_eq!(
        f.gate
            .check(Feature::FullAudit, 1)
            .unwrap_err()
            .http_status(),
        401
    );
    f.clock.advance(Duration::hours(1));
    assert!(f.gate.check(Feature::FullAudit, 1).is_ok());
}

#[test]
fn expired_token_drops_to_free() {
    let f = fixture(GateConfig::default());
    let mut c = claims(Tier::Team);
    c.exp = t0() + Duration::seconds(1);
    f.install(&c, t0());
    assert!(f.gate.check(Feature::FullAudit, 1).is_ok());

    f.clock.advance(Duration::seconds(1)); // exp is exclusive
    let denied = f.gate.check(Feature::FullAudit, 1).unwrap_err();
    assert_eq!(denied.http_status(), 402);
    assert_eq!(denied.error.code(), "license_expired");
    let lint = f.gate.check(Feature::StaticLint, 1).unwrap();
    assert_eq!(lint.tier, Tier::Free);
    assert!(matches!(lint.status, LicenseStatus::Expired { .. }));
}

// ---- grace period ------------------------------------------------------

#[test]
fn grace_boundary_is_inclusive_at_exactly_seven_days() {
    let f = fixture(with_grace());
    f.install(&claims(Tier::Professional), t0());

    f.clock.set(t0() + Duration::hours(23));
    assert_eq!(
        f.gate.check(Feature::FullAudit, 1).unwrap().status,
        LicenseStatus::Valid
    );

    f.clock.set(t0() + Duration::days(7) - Duration::seconds(1));
    let ok = f.gate.check(Feature::FullAudit, 1).unwrap();
    assert!(matches!(ok.status, LicenseStatus::Grace { .. }));
    let headers = ok.headers();
    let remaining = headers
        .iter()
        .find(|(k, _)| *k == "x-license-grace-remaining")
        .map(|(_, v)| v.as_str());
    assert_eq!(remaining, Some("1"));

    f.clock.set(t0() + Duration::days(7));
    assert!(
        f.gate.check(Feature::FullAudit, 1).is_ok(),
        "exactly 7 days is inside"
    );

    f.clock.set(t0() + Duration::days(7) + Duration::seconds(1));
    let denied = f.gate.check(Feature::FullAudit, 1).unwrap_err();
    assert_eq!(denied.http_status(), 402);
    assert!(matches!(denied.error, LicenseError::GraceExceeded { .. }));
    // Falls back to Free rather than blocking everything.
    assert_eq!(
        f.gate.check(Feature::StaticLint, 1).unwrap().tier,
        Tier::Free
    );
}

#[test]
fn without_grace_only_exp_matters() {
    let f = fixture(GateConfig::default());
    f.install(&claims(Tier::Professional), t0());
    f.clock.set(t0() + Duration::days(300));
    assert_eq!(
        f.gate.check(Feature::FullAudit, 1).unwrap().status,
        LicenseStatus::Valid
    );
}

#[test]
fn reinstalling_after_online_check_restores_the_tier() {
    let f = fixture(with_grace());
    f.install(&claims(Tier::Professional), t0());
    f.clock.set(t0() + Duration::days(8));
    assert!(f.gate.check(Feature::FullAudit, 1).is_err());
    f.install(&claims(Tier::Professional), f.clock.now());
    assert!(f.gate.check(Feature::FullAudit, 1).is_ok());
}

// ---- quota -------------------------------------------------------------

#[test]
fn refused_requests_consume_no_quota() {
    let f = fixture(GateConfig::default());
    // Free: 10/day. Ten 402s must not eat into it.
    for _ in 0..10 {
        assert!(f.gate.check(Feature::FullAudit, 1).is_err());
    }
    for _ in 0..10 {
        assert!(f.gate.check(Feature::StaticLint, 1).is_ok());
    }
    let denied = f.gate.check(Feature::StaticLint, 1).unwrap_err();
    assert_eq!(denied.http_status(), 429);
    assert_eq!(
        denied.jsonrpc_code(),
        rgaa_license::gate::JSONRPC_QUOTA_EXCEEDED
    );
    // 12:00 → next midnight UTC.
    assert_eq!(denied.retry_after_secs(), Some(12 * 3600));
    let body = denied.body();
    assert_eq!(body["code"], "quota_exceeded");
    assert_eq!(body["limit"], 10);
    assert_eq!(body["retry_after"], 12 * 3600);
    assert!(body["error"].is_string(), "old clients read `error`");
}

#[test]
fn access_checks_do_not_meter() {
    let f = fixture(GateConfig::default());
    f.install(&claims(Tier::Professional), t0());
    for _ in 0..1000 {
        f.gate.check(Feature::Batch, 0).unwrap();
    }
    assert_eq!(
        f.gate
            .check(Feature::Batch, 1)
            .unwrap()
            .rate
            .unwrap()
            .remaining,
        499
    );
}

#[test]
fn quota_claim_overrides_tier_default() {
    let f = fixture(GateConfig::default());
    let mut c = claims(Tier::Team);
    c.quota = Some(2);
    f.install(&c, t0());
    f.gate.check(Feature::FullAudit, 2).unwrap();
    assert_eq!(
        f.gate
            .check(Feature::FullAudit, 1)
            .unwrap_err()
            .http_status(),
        429
    );
}

#[test]
fn quota_not_enforced_locally_but_tier_still_is() {
    let f = fixture(GateConfig {
        enforce_quota: false,
        ..GateConfig::default()
    });
    for _ in 0..50 {
        let ok = f.gate.check(Feature::StaticLint, 1).unwrap();
        assert!(ok.rate.is_none());
    }
    assert_eq!(
        f.gate.check(Feature::Igt, 1).unwrap_err().http_status(),
        402
    );
}

#[test]
fn upgrade_url_only_on_402() {
    let f = fixture(GateConfig {
        upgrade_url: Some("https://example.invalid/upgrade".into()),
        ..GateConfig::default()
    });
    let body = f.gate.check(Feature::Remediation, 1).unwrap_err().body();
    assert_eq!(body["upgrade_url"], "https://example.invalid/upgrade");
    assert_eq!(body["required_tier"], "professional");
}

#[test]
fn unrestricted_double_admits_everything() {
    let gate = LicenseGate::unrestricted();
    for _ in 0..100 {
        for feature in Feature::ALL {
            assert_eq!(gate.check(feature, 1).unwrap().tier, Tier::Team);
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn concurrent_requests_never_over_admit() {
    let f = fixture(GateConfig::default());
    let mut tasks = Vec::new();
    for _ in 0..64 {
        let gate = f.gate.clone();
        tasks.push(tokio::spawn(async move {
            gate.check(Feature::StaticLint, 1).is_ok()
        }));
    }
    let mut admitted = 0;
    for t in tasks {
        if t.await.unwrap() {
            admitted += 1;
        }
    }
    assert_eq!(admitted, 10);
}

// ---- middleware --------------------------------------------------------

fn classify(_m: &axum::http::Method, path: &str) -> Classification {
    match path {
        "/health" => Classification::Exempt,
        "/lint" => Classification::Metered(Feature::StaticLint, 1),
        "/audit" => Classification::Metered(Feature::FullAudit, 1),
        _ => Classification::Access(Feature::FullAudit),
    }
}

fn app(gate: Arc<LicenseGate>) -> Router {
    Router::new()
        .route("/lint", post(|| async { "linted" }))
        .route("/audit", post(|| async { "audited" }))
        .route("/audit/:id", get(|| async { "result" }))
        .route_layer(from_fn_with_state(
            LicenseLayerState::new(gate, classify),
            enforce,
        ))
        .route("/health", get(|| async { "ok" }))
}

async fn call(app: &Router, method: &str, path: &str) -> axum::response::Response {
    app.clone()
        .oneshot(
            Request::builder()
                .method(method)
                .uri(path)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap()
}

fn header<'a>(r: &'a axum::response::Response, name: &str) -> Option<&'a str> {
    r.headers().get(name).and_then(|v| v.to_str().ok())
}

#[tokio::test]
async fn middleware_sets_headers_on_success() {
    let f = fixture(GateConfig::default());
    let app = app(f.gate.clone());
    let r = call(&app, "POST", "/lint").await;
    assert_eq!(r.status(), StatusCode::OK);
    assert_eq!(header(&r, "x-license-tier"), Some("free"));
    assert_eq!(header(&r, "x-license-status"), Some("unlicensed"));
    assert_eq!(header(&r, "x-ratelimit-limit"), Some("10"));
    assert_eq!(header(&r, "x-ratelimit-remaining"), Some("9"));
    let reset: i64 = header(&r, "x-ratelimit-reset").unwrap().parse().unwrap();
    assert_eq!(
        reset,
        Utc.with_ymd_and_hms(2026, 11, 20, 0, 0, 0)
            .unwrap()
            .timestamp()
    );
    assert_eq!(header(&r, "retry-after"), None);
}

#[tokio::test]
async fn middleware_402_and_429() {
    let f = fixture(GateConfig::default());
    let app = app(f.gate.clone());

    let r = call(&app, "POST", "/audit").await;
    assert_eq!(r.status(), StatusCode::PAYMENT_REQUIRED);
    assert_eq!(header(&r, "retry-after"), None);
    let body: serde_json::Value =
        serde_json::from_slice(&axum::body::to_bytes(r.into_body(), 1 << 16).await.unwrap())
            .unwrap();
    assert_eq!(body["code"], "feature_not_in_tier");

    for _ in 0..10 {
        assert_eq!(call(&app, "POST", "/lint").await.status(), StatusCode::OK);
    }
    let r = call(&app, "POST", "/lint").await;
    assert_eq!(r.status(), StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(header(&r, "retry-after"), Some("43200"));
    assert_eq!(header(&r, "x-ratelimit-remaining"), Some("0"));
}

#[tokio::test]
async fn health_is_never_gated() {
    let f = fixture(GateConfig::default());
    f.gate.install(Some(&LicenseFile {
        token: "broken".into(),
        last_validated: t0(),
    }));
    let app = app(f.gate.clone());
    let r = call(&app, "GET", "/health").await;
    assert_eq!(r.status(), StatusCode::OK);
    assert_eq!(header(&r, "x-license-tier"), None);
    assert_eq!(
        call(&app, "POST", "/lint").await.status(),
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn access_routes_need_the_tier_but_not_quota() {
    let f = fixture(GateConfig::default());
    f.install(&claims(Tier::Professional), t0());
    let app = app(f.gate.clone());
    let r = call(&app, "GET", "/audit/42").await;
    assert_eq!(r.status(), StatusCode::OK);
    assert_eq!(header(&r, "x-ratelimit-remaining"), Some("500"));
}

// ---- usage reporting ---------------------------------------------------

/// A sink that never answers, or answers with an error a few times first.
struct FlakySink {
    hang: bool,
    fail_first: usize,
    calls: AtomicUsize,
    delivered: std::sync::Mutex<Vec<UsageEvent>>,
}

#[async_trait]
impl UsageSink for FlakySink {
    async fn report(&self, events: &[UsageEvent]) -> Result<(), LicenseError> {
        let n = self.calls.fetch_add(1, Ordering::SeqCst);
        if self.hang {
            std::future::pending::<()>().await;
        }
        if n < self.fail_first {
            return Err(LicenseError::Sync("service down".into()));
        }
        self.delivered.lock().unwrap().extend_from_slice(events);
        Ok(())
    }
}

fn reporter_config() -> ReporterConfig {
    ReporterConfig {
        flush_every: StdDuration::from_secs(60),
        sink_timeout: StdDuration::from_secs(5),
        max_pending: 100,
    }
}

#[tokio::test(start_paused = true)]
async fn hanging_sink_never_blocks_requests() {
    let sink = Arc::new(FlakySink {
        hang: true,
        fail_first: 0,
        calls: AtomicUsize::new(0),
        delivered: Default::default(),
    });
    let (reporter, _task) = UsageReporter::spawn(sink.clone(), reporter_config());
    let f = fixture(GateConfig::default());
    let gate = Arc::new(
        LicenseGate::new(
            f.issuer.verifier(),
            Arc::new(UsageCounter::in_memory()),
            f.clock.clone(),
            GateConfig::default(),
        )
        .with_reporter(reporter),
    );
    let app = app(gate);
    tokio::time::sleep(StdDuration::from_secs(61)).await; // a flush is now hanging
    let r = tokio::time::timeout(StdDuration::from_millis(200), call(&app, "POST", "/lint"))
        .await
        .expect("request must not wait on usage sync");
    assert_eq!(r.status(), StatusCode::OK);
}

#[tokio::test(start_paused = true)]
async fn failed_batches_are_kept_and_retried() {
    let sink = Arc::new(FlakySink {
        hang: false,
        fail_first: 2,
        calls: AtomicUsize::new(0),
        delivered: Default::default(),
    });
    let (reporter, task) = UsageReporter::spawn(sink.clone(), reporter_config());
    for _ in 0..3 {
        reporter.record(UsageEvent {
            feature: Feature::FullAudit,
            units: 1,
            at: t0(),
            subject: None,
        });
    }
    // Two failing flushes, then a successful one.
    tokio::time::sleep(StdDuration::from_secs(181)).await;
    assert_eq!(sink.calls.load(Ordering::SeqCst), 3);
    assert_eq!(sink.delivered.lock().unwrap().len(), 3);

    drop(reporter);
    task.await.unwrap();
}
