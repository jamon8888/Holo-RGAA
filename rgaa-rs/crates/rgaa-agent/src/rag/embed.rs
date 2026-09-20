//! Query-embedding abstraction for the RAG tools.
//!
//! Kept separate from [`super::store::RagReader`] (which takes a raw
//! embedding vector) so tests can supply a deterministic fake embedder
//! instead of loading the real `fastembed` model — the store's query logic
//! is exercised without a model download, and the real embedder is wired in
//! once (see [`crate::embeddings::HybridEmbeddingProvider`]'s impl below).

use rig_core::embeddings::EmbeddingModel;

/// Something that can turn a text query into an embedding vector for
/// [`super::store::RagReader::query_referentiel`]/`query_crawl`.
pub trait EmbedQuery: Send + Sync {
    fn embed_query(
        &self,
        text: &str,
    ) -> impl std::future::Future<Output = Result<Vec<f32>, String>> + Send;
}

impl EmbedQuery for crate::embeddings::HybridEmbeddingProvider {
    async fn embed_query(&self, text: &str) -> Result<Vec<f32>, String> {
        let embedding = self
            .embed_text(text)
            .await
            .map_err(|e| format!("embedding failed: {e}"))?;
        Ok(embedding.vec.into_iter().map(|v| v as f32).collect())
    }
}
