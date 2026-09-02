use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditSummary {
    pub id: String,
    pub url: String,
    pub taux_global: f64,
    pub etat_conformite: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

pub struct Storage;

impl Storage {
    pub fn new(_db_path: &std::path::Path) -> anyhow::Result<Self> {
        todo!()
    }

    pub fn save_audit(&self, _audit: &rgaa_core::AuditResult) -> anyhow::Result<String> {
        todo!()
    }

    pub fn list_audits(&self, _limit: usize, _offset: usize) -> anyhow::Result<Vec<AuditSummary>> {
        todo!()
    }
}
