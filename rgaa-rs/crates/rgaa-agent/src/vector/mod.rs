pub mod schema;

use lancedb::arrow::arrow_schema::SchemaRef;
use lancedb::database::CreateTableMode;
use lancedb::Connection;

/// Vector store for RGAA criteria, findings, and remediation patterns, backed
/// by LanceDB. Tables are created idempotently on construction.
pub struct LanceDbVectorStore {
    db: Connection,
}

impl LanceDbVectorStore {
    /// Opens a LanceDB connection and ensures the criteria, findings, and
    /// remediation-pattern tables exist with the expected schema.
    ///
    /// # Errors
    /// Returns [`crate::error::AgentError::LanceDb`] if the database cannot be
    /// opened or if table creation/validation fails.
    pub async fn new(path: &str) -> Result<Self, crate::error::AgentError> {
        let db = lancedb::connect(path)
            .execute()
            .await
            .map_err(|e| crate::error::AgentError::LanceDb(e.to_string()))?;

        let store = Self { db };
        store.initialize_tables().await?;
        Ok(store)
    }

    /// Creates the criteria, findings, and remediation-pattern tables if absent.
    ///
    /// # Errors
    /// Returns [`crate::error::AgentError::LanceDb`] if table creation or schema
    /// validation fails.
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

/// Creates a table if absent, then validates its schema against `expected`.
async fn create_table(
    db: &Connection,
    name: &str,
    expected: SchemaRef,
) -> Result<(), crate::error::AgentError> {
    db.create_empty_table(name, expected.clone())
        .mode(CreateTableMode::exist_ok(|req| req))
        .execute()
        .await
        .map_err(|e| crate::error::AgentError::LanceDb(e.to_string()))?;

    // Open and verify schema matches expected
    let table = db
        .open_table(name)
        .execute()
        .await
        .map_err(|e| crate::error::AgentError::LanceDb(e.to_string()))?;
    let actual = table
        .schema()
        .await
        .map_err(|e| crate::error::AgentError::LanceDb(e.to_string()))?;

    if !schemas_match(&actual, &expected) {
        return Err(crate::error::AgentError::LanceDb(format!(
            "schema mismatch for table '{}': expected {:?}, got {:?}",
            name, expected, actual
        )));
    }
    Ok(())
}

/// Returns true if two schemas are equal (ignoring metadata differences).
fn schemas_match(a: &SchemaRef, b: &SchemaRef) -> bool {
    a.fields().len() == b.fields().len()
        && a.fields().iter().zip(b.fields().iter()).all(|(fa, fb)| {
            fa.name() == fb.name()
                && fa.data_type() == fb.data_type()
                && fa.is_nullable() == fb.is_nullable()
        })
}
