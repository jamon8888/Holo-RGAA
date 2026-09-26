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
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Consecutive Holo3 call failures (across the whole shared agent, not just
/// one audit) before the circuit breaker trips and further calls fail loud
/// instead of being attempted.
const CIRCUIT_BREAKER_THRESHOLD: u32 = 5;

/// How long the breaker stays open before letting a single trial call
/// through again (half-open). A transient blip (an API-side 404, a dropped
/// connection, a local backend hiccup) shouldn't permanently fail the rest
/// of a multi-hour audit — only a trial call that *also* fails re-opens it
/// for another cooldown window.
const CIRCUIT_BREAKER_COOLDOWN: Duration = Duration::from_secs(60);

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

/// Batch evaluation response from the LLM
#[derive(Deserialize, Debug, Clone)]
struct BatchEvaluationResponse {
    /// Echoed back by the model — the batch prompt asks for it explicitly.
    /// Results are matched on this, never on array position: a model that
    /// reorders or omits an element would otherwise have its verdict recorded
    /// against a different RGAA criterion.
    #[serde(default)]
    criterion_id: String,
    verdict: String,
    confidence: f64,
    justification: String,
}

/// Extracts the outermost JSON array from `text`.
///
/// Models routinely wrap the array in a ```json fence or a sentence of prose,
/// which makes a bare `serde_json::from_str` on the whole reply fail.
fn extract_json_array(text: &str) -> Option<&str> {
    let start = text.find('[')?;
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    for (offset, c) in text[start..].char_indices() {
        if in_string {
            match c {
                _ if escaped => escaped = false,
                '\\' => escaped = true,
                '"' => in_string = false,
                _ => {}
            }
            continue;
        }
        match c {
            '"' => in_string = true,
            '[' => depth += 1,
            ']' => {
                depth -= 1;
                if depth == 0 {
                    return Some(&text[start..=start + offset]);
                }
            }
            _ => {}
        }
    }
    None
}

/// Number of criteria to evaluate per batch LLM call.
/// Default: 5 (balances prompt size vs. number of API calls).
const BATCH_SIZE: usize = 5;

/// RGAA agentic evaluator for IA-assistée criteria.
///
/// Holds one agent per [`ModelTier`] — the tactical tier for most criteria,
/// the reasoning tier for the visual/hard ones [`tier_for`] routes there —
/// each on the model its `RGAA_LLM_MODEL_TACTICAL` / `RGAA_LLM_MODEL_REASONING`
/// variable names, both against the provider resolved by
/// [`rgaa_core::LlmSettings`]. When neither tier is overridden the two
/// agents run the same model, which is the single-model setup this used to
/// hardcode. Token-bucket rate limiting is applied per tier. Conversation
/// memory and vector retrieval are available via [`LanceDbMemory`] and
/// [`LanceDbVectorStore`] but are not yet integrated into the evaluation path.
#[derive(Clone)]
pub struct RgaaAgent {
    /// Agent for [`ModelTier::Tactical`].
    tactical: Agent,
    /// Agent for [`ModelTier::Reasoning`]; the same model as `tactical`
    /// unless the operator set a reasoning-specific one.
    reasoning: Agent,
    rate_limiter: Arc<Ratelimiter>,
    agent_concurrency: usize,
    /// Shared across every clone (and every concurrent task spawned from
    /// `run_ia_assiste`/`run_partially_automatable`) so a real outage trips
    /// the breaker for the whole audit, not just one task's local retries.
    consecutive_failures: Arc<AtomicU32>,
    /// When the breaker last tripped (or last re-tripped on a failed
    /// half-open trial); `None` while closed. Gates the half-open retry in
    /// [`Self::breaker_open`].
    tripped_at: Arc<Mutex<Option<Instant>>>,
}

impl RgaaAgent {
    /// Builds the agent and rate limiter.
    ///
    /// # Errors
    /// Returns [`AgentError`] if the OpenAI-compatible client or the rate
    /// limiter fails to initialize.
    #[tracing::instrument(skip_all)]
    pub async fn new(config: &AgentConfig) -> Result<Self, AgentError> {
        if config.agent_concurrency == 0 {
            return Err(AgentError::Config("agent_concurrency must be > 0".into()));
        }
        // 1. One OpenAI-compatible client for the configured provider —
        //    Holo3, OpenAI, Groq, a local Ollama, anything in
        //    `rgaa_core::PROVIDERS`. Both tiers share it: they differ by
        //    model, not by endpoint.
        let client = openai::Client::builder()
            .base_url(&config.base_url)
            .api_key(&config.api_key)
            // Without an explicit HTTP backend, rig builds a default
            // `reqwest::Client` with no timeout of its own, so the resolved
            // `RGAA_LLM_TIMEOUT_SECS` (and the 600s local default that slow
            // CPU inference needs) would never reach the wire.
            .http_client(config.http_client()?)
            .build()
            .map_err(|e| AgentError::RigAgent(e.to_string()))?
            .completions_api();

        // 2. Create rate limiter from config (tactical/reasoning RPM)
        let rate_limiter = Arc::new(Ratelimiter::new(config.tactical_rpm, config.reasoning_rpm));

        // 3. Build one agent per tier. When both tiers resolve to the same
        //    model this builds the same agent twice, which is cheap — no
        //    request is issued until `prompt`.
        let build_agent = |model: &str| {
            client
                .agent(model)
                .preamble(
                    "You are an RGAA accessibility expert. Evaluate criteria and provide verdicts.",
                )
                .append_preamble(&page_discovery_preamble())
                .tool(SpiderTool::new())
                // Without this, rig-agent's implicit budget is a single model
                // call (see rig-agent's `default_max_turns` docs); a model that
                // reaches for `crawl_site` instead of answering directly then
                // has no turn left to read the tool result and produce a
                // verdict, and fails with MaxTurnsError. 3 turns covers one
                // tool call plus the follow-up answer, with a little slack.
                .default_max_turns(3)
                .build()
        };

        tracing::info!(
            provider = %config.provider,
            base_url = %config.base_url,
            model_tactical = %config.model_tactical(),
            model_reasoning = %config.model_reasoning(),
            "LLM route configured"
        );

        Ok(Self {
            tactical: build_agent(config.model_tactical()),
            reasoning: build_agent(config.model_reasoning()),
            rate_limiter,
            // Floored at 1 independently of `AgentConfig::from_env`, which
            // already drops a zero: the field is public, so a hand-built
            // config could still carry one, and `buffer_unordered(0)` never
            // polls its source stream — the audit would hang rather than fail.
            agent_concurrency: config.agent_concurrency.max(1),
            consecutive_failures: Arc::new(AtomicU32::new(0)),
            tripped_at: Arc::new(Mutex::new(None)),
        })
    }

    /// The agent bound to `tier`'s model.
    fn agent_for(&self, tier: ModelTier) -> &Agent {
        match tier {
            ModelTier::Tactical => &self.tactical,
            ModelTier::Reasoning => &self.reasoning,
        }
    }

    /// True when the shared circuit breaker is open — a real Holo3 outage has
    /// already been observed, so further calls fail loud instead of piling
    /// more failed requests (and NeedsReview filler) onto a dead upstream.
    ///
    /// The breaker is half-open once [`CIRCUIT_BREAKER_COOLDOWN`] has
    /// elapsed since it tripped: this returns `false` for a single trial
    /// call (racing concurrent callers may each see `false` and each spend
    /// a call, which is acceptable — `buffer_unordered` callers run at
    /// concurrency 1), and [`Self::record_failure`] re-arms the cooldown if
    /// that trial also fails.
    fn breaker_open(&self) -> bool {
        if self.consecutive_failures.load(Ordering::Acquire) < CIRCUIT_BREAKER_THRESHOLD {
            return false;
        }
        let tripped_at = *self.tripped_at.lock().expect("tripped_at mutex poisoned");
        match tripped_at {
            Some(t) => t.elapsed() < CIRCUIT_BREAKER_COOLDOWN,
            None => false,
        }
    }

    fn record_success(&self) {
        self.consecutive_failures.store(0, Ordering::Release);
        *self.tripped_at.lock().expect("tripped_at mutex poisoned") = None;
    }

    /// Records a failure and returns the new consecutive-failure count.
    fn record_failure(&self) -> u32 {
        let failures = self.consecutive_failures.fetch_add(1, Ordering::AcqRel) + 1;
        if failures >= CIRCUIT_BREAKER_THRESHOLD {
            *self.tripped_at.lock().expect("tripped_at mutex poisoned") = Some(Instant::now());
        }
        failures
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

        let tier = tier_for(criterion.id);
        self.rate_limiter.acquire(tier).await;

        match self.agent_for(tier).prompt(prompt.as_str()).await {
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
    ///
    /// Criteria are evaluated in batches of `BATCH_SIZE` to reduce the number
    /// of LLM API calls.
    pub async fn run_ia_assiste(
        self: std::sync::Arc<Self>,
        criteria: Vec<Criterion>,
        page_context: PageContext,
    ) -> HashMap<String, CriterionResult> {
        use futures::stream::{self, StreamExt};

        let rendered_context = Arc::new(PromptBuilder::render_context(&page_context));

        // Grouped by tier before chunking, so a batch never mixes tiers and
        // `tier_for`'s routing survives: forcing `Tactical` on every batch sent
        // the visual criteria to the cheap model and billed the wrong bucket.
        let mut tactical: Vec<Criterion> = Vec::new();
        let mut reasoning: Vec<Criterion> = Vec::new();
        for criterion in criteria {
            match tier_for(criterion.id) {
                ModelTier::Tactical => tactical.push(criterion),
                ModelTier::Reasoning => reasoning.push(criterion),
            }
        }

        let batches: Vec<(Vec<Criterion>, ModelTier)> = tactical
            .chunks(BATCH_SIZE)
            .map(|c| (c.to_vec(), ModelTier::Tactical))
            .chain(
                reasoning
                    .chunks(BATCH_SIZE)
                    .map(|c| (c.to_vec(), ModelTier::Reasoning)),
            )
            .collect();

        // Batches run concurrently up to `agent_concurrency` — awaiting them
        // one by one meant the configured concurrency was never used on this
        // path, which is the whole point of batching here.
        let concurrency = self.agent_concurrency;
        stream::iter(batches)
            .map(|(batch, tier)| {
                let self_ = self.clone();
                let rendered_context = rendered_context.clone();
                async move { self_.evaluate_batch(batch, rendered_context, tier).await }
            })
            .buffer_unordered(concurrency)
            .fold(HashMap::new(), |mut acc, batch_results| async move {
                acc.extend(batch_results);
                acc
            })
            .await
    }

    /// Evaluate a batch of criteria with a single LLM call
    async fn evaluate_batch(
        self: std::sync::Arc<Self>,
        criteria: Vec<Criterion>,
        rendered_context: Arc<String>,
        tier: ModelTier,
    ) -> HashMap<String, CriterionResult> {
        if criteria.is_empty() {
            return HashMap::new();
        }
        let criterion_ids: Vec<&str> = criteria.iter().map(|c| c.id).collect();

        // Build batch prompt
        let prompt = PromptBuilder::build_batch_from_rendered(&criterion_ids, &rendered_context);

        // The breaker gates the batch path too: without this, every batch of
        // a dead upstream spent a request and only the per-criterion fallback
        // ever tripped it, so other callers kept hammering it.
        if self.breaker_open() {
            tracing::warn!(criteria = ?criterion_ids, "circuit breaker open; skipping batch call");
            return self.evaluate_individually(criteria, rendered_context).await;
        }

        // Rate limit
        self.rate_limiter.acquire(tier).await;

        // Call LLM
        let response = match self.agent_for(tier).prompt(prompt.as_str()).await {
            Ok(response) => {
                self.record_success();
                response
            }
            Err(e) => {
                let failures = self.record_failure();
                if failures >= CIRCUIT_BREAKER_THRESHOLD {
                    tracing::warn!(consecutive_failures = failures, "circuit breaker tripped");
                }
                tracing::warn!(criteria = ?criterion_ids, error = %e, "batch evaluation failed");
                // Fall back to individual evaluation on error
                return self.evaluate_individually(criteria, rendered_context).await;
            }
        };

        // Parse the batch response, keyed by the `criterion_id` the prompt
        // asks the model to echo. The array is extracted from the reply first
        // because models wrap it in a ```json fence or a line of prose.
        let batch_responses: Vec<BatchEvaluationResponse> = extract_json_array(&response)
            .and_then(|array| serde_json::from_str(array).ok())
            .or_else(|| serde_json::from_str(&response).ok())
            .unwrap_or_default();

        let mut by_id: HashMap<&str, &BatchEvaluationResponse> = HashMap::new();
        let mut duplicated: Vec<&str> = Vec::new();
        for r in &batch_responses {
            let id = r.criterion_id.trim();
            if id.is_empty() {
                continue;
            }
            if by_id.insert(id, r).is_some() {
                // Two results for one criterion: neither can be trusted, so
                // the criterion goes to the individual path below.
                duplicated.push(id);
            }
        }
        for id in duplicated {
            by_id.remove(id);
        }

        // Map responses to results
        let mut results = HashMap::new();
        let mut unmatched: Vec<Criterion> = Vec::new();
        for criterion in &criteria {
            let Some(response) = by_id.get(criterion.id) else {
                // No usable result for this criterion. Evaluating it on its own
                // is the only honest option — defaulting it to "na" would
                // record a verdict the model never gave.
                unmatched.push(criterion.clone());
                continue;
            };

            let status = map_verdict(&HoloResponse {
                verdict: response.verdict.clone(),
                confidence: response.confidence,
                justification: response.justification.clone(),
            });

            results.insert(
                criterion.id.to_string(),
                CriterionResult {
                    criterion_id: criterion.id.to_string(),
                    title: criterion.title.clone(),
                    classification: criterion.classification,
                    status,
                    violations: vec![],
                    confidence: Some(response.confidence),
                    justification: Some(response.justification.clone()),
                    source: "agent-batch".to_string(),
                    citations: vec![],
                },
            );
        }

        if !unmatched.is_empty() {
            tracing::warn!(
                missing = unmatched.len(),
                of = criteria.len(),
                "batch response did not cover every criterion; evaluating the rest individually"
            );
            let fallback = self
                .evaluate_individually(unmatched, rendered_context)
                .await;
            results.extend(fallback);
        }

        results
    }

    /// Fallback: evaluate criteria individually when batch fails
    async fn evaluate_individually(
        self: std::sync::Arc<Self>,
        criteria: Vec<Criterion>,
        rendered_context: Arc<String>,
    ) -> HashMap<String, CriterionResult> {
        use futures::stream::{self, StreamExt};

        let results = stream::iter(criteria)
            .map(|criterion| {
                let self_ = self.clone();
                let rendered_context = rendered_context.clone();
                let criterion_id = criterion.id;
                async move {
                    let result = self_
                        .evaluate_criterion_rendered(&criterion, &rendered_context)
                        .await;
                    (criterion_id.to_string(), result)
                }
            })
            .buffer_unordered(1) // Sequential for fallback
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
        self: std::sync::Arc<Self>,
        criteria: Vec<Criterion>,
        page_context: PageContext,
    ) -> HashMap<String, CriterionResult> {
        use futures::stream::{self, StreamExt};

        let rendered_context = Arc::new(PromptBuilder::render_context(&page_context));
        let concurrency = self.agent_concurrency;
        let results = stream::iter(criteria)
            .map(|criterion| {
                let self_ = self.clone();
                let rendered_context = rendered_context.clone();
                async move {
                    let result = self_
                        .evaluate_criterion_for_human_review(&criterion, &rendered_context)
                        .await;
                    (criterion.id.to_string(), result)
                }
            })
            .buffer_unordered(concurrency)
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

        let tier = tier_for(criterion.id);
        self.rate_limiter.acquire(tier).await;

        match self.agent_for(tier).prompt(prompt.as_str()).await {
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

#[cfg(test)]
mod batch_tests {
    use super::*;

    fn parse(text: &str) -> Vec<BatchEvaluationResponse> {
        extract_json_array(text)
            .and_then(|a| serde_json::from_str(a).ok())
            .or_else(|| serde_json::from_str(text).ok())
            .unwrap_or_default()
    }

    #[test]
    fn extracts_the_array_from_a_fenced_reply() {
        // The usual shape of a model reply: prose, a ```json fence, more prose.
        let reply = "Voici mon analyse :\n```json\n[{\"criterion_id\":\"1.1\",\
                     \"verdict\":\"pass\",\"confidence\":0.9,\"justification\":\"ok\"}]\n```\nFin.";
        let parsed = parse(reply);
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].criterion_id, "1.1");
        assert_eq!(parsed[0].verdict, "pass");
    }

    #[test]
    fn extracts_a_bare_array_too() {
        let reply =
            r#"[{"criterion_id":"3.1","verdict":"fail","confidence":0.5,"justification":"x"}]"#;
        assert_eq!(parse(reply)[0].criterion_id, "3.1");
    }

    #[test]
    fn a_bracket_inside_a_string_does_not_end_the_array() {
        let reply = r#"[{"criterion_id":"1.1","verdict":"fail","confidence":0.5,
                        "justification":"le texte ] et [ sont cités"}]"#;
        let parsed = parse(reply);
        assert_eq!(parsed.len(), 1);
        assert!(parsed[0].justification.contains(']'));
    }

    #[test]
    fn nothing_parsable_yields_no_responses_rather_than_a_wrong_one() {
        assert!(parse("Je n'ai pas pu évaluer ces critères.").is_empty());
        assert!(parse("").is_empty());
    }

    /// The mapping rule this whole rewrite exists for: a reordered or partial
    /// batch must never land a verdict on the wrong criterion.
    #[test]
    fn responses_are_matched_by_id_not_by_position() {
        let reply = r#"[
            {"criterion_id":"3.1","verdict":"fail","confidence":0.8,"justification":"contraste"},
            {"criterion_id":"1.1","verdict":"pass","confidence":0.9,"justification":"alt ok"}
        ]"#;
        let parsed = parse(reply);
        let by_id: HashMap<&str, &BatchEvaluationResponse> = parsed
            .iter()
            .map(|r| (r.criterion_id.as_str(), r))
            .collect();

        // Positional mapping would have given 1.1 the "fail" meant for 3.1.
        assert_eq!(by_id["1.1"].verdict, "pass");
        assert_eq!(by_id["3.1"].verdict, "fail");
    }

    #[test]
    fn a_criterion_absent_from_the_response_has_no_entry() {
        // It must fall through to individual evaluation, not be defaulted to
        // "na" — a verdict the model never gave.
        let reply =
            r#"[{"criterion_id":"1.1","verdict":"pass","confidence":0.9,"justification":"ok"}]"#;
        let parsed = parse(reply);
        let by_id: HashMap<&str, &BatchEvaluationResponse> = parsed
            .iter()
            .map(|r| (r.criterion_id.as_str(), r))
            .collect();
        assert!(by_id.contains_key("1.1"));
        assert!(!by_id.contains_key("3.1"));
    }

    #[test]
    fn criteria_split_into_batches_keep_their_tier() {
        // 1.1 is textual, 3.1 needs the visual model: grouping by tier first is
        // what stops a batch from being billed and answered on the wrong one.
        assert_eq!(tier_for("3.1"), ModelTier::Reasoning);
        assert_eq!(tier_for("1.1"), ModelTier::Tactical);
    }
}
