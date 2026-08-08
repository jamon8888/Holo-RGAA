use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tower_http::cors::{Any, CorsLayer};
use uuid::Uuid;

use rgaa_storage::Repository;

#[derive(Clone)]
struct AppState {
    repository: Repository,
}

#[derive(Deserialize)]
struct CreateAuditRequest {
    url: String,
}

#[derive(Serialize)]
struct CreateAuditResponse {
    id: Uuid,
}

#[derive(Deserialize)]
struct ListAuditsQuery {
    limit: Option<i64>,
    offset: Option<i64>,
}

#[derive(Serialize)]
struct AuditListItem {
    id: Uuid,
    url: String,
    status: String,
    created_at: String,
}

#[derive(Serialize)]
struct HealthResponse {
    status: String,
}

async fn health_check() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
    })
}

async fn create_audit(
    State(state): State<AppState>,
    Json(payload): Json<CreateAuditRequest>,
) -> Result<(StatusCode, Json<CreateAuditResponse>), (StatusCode, String)> {
    let id = state
        .repository
        .create_audit(&payload.url)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok((StatusCode::CREATED, Json(CreateAuditResponse { id })))
}

async fn get_audit(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let result = state
        .repository
        .get_audit(id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    match result {
        Some(data) => Ok(Json(data)),
        None => Err((StatusCode::NOT_FOUND, "Audit not found".to_string())),
    }
}

async fn list_audits(
    State(state): State<AppState>,
    Query(query): Query<ListAuditsQuery>,
) -> Result<Json<Vec<AuditListItem>>, (StatusCode, String)> {
    let limit = query.limit.unwrap_or(20);
    let offset = query.offset.unwrap_or(0);
    let rows = state
        .repository
        .list_audits(limit, offset)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let items: Vec<AuditListItem> = rows
        .into_iter()
        .map(|row| AuditListItem {
            id: row.id,
            url: row.url,
            status: row.status,
            created_at: row.created_at.to_rfc3339(),
        })
        .collect();
    Ok(Json(items))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://localhost/rgaa".to_string());

    let pool = PgPool::connect(&database_url).await?;

    let repository = Repository::new(&pool);
    let state = AppState { repository };

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/health", get(health_check))
        .route("/audits", post(create_audit))
        .route("/audits", get(list_audits))
        .route("/audits/{id}", get(get_audit))
        .layer(cors)
        .with_state(state);

    let addr = std::env::var("LISTEN_ADDR")
        .unwrap_or_else(|_| "0.0.0.0:3000".to_string());
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("Listening on {}", addr);
    axum::serve(listener, app).await?;

    Ok(())
}
