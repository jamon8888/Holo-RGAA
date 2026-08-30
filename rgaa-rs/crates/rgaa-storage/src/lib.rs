pub mod postgres;
pub mod repository;

pub use repository::{hash_api_key, AuditRow, CriterionResultRow, Repository};
pub use postgres::PostgresStorage;

use async_trait::async_trait;
use rgaa_core::AuditResult;
use serde_json::Value;

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("not found: {0}")]
    NotFound(String),
}

#[async_trait]
pub trait Storage: Send + Sync {
    async fn save_audit(&self, audit: &AuditResult) -> Result<String, StorageError>;
    async fn get_audit(&self, id: &str) -> Result<Option<AuditResult>, StorageError>;
    async fn list_audits(&self, limit: usize, offset: usize) -> Result<Vec<AuditSummary>, StorageError>;
    async fn delete_audit(&self, id: &str) -> Result<(), StorageError>;
    async fn save_audit_log(&self, audit_id: &str, action: &str, details: Option<Value>) -> Result<String, StorageError>;
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AuditSummary {
    pub id: String,
    pub url: String,
    pub taux_global: f64,
    pub etat_conformite: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::PgPool;

    #[tokio::test]
    #[ignore = "requires database"]
    async fn test_create_audit() {
        let pool = PgPool::connect("postgres://localhost/rgaa").await.unwrap();
        let repo = Repository::new(&pool);
        let id = repo.create_audit("https://example.test").await.unwrap();
        assert!(!id.is_nil());
    }
}
