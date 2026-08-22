pub mod schema;

use lancedb::arrow::arrow_schema::SchemaRef;
use lancedb::database::CreateTableMode;
use lancedb::Connection;
use std::sync::Arc;

use crate::embeddings::HybridEmbeddingProvider;

/// Vector store for RGAA criteria, findings, and remediation patterns, backed
/// by LanceDB. Tables are created idempotently on construction.
pub struct LanceDbVectorStore {
    db: Connection,
    embedding_model: Arc<HybridEmbeddingProvider>,
}

impl LanceDbVectorStore {
    pub async fn new(
        path: &str,
        embedding_model: HybridEmbeddingProvider,
    ) -> Result<Self, crate::error::AgentError> {
        let db = lancedb::connect(path)
            .execute()
            .await
            .map_err(|e| crate::error::AgentError::LanceDb(e.to_string()))?;

        let store = Self {
            db,
            embedding_model: Arc::new(embedding_model),
        };
        store.initialize_tables().await?;
        Ok(store)
    }

    /// Creates the criteria, findings, and remediation-pattern tables if absent.
    pub async fn initialize_tables(&self) -> Result<(), crate::error::AgentError> {
        create_table(&self.db, "rgaa_criteria", schema::rgaa_criteria_schema()).await?;
        create_table(&self.db, "rgaa_findings", schema::rgaa_findings_schema()).await?;
        create_table(
            &self.db,
            "rgaa_remediation_patterns",
            schema::rgaa_remediation_patterns_schema(),
        )
        .await?;
        Ok(())
    }
}

/// Creates a table only if it does not already exist.
async fn create_table(
    db: &Connection,
    name: &str,
    schema: SchemaRef,
) -> Result<(), crate::error::AgentError> {
        db.create_empty_table(name, schema)
            .mode(CreateTableMode::exist_ok(|req| req))
            .execute()
        .await
        .map_err(|e| crate::error::AgentError::LanceDb(e.to_string()))?;
    Ok(())
}
