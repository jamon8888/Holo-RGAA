pub mod schema;

use lancedb::Connection;
use std::sync::Arc;

use crate::embeddings::HybridEmbeddingProvider;

pub struct LanceDbVectorStore {
    db: Connection,
    embedding_model: Arc<HybridEmbeddingProvider>,
}

impl LanceDbVectorStore {
    pub async fn new(path: &str, embedding_model: HybridEmbeddingProvider) -> Result<Self, crate::error::AgentError> {
        let db = lancedb::connect(path)
            .execute()
            .await
            .map_err(|e| crate::error::AgentError::LanceDb(e.to_string()))?;

        Ok(Self {
            db,
            embedding_model: Arc::new(embedding_model),
        })
    }

    pub async fn initialize_tables(path: &str) -> Result<(), crate::error::AgentError> {
        let db = lancedb::connect(path)
            .execute()
            .await
            .map_err(|e| crate::error::AgentError::LanceDb(e.to_string()))?;

        let table_names = db.table_names()
            .execute()
            .await
            .map_err(|e| crate::error::AgentError::LanceDb(e.to_string()))?;

        if !table_names.contains(&"rgaa_criteria".to_string()) {
            db.create_empty_table("rgaa_criteria", schema::rgaa_criteria_schema())
                .execute()
                .await
                .map_err(|e| crate::error::AgentError::LanceDb(e.to_string()))?;
        }

        if !table_names.contains(&"rgaa_findings".to_string()) {
            db.create_empty_table("rgaa_findings", schema::rgaa_findings_schema())
                .execute()
                .await
                .map_err(|e| crate::error::AgentError::LanceDb(e.to_string()))?;
        }

        if !table_names.contains(&"rgaa_remediation_patterns".to_string()) {
            db.create_empty_table("rgaa_remediation_patterns", schema::rgaa_remediation_patterns_schema())
                .execute()
                .await
                .map_err(|e| crate::error::AgentError::LanceDb(e.to_string()))?;
        }

        Ok(())
    }
}