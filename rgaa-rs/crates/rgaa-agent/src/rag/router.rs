//! Deterministic routing between the dual RAG index for one criterion
//! evaluation — ticket #126.
//!
//! The evaluator queries the regulatory corpus plus the crawl index as a
//! complement; the verifier queries the regulatory corpus alone (see
//! [`EvaluationRole`]). Routing is a pure function of the two indices'
//! retrieval stats: the same page/criterion pair always routes the same
//! way, so two audits of the same page take an identical retrieval path —
//! nothing here depends on page count, so it applies identically to a
//! mono-page audit and to any one page of a multi-page crawl audit.

use super::embed::EmbedQuery;
use super::store::RagReader;
use crate::error::AgentError;
use crate::references::{CrawlReference, References, ReferentielReference};

/// Retrieval-sufficiency thresholds. Below either bar, the primary index is
/// considered "weak" and the router falls back to (for the evaluator) the
/// crawl index, or (with both indices weak, or the primary completely
/// empty) escalates to human review without an LLM call.
#[derive(Debug, Clone, Copy)]
pub struct RouterThresholds {
    /// Minimum document count for an index to be considered sufficient on
    /// its own.
    pub min_docs: usize,
    /// Minimum best cosine similarity (see [`super::store::RagStats::best_score`])
    /// for an index to be considered sufficient on its own.
    pub min_score: f32,
}

impl Default for RouterThresholds {
    fn default() -> Self {
        Self {
            min_docs: 2,
            min_score: 0.3,
        }
    }
}

/// Which worker is being routed for — determines which indices are
/// consulted (see [`Router::route`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvaluationRole {
    /// Queries the regulatory corpus, complemented by the crawl index.
    Evaluator,
    /// Queries the regulatory corpus alone — never the crawl index, so its
    /// verdict is always checkable against a versioned, citable source
    /// (see the spec's "vérificateur ... RAG réglementaire seul").
    Verifier,
}

/// What the router decided for one criterion evaluation.
#[derive(Debug, Clone, PartialEq)]
pub enum RouteDecision {
    /// Enough grounding was found — evaluate with these references.
    Proceed(References),
    /// Retrieval came up empty or too weak to ground a verdict —
    /// escalate straight to human review. No further LLM call should be
    /// made for this criterion; see the module-level fallback rule.
    Escalate { reason: String },
}

impl RouteDecision {
    #[cfg(test)]
    fn is_escalate(&self) -> bool {
        matches!(self, Self::Escalate { .. })
    }
}

/// Routes one criterion evaluation to a [`RouteDecision`] against the dual
/// RAG index.
///
/// Fallback rule (both roles): if the regulatory corpus returns **zero**
/// documents, the router escalates unconditionally — the crawl index is
/// never allowed to single-handedly decide a verdict, so with nothing at
/// all from the regulatory corpus there is nothing for crawl to
/// complement. Above zero but still under [`RouterThresholds`] ("weak"),
/// the evaluator falls back to (also) querying crawl; if crawl is then
/// also empty, or for the verifier which has no second index at all, the
/// router escalates the same way. Escalation never spends an extra LLM
/// call — it is a purely retrieval-stats decision.
pub struct Router<'a, E> {
    reader: &'a RagReader,
    embedder: &'a E,
    thresholds: RouterThresholds,
}

impl<'a, E: EmbedQuery> Router<'a, E> {
    pub fn new(reader: &'a RagReader, embedder: &'a E) -> Self {
        Self::with_thresholds(reader, embedder, RouterThresholds::default())
    }

    pub fn with_thresholds(
        reader: &'a RagReader,
        embedder: &'a E,
        thresholds: RouterThresholds,
    ) -> Self {
        Self {
            reader,
            embedder,
            thresholds,
        }
    }

    /// Routes one `query` (typically the rendered evaluation prompt context
    /// for `criterion_id`) for `role`.
    ///
    /// # Errors
    /// Returns [`AgentError`] if embedding the query or querying either
    /// index fails outright (a genuine retrieval error, distinct from a
    /// retrieval that simply came back empty — that's [`RouteDecision::Escalate`],
    /// not an `Err`).
    pub async fn route(
        &self,
        query: &str,
        role: EvaluationRole,
    ) -> Result<RouteDecision, AgentError> {
        let embedding = self
            .embedder
            .embed_query(query)
            .await
            .map_err(AgentError::Embedding)?;

        let referentiel = self
            .reader
            .query_referentiel(&embedding, super::store::DEFAULT_K, None)
            .await?;

        if referentiel.stats.count == 0 {
            return Ok(RouteDecision::Escalate {
                reason: "regulatory corpus retrieval returned no documents".to_string(),
            });
        }

        let referentiel_refs: Vec<ReferentielReference> = referentiel
            .documents
            .into_iter()
            .map(|d| ReferentielReference {
                test_id: d.test_id,
                referentiel_version: d.referentiel_version,
                content: d.content,
            })
            .collect();

        if role == EvaluationRole::Verifier {
            // Referentiel alone, sufficient or not — no second index for
            // the verifier to fall back to, and no crawl involvement ever.
            return Ok(RouteDecision::Proceed(References {
                referentiel: referentiel_refs,
                crawl: vec![],
            }));
        }

        let referentiel_weak = referentiel.stats.count < self.thresholds.min_docs
            || referentiel
                .stats
                .best_score
                .is_none_or(|s| s < self.thresholds.min_score);

        // Evaluator: crawl is always consulted as a complement, not only on
        // fallback — see the module doc.
        let crawl = self
            .reader
            .query_crawl(&embedding, super::store::DEFAULT_K, None)
            .await?;

        if referentiel_weak && crawl.stats.count == 0 {
            return Ok(RouteDecision::Escalate {
                reason: "regulatory corpus retrieval was weak and crawl retrieval was empty"
                    .to_string(),
            });
        }

        let crawl_refs: Vec<CrawlReference> = crawl
            .documents
            .into_iter()
            .map(|d| CrawlReference {
                url: d.url,
                content: d.content,
            })
            .collect();

        Ok(RouteDecision::Proceed(References {
            referentiel: referentiel_refs,
            crawl: crawl_refs,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rag::store::{CrawlRecord, RagStore, ReferentielRecord};
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

    fn fake_embedding(text: &str) -> Vec<f32> {
        let bytes = text.as_bytes();
        (0..crate::vector::schema::EMBEDDING_DIM)
            .map(|i| {
                let b = bytes.get(i % bytes.len().max(1)).copied().unwrap_or(0);
                (b as f32 / 255.0) - 0.5
            })
            .collect()
    }

    async fn store_with_two_referentiel_docs(dir: &TempDir) -> RagStore {
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
                    id: "1.1.2-a".into(),
                    test_id: "1.1.2".into(),
                    referentiel_version: "2024.1".into(),
                    content: "Chaque zone d'une image réactive a une alternative textuelle.".into(),
                    embedding: fake_embedding("alt text image map"),
                },
            ])
            .await
            .unwrap();
        store
    }

    #[tokio::test]
    async fn evaluator_proceeds_with_referentiel_and_crawl_when_both_sufficient() {
        let dir = TempDir::new().unwrap();
        let store = store_with_two_referentiel_docs(&dir).await;
        store
            .insert_crawl(vec![CrawlRecord {
                id: "evt-1".into(),
                url: "https://example.test/".into(),
                captured_at: "2025-01-01T00:00:00Z".into(),
                evidence_hash: "sha1n-v1-0".into(),
                content: "Image sans alt détectée.".into(),
                expires_at: None,
                embedding: fake_embedding("alt text image"),
            }])
            .await
            .unwrap();

        let reader = store.reader();
        let router = Router::new(&reader, &FakeEmbedder);
        let decision = router
            .route("alt text image", EvaluationRole::Evaluator)
            .await
            .unwrap();

        match decision {
            RouteDecision::Proceed(refs) => {
                assert!(!refs.referentiel.is_empty());
                assert!(!refs.crawl.is_empty());
            }
            RouteDecision::Escalate { reason } => {
                panic!("expected Proceed, got Escalate: {reason}")
            }
        }
    }

    #[tokio::test]
    async fn verifier_never_queries_crawl() {
        let dir = TempDir::new().unwrap();
        let store = store_with_two_referentiel_docs(&dir).await;
        // Seed crawl too — if the verifier ever consulted it, this would
        // change the outcome; it must not.
        store
            .insert_crawl(vec![CrawlRecord {
                id: "evt-1".into(),
                url: "https://example.test/".into(),
                captured_at: "2025-01-01T00:00:00Z".into(),
                evidence_hash: "sha1n-v1-0".into(),
                content: "Crawl evidence that must stay unused by the verifier.".into(),
                expires_at: None,
                embedding: fake_embedding("alt text image"),
            }])
            .await
            .unwrap();

        let reader = store.reader();
        let router = Router::new(&reader, &FakeEmbedder);
        let decision = router
            .route("alt text image", EvaluationRole::Verifier)
            .await
            .unwrap();

        match decision {
            RouteDecision::Proceed(refs) => {
                assert!(!refs.referentiel.is_empty());
                assert!(
                    refs.crawl.is_empty(),
                    "verifier must never receive crawl references"
                );
            }
            RouteDecision::Escalate { reason } => {
                panic!("expected Proceed, got Escalate: {reason}")
            }
        }
    }

    #[tokio::test]
    async fn both_indices_empty_escalates_without_llm_call() {
        let dir = TempDir::new().unwrap();
        // Fresh store: both tables exist but are empty.
        let store = RagStore::open(dir.path().to_str().unwrap()).await.unwrap();
        let reader = store.reader();
        let router = Router::new(&reader, &FakeEmbedder);

        let evaluator_decision = router
            .route("anything", EvaluationRole::Evaluator)
            .await
            .unwrap();
        assert!(evaluator_decision.is_escalate());

        let verifier_decision = router
            .route("anything", EvaluationRole::Verifier)
            .await
            .unwrap();
        assert!(verifier_decision.is_escalate());
    }

    #[tokio::test]
    async fn empty_referentiel_escalates_even_with_rich_crawl_evidence() {
        // Crawl alone must never decide a verdict — even a well-populated
        // crawl index cannot substitute for an empty regulatory corpus.
        let dir = TempDir::new().unwrap();
        let store = RagStore::open(dir.path().to_str().unwrap()).await.unwrap();
        store
            .insert_crawl(vec![
                CrawlRecord {
                    id: "evt-1".into(),
                    url: "https://example.test/".into(),
                    captured_at: "2025-01-01T00:00:00Z".into(),
                    evidence_hash: "sha1n-v1-0".into(),
                    content: "Plenty of crawl evidence.".into(),
                    expires_at: None,
                    embedding: fake_embedding("anything"),
                },
                CrawlRecord {
                    id: "evt-2".into(),
                    url: "https://example.test/2".into(),
                    captured_at: "2025-01-01T00:00:00Z".into(),
                    evidence_hash: "sha1n-v1-1".into(),
                    content: "Even more crawl evidence.".into(),
                    expires_at: None,
                    embedding: fake_embedding("anything else"),
                },
            ])
            .await
            .unwrap();

        let reader = store.reader();
        let router = Router::new(&reader, &FakeEmbedder);
        let decision = router
            .route("anything", EvaluationRole::Evaluator)
            .await
            .unwrap();
        assert!(decision.is_escalate());
    }

    #[tokio::test]
    async fn weak_referentiel_falls_back_to_crawl_when_crawl_has_evidence() {
        let dir = TempDir::new().unwrap();
        // Only one referentiel doc: below `min_docs` (2), so "weak".
        let store = RagStore::open(dir.path().to_str().unwrap()).await.unwrap();
        store
            .rebuild_referentiel(vec![ReferentielRecord {
                id: "1.1.1-a".into(),
                test_id: "1.1.1".into(),
                referentiel_version: "2024.1".into(),
                content: "Single weak referentiel document.".into(),
                embedding: fake_embedding("query"),
            }])
            .await
            .unwrap();
        store
            .insert_crawl(vec![CrawlRecord {
                id: "evt-1".into(),
                url: "https://example.test/".into(),
                captured_at: "2025-01-01T00:00:00Z".into(),
                evidence_hash: "sha1n-v1-0".into(),
                content: "Crawl evidence backing up the weak referentiel signal.".into(),
                expires_at: None,
                embedding: fake_embedding("query"),
            }])
            .await
            .unwrap();

        let reader = store.reader();
        let router = Router::new(&reader, &FakeEmbedder);
        let decision = router
            .route("query", EvaluationRole::Evaluator)
            .await
            .unwrap();

        match decision {
            RouteDecision::Proceed(refs) => {
                // Crawl supplements a *non-empty* referentiel signal — it
                // never stands alone.
                assert!(!refs.referentiel.is_empty());
                assert!(!refs.crawl.is_empty());
            }
            RouteDecision::Escalate { reason } => {
                panic!("expected Proceed, got Escalate: {reason}")
            }
        }
    }

    #[tokio::test]
    async fn routing_is_identical_for_two_independent_pages_mono_or_multi_page() {
        // Nothing about the router is page-count-aware: routing the same
        // query twice (as a mono-page audit would call it once, and a
        // multi-page crawl audit would call it once per page) is
        // deterministic and independent each time.
        let dir = TempDir::new().unwrap();
        let store = store_with_two_referentiel_docs(&dir).await;
        let reader = store.reader();
        let router = Router::new(&reader, &FakeEmbedder);

        let first = router
            .route("alt text image", EvaluationRole::Verifier)
            .await
            .unwrap();
        let second = router
            .route("alt text image", EvaluationRole::Verifier)
            .await
            .unwrap();
        assert_eq!(first, second);
    }
}
