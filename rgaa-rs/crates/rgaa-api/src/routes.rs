use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};

use rgaa_core::{AuditResult, CrawlConfig, RgaaCriteria};

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
        .run(&payload.url, &config)
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
