use crate::config::AgentConfig;
use crate::embeddings::HybridEmbeddingProvider;
use crate::error::AgentError;
use crate::memory::LanceDbMemory;
use crate::prompts::PromptBuilder;
use crate::vector::LanceDbVectorStore;
use crate::verify::map_verdict;
use rig_agent::agent::Agent;
use rig_agent::client::AgentClientExt;
use rig_agent::completion::Prompt;
use rig_core::providers::openai;
use rgaa_core::{Classification, Criterion, CriterionResult, CriterionStatus};
use rgaa_holo::{HoloResponse, PageContext};
use std::collections::HashMap;
use std::sync::Arc;

/// Dual-model RGAA agent: a fast reasoning model (Holo3) for criterion
/// evaluation, backed by LanceDB conversation memory and a vector store.
pub struct RgaaAgent {
    agent: Agent,
    memory: Arc<LanceDbMemory>,
    vector_store: Arc<LanceDbVectorStore>,
}

impl RgaaAgent {
    /// Builds the agent, embedding provider, LanceDB memory, and vector store.
    ///
    /// # Errors
    /// Returns [`AgentError`] if the OpenAI-compatible client or the embedding
    /// backend fails to initialize, or if LanceDB storage cannot be opened.
    #[tracing::instrument(skip_all)]
    pub async fn new(config: &AgentConfig) -> Result<Self, AgentError> {
        // 1. Create OpenAI-compatible client pointing at Holo3
        let client = openai::Client::builder()
            .base_url(&config.holo3_base_url)
            .api_key(&config.api_key)
            .build()
            .map_err(|e| AgentError::RigAgent(e.to_string()))?;

        // 2. Create embedding provider
        let embeddings = HybridEmbeddingProvider::new(config)?;

        // 3. Create LanceDB memory
        let memory = LanceDbMemory::new(&config.lancedb_path).await?;
        let memory = Arc::new(memory);

        // 4. Create vector store
        let vector_store = LanceDbVectorStore::new(&config.lancedb_path, embeddings.clone()).await?;
        let vector_store = Arc::new(vector_store);

        // 5. Build agent with preamble
        let agent = client
            .agent(config.model.as_str())
            .preamble("You are an RGAA accessibility expert. Evaluate criteria and provide verdicts.")
            .build();

        Ok(Self {
            agent,
            memory,
            vector_store,
        })
    }

    /// Evaluates a single IA-assistée criterion against the given page context.
    ///
    /// Builds the evaluator prompt with [`PromptBuilder`], queries the Holo3
    /// reasoning model, and maps the structured [`HoloResponse`] to a
    /// [`CriterionStatus`] via [`map_verdict`]. On model failure the criterion
    /// is flagged [`CriterionStatus::NeedsReview`] with the error captured in
    /// the justification.
    #[tracing::instrument(skip_all)]
    pub async fn evaluate_criterion(
        &self,
        criterion: &Criterion,
        page_context: &PageContext,
    ) -> CriterionResult {
        let prompt = PromptBuilder::build(&criterion.id, page_context);

        match self.agent.prompt(prompt.as_str()).await {
            Ok(response) => {
                let parsed = HoloResponse::extract_json(&response)
                    .unwrap_or_else(|| HoloResponse {
                        verdict: "na".to_string(),
                        confidence: 0.0,
                        justification: response.clone(),
                    });
                let status = map_verdict(parsed.clone());
                CriterionResult {
                    criterion_id: criterion.id.to_string(),
                    title: criterion.title.to_string(),
                    classification: Classification::IaAssiste,
                    status,
                    violations: vec![],
                    confidence: Some(parsed.confidence),
                    justification: Some(parsed.justification),
                    source: "agent".to_string(),
                }
            }
            Err(e) => {
                tracing::warn!(criterion = criterion.id, error = %e, "evaluation failed");
                CriterionResult {
                    criterion_id: criterion.id.to_string(),
                    title: criterion.title.to_string(),
                    classification: Classification::IaAssiste,
                    status: CriterionStatus::NeedsReview,
                    violations: vec![],
                    confidence: None,
                    justification: Some(format!("Erreur: {e}")),
                    source: "agent-error".to_string(),
                }
            }
        }
    }

    /// Evaluates every criterion in `criteria`, returning a result map keyed by
    /// criterion id.
    pub async fn run_ia_assiste(
        &self,
        criteria: &[Criterion],
        page_context: &PageContext,
    ) -> HashMap<String, CriterionResult> {
        let mut results = HashMap::with_capacity(criteria.len());
        for criterion in criteria {
            let result = self.evaluate_criterion(criterion, page_context).await;
            results.insert(criterion.id.to_string(), result);
        }
        results
    }
}
