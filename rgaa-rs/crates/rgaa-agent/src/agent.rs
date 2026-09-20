use crate::config::AgentConfig;
use crate::criteria_defs::VISUAL_CRITERIA;
use crate::error::AgentError;
use crate::prompts::{page_discovery_preamble, PromptBuilder};
use crate::ratelimit::{ModelTier, Ratelimiter};
use crate::verify::map_verdict;
use rgaa_core::{Classification, Criterion, CriterionResult, CriterionStatus};
use rgaa_holo::{HoloClient, HoloResponse, PageContext};
use rgaa_spider::SpiderTool;
use rig_agent::agent::Agent;
use rig_agent::client::AgentClientExt;
use rig_agent::completion::Prompt;
use rig_core::providers::openai;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

/// Consecutive Holo3 call failures (across the whole shared agent, not just
/// one audit) before the circuit breaker trips and further calls fail loud
/// instead of being attempted.
const CIRCUIT_BREAKER_THRESHOLD: u32 = 5;

/// Picks the model tier for `criterion_id`: criteria that need visual
/// understanding (see [`VISUAL_CRITERIA`]) route to the reasoning tier,
/// everything else to the cheaper tactical tier.
fn tier_for(criterion_id: &str) -> ModelTier {
    if VISUAL_CRITERIA.contains(&criterion_id) {
        ModelTier::Reasoning
    } else {
        ModelTier::Tactical
    }
}

/// RGAA agentic evaluator for IA-assistée criteria.
///
/// Uses a single Holo3 model with token-bucket rate limiting, tiered by
/// [`tier_for`]. Conversation memory and vector retrieval are available via
/// [`LanceDbMemory`] and [`LanceDbVectorStore`] but are not yet
/// integrated into the evaluation path.
#[derive(Clone)]
pub struct RgaaAgent {
    agent: Agent,
    rate_limiter: Arc<Ratelimiter>,
    /// Shared across every clone (and every concurrent task spawned from
    /// `run_ia_assiste`/`run_partially_automatable`) so a real outage trips
    /// the breaker for the whole audit, not just one task's local retries.
    consecutive_failures: Arc<AtomicU32>,
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
        let rate_limiter = Arc::new(Ratelimiter::new(config.tactical_rpm, config.reasoning_rpm));

        // 3. Build agent with preamble and spider tool
        let agent = client
            .agent(config.model.as_str())
            .preamble(
                "You are an RGAA accessibility expert. Evaluate criteria and provide verdicts.",
            )
            .append_preamble(&page_discovery_preamble())
            .tool(SpiderTool::new())
            .build();

        Ok(Self {
            agent,
            rate_limiter,
            consecutive_failures: Arc::new(AtomicU32::new(0)),
        })
    }

    /// True when the shared circuit breaker is open — a real Holo3 outage has
    /// already been observed, so further calls fail loud instead of piling
    /// more failed requests (and NeedsReview filler) onto a dead upstream.
    fn breaker_open(&self) -> bool {
        self.consecutive_failures.load(Ordering::Acquire) >= CIRCUIT_BREAKER_THRESHOLD
    }

    fn record_success(&self) {
        self.consecutive_failures.store(0, Ordering::Release);
    }

    /// Records a failure and returns the new consecutive-failure count.
    fn record_failure(&self) -> u32 {
        self.consecutive_failures.fetch_add(1, Ordering::AcqRel) + 1
    }

    /// Evaluates a single IA-assistée criterion against the given page context.
    ///
    /// Renders `page_context` and builds the evaluator prompt with
    /// [`PromptBuilder`]; prefer [`Self::run_ia_assiste`] when evaluating
    /// several criteria against the same page, which renders the context
    /// once and reuses it. Queries the Holo3 model on the tier [`tier_for`]
    /// picks for this criterion, and maps the structured [`HoloResponse`] to
    /// a [`CriterionStatus`] via [`map_verdict`]. On model failure the
    /// criterion is flagged [`CriterionStatus::NeedsReview`] with the error
    /// captured in the justification; if the shared circuit breaker is
    /// already open, no call is attempted and the criterion is flagged
    /// [`CriterionStatus::Error`] instead.
    #[tracing::instrument(skip_all)]
    pub async fn evaluate_criterion(
        &self,
        criterion: &Criterion,
        page_context: &PageContext,
    ) -> CriterionResult {
        let rendered_context = PromptBuilder::render_context(page_context);
        self.evaluate_criterion_rendered(criterion, &rendered_context)
            .await
    }

    /// As [`Self::evaluate_criterion`], but takes an already-rendered page
    /// context (see [`PromptBuilder::render_context`]) instead of rendering
    /// it fresh — the shape `run_ia_assiste` uses to render once per URL.
    async fn evaluate_criterion_rendered(
        &self,
        criterion: &Criterion,
        rendered_context: &str,
    ) -> CriterionResult {
        if self.breaker_open() {
            tracing::warn!(
                criterion = criterion.id,
                "circuit breaker open; skipping Holo3 call"
            );
            return CriterionResult {
                criterion_id: criterion.id.to_string(),
                title: criterion.title.to_string(),
                classification: Classification::IaAssiste,
                status: CriterionStatus::Error,
                violations: vec![],
                confidence: None,
                justification: Some(
                    "Circuit breaker open: too many consecutive Holo3 failures".to_string(),
                ),
                source: "agent-circuit-breaker".to_string(),
                citations: vec![],
            };
        }

        let prompt = PromptBuilder::build_from_rendered(criterion.id, rendered_context);

        self.rate_limiter.acquire(tier_for(criterion.id)).await;

        match self.agent.prompt(prompt.as_str()).await {
            Ok(response) => {
                self.record_success();
                let parsed = HoloClient::extract_json(&response).unwrap_or_else(|| HoloResponse {
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
                    citations: vec![],
                }
            }
            Err(e) => {
                let failures = self.record_failure();
                if failures >= CIRCUIT_BREAKER_THRESHOLD {
                    tracing::warn!(
                        consecutive_failures = failures,
                        "Holo3 circuit breaker tripped"
                    );
                }
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
                    citations: vec![],
                }
            }
        }
    }

    /// Evaluates every criterion in `criteria`, returning a result map keyed by
    /// criterion id.
    ///
    /// Renders `page_context` once and reuses it across every criterion
    /// (rather than re-rendering per criterion) since all of them evaluate
    /// the same page. Uses bounded concurrency with the internal rate
    /// limiter, tiered per criterion by [`tier_for`], to avoid overwhelming
    /// the Holo3 API while keeping evaluations parallel.
    pub async fn run_ia_assiste(
        &self,
        criteria: &[Criterion],
        page_context: &PageContext,
    ) -> HashMap<String, CriterionResult> {
        use futures::stream::{self, StreamExt};

        let rendered_context = Arc::new(PromptBuilder::render_context(page_context));
        let results = stream::iter(criteria.iter().cloned())
            .map(|criterion| {
                let self_ = Arc::new(self.clone());
                let rendered_context = rendered_context.clone();
                async move {
                    let result = self_
                        .evaluate_criterion_rendered(&criterion, &rendered_context)
                        .await;
                    (criterion.id.to_string(), result)
                }
            })
            .buffer_unordered(4) // bounded parallelism
            .collect::<HashMap<_, _>>()
            .await;

        results
    }

    /// Evaluates PartiallyAutomatable criteria that require human review.
    ///
    /// Unlike IA_ASSISTE criteria which may be fully machine-evaluated,
    /// PartiallyAutomatable criteria need human judgment on the portions
    /// not covered by automated checks. Results are marked [`CriterionStatus::NeedsReview`].
    pub async fn run_partially_automatable(
        &self,
        criteria: &[Criterion],
        page_context: &PageContext,
    ) -> HashMap<String, CriterionResult> {
        use futures::stream::{self, StreamExt};

        let rendered_context = Arc::new(PromptBuilder::render_context(page_context));
        let results = stream::iter(criteria.iter().cloned())
            .map(|criterion| {
                let self_ = Arc::new(self.clone());
                let rendered_context = rendered_context.clone();
                async move {
                    let result = self_
                        .evaluate_criterion_for_human_review(&criterion, &rendered_context)
                        .await;
                    (criterion.id.to_string(), result)
                }
            })
            .buffer_unordered(4)
            .collect::<HashMap<_, _>>()
            .await;

        results
    }

    /// Evaluates a criterion where human review is required, from an
    /// already-rendered page context — see [`PromptBuilder::render_context`].
    async fn evaluate_criterion_for_human_review(
        &self,
        criterion: &Criterion,
        rendered_context: &str,
    ) -> CriterionResult {
        if self.breaker_open() {
            tracing::warn!(
                criterion = criterion.id,
                "circuit breaker open; skipping Holo3 call"
            );
            return CriterionResult {
                criterion_id: criterion.id.to_string(),
                title: criterion.title.to_string(),
                classification: criterion.classification,
                status: CriterionStatus::Error,
                violations: vec![],
                confidence: None,
                justification: Some(
                    "Circuit breaker open: too many consecutive Holo3 failures".to_string(),
                ),
                source: "agent-circuit-breaker".to_string(),
                citations: vec![],
            };
        }

        let prompt = PromptBuilder::build_from_rendered(criterion.id, rendered_context);

        self.rate_limiter.acquire(tier_for(criterion.id)).await;

        match self.agent.prompt(prompt.as_str()).await {
            Ok(response) => {
                self.record_success();
                let parsed = HoloClient::extract_json(&response).unwrap_or_else(|| HoloResponse {
                    verdict: "na".to_string(),
                    confidence: 0.0,
                    justification: response.clone(),
                });
                let _ = map_verdict(&parsed);
                CriterionResult {
                    criterion_id: criterion.id.to_string(),
                    title: criterion.title.to_string(),
                    classification: criterion.classification,
                    status: CriterionStatus::NeedsReview,
                    violations: vec![],
                    confidence: Some(parsed.confidence),
                    justification: Some(parsed.justification),
                    source: "agent".to_string(),
                    citations: vec![],
                }
            }
            Err(e) => {
                let failures = self.record_failure();
                if failures >= CIRCUIT_BREAKER_THRESHOLD {
                    tracing::warn!(
                        consecutive_failures = failures,
                        "Holo3 circuit breaker tripped"
                    );
                }
                tracing::warn!(criterion = criterion.id, error = %e, "evaluation failed");
                CriterionResult {
                    criterion_id: criterion.id.to_string(),
                    title: criterion.title.to_string(),
                    classification: criterion.classification,
                    status: CriterionStatus::NeedsReview,
                    violations: vec![],
                    confidence: None,
                    justification: Some(format!("Erreur: {e}")),
                    source: "agent-error".to_string(),
                    citations: vec![],
                }
            }
        }
    }
}
