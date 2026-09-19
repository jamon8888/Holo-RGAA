pub mod routes;

use axum::{
    middleware,
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};

use rgaa_orchestrator::Orchestrator;
use rgaa_storage::Storage;

#[derive(Clone)]
pub struct AppState {
    pub orchestrator: Arc<Orchestrator>,
    pub storage: Arc<dyn Storage>,
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

    let legacy_routes = Router::new()
        .route("/audit", post(routes::run_audit))
        .route("/audit/{id}", get(routes::get_audit));

    Router::new()
        .merge(public_routes)
        .merge(protected_routes)
        .merge(legacy_routes)
        .layer(cors)
        .with_state(state)
}
