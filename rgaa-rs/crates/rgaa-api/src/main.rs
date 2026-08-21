use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tower_http::cors::{Any, CorsLayer};
use uuid::Uuid;

use rgaa_core::AuditBundle;
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

#[derive(Deserialize)]
struct BundleQuery {
    audit_id: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
}

#[derive(Serialize)]
struct BundleListItem {
    audit_id: String,
    url: String,
    status: String,
    schema_version: String,
    created_at: String,
}

#[derive(Deserialize)]
struct PolicyEvalRequest {
    audit_id: String,
    baseline_id: Option<String>,
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

async fn upload_bundle(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(bundle): Json<rgaa_core::AuditBundle>,
) -> Result<StatusCode, (StatusCode, String)> {
    let api_key = get_api_key(&headers)?;

    state
        .repository
        .validate_api_key(&api_key, "bundle:write")
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((
            StatusCode::UNAUTHORIZED,
            "Invalid or expired API key".to_string(),
        ))?;

    if bundle.schema_version != "1.0" {
        return Err((
            StatusCode::BAD_REQUEST,
            "Unsupported schema version".to_string(),
        ));
    }
    bundle
        .validate()
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    state
        .repository
        .put_bundle(&bundle)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::CREATED)
}

async fn get_bundle(
    headers: HeaderMap,
    State(state): State<AppState>,
    Query(query): Query<BundleQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let api_key = get_api_key(&headers)?;

    state
        .repository
        .validate_api_key(&api_key, "bundle:read")
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((
            StatusCode::UNAUTHORIZED,
            "Invalid or expired API key".to_string(),
        ))?;

    if let Some(audit_id_str) = query.audit_id {
        let result = state
            .repository
            .get_bundle_by_audit_id(&audit_id_str)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        match result {
            Some(data) => Ok(Json(data)),
            None => Err((StatusCode::NOT_FOUND, "Bundle not found".to_string())),
        }
    } else {
        let limit = query.limit.unwrap_or(20);
        let offset = query.offset.unwrap_or(0);
        let rows = state
            .repository
            .list_audits(limit, offset)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        let items: Vec<BundleListItem> = rows
            .into_iter()
            .map(|row| BundleListItem {
                audit_id: row.id.to_string(),
                url: row.url,
                status: row.status,
                schema_version: "1.0".to_string(),
                created_at: row.created_at.to_rfc3339(),
            })
            .collect();
        Ok(Json(serde_json::json!(items)))
    }
}

async fn list_findings(
    headers: HeaderMap,
    State(state): State<AppState>,
    Query(query): Query<BundleQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let api_key = get_api_key(&headers)?;

    state
        .repository
        .validate_api_key(&api_key, "bundle:read")
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((
            StatusCode::UNAUTHORIZED,
            "Invalid or expired API key".to_string(),
        ))?;

    let audit_id_str = query
        .audit_id
        .ok_or((StatusCode::BAD_REQUEST, "audit_id required".to_string()))?;
    let audit_id = Uuid::parse_str(&audit_id_str)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid audit_id".to_string()))?;

    let limit = query.limit.unwrap_or(50);
    let offset = query.offset.unwrap_or(0);

    let rows = state
        .repository
        .list_findings(audit_id, None, None, None, limit, offset)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(serde_json::to_value(rows).unwrap()))
}

async fn evaluate_policy(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(payload): Json<PolicyEvalRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let api_key = get_api_key(&headers)?;

    state
        .repository
        .validate_api_key(&api_key, "policy:evaluate")
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((
            StatusCode::UNAUTHORIZED,
            "Invalid or expired API key".to_string(),
        ))?;

    let audit_id = Uuid::parse_str(&payload.audit_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid audit_id".to_string()))?;
    let baseline_id = payload
        .baseline_id
        .as_ref()
        .map(|s| Uuid::parse_str(s))
        .transpose()
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid baseline_id".to_string()))?;

    let bundle_json = state
        .repository
        .get_bundle(audit_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "Audit not found".to_string()))?;

    let bundle: AuditBundle = serde_json::from_value(bundle_json)
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    let baseline_bundle = if let Some(bid) = baseline_id {
        let bjson = state
            .repository
            .get_bundle(bid)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
            .ok_or((StatusCode::NOT_FOUND, "Baseline not found".to_string()))?;
        Some(
            serde_json::from_value::<AuditBundle>(bjson)
                .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?,
        )
    } else {
        None
    };

    let policy = rgaa_remediation::RemediationPolicy::default();
    let result = policy.evaluate(&bundle, baseline_bundle.as_ref());

    state
        .repository
        .store_policy_evaluation(&result, audit_id, baseline_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(serde_json::to_value(result).unwrap()))
}

fn get_api_key(headers: &HeaderMap) -> Result<String, (StatusCode, String)> {
    headers
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .map(|s| s.to_string())
        .ok_or((
            StatusCode::UNAUTHORIZED,
            "Missing or invalid Authorization header".to_string(),
        ))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    let database_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| "postgres://localhost/rgaa".to_string());

    let pool = PgPool::connect(&database_url).await?;

    let repository = Repository::new(&pool);
    let state = AppState { repository };

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/health", get(health_check))
        .route("/audits", post(create_audit).get(list_audits))
        .route("/audits/{id}", get(get_audit))
        .route("/v1/audit-bundles", post(upload_bundle).get(get_bundle))
        .route("/v1/findings", get(list_findings))
        .route("/v1/policy/evaluate", post(evaluate_policy))
        .layer(cors)
        .with_state(state);

    let addr = std::env::var("LISTEN_ADDR").unwrap_or_else(|_| "0.0.0.0:3000".to_string());
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("Listening on {}", addr);
    axum::serve(listener, app).await?;

    Ok(())
}
