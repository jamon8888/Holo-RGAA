//! Read-only access to the remediation-pattern index during an audit.
//!
//! [`PatternReader::open`] pins the table to its version at open time (see
//! `lancedb::Table::checkout`), so the whole audit sees one fixed snapshot
//! regardless of any offline batch update landing concurrently — the spec's
//! "Index des patterns de remédiation : lecture seule pendant l'évaluation,
//! mises à jour par lots hors-ligne et revus, hors chemin chaud."

use super::util::{
    collect_batches, escape_literal, f32_column, i32_column, lancedb_err, nullable_utf8,
    utf8_column,
};
use crate::error::AgentError;
use crate::vector::schema::REMEDIATION_PATTERNS_TABLE;
use crate::vector::LanceDbVectorStore;
use lancedb::query::QueryBase;
use lancedb::{DistanceType, Table};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct RemediationPattern {
    pub id: String,
    pub rule: String,
    pub framework: String,
    pub before_html: Option<String>,
    pub after_html: Option<String>,
    pub description: Option<String>,
    pub success_count: i32,
    /// Cosine similarity to the query (`1.0 - distance`), higher is better.
    pub score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PatternQueryOutput {
    pub patterns: Vec<RemediationPattern>,
}

/// Read-only, version-pinned handle onto the remediation-pattern index.
/// Frozen at [`Self::open`] time — has no insert/update method at all, and
/// is checked out to a fixed table version so it stays frozen even if
/// another process writes to the table while this reader is in use.
pub struct PatternReader {
    table: Table,
}

impl PatternReader {
    /// Opens the remediation-pattern table and pins it to its version at
    /// this moment. Build one `PatternReader` per audit (not per
    /// criterion) so every criterion evaluated sees the identical pattern
    /// set, even if a batch update runs concurrently elsewhere.
    ///
    /// # Errors
    /// Returns [`AgentError::LanceDb`] on any LanceDB failure.
    pub async fn open(store: &LanceDbVectorStore) -> Result<Self, AgentError> {
        let table = store
            .connection()
            .open_table(REMEDIATION_PATTERNS_TABLE)
            .execute()
            .await
            .map_err(lancedb_err)?;
        let version = table.version().await.map_err(lancedb_err)?;
        table.checkout(version).await.map_err(lancedb_err)?;
        Ok(Self { table })
    }

    /// Vector search for patterns matching `rule`, optionally narrowed to
    /// one `framework`. Returns an empty result (not an error) when the
    /// index has no rows or nothing matches.
    ///
    /// # Errors
    /// Returns [`AgentError::LanceDb`] on any LanceDB failure.
    pub async fn query(
        &self,
        embedding: &[f32],
        k: usize,
        framework: Option<&str>,
    ) -> Result<PatternQueryOutput, AgentError> {
        if self.table.count_rows(None).await.map_err(lancedb_err)? == 0 {
            return Ok(PatternQueryOutput { patterns: vec![] });
        }

        let mut query = self
            .table
            .vector_search(embedding)
            .map_err(lancedb_err)?
            .distance_type(DistanceType::Cosine)
            .limit(k);
        if let Some(framework) = framework {
            query = query.only_if(format!("framework = '{}'", escape_literal(framework)));
        }
        let batches = collect_batches(query).await?;

        let mut patterns = Vec::new();
        for batch in &batches {
            let ids = utf8_column(batch, "id")?;
            let rules = utf8_column(batch, "rule")?;
            let frameworks = utf8_column(batch, "framework")?;
            let before = utf8_column(batch, "before_html")?;
            let after = utf8_column(batch, "after_html")?;
            let descriptions = utf8_column(batch, "description")?;
            let success_counts = i32_column(batch, "success_count")?;
            let distances = f32_column(batch, "_distance")?;
            for i in 0..batch.num_rows() {
                patterns.push(RemediationPattern {
                    id: ids.value(i).to_string(),
                    rule: rules.value(i).to_string(),
                    framework: frameworks.value(i).to_string(),
                    before_html: nullable_utf8(before, i),
                    after_html: nullable_utf8(after, i),
                    description: nullable_utf8(descriptions, i),
                    success_count: success_counts.value(i),
                    score: 1.0 - distances.value(i),
                });
            }
        }
        Ok(PatternQueryOutput { patterns })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vector::LanceDbVectorStore;
    use arrow::array::{Int32Array, RecordBatch, StringArray};
    use std::sync::Arc;
    use tempfile::TempDir;

    fn fake_embedding(seed: f32) -> Vec<f32> {
        vec![seed; crate::vector::schema::EMBEDDING_DIM]
    }

    async fn insert_pattern(store: &LanceDbVectorStore, id: &str, framework: &str, seed: f32) {
        use arrow::array::types::Float32Type;
        use arrow::array::FixedSizeListArray;

        let schema = crate::vector::schema::rgaa_remediation_patterns_schema();
        let embedding = fake_embedding(seed);
        let batch = RecordBatch::try_new(
            schema,
            vec![
                Arc::new(StringArray::from(vec![id])),
                Arc::new(StringArray::from(vec!["image-alt"])),
                Arc::new(StringArray::from(vec![framework])),
                Arc::new(StringArray::from(vec![Some("<img>")])),
                Arc::new(StringArray::from(vec![Some("<img alt=\"\">")])),
                Arc::new(StringArray::from(vec![Some("add alt")])),
                Arc::new(Int32Array::from(vec![3])),
                Arc::new(
                    FixedSizeListArray::from_iter_primitive::<Float32Type, _, _>(
                        vec![Some(embedding.into_iter().map(Some))],
                        crate::vector::schema::EMBEDDING_DIM as i32,
                    ),
                ),
            ],
        )
        .unwrap();

        let table = store
            .connection()
            .open_table(REMEDIATION_PATTERNS_TABLE)
            .execute()
            .await
            .unwrap();
        table.add(batch).execute().await.unwrap();
    }

    #[tokio::test]
    async fn query_returns_patterns_with_scores() {
        let dir = TempDir::new().unwrap();
        let store = LanceDbVectorStore::new(dir.path().to_str().unwrap())
            .await
            .unwrap();
        insert_pattern(&store, "p1", "react", 0.5).await;

        let reader = PatternReader::open(&store).await.unwrap();
        let output = reader.query(&fake_embedding(0.5), 5, None).await.unwrap();
        assert_eq!(output.patterns.len(), 1);
        assert_eq!(output.patterns[0].id, "p1");
        assert_eq!(output.patterns[0].framework, "react");
    }

    #[tokio::test]
    async fn empty_index_returns_empty_result_not_an_error() {
        let dir = TempDir::new().unwrap();
        let store = LanceDbVectorStore::new(dir.path().to_str().unwrap())
            .await
            .unwrap();
        let reader = PatternReader::open(&store).await.unwrap();
        let output = reader.query(&fake_embedding(0.1), 5, None).await.unwrap();
        assert!(output.patterns.is_empty());
    }

    #[tokio::test]
    async fn reader_stays_frozen_after_a_concurrent_insert() {
        let dir = TempDir::new().unwrap();
        let store = LanceDbVectorStore::new(dir.path().to_str().unwrap())
            .await
            .unwrap();
        insert_pattern(&store, "p1", "react", 0.5).await;

        // Reader opened (and pinned) with only "p1" present.
        let reader = PatternReader::open(&store).await.unwrap();

        // An offline batch job inserts a second pattern after the reader
        // was opened — simulating a concurrent update outside the hot path.
        insert_pattern(&store, "p2", "vue", 0.9).await;

        // The already-open reader still only sees "p1": patterns don't
        // change mid-audit, per #129's AC.
        let output = reader.query(&fake_embedding(0.5), 10, None).await.unwrap();
        assert_eq!(output.patterns.len(), 1);
        assert_eq!(output.patterns[0].id, "p1");

        // A freshly opened reader does see both — confirms the freeze is a
        // property of the pinned reader, not of the underlying table.
        let fresh_reader = PatternReader::open(&store).await.unwrap();
        let fresh_output = fresh_reader
            .query(&fake_embedding(0.5), 10, None)
            .await
            .unwrap();
        assert_eq!(fresh_output.patterns.len(), 2);
    }
}
