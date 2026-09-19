use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use rgaa_core::{AuditBundle, AuditResult, CrawlConfig, RgaaCriteria};
use rgaa_remediation::RemediationPolicy;
use rgaa_storage::Repository;

use crate::AppState;

#[derive(Deserialize)]
pub struct AuditRequest {
    pub url: String,
}

#[derive(Serialize)]
pub struct AuditResponse {
    pub audit_id: String,
    pub url: String,
    pub taux_global: f64,
    pub coverage_percent: f64,
    pub etat_conformite: String,
    pub passed: usize,
    pub failed: usize,
    pub na: usize,
}

impl From<AuditResult> for AuditResponse {
    fn from(result: AuditResult) -> Self {
        Self {
            audit_id: result.audit_id,
            url: result.url,
            taux_global: result.taux_global,
            coverage_percent: result.coverage_percent,
            etat_conformite: result.etat_conformite,
            passed: result.passed,
            failed: result.failed,
            na: result.na,
        }
    }
}

#[derive(Serialize)]
pub struct CriteriaResponse {
    pub id: String,
    pub title: String,
    pub classification: String,
}

pub async fn run_audit(
    State(state): State<AppState>,
    Json(payload): Json<AuditRequest>,
) -> Result<Json<AuditResponse>, StatusCode> {
    let config = CrawlConfig::default();
    let result = state
        .orchestrator
        .run_crawl_and_audit(&payload.url, &config)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if let Some(storage) = state
        .storage
        .get_audit(&result.audit_id)
        .await
        .ok()
        .flatten()
    {
        return Ok(Json(AuditResponse::from(storage)));
    }

    Ok(Json(AuditResponse::from(result)))
}

pub async fn get_audit(
    State(state): State<AppState>,
    Path(audit_id): Path<String>,
) -> Result<Json<AuditResponse>, StatusCode> {
    state
        .storage
        .get_audit(&audit_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .map(|r| Json(AuditResponse::from(r)))
        .ok_or(StatusCode::NOT_FOUND)
}

pub async fn list_criteria() -> Json<Vec<CriteriaResponse>> {
    let criteria = RgaaCriteria::all()
        .iter()
        .map(|c| CriteriaResponse {
            id: c.id.to_string(),
            title: c.title.to_string(),
            classification: format!("{:?}", c.classification),
        })
        .collect();
    Json(criteria)
}

pub async fn health() -> &'static str {
    "OK"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audit_response_from_audit_result() {
        let result = AuditResult {
            audit_id: "test-id".to_string(),
            url: "https://example.com".to_string(),
            pages: vec![],
            total_criteria: 106,
            passed: 50,
            failed: 10,
            na: 46,
            overall_compliance: 83.33,
            taux_global: 83.33,
            coverage_percent: 56.6,
            etat_conformite: "partielle".to_string(),
            duration_ms: 1000,
        };

        let response = AuditResponse::from(result.clone());
        assert_eq!(response.audit_id, result.audit_id);
        assert_eq!(response.url, result.url);
        assert_eq!(response.taux_global, result.taux_global);
        assert_eq!(response.passed, result.passed);
        assert_eq!(response.failed, result.failed);
        assert_eq!(response.na, result.na);
    }
}

// Bundle request/response types
#[derive(Deserialize)]
pub struct CreateBundleRequest {
    pub bundle: AuditBundle,
}

#[derive(Serialize)]
pub struct BundleResponse {
    pub audit_id: String,
    pub schema_version: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Deserialize)]
pub struct ListBundlesQuery {
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

#[derive(Serialize)]
pub struct ListBundlesResponse {
    pub bundles: Vec<BundleSummary>,
}

#[derive(Serialize)]
pub struct BundleSummary {
    pub audit_id: String,
    pub url: String,
    pub schema_version: String,
    pub status: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Deserialize)]
pub struct ListFindingsQuery {
    pub audit_id: String,
    pub status: Option<String>,
    pub severity: Option<String>,
    pub rule: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

#[derive(Serialize)]
pub struct FindingsResponse {
    pub findings: Vec<rgaa_storage::FindingRow>,
}

#[derive(Deserialize)]
pub struct PolicyEvaluateRequest {
    pub bundle: AuditBundle,
    pub baseline_audit_id: Option<String>,
}

#[derive(Serialize)]
pub struct PolicyEvaluateResponse {
    pub passed: bool,
    pub failures: Vec<rgaa_remediation::PolicyFailure>,
    pub warnings: Vec<rgaa_remediation::PolicyWarning>,
    pub counts: rgaa_remediation::PolicyCounts,
}

// Authentication middleware
pub async fn auth_middleware(
    State(storage): State<Arc<dyn rgaa_storage::Storage>>,
    headers: HeaderMap,
    request: axum::http::Request<axum::body::Body>,
    next: axum::middleware::Next,
) -> Result<axum::response::Response, StatusCode> {
    let api_key = headers
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "));

    if let Some(key) = api_key {
        let repo = Repository::new(storage.pool());
        if let Ok(Some(_key_row)) = repo.validate_api_key(key, "audit:write").await {
            return Ok(next.run(request).await);
        }
    }

    Err(StatusCode::UNAUTHORIZED)
}

// Bundle handlers
pub async fn create_audit_bundle(
    State(state): State<AppState>,
    Json(payload): Json<CreateBundleRequest>,
) -> Result<Json<BundleResponse>, StatusCode> {
    state
        .storage
        .put_bundle(&payload.bundle)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(BundleResponse {
        audit_id: payload.bundle.audit_id.clone(),
        schema_version: payload.bundle.schema_version.clone(),
        created_at: chrono::Utc::now(),
    }))
}

pub async fn get_audit_bundle(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<AuditBundle>, StatusCode> {
    state
        .storage
        .get_bundle_by_audit_id(&id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

pub async fn list_audit_bundles(
    State(state): State<AppState>,
    Query(query): Query<ListBundlesQuery>,
) -> Result<Json<ListBundlesResponse>, StatusCode> {
    let limit = query.limit.unwrap_or(50);
    let offset = query.offset.unwrap_or(0);

    let rows = state
        .storage
        .list_audits(limit, offset)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let bundles = rows
        .into_iter()
        .map(|row| BundleSummary {
            audit_id: row.id.to_string(),
            url: row.url,
            schema_version: "1.0".to_string(),
            status: if row.taux_global >= 100.0 {
                "passed"
            } else if row.taux_global >= 50.0 {
                "needs_review"
            } else {
                "failed"
            }
            .to_string(),
            created_at: row.created_at,
        })
        .collect();

    Ok(Json(ListBundlesResponse { bundles }))
}

pub async fn delete_audit_bundle(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    let uuid = uuid::Uuid::parse_str(&id).map_err(|_| StatusCode::BAD_REQUEST)?;
    state
        .storage
        .delete_audit(&uuid.to_string())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn list_findings(
    State(state): State<AppState>,
    Query(query): Query<ListFindingsQuery>,
) -> Result<Json<FindingsResponse>, StatusCode> {
    let audit_id = uuid::Uuid::parse_str(&query.audit_id).map_err(|_| StatusCode::BAD_REQUEST)?;
    let limit = query.limit.unwrap_or(100) as i64;
    let offset = query.offset.unwrap_or(0) as i64;

    let repo = Repository::new(state.storage.pool());
    let findings = repo
        .list_findings(
            audit_id,
            query.status.as_deref(),
            query.severity.as_deref(),
            query.rule.as_deref(),
            limit,
            offset,
        )
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(FindingsResponse { findings }))
}

pub async fn evaluate_policy(
    State(state): State<AppState>,
    Json(payload): Json<PolicyEvaluateRequest>,
) -> Result<Json<PolicyEvaluateResponse>, StatusCode> {
    let policy = RemediationPolicy::default();
    let baseline = if let Some(baseline_id) = payload.baseline_audit_id {
        let uuid = uuid::Uuid::parse_str(&baseline_id).ok();
        if let Some(id) = uuid {
            state
                .storage
                .get_bundle_by_audit_id(&id.to_string())
                .await
                .ok()
                .flatten()
        } else {
            None
        }
    } else {
        None
    };

    let result = policy.evaluate(&payload.bundle, baseline.as_ref());
    Ok(Json(PolicyEvaluateResponse {
        passed: result.passed,
        failures: result.failures,
        warnings: result.warnings,
        counts: result.counts,
    }))
}
