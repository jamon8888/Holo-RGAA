pub mod batch;
pub mod routes;
pub mod webhook;

use axum::{
    error_handling::HandleErrorLayer,
    http::{HeaderName, Method, StatusCode},
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

use rgaa_license::middleware::{enforce, Classification, LicenseLayerState};
use rgaa_license::{Feature, LicenseGate};
use rgaa_orchestrator::Orchestrator;
use rgaa_storage::Storage;

use crate::batch::BatchService;
use crate::routes::{ApiKeyValidator, StorageApiKeys};

#[derive(Clone)]
pub struct AppState {
    pub orchestrator: Arc<Orchestrator>,
    pub storage: Arc<dyn Storage>,
    /// Batch endpoints carry their own store/runner/clock rather than
    /// reading them off `AppState`, so [`batch::batch_router`] can be
    /// served standalone in tests without a database or a live browser.
    pub batches: BatchService,
}

/// Request timeout, in seconds, for every route. A long audit call cannot
/// hold a server worker forever — the timeout surfaces as a marked error
/// (408) instead. Configurable via `RGAA_API_REQUEST_TIMEOUT_SECS`.
/// Default raised to 600s (10min) to accommodate multi-criterion LLM evaluation.
fn request_timeout() -> Duration {
    let secs = std::env::var("RGAA_API_REQUEST_TIMEOUT_SECS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(600);
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

/// Which licence check each `/v1` route gets (#87).
///
/// Only consulted for routes the licence layer is attached to (`route_layer`
/// on the `/v1` sub-routers), so `/health`, `GET /criteria` and the legacy
/// `/audit` routes never reach it.
///
/// `POST /v1/batches` is only an access check here: its cost is the number of
/// URLs in the body, which a path classifier cannot see, so
/// [`batch::create_batch`] meters it after parsing.
///
/// An unlisted `/v1` route fails closed — metered as a full audit — so a new
/// endpoint added without a line here is never served for free.
#[must_use]
pub fn classify_route(method: &Method, path: &str) -> Classification {
    let bundles = path == "/v1/audit-bundles" || path.starts_with("/v1/audit-bundles/");
    let batches = path == "/v1/batches" || path.starts_with("/v1/batches/");
    match path {
        "/v1/audit-bundles" if method == Method::POST => {
            Classification::Metered(Feature::FullAudit, 1)
        }
        _ if bundles => Classification::Access(Feature::FullAudit),
        "/v1/findings" => Classification::Access(Feature::FullAudit),
        "/v1/policy/evaluate" => Classification::Metered(Feature::Remediation, 1),
        _ if batches => Classification::Access(Feature::Batch),
        _ => Classification::Metered(Feature::FullAudit, 1),
    }
}

/// Configuration error for `RGAA_API_CORS_ORIGINS`, reported at startup.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum CorsConfigError {
    /// `*` would emit a wildcard policy: refused, the origins must be listed.
    #[error("RGAA_API_CORS_ORIGINS: wildcard `*` is not allowed, list explicit origins")]
    Wildcard,
    /// An entry that is not a bare `http(s)://host[:port]` origin.
    #[error("RGAA_API_CORS_ORIGINS: invalid origin `{0}` (expected http:// or https:// origin)")]
    Invalid(String),
}

/// Parses a comma-separated list of explicit origins.
///
/// An empty or absent value means no cross-origin access. `*` and any entry that
/// is not a bare origin (scheme + host, no path, query or wildcard) are errors:
/// silently dropping them would start the API without the origins the operator
/// intended.
pub fn parse_cors_origins(
    raw: Option<&str>,
) -> Result<Vec<axum::http::HeaderValue>, CorsConfigError> {
    let Some(raw) = raw else {
        return Ok(Vec::new());
    };
    let mut origins = Vec::new();
    for entry in raw.split(',').map(str::trim).filter(|e| !e.is_empty()) {
        if entry == "*" {
            return Err(CorsConfigError::Wildcard);
        }
        let rest = entry
            .strip_prefix("https://")
            .or_else(|| entry.strip_prefix("http://"))
            .ok_or_else(|| CorsConfigError::Invalid(entry.to_string()))?;
        if rest.is_empty() || rest.contains(['/', '*', '?', '#', ' ']) {
            return Err(CorsConfigError::Invalid(entry.to_string()));
        }
        let value = entry
            .parse::<axum::http::HeaderValue>()
            .map_err(|_| CorsConfigError::Invalid(entry.to_string()))?;
        origins.push(value);
    }
    Ok(origins)
}

/// Production entry point: API keys are checked against the `api_keys`
/// table of `state.storage`.
pub fn build_app(
    state: AppState,
    license: Arc<LicenseGate>,
) -> Result<Router, CorsConfigError> {
    let api_keys: Arc<dyn ApiKeyValidator> = Arc::new(StorageApiKeys::new(state.storage.clone()));
    build_app_with_api_keys(state, license, api_keys)
}

/// [`build_app`] with the API-key check injected, so the full route wiring
/// can be tested without a database.
///
/// Layer order on every `/v1` route, outermost first:
/// 1. CORS (on the whole app, so 401/402/429 still carry CORS headers and
///    expose the licence headers to browsers);
/// 2. API-key auth — an anonymous caller gets 401 and learns nothing about
///    the installation's licence;
/// 3. the licence check;
/// 4. (batch creation only) the handler's own per-URL metering.
///
/// With axum's `route_layer`, the layer added *last* is the outermost, hence
/// licence first and auth second below. The licence layer must also stay
/// outside any `apply_resilience` stack: a refused request then takes no
/// concurrency slot, and its 429 is not rewritten into a 503 by
/// `HandleErrorLayer`.
pub fn build_app_with_api_keys(
    state: AppState,
    license: Arc<LicenseGate>,
    api_keys: Arc<dyn ApiKeyValidator>,
) -> Result<Router, CorsConfigError> {
    // CORS origins: comma-separated list from RGAA_API_CORS_ORIGINS, or empty
    // (no cross-origin access) in production. Never `*` — that exposes audit
    // bundles to any site a user visits while authenticated.
    let raw_origins = std::env::var("RGAA_API_CORS_ORIGINS").ok();
    let cors_origins = parse_cors_origins(raw_origins.as_deref())?;

    let cors = CorsLayer::new()
        .allow_methods(Any)
        .allow_headers(Any)
        // `Any` does not cover response headers: without this list a browser
        // client cannot read X-License-* / X-RateLimit-* / Retry-After.
        .expose_headers(rgaa_license::gate::headers::EXPOSED.map(HeaderName::from_static));
    let cors = if !cors_origins.is_empty() {
        cors.allow_origin(cors_origins)
    } else {
        cors
    };

    let license_layer = LicenseLayerState::new(license.clone(), classify_route);

    let public_routes = Router::new()
        .route("/health", get(routes::health))
        .route("/criteria", get(routes::list_criteria));

    // Path captures are `:id`, not `{id}`. This crate is on axum 0.7, where
    // braces are a *literal* path segment — the brace syntax only became
    // captures in 0.8. Every route here used braces and so matched nothing:
    // `GET /v1/audit-bundles/<uuid>` and `GET /audit/<id>` always 404'd.
    //
    // It survived because it reads correctly and because the only test
    // touching it asserted that an unknown id returns 404 — which passed for
    // the wrong reason, since every id returned 404. Do not "modernise" these
    // back to braces without bumping axum first.
    let protected_routes = Router::new()
        .route("/v1/audit-bundles", post(routes::create_audit_bundle))
        .route("/v1/audit-bundles/:id", get(routes::get_audit_bundle))
        .route("/v1/audit-bundles", get(routes::list_audit_bundles))
        .route(
            "/v1/audit-bundles/:id",
            axum::routing::delete(routes::delete_audit_bundle),
        )
        .route("/v1/findings", get(routes::list_findings))
        .route("/v1/policy/evaluate", post(routes::evaluate_policy))
        // Inner: licence. Outer (added last, runs first): auth.
        .route_layer(middleware::from_fn_with_state(
            license_layer.clone(),
            enforce,
        ))
        .route_layer(middleware::from_fn_with_state(
            api_keys.clone(),
            routes::auth_middleware,
        ));

    // Timeout/concurrency-limit/load-shed apply to the legacy audit
    // endpoints only — `/health` must keep answering (liveness stays
    // meaningful even while audits are shed) and `/criteria` is a cheap
    // static lookup.
    //
    // DEPRECATED, and deliberately NOT licence-checked (#87): these routes
    // predate API keys and are unauthenticated, so there is no account to
    // meter against, and gating them now would break existing callers
    // without notice. Use `POST /v1/audit-bundles` / `POST /v1/batches`.
    // Gating or removing them is a separate, announced change.
    let legacy_routes = Router::new()
        .route("/audit", post(routes::run_audit))
        .route("/audit/:id", get(routes::get_audit));
    let legacy_routes =
        apply_resilience(legacy_routes, max_concurrent_audits(), request_timeout()).route_layer(
            middleware::from_fn_with_state(state.storage.clone(), routes::auth_middleware),
        );

    // Same Bearer-key gate as `/v1/audit-bundles`: a batch fans out up to
    // `MAX_BATCH_URLS` full audits from a single POST, which is
    // by far the most expensive thing this server can be asked to do.
    // Deliberately outside `apply_resilience`: creation returns immediately
    // and the audits themselves run under the orchestrator's own
    // concurrency bound, so shedding polls of an accepted batch would only
    // hide progress a client already paid for.
    //
    // Licence: an access check on every batch route, then per-URL metering
    // inside `create_batch`. `restore_handler_license_headers` sits between
    // licence and auth so the handler's post-consumption rate headers win
    // over the layer's pre-consumption ones.
    let batch_routes = batch::batch_router(state.batches.clone().with_license(license))
        .route_layer(middleware::from_fn_with_state(license_layer, enforce))
        .route_layer(middleware::from_fn(batch::restore_handler_license_headers))
        .route_layer(middleware::from_fn_with_state(
            api_keys,
            routes::auth_middleware,
        ));

    Ok(Router::new()
        .merge(public_routes)
        .merge(protected_routes)
        .merge(legacy_routes)
        // `with_state` before merging the batch routes, not after: those
        // carry their own `BatchService` state and are already
        // `Router<()>`, so merging them into a router still generic over
        // `AppState` does not typecheck. CORS goes on last so it covers
        // every route rather than only the ones declared above it.
        .with_state(state)
        .merge(batch_routes)
        .layer(cors))
}

#[cfg(test)]
mod cors_tests {
    use super::{parse_cors_origins, CorsConfigError};

    #[test]
    fn absent_or_empty_means_no_origins() {
        assert_eq!(parse_cors_origins(None), Ok(Vec::new()));
        assert_eq!(parse_cors_origins(Some("  , ")), Ok(Vec::new()));
    }

    #[test]
    fn explicit_origins_are_accepted() {
        let origins = parse_cors_origins(Some("https://app.example.org, http://localhost:5173"))
            .expect("valid origins");
        assert_eq!(origins.len(), 2);
    }

    /// Browsers send IPv6 origins bracketed (`http://[::1]:3000`); the value is
    /// kept byte-for-byte, so it matches the `Origin` header exactly.
    #[test]
    fn ipv6_literal_origins_are_accepted_verbatim() {
        let raw = "http://[::1]:3000,https://[2001:db8::1]:8443";
        let origins = parse_cors_origins(Some(raw)).expect("valid IPv6 origins");
        let got: Vec<&str> = origins.iter().map(|o| o.to_str().unwrap()).collect();
        assert_eq!(got, ["http://[::1]:3000", "https://[2001:db8::1]:8443"]);
    }

    #[test]
    fn wildcard_is_rejected_not_emitted() {
        assert_eq!(
            parse_cors_origins(Some("*")),
            Err(CorsConfigError::Wildcard)
        );
        assert_eq!(
            parse_cors_origins(Some("https://a.example.org,*")),
            Err(CorsConfigError::Wildcard)
        );
    }

    #[test]
    fn malformed_entries_are_rejected_not_dropped() {
        for bad in [
            "app.example.org",
            "https://",
            "https://a.example.org/path",
            "https://*.example.org",
        ] {
            assert_eq!(
                parse_cors_origins(Some(bad)),
                Err(CorsConfigError::Invalid(bad.to_string())),
                "{bad} must be a config error"
            );
        }
    }
}
