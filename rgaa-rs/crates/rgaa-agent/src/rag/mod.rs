//! Dual-index RAG retrieval: the versioned regulatory corpus and the
//! per-audit crawl index, plus the read-only tools an agent worker calls
//! them through. Part of the #121 target architecture (ticket #124).

pub mod crawl_writer;
pub mod embed;
pub mod harness;
pub mod patterns;
pub mod router;
pub mod schema;
pub mod seed;
pub mod store;
pub mod tools;
mod util;
pub mod verifier;

pub use crawl_writer::{CrawlWriter, DEFAULT_TTL};
pub use embed::EmbedQuery;
pub use seed::{ReferentielSeeder, REFERENTIEL_VERSION};
pub use harness::{
    check_budget, run_baseline, run_baseline_with_cassette, BaselineCase, BaselineReport,
    BudgetEnvelope, BudgetExceeded, ConfusionEntry, CostSummary, CriterionConfusion,
    HallucinationCounters, DEFAULT_BUDGET_MARGIN,
};
>>>>>>> d7a0445 (feat(holo,agent): VCR cassette replay + baseline measurement harness)
pub use patterns::{PatternQueryOutput, PatternReader, RemediationPattern};
pub use router::{EvaluationRole, RouteDecision, Router, RouterThresholds};
pub use store::{
    CrawlDocument, CrawlQueryOutput, CrawlRecord, RagReader, RagStats, RagStore,
    ReferentielDocument, ReferentielQueryOutput, ReferentielRecord,
};
pub use tools::{
    CrawlSearchArgs, CrawlSearchTool, RagToolError, ReferentielSearchArgs, ReferentielSearchTool,
};
pub use verifier::{Verifier, VerifierError, VerifierOutcome, VerifierResponse};
pub use verifier::{Verifier, VerifierError, VerifierOutcome, VerifierResponse};

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    /// Deterministic fake embedder: hashes the query text into a fixed
    /// vector so tests never need the real `fastembed` model or network.
    struct FakeEmbedder;

    impl EmbedQuery for FakeEmbedder {
        async fn embed_query(&self, text: &str) -> Result<Vec<f32>, String> {
            Ok(fake_embedding(text))
        }
    }

    /// A tiny deterministic embedding: every dimension derived from a
    /// byte of `text` (cycled), distinct enough that different strings
    /// produce distinguishable vectors for cosine similarity.
    fn fake_embedding(text: &str) -> Vec<f32> {
        let bytes = text.as_bytes();
        (0..crate::vector::schema::EMBEDDING_DIM)
            .map(|i| {
                let b = bytes.get(i % bytes.len().max(1)).copied().unwrap_or(0);
                (b as f32 / 255.0) - 0.5
            })
            .collect()
    }

    async fn seeded_store(dir: &TempDir) -> RagStore {
        let store = RagStore::open(dir.path().to_str().unwrap()).await.unwrap();
        store
            .rebuild_referentiel(vec![
                ReferentielRecord {
                    id: "1.1.1-a".into(),
                    test_id: "1.1.1".into(),
                    referentiel_version: "2024.1".into(),
                    content: "Chaque image porteuse d'information a une alternative textuelle."
                        .into(),
                    embedding: fake_embedding("alt text image"),
                },
                ReferentielRecord {
                    id: "3.2.1-a".into(),
                    test_id: "3.2.1".into(),
                    referentiel_version: "2024.1".into(),
                    content: "Le contraste entre le texte et son arrière-plan est suffisant."
                        .into(),
                    embedding: fake_embedding("color contrast"),
                },
            ])
            .await
            .unwrap();
        store
            .insert_crawl(vec![CrawlRecord {
                id: "evt-1".into(),
                url: "https://example.test/contact".into(),
                captured_at: "2025-01-01T00:00:00Z".into(),
                evidence_hash: "sha256:abc".into(),
                content: "Formulaire de contact sans label visible sur le champ email.".into(),
                expires_at: None,
                embedding: fake_embedding("contact form missing label"),
            }])
            .await
            .unwrap();
        store
    }

    #[tokio::test]
    async fn referentiel_search_tool_returns_documents_and_stats() {
        use rig_core::tool::PortableTool;

        let dir = TempDir::new().unwrap();
        let store = seeded_store(&dir).await;
        let tool = ReferentielSearchTool::new(store.reader(), FakeEmbedder);

        let output = tool
            .call(ReferentielSearchArgs {
                query: "alt text image".into(),
                k: Some(2),
                test_id: None,
            })
            .await
            .unwrap();

        assert_eq!(output.documents.len(), 2);
        assert_eq!(output.stats.count, 2);
        assert!(output.stats.best_score.is_some());
        // The closest match should be the alt-text document, not contrast.
        assert_eq!(output.documents[0].test_id, "1.1.1");
    }

    #[tokio::test]
    async fn referentiel_search_narrows_by_test_id() {
        use rig_core::tool::PortableTool;

        let dir = TempDir::new().unwrap();
        let store = seeded_store(&dir).await;
        let tool = ReferentielSearchTool::new(store.reader(), FakeEmbedder);

        let output = tool
            .call(ReferentielSearchArgs {
                query: "anything".into(),
                k: Some(5),
                test_id: Some("3.2.1".into()),
            })
            .await
            .unwrap();

        assert_eq!(output.documents.len(), 1);
        assert_eq!(output.documents[0].test_id, "3.2.1");
    }

    #[tokio::test]
    async fn crawl_search_tool_returns_documents_and_stats() {
        use rig_core::tool::PortableTool;

        let dir = TempDir::new().unwrap();
        let store = seeded_store(&dir).await;
        let tool = CrawlSearchTool::new(store.reader(), FakeEmbedder);

        let output = tool
            .call(CrawlSearchArgs {
                query: "contact form missing label".into(),
                k: None,
                url: None,
            })
            .await
            .unwrap();

        assert_eq!(output.documents.len(), 1);
        assert_eq!(output.stats.count, 1);
        assert_eq!(output.documents[0].url, "https://example.test/contact");
    }

    #[tokio::test]
    async fn empty_index_returns_empty_result_not_an_error() {
        use rig_core::tool::PortableTool;

        let dir = TempDir::new().unwrap();
        // A fresh store: both tables exist but are empty — exactly the
        // shape a brand-new audit's crawl index starts in before #129
        // writes anything, or a referentiel index before #128 seeds it.
        let store = RagStore::open(dir.path().to_str().unwrap()).await.unwrap();

        let referentiel = ReferentielSearchTool::new(store.reader(), FakeEmbedder)
            .call(ReferentielSearchArgs {
                query: "anything".into(),
                k: None,
                test_id: None,
            })
            .await
            .unwrap();
        assert_eq!(referentiel.documents.len(), 0);
        assert_eq!(referentiel.stats.count, 0);
        assert_eq!(referentiel.stats.best_score, None);

        let crawl = CrawlSearchTool::new(store.reader(), FakeEmbedder)
            .call(CrawlSearchArgs {
                query: "anything".into(),
                k: None,
                url: None,
            })
            .await
            .unwrap();
        assert_eq!(crawl.documents.len(), 0);
        assert_eq!(crawl.stats.count, 0);
        assert_eq!(crawl.stats.best_score, None);
    }

    #[tokio::test]
    async fn reader_type_exposes_no_write_method() {
        // Compile-time guarantee, not a runtime assertion: `RagReader` is
        // the only handle the tools hold, and it has no insert/delete
        // method at all (see its impl block) — this test exists to fail
        // loudly if that ever changes without an explicit decision, since
        // `cargo check` alone wouldn't flag a *removed* guarantee.
        let dir = TempDir::new().unwrap();
        let store = RagStore::open(dir.path().to_str().unwrap()).await.unwrap();
        let reader: RagReader = store.reader();
        // Only query methods compile against `reader`; `reader.insert_*`
        // or `reader.rebuild_*` would be a compile error if uncommented.
        let _ = reader
            .query_referentiel(&fake_embedding("x"), 1, None)
            .await
            .unwrap();
    }
}
