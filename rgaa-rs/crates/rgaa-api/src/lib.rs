pub mod routes;

use axum::{
    error_handling::HandleErrorLayer,
    http::StatusCode,
    middleware,
    routing::{get, post},
    BoxError, Router,
};
use std::sync::Arc;
use std::time::Duration;
use tower::limit::GlobalConcurrencyLimitLayer;
use tower::ServiceBuilder;
use tower_http::cors::{Any, CorsLayer};
use tower_http::timeout::TimeoutLayer;

use rgaa_orchestrator::Orchestrator;
use rgaa_storage::Storage;

#[derive(Clone)]
pub struct AppState {
    pub orchestrator: Arc<Orchestrator>,
    pub storage: Arc<dyn Storage>,
}

/// Request timeout, in seconds, for every route. A long audit call cannot
/// hold a server worker forever — the timeout surfaces as a marked error
/// (408) instead. Configurable via `RGAA_API_REQUEST_TIMEOUT_SECS`.
fn request_timeout() -> Duration {
    let secs = std::env::var("RGAA_API_REQUEST_TIMEOUT_SECS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(120);
    Duration::from_secs(secs)
}

/// Maximum audits in flight at once, matching the pipeline's own operating
/// point (#42: 8 concurrent audits). Configurable via
/// `RGAA_API_MAX_CONCURRENT_AUDITS`.
fn max_concurrent_audits() -> usize {
    std::env::var("RGAA_API_MAX_CONCURRENT_AUDITS")
        .ok()
        .and_then(|v| v.parse().ok())
        .filter(|&v| v > 0)
        .unwrap_or(8)
}

/// Converts an overload (load-shed) error into an explicit HTTP response
/// instead of the connection just hanging or dropping. A burst beyond the
/// concurrency limit gets `503` immediately; in-flight requests are
/// unaffected — they keep running under their own timeout.
///
/// `tower_http::timeout::TimeoutLayer` (unlike `tower::timeout`) never
/// reaches this handler: it returns its own `408 Request Timeout` response
/// directly rather than an error, so `LoadShed`'s overload is the only
/// error this stack can actually produce.
async fn handle_overload_error(_err: BoxError) -> (StatusCode, String) {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        "server is at capacity, try again shortly".to_string(),
    )
}

/// Applies the timeout/concurrency-limit/load-shed stack to `router`.
///
/// Order (outermost first): convert overload/timeout errors to HTTP
/// responses, shed load instead of queueing unboundedly once the
/// concurrency limit is hit, bound concurrency, then enforce the
/// per-request timeout closest to the handler. Exposed (not just inlined
/// into [`build_app`]) so it can be exercised directly against a minimal
/// router in tests, without needing a live orchestrator/storage.
///
/// Uses [`GlobalConcurrencyLimitLayer`], not `ServiceBuilder::concurrency_limit`
/// (sugar for the plain `ConcurrencyLimitLayer`): axum's `Router::layer` can
/// invoke `Layer::layer` on the same layer value more than once while
/// building its route tree, and `ConcurrencyLimitLayer::layer` constructs a
/// brand-new `Arc<Semaphore>` on every call — silently giving each
/// invocation its own full-capacity limit instead of one shared bound
/// (confirmed with an isolated pure-tower reproduction: `ConcurrencyLimitLayer`
/// applied once behaves correctly, but the plain `.concurrency_limit()`
/// sugar through a real `axum::serve` router let 10 concurrent requests
/// through a limit of 2 unshed). `GlobalConcurrencyLimitLayer` holds its
/// semaphore in the layer value itself and only clones the `Arc` on
/// `layer()`, so every invocation shares the same bound.
pub fn apply_resilience<S>(router: Router<S>, max_concurrent: usize, timeout: Duration) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    let resilience = ServiceBuilder::new()
        .layer(HandleErrorLayer::new(handle_overload_error))
        .load_shed()
        .layer(GlobalConcurrencyLimitLayer::new(max_concurrent))
        .layer(TimeoutLayer::new(timeout));
    router.layer(resilience)
}

pub fn build_app(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let public_routes = Router::new()
        .route("/health", get(routes::health))
        .route("/criteria", get(routes::list_criteria));

    let protected_routes = Router::new()
        .route("/v1/audit-bundles", post(routes::create_audit_bundle))
        .route("/v1/audit-bundles/{id}", get(routes::get_audit_bundle))
        .route("/v1/audit-bundles", get(routes::list_audit_bundles))
        .route(
            "/v1/audit-bundles/{id}",
            axum::routing::delete(routes::delete_audit_bundle),
        )
        .route("/v1/findings", get(routes::list_findings))
        .route("/v1/policy/evaluate", post(routes::evaluate_policy))
        .route_layer(middleware::from_fn_with_state(
            state.storage.clone(),
            routes::auth_middleware,
        ));

    // Timeout/concurrency-limit/load-shed apply to the legacy audit
    // endpoints only — `/health` must keep answering (liveness stays
    // meaningful even while audits are shed) and `/criteria` is a cheap
    // static lookup.
    let legacy_routes = Router::new()
        .route("/audit", post(routes::run_audit))
        .route("/audit/{id}", get(routes::get_audit));
    let legacy_routes = apply_resilience(legacy_routes, max_concurrent_audits(), request_timeout());

    Router::new()
        .merge(public_routes)
        .merge(protected_routes)
        .merge(legacy_routes)
        .layer(cors)
        .with_state(state)
}
