use crate::config::AgentConfig;
use crate::error::AgentError;
use crate::prompts::PromptBuilder;
use crate::ratelimit::Ratelimiter;
use crate::verify::map_verdict;
use rgaa_core::{Classification, Criterion, CriterionResult, CriterionStatus};
use rgaa_holo::{HoloClient, HoloResponse, PageContext};
use rig_agent::agent::Agent;
use rig_agent::client::AgentClientExt;
use rig_agent::completion::Prompt;
use rig_core::providers::openai;
use std::collections::HashMap;
use std::sync::Arc;

/// RGAA agentic evaluator for IA-assistée criteria.
///
/// Uses a single Holo3 model with token-bucket rate limiting.
/// Conversation memory and vector retrieval are available via
/// [`LanceDbMemory`] and [`LanceDbVectorStore`] but are not yet
/// integrated into the evaluation path.
pub struct RgaaAgent {
    agent: Agent,
    rate_limiter: Arc<Ratelimiter>,
}

impl RgaaAgent {
    /// Builds the agent and rate limiter.
    ///
    /// # Errors
    /// Returns [`AgentError`] if the OpenAI-compatible client or the rate
    /// limiter fails to initialize.
    #[tracing::instrument(skip_all)]
    pub async fn new(config: &AgentConfig) -> Result<Self, AgentError> {
        // 1. Create OpenAI-compatible client pointing at Holo3
        let client = openai::Client::builder()
            .base_url(&config.holo3_base_url)
            .api_key(&config.api_key)
            .build()
            .map_err(|e| AgentError::RigAgent(e.to_string()))?;

        // 2. Create rate limiter from config (tactical/reasoning RPM)
        let rate_limiter = Arc::new(Ratelimiter::new(
            10, // tactical RPM
            20, // reasoning RPM
        ));

        // 3. Build agent with preamble
        let agent = client
            .agent(config.model.as_str())
            .preamble(
                "You are an RGAA accessibility expert. Evaluate criteria and provide verdicts.",
            )
            .build();

        Ok(Self {
            agent,
            rate_limiter,
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

        // Apply rate limiting (tactical tier for standard criteria)
        self.rate_limiter
            .acquire(crate::ratelimit::ModelTier::Tactical)
            .await;

        match self.agent.prompt(prompt.as_str()).await {
            Ok(response) => {
                let parsed =
                    HoloClient::extract_json(&response).unwrap_or_else(|| HoloResponse {
                        verdict: "na".to_string(),
                        confidence: 0.0,
                        justification: response.clone(),
                    });
                let status = map_verdict(&parsed);
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
    ///
    /// Uses bounded concurrency with the internal rate limiter to avoid
    /// overwhelming the Holo3 API while keeping evaluations parallel.
    pub async fn run_ia_assiste(
        &self,
        criteria: &[Criterion],
        page_context: &PageContext,
    ) -> HashMap<String, CriterionResult> {
        use futures::stream::{self, StreamExt};

        let page_context = Arc::new(page_context.clone());
        let results = stream::iter(criteria.iter().cloned())
            .map(|criterion| {
                let self_ = Arc::new(self.clone());
                let page_context = page_context.clone();
                async move {
                    let result = self_.evaluate_criterion(&criterion, &page_context).await;
                    (criterion.id.to_string(), result)
                }
            })
            .buffer_unordered(4) // bounded parallelism
            .collect::<HashMap<_, _>>()
            .await;

        results
    }
}
