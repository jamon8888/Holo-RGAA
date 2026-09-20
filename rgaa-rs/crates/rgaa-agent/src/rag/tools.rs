//! Read-only RAG retrieval tools: one per index ([`ReferentielSearchTool`],
//! [`CrawlSearchTool`]), each holding a [`RagReader`] — which has no insert
//! or delete method — so neither tool can write to an index no matter what
//! a model asks for.

use super::embed::EmbedQuery;
use super::store::{CrawlQueryOutput, RagReader, ReferentielQueryOutput, DEFAULT_K};
use rig_core::tool::PortableTool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Errors surfaced by the RAG retrieval tools.
#[derive(Debug, thiserror::Error)]
pub enum RagToolError {
    #[error("embedding failed: {0}")]
    Embedding(String),
    #[error("retrieval failed: {0}")]
    Retrieval(#[from] crate::error::AgentError),
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct ReferentielSearchArgs {
    /// Natural-language query to search the regulatory corpus with.
    pub query: String,
    /// Max documents to return. Defaults to 5.
    #[serde(default)]
    pub k: Option<usize>,
    /// Optional RGAA test ID to narrow the search to (e.g. `"1.1.1"`).
    #[serde(default)]
    pub test_id: Option<String>,
}

/// Read-only vector search over the versioned regulatory corpus. Never
/// decides a verdict alone — the router (#126) combines this with the crawl
/// index and a confidence threshold.
pub struct ReferentielSearchTool<E> {
    reader: RagReader,
    embedder: E,
}

impl<E> ReferentielSearchTool<E> {
    pub fn new(reader: RagReader, embedder: E) -> Self {
        Self { reader, embedder }
    }
}

impl<E: EmbedQuery> PortableTool for ReferentielSearchTool<E> {
    const NAME: &'static str = "search_referentiel";
    type Error = RagToolError;
    type Args = ReferentielSearchArgs;
    type Output = ReferentielQueryOutput;

    fn description(&self) -> String {
        "Search the versioned RGAA regulatory corpus for documents relevant \
         to a criterion. Read-only: never writes to the index. Returns \
         documents plus retrieval statistics (count, best_score) so the \
         caller can decide whether the match is strong enough."
            .to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::to_value(schemars::schema_for!(ReferentielSearchArgs)).expect("valid schema")
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let embedding = self
            .embedder
            .embed_query(&args.query)
            .await
            .map_err(RagToolError::Embedding)?;
        let output = self
            .reader
            .query_referentiel(
                &embedding,
                args.k.unwrap_or(DEFAULT_K),
                args.test_id.as_deref(),
            )
            .await?;
        Ok(output)
    }
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct CrawlSearchArgs {
    /// Natural-language query to search the per-audit crawl index with.
    pub query: String,
    /// Max documents to return. Defaults to 5.
    #[serde(default)]
    pub k: Option<usize>,
    /// Optional normalized URL to narrow the search to.
    #[serde(default)]
    pub url: Option<String>,
}

/// Read-only vector search over the per-audit crawl index — structured
/// deep-extraction evidence only, rebuilt fresh every audit. Never decides a
/// verdict alone: the crawl index is always a complement to the
/// regulatory corpus, never the sole basis (see #126).
pub struct CrawlSearchTool<E> {
    reader: RagReader,
    embedder: E,
}

impl<E> CrawlSearchTool<E> {
    pub fn new(reader: RagReader, embedder: E) -> Self {
        Self { reader, embedder }
    }
}

impl<E: EmbedQuery> PortableTool for CrawlSearchTool<E> {
    const NAME: &'static str = "search_crawl";
    type Error = RagToolError;
    type Args = CrawlSearchArgs;
    type Output = CrawlQueryOutput;

    fn description(&self) -> String {
        "Search the current audit's crawl index (structured deep-extraction \
         evidence only, never raw HTML) for documents relevant to a \
         criterion. Read-only: never writes to the index. Returns documents \
         plus retrieval statistics (count, best_score)."
            .to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::to_value(schemars::schema_for!(CrawlSearchArgs)).expect("valid schema")
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let embedding = self
            .embedder
            .embed_query(&args.query)
            .await
            .map_err(RagToolError::Embedding)?;
        let output = self
            .reader
            .query_crawl(&embedding, args.k.unwrap_or(DEFAULT_K), args.url.as_deref())
            .await?;
        Ok(output)
    }
}
