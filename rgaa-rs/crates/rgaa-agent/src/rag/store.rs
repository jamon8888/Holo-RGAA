//! Dual-index RAG store: the versioned regulatory corpus
//! ([`schema::RAG_REFERENTIEL_TABLE`]) and the per-audit crawl index
//! ([`schema::RAG_CRAWL_TABLE`]), both backed by LanceDB.
//!
//! [`RagStore`] is the seeding/write-side handle used by the offline
//! referentiel seeding pipeline (#128) and the per-audit crawl
//! writer/purger (#129). [`RagReader`], obtained via
//! [`RagStore::reader`], exposes only the query methods — it has no insert
//! or delete method at all, so a caller holding a `RagReader` (as every
//! [`super::tools`] `PortableTool` does) cannot write to either index
//! regardless of what a model asks it to do.

use super::schema::{self, RAG_CRAWL_TABLE, RAG_REFERENTIEL_TABLE};
use crate::error::AgentError;
use arrow::array::{Float32Array, Int64Array, RecordBatch, StringArray};
use lancedb::arrow::arrow_schema::SchemaRef;
use lancedb::database::CreateTableMode;
use lancedb::index::vector::IvfFlatIndexBuilder;
use lancedb::index::Index;
use lancedb::query::{ExecutableQuery, QueryBase};
use lancedb::{Connection, DistanceType, Table};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Default number of documents a query returns absent an explicit `k`.
pub const DEFAULT_K: usize = 5;

/// Retrieval statistics a router (#126) can act on without parsing text.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct RagStats {
    /// Number of documents returned (after any `limit`/`k` cap).
    pub count: usize,
    /// Highest similarity among the returned documents (cosine similarity,
    /// `1.0 - distance`; higher is better). `None` when `count` is 0 — an
    /// empty index or a query that matched nothing, not an error.
    pub best_score: Option<f32>,
}

impl RagStats {
    fn from_scores(scores: &[f32]) -> Self {
        Self {
            count: scores.len(),
            best_score: scores.iter().copied().max_by(|a, b| a.total_cmp(b)),
        }
    }
}

/// One retrieved document from the regulatory-corpus index, carrying enough
/// to build a [`rgaa_core::Citation::Referentiel`].
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct ReferentielDocument {
    pub test_id: String,
    pub referentiel_version: String,
    pub content: String,
    /// Cosine similarity to the query (`1.0 - distance`), higher is better.
    pub score: f32,
}

/// One retrieved document from the crawl index, carrying enough to build a
/// [`rgaa_core::Citation::Crawl`].
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct CrawlDocument {
    pub url: String,
    pub captured_at: String,
    pub evidence_hash: String,
    pub content: String,
    /// Cosine similarity to the query (`1.0 - distance`), higher is better.
    pub score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ReferentielQueryOutput {
    pub documents: Vec<ReferentielDocument>,
    pub stats: RagStats,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CrawlQueryOutput {
    pub documents: Vec<CrawlDocument>,
    pub stats: RagStats,
}

/// A row to seed into [`schema::RAG_REFERENTIEL_TABLE`].
pub struct ReferentielRecord {
    pub id: String,
    pub test_id: String,
    pub referentiel_version: String,
    pub content: String,
    pub embedding: Vec<f32>,
}

/// A row to seed into [`schema::RAG_CRAWL_TABLE`].
pub struct CrawlRecord {
    pub id: String,
    pub url: String,
    pub captured_at: String,
    pub evidence_hash: String,
    pub content: String,
    /// Unix-seconds expiry; rows past it are eligible for purge (#129).
    pub expires_at: Option<i64>,
    pub embedding: Vec<f32>,
}

/// Read-only handle onto the dual RAG index. Every method is a query — there
/// is no insert or delete here by construction, so a tool holding one cannot
/// write to either index.
#[derive(Clone)]
pub struct RagReader {
    db: Connection,
}

impl RagReader {
    /// Vector search over [`schema::RAG_REFERENTIEL_TABLE`], optionally
    /// narrowed to one RGAA test. Returns an empty (not erroring) result
    /// when the table has no rows or nothing matches, so a router's
    /// zero-results fallback path can act on it directly.
    pub async fn query_referentiel(
        &self,
        embedding: &[f32],
        k: usize,
        test_id: Option<&str>,
    ) -> Result<ReferentielQueryOutput, AgentError> {
        let table = self.open_table(RAG_REFERENTIEL_TABLE).await?;
        if table.count_rows(None).await.map_err(lancedb_err)? == 0 {
            return Ok(ReferentielQueryOutput {
                documents: vec![],
                stats: RagStats::from_scores(&[]),
            });
        }

        let mut query = table
            .vector_search(embedding)
            .map_err(lancedb_err)?
            .distance_type(DistanceType::Cosine)
            .limit(k);
        if let Some(test_id) = test_id {
            query = query.only_if(format!("test_id = '{}'", escape_literal(test_id)));
        }
        let batches = collect_batches(query).await?;

        let mut documents = Vec::new();
        for batch in &batches {
            let test_ids = utf8_column(batch, "test_id")?;
            let versions = utf8_column(batch, "referentiel_version")?;
            let contents = utf8_column(batch, "content")?;
            let distances = f32_column(batch, "_distance")?;
            for i in 0..batch.num_rows() {
                documents.push(ReferentielDocument {
                    test_id: test_ids.value(i).to_string(),
                    referentiel_version: versions.value(i).to_string(),
                    content: contents.value(i).to_string(),
                    score: 1.0 - distances.value(i),
                });
            }
        }
        let scores: Vec<f32> = documents.iter().map(|d| d.score).collect();
        Ok(ReferentielQueryOutput {
            stats: RagStats::from_scores(&scores),
            documents,
        })
    }

    /// Vector search over [`schema::RAG_CRAWL_TABLE`], optionally narrowed
    /// to one normalized URL. Returns an empty (not erroring) result when
    /// the table has no rows or nothing matches — the crawl index for a
    /// fresh audit starts empty until the writer (#129) populates it.
    pub async fn query_crawl(
        &self,
        embedding: &[f32],
        k: usize,
        url: Option<&str>,
    ) -> Result<CrawlQueryOutput, AgentError> {
        let table = self.open_table(RAG_CRAWL_TABLE).await?;
        if table.count_rows(None).await.map_err(lancedb_err)? == 0 {
            return Ok(CrawlQueryOutput {
                documents: vec![],
                stats: RagStats::from_scores(&[]),
            });
        }

        let mut query = table
            .vector_search(embedding)
            .map_err(lancedb_err)?
            .distance_type(DistanceType::Cosine)
            .limit(k);
        if let Some(url) = url {
            query = query.only_if(format!("url = '{}'", escape_literal(url)));
        }
        let batches = collect_batches(query).await?;

        let mut documents = Vec::new();
        for batch in &batches {
            let urls = utf8_column(batch, "url")?;
            let captured_ats = utf8_column(batch, "captured_at")?;
            let hashes = utf8_column(batch, "evidence_hash")?;
            let contents = utf8_column(batch, "content")?;
            let distances = f32_column(batch, "_distance")?;
            for i in 0..batch.num_rows() {
                documents.push(CrawlDocument {
                    url: urls.value(i).to_string(),
                    captured_at: captured_ats.value(i).to_string(),
                    evidence_hash: hashes.value(i).to_string(),
                    content: contents.value(i).to_string(),
                    score: 1.0 - distances.value(i),
                });
            }
        }
        let scores: Vec<f32> = documents.iter().map(|d| d.score).collect();
        Ok(CrawlQueryOutput {
            stats: RagStats::from_scores(&scores),
            documents,
        })
    }

    async fn open_table(&self, name: &str) -> Result<Table, AgentError> {
        self.db
            .open_table(name)
            .execute()
            .await
            .map_err(lancedb_err)
    }
}

/// Seeding/write-side handle onto the dual RAG index. Used by the
/// referentiel seeding pipeline (#128) and the crawl writer/purger (#129);
/// tool authors should hold a [`RagReader`] (via [`Self::reader`]) instead.
pub struct RagStore {
    db: Connection,
}

impl RagStore {
    /// Opens (or creates) the LanceDB database at `path` and ensures both
    /// RAG tables exist with the expected schema.
    ///
    /// # Errors
    /// Returns [`AgentError::LanceDb`] if the database cannot be opened or
    /// table creation/validation fails.
    pub async fn open(path: &str) -> Result<Self, AgentError> {
        let db = lancedb::connect(path)
            .execute()
            .await
            .map_err(lancedb_err)?;
        let store = Self { db };
        store.initialize_tables().await?;
        Ok(store)
    }

    async fn initialize_tables(&self) -> Result<(), AgentError> {
        create_empty_table(
            &self.db,
            RAG_REFERENTIEL_TABLE,
            schema::rag_referentiel_schema(),
        )
        .await?;
        create_empty_table(&self.db, RAG_CRAWL_TABLE, schema::rag_crawl_schema()).await?;
        Ok(())
    }

    /// A read-only handle sharing this store's connection. Cheap to clone —
    /// [`Connection`] is a handle, not a fresh connection.
    pub fn reader(&self) -> RagReader {
        RagReader {
            db: self.db.clone(),
        }
    }

    /// Replaces the full contents of [`schema::RAG_REFERENTIEL_TABLE`] with
    /// `records` — the whole-index rebuild #128 does on every referentiel
    /// version change (never migrated in place).
    ///
    /// The replacement batch is built and validated first, then installed
    /// with a single overwriting create: a batch failure can never leave
    /// the live table dropped.
    ///
    /// # Errors
    /// Returns [`AgentError`] on batch or LanceDB failure.
    pub async fn rebuild_referentiel(
        &self,
        records: Vec<ReferentielRecord>,
    ) -> Result<(), AgentError> {
        let batch = referentiel_batch(&records)?;
        self.db
            .create_table(RAG_REFERENTIEL_TABLE, batch)
            .mode(CreateTableMode::Overwrite)
            .execute()
            .await
            .map_err(lancedb_err)?;
        Ok(())
    }

    /// Appends `records` to [`schema::RAG_CRAWL_TABLE`] — the per-audit
    /// evidence write #129 performs before purging expired rows.
    ///
    /// # Errors
    /// Returns [`AgentError::LanceDb`] on any LanceDB failure.
    pub async fn insert_crawl(&self, records: Vec<CrawlRecord>) -> Result<(), AgentError> {
        if records.is_empty() {
            return Ok(());
        }
        let table = self
            .db
            .open_table(RAG_CRAWL_TABLE)
            .execute()
            .await
            .map_err(lancedb_err)?;
        let batch = crawl_batch(&records)?;
        table.add(batch).execute().await.map_err(lancedb_err)?;
        Ok(())
    }

    /// Builds an ANN index over `table`'s `embedding` column once it holds
    /// enough rows (LanceDB requires a minimum row count for IVF training;
    /// below that, vector search still works as an exact brute-force scan).
    pub async fn create_ann_index(&self, table_name: &str) -> Result<(), AgentError> {
        let table = self
            .db
            .open_table(table_name)
            .execute()
            .await
            .map_err(lancedb_err)?;
        table
            .create_index(
                &["embedding"],
                Index::IvfFlat(IvfFlatIndexBuilder::default()),
            )
            .execute()
            .await
            .map_err(lancedb_err)?;
        Ok(())
    }
}

fn referentiel_batch(records: &[ReferentielRecord]) -> Result<RecordBatch, AgentError> {
    let schema = schema::rag_referentiel_schema();
    let ids = StringArray::from_iter_values(records.iter().map(|r| r.id.as_str()));
    let test_ids = StringArray::from_iter_values(records.iter().map(|r| r.test_id.as_str()));
    let versions =
        StringArray::from_iter_values(records.iter().map(|r| r.referentiel_version.as_str()));
    let contents = StringArray::from_iter_values(records.iter().map(|r| r.content.as_str()));
    let embeddings = fixed_size_embedding_array(records.iter().map(|r| r.embedding.as_slice()))?;

    RecordBatch::try_new(
        schema,
        vec![
            Arc::new(ids),
            Arc::new(test_ids),
            Arc::new(versions),
            Arc::new(contents),
            Arc::new(embeddings),
        ],
    )
    .map_err(|e| AgentError::LanceDb(format!("failed to build referentiel batch: {e}")))
}

fn crawl_batch(records: &[CrawlRecord]) -> Result<RecordBatch, AgentError> {
    let schema = schema::rag_crawl_schema();
    let ids = StringArray::from_iter_values(records.iter().map(|r| r.id.as_str()));
    let urls = StringArray::from_iter_values(records.iter().map(|r| r.url.as_str()));
    let captured_ats =
        StringArray::from_iter_values(records.iter().map(|r| r.captured_at.as_str()));
    let hashes = StringArray::from_iter_values(records.iter().map(|r| r.evidence_hash.as_str()));
    let contents = StringArray::from_iter_values(records.iter().map(|r| r.content.as_str()));
    let expires_ats = Int64Array::from_iter(records.iter().map(|r| r.expires_at));
    let embeddings = fixed_size_embedding_array(records.iter().map(|r| r.embedding.as_slice()))?;

    RecordBatch::try_new(
        schema,
        vec![
            Arc::new(ids),
            Arc::new(urls),
            Arc::new(captured_ats),
            Arc::new(hashes),
            Arc::new(contents),
            Arc::new(expires_ats),
            Arc::new(embeddings),
        ],
    )
    .map_err(|e| AgentError::LanceDb(format!("failed to build crawl batch: {e}")))
}

/// Builds the fixed-size embedding column, rejecting any vector whose
/// length differs from [`EMBEDDING_DIM`](crate::vector::schema::EMBEDDING_DIM)
/// instead of letting Arrow panic or misalign rows.
fn fixed_size_embedding_array<'a>(
    vectors: impl Iterator<Item = &'a [f32]>,
) -> Result<arrow::array::FixedSizeListArray, AgentError> {
    use arrow::array::types::Float32Type;
    let dim = crate::vector::schema::EMBEDDING_DIM;
    let checked: Vec<&[f32]> = vectors
        .map(|v| {
            if v.len() == dim {
                Ok(v)
            } else {
                Err(AgentError::Embedding(format!(
                    "embedding de dimension {} au lieu de {dim}",
                    v.len()
                )))
            }
        })
        .collect::<Result<_, _>>()?;
    Ok(arrow::array::FixedSizeListArray::from_iter_primitive::<
        Float32Type,
        _,
        _,
    >(
        checked
            .into_iter()
            .map(|v| Some(v.iter().copied().map(Some))),
        dim as i32,
    ))
}

async fn create_empty_table(
    db: &Connection,
    name: &str,
    schema: SchemaRef,
) -> Result<(), AgentError> {
    db.create_empty_table(name, schema.clone())
        .mode(CreateTableMode::exist_ok(|req| req))
        .execute()
        .await
        .map_err(lancedb_err)?;
    Ok(())
}

async fn collect_batches(
    query: lancedb::query::VectorQuery,
) -> Result<Vec<RecordBatch>, AgentError> {
    use futures::TryStreamExt;
    query
        .execute()
        .await
        .map_err(lancedb_err)?
        .try_collect::<Vec<_>>()
        .await
        .map_err(lancedb_err)
}

fn utf8_column<'a>(batch: &'a RecordBatch, name: &str) -> Result<&'a StringArray, AgentError> {
    batch
        .column_by_name(name)
        .and_then(|c| c.as_any().downcast_ref::<StringArray>())
        .ok_or_else(|| AgentError::LanceDb(format!("missing/invalid column `{name}`")))
}

fn f32_column<'a>(batch: &'a RecordBatch, name: &str) -> Result<&'a Float32Array, AgentError> {
    batch
        .column_by_name(name)
        .and_then(|c| c.as_any().downcast_ref::<Float32Array>())
        .ok_or_else(|| AgentError::LanceDb(format!("missing/invalid column `{name}`")))
}

/// Escapes a single-quote-delimited SQL literal for use in `only_if`.
fn escape_literal(s: &str) -> String {
    s.replace('\'', "''")
}

fn lancedb_err(e: impl std::fmt::Display) -> AgentError {
    AgentError::LanceDb(e.to_string())
}
