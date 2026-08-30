pub mod routes;

use axum::{
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tracing::Level;
use tracing_subscriber::FmtSubscriber;

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

    Router::new()
        .route("/audit", post(routes::run_audit))
        .route("/audit/{id}", get(routes::get_audit))
        .route("/criteria", get(routes::list_criteria))
        .route("/health", get(routes::health))
        .layer(cors)
        .with_state(state)
}

pub fn init_tracing() {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    let _ = tracing::subscriber::set_global_default(subscriber);
}
