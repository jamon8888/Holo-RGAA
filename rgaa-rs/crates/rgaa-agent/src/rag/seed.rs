//! Build-time seeding of the versioned regulatory-corpus index
//! ([`schema::RAG_REFERENTIEL_TABLE`]) from the RGAA criteria catalog.
//!
//! One row per RGAA *test* (not per criterion) — RGAA 4.1.2 has 106
//! criteria but 258 tests, and chunking at test granularity is what gives
//! the index enough rows to train an ANN index (see
//! [`RagStore::create_ann_index`]) without needing a separate
//! over-chunking step. The whole table is rebuilt wholesale on every seed
//! run via [`RagStore::rebuild_referentiel`] — the index is derived and
//! reconstructible, never migrated in place.

use super::store::{RagStore, ReferentielRecord};
use crate::embeddings::HybridEmbeddingProvider;
use crate::error::AgentError;
use crate::rag::embed::EmbedQuery;
use rgaa_core::RgaaCatalog;

/// The regulatory corpus version stamped on every seeded row. One embedding
/// model equals one index version (see [`HybridEmbeddingProvider`]): bump
/// this whenever the catalog content or the embedding model changes, so a
/// stale citation from a prior version is never silently mixed with the
/// new index. Independent of the crate's own `Cargo.toml` version.
pub const REFERENTIEL_VERSION: &str = "rgaa-4.1.2";

/// Builds and (re)seeds [`schema::RAG_REFERENTIEL_TABLE`](super::schema::RAG_REFERENTIEL_TABLE)
/// from [`RgaaCatalog`].
pub struct ReferentielSeeder<'a, E> {
    embedder: &'a E,
    version: &'a str,
}

impl<'a, E: EmbedQuery> ReferentielSeeder<'a, E> {
    /// Seeds with [`REFERENTIEL_VERSION`].
    pub fn new(embedder: &'a E) -> Self {
        Self::with_version(embedder, REFERENTIEL_VERSION)
    }

    /// Seeds with an explicit version string — used by tests and by a
    /// future version-bump workflow that needs to seed a non-default
    /// version without editing [`REFERENTIEL_VERSION`].
    pub fn with_version(embedder: &'a E, version: &'a str) -> Self {
        Self { embedder, version }
    }

    /// Builds one [`ReferentielRecord`] per RGAA test across every
    /// criterion in [`RgaaCatalog::all`], embedding each test's content
    /// (criterion title + test description lines) with `embedder`.
    ///
    /// # Errors
    /// Returns [`AgentError::Embedding`] if embedding any test fails.
    pub async fn build_records(&self) -> Result<Vec<ReferentielRecord>, AgentError> {
        let mut records = Vec::new();
        for theme in RgaaCatalog::all() {
            for cw in &theme.criteria {
                let criterion = &cw.criterium;
                let criterion_id = criterion.id_for_theme(theme.number);
                for (test_number, lines) in &criterion.tests {
                    let test_id = format!("{criterion_id}.{test_number}");
                    let content = format!(
                        "Critère RGAA {criterion_id} — {}\n\n{}",
                        criterion.title,
                        lines.join("\n")
                    );
                    let embedding = self
                        .embedder
                        .embed_query(&content)
                        .await
                        .map_err(AgentError::Embedding)?;
                    records.push(ReferentielRecord {
                        id: format!("{}-{test_id}", self.version),
                        test_id,
                        referentiel_version: self.version.to_string(),
                        content,
                        embedding,
                    });
                }
            }
        }
        Ok(records)
    }

    /// Rebuilds `store`'s referentiel table wholesale from the current
    /// catalog. Returns the number of rows seeded.
    ///
    /// # Errors
    /// Returns [`AgentError`] if embedding or the LanceDB rebuild fails.
    pub async fn seed(&self, store: &RagStore) -> Result<usize, AgentError> {
        let records = self.build_records().await?;
        let count = records.len();
        store.rebuild_referentiel(records).await?;
        Ok(count)
    }
}

/// Convenience alias for the production seeder: [`HybridEmbeddingProvider`]
/// implements [`EmbedQuery`] (see `rag::embed`).
pub type DefaultReferentielSeeder<'a> = ReferentielSeeder<'a, HybridEmbeddingProvider>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rag::store::RagStore;
    use std::collections::HashSet;
    use tempfile::TempDir;

    struct FakeEmbedder;

    impl EmbedQuery for FakeEmbedder {
        async fn embed_query(&self, text: &str) -> Result<Vec<f32>, String> {
            let bytes = text.as_bytes();
            Ok((0..crate::vector::schema::EMBEDDING_DIM)
                .map(|i| {
                    let b = bytes.get(i % bytes.len().max(1)).copied().unwrap_or(0);
                    (b as f32 / 255.0) - 0.5
                })
                .collect())
        }
    }

    #[tokio::test]
    async fn build_records_covers_every_rgaa_test() {
        let seeder = ReferentielSeeder::new(&FakeEmbedder);
        let records = seeder.build_records().await.unwrap();

        // RGAA 4.1.2: 106 criteria, 258 tests total — enough rows for ANN
        // training, which is exactly why seeding chunks per test.
        assert_eq!(records.len(), 258);

        let ids: HashSet<&str> = records.iter().map(|r| r.test_id.as_str()).collect();
        assert_eq!(ids.len(), 258, "every test_id must be unique");

        for record in &records {
            assert_eq!(record.referentiel_version, REFERENTIEL_VERSION);
            assert!(!record.content.is_empty());
            assert_eq!(record.embedding.len(), crate::vector::schema::EMBEDDING_DIM);
        }
    }

    #[tokio::test]
    async fn every_record_cites_its_test_and_version() {
        let seeder = ReferentielSeeder::new(&FakeEmbedder);
        let records = seeder.build_records().await.unwrap();

        let test_1_1_1 = records
            .iter()
            .find(|r| r.test_id == "1.1.1")
            .expect("test 1.1.1 must be seeded");
        assert_eq!(test_1_1_1.referentiel_version, "rgaa-4.1.2");
        assert!(test_1_1_1.content.contains("1.1"));
    }

    #[tokio::test]
    async fn seed_populates_a_queryable_index() {
        use crate::rag::tools::{ReferentielSearchArgs, ReferentielSearchTool};
        use rig_core::tool::PortableTool;

        let dir = TempDir::new().unwrap();
        let store = RagStore::open(dir.path().to_str().unwrap()).await.unwrap();
        let seeder = ReferentielSeeder::new(&FakeEmbedder);
        let seeded = seeder.seed(&store).await.unwrap();
        assert_eq!(seeded, 258);

        let tool = ReferentielSearchTool::new(store.reader(), FakeEmbedder);
        let output = tool
            .call(ReferentielSearchArgs {
                query: "alternative textuelle image".into(),
                k: Some(3),
                test_id: None,
            })
            .await
            .unwrap();
        assert_eq!(output.documents.len(), 3);
        assert_eq!(output.stats.count, 3);
    }

    #[tokio::test]
    async fn reseeding_a_new_version_replaces_the_old_one_wholesale() {
        let dir = TempDir::new().unwrap();
        let store = RagStore::open(dir.path().to_str().unwrap()).await.unwrap();

        ReferentielSeeder::with_version(&FakeEmbedder, "rgaa-4.1.2")
            .seed(&store)
            .await
            .unwrap();
        ReferentielSeeder::with_version(&FakeEmbedder, "rgaa-4.2.0")
            .seed(&store)
            .await
            .unwrap();

        let reader = store.reader();
        let output = reader
            .query_referentiel(&[0.1; crate::vector::schema::EMBEDDING_DIM], 500, None)
            .await
            .unwrap();
        // Every remaining row is from the new version — the old version's
        // rows were dropped, not merged alongside.
        assert!(!output.documents.is_empty());
        assert!(output
            .documents
            .iter()
            .all(|d| d.referentiel_version == "rgaa-4.2.0"));
    }
}
