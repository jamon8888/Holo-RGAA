use crate::models::ModelRouter;
use crate::prompts::PromptBuilder;
use crate::verify::map_verdict;
use rgaa_core::{Classification, Criterion, CriterionResult, CriterionStatus};
use rgaa_holo::PageContext;
use std::collections::HashMap;
use tracing::warn;

/// Configuration for the RgaaAgent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RigAgentConfig {
    /// The model identifier to use for evaluations.
    pub model: String,
    /// Maximum number of concurrent evaluations.
    pub max_concurrent: usize,
    /// Optional filter for specific criteria IDs. If None, all criteria are evaluated.
    pub criteria_filter: Option<Vec<String>>,
}

impl Default for RigAgentConfig {
    fn default() -> Self {
        Self {
            model: "holo3-1-35b-a3b".to_string(),
            max_concurrent: 5,
            criteria_filter: None,
        }
    }
}

/// Builder pattern for constructing RgaaAgent instances.
pub struct AgentBuilder {
    config: RigAgentConfig,
}

impl AgentBuilder {
    /// Creates a new `AgentBuilder` with default configuration.
    pub fn new() -> Self {
        Self {
            config: RigAgentConfig::default(),
        }
    }

    /// Sets the model identifier for evaluations.
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.config.model = model.into();
        self
    }

    /// Sets the maximum number of concurrent evaluations.
    pub fn max_concurrent(mut self, max_concurrent: usize) -> Self {
        self.config.max_concurrent = max_concurrent;
        self
    }

    /// Sets a filter for specific criteria IDs.
    pub fn criteria_filter(mut self, criteria_filter: Vec<String>) -> Self {
        self.config.criteria_filter = Some(criteria_filter);
        self
    }

    /// Returns the configured `RigAgentConfig` without building the agent.
    pub fn build_config(self) -> RigAgentConfig {
        self.config
    }

    /// Builds and returns an `RgaaAgent` with the configured settings.
    #[must_use]
    pub fn build(self) -> RgaaAgent {
        let config = self.config;
        let router = ModelRouter::from_config(
            &config.model,
            config.max_concurrent,
            config.criteria_filter,
        );
        RgaaAgent::new(router)
    }
}

impl Default for AgentBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Creates a new `RgaaAgent` with default settings.
///
/// This is a convenience function equivalent to `AgentBuilder::new().build()`.
#[must_use]
pub fn create_simple_agent() -> RgaaAgent {
    AgentBuilder::new().build()
}

pub struct RgaaAgent {
    model_router: ModelRouter,
}

impl RgaaAgent {
    /// Creates a new `RgaaAgent` with the given model router.
    ///
    /// # Arguments
    ///
    /// * `model_router` - The router that determines which model tier to use for each criterion.
    #[must_use]
    pub fn new(model_router: ModelRouter) -> Self {
        Self { model_router }
    }

    /// Evaluate all IA_ASSISTE criteria sequentially (rate-limited).
    ///
    /// Returns a map of criterion_id → CriterionResult.
    ///
    /// # Arguments
    ///
    /// * `criteria` - The list of criteria to evaluate.
    /// * `page_context` - The page context containing extracted HTML information.
    /// * `_screenshot` - Optional base64-encoded screenshot for visual evaluation.
    ///
    /// # Returns
    ///
    /// A `HashMap` mapping criterion IDs to their evaluation results.
    pub async fn run_ia_assiste(
        &self,
        criteria: &[Criterion],
        page_context: &PageContext,
        _screenshot: Option<&str>,
    ) -> HashMap<String, CriterionResult> {
        let mut results = HashMap::with_capacity(criteria.len());

        for criterion in criteria {
            let result = self
                .evaluate_criterion(criterion, page_context, _screenshot)
                .await;
            results.insert(criterion.id.to_string(), result);
        }

        results
    }

    /// Evaluates a single criterion against the page context.
    ///
    /// Routes the criterion to the appropriate model tier, acquires a rate limit
    /// permit, builds the evaluation prompt, and calls the HoloClient API.
    /// Returns a `CriterionResult` with the parsed verdict, confidence, and
    /// justification. On API failure, falls back to `NeedsReview` status.
    ///
    /// # Arguments
    ///
    /// * `criterion` - The criterion to evaluate.
    /// * `page_context` - The page context containing extracted HTML information.
    /// * `_screenshot` - Optional base64-encoded screenshot for visual evaluation.
    ///
    /// # Returns
    ///
    /// A `CriterionResult` with the evaluation outcome from the LLM.
    async fn evaluate_criterion(
        &self,
        criterion: &Criterion,
        page_context: &PageContext,
        _screenshot: Option<&str>,
    ) -> CriterionResult {
        let tier = self.model_router.route_for(criterion.id);

        // Build prompt with criterion definition
        let prompt = if let Some(img) = _screenshot {
            PromptBuilder::build_with_image(criterion.id, page_context, img)
        } else {
            PromptBuilder::build(criterion.id, page_context)
        };

        // Acquire rate limit permit
        self.model_router
            .rate_limiter()
            .acquire(match tier {
                crate::models::SelectedTier::Tactical => crate::ratelimit::ModelTier::Tactical,
                crate::models::SelectedTier::Reasoning => crate::ratelimit::ModelTier::Reasoning,
            })
            .await;

        // Call HoloClient via the tier-appropriate client
        let client = self.model_router.client_for_tier(tier);

        match client.evaluate_multimodal(&prompt, _screenshot).await {
            Ok(response) => {
                let status = map_verdict(response.clone());
                CriterionResult {
                    criterion_id: criterion.id.to_string(),
                    title: criterion.title.to_string(),
                    classification: Classification::IaAssiste,
                    status,
                    violations: vec![],
                    confidence: Some(response.confidence),
                    justification: Some(response.justification),
                    source: "agent".to_string(),
                }
            }
            Err(e) => {
                warn!(
                    criterion_id = criterion.id,
                    error = %e,
                    "LLM evaluation failed, falling back to NeedsReview"
                );
                CriterionResult {
                    criterion_id: criterion.id.to_string(),
                    title: criterion.title.to_string(),
                    classification: Classification::IaAssiste,
                    status: CriterionStatus::NeedsReview,
                    violations: vec![],
                    confidence: None,
                    justification: Some(format!("LLM evaluation failed: {e}")),
                    source: "agent".to_string(),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::ModelRouter;
    use rgaa_core::RgaaCriteria;
    use rgaa_holo::{HoloClient, PageContext};
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::Arc;

    fn sample_context() -> PageContext {
        PageContext {
            title: Some("Page Test".to_string()),
            lang: Some("fr".to_string()),
            headings: vec![],
            images: vec![],
            iframes: vec![],
            links: vec![],
            forms: vec![],
            media: vec![],
            navigation: vec![],
        }
    }

    fn spawn_mock_server(body: &'static str) -> (String, Arc<std::thread::JoinHandle<()>>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let handle = std::thread::spawn(move || {
            for stream in listener.incoming() {
                match stream {
                    Ok(mut s) => {
                        std::thread::spawn(move || {
                            let mut buf = [0u8; 4096];
                            let _ = s.read(&mut buf);
                            let response = format!(
                                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                                body.len(),
                                body
                            );
                            let _ = s.write_all(response.as_bytes());
                            let _ = s.flush();
                        });
                    }
                    Err(_) => break,
                }
            }
        });
        (addr.to_string(), Arc::new(handle))
    }

    fn agent_with_mock(body: &'static str) -> (RgaaAgent, Arc<std::thread::JoinHandle<()>>) {
        let (addr, handle) = spawn_mock_server(body);
        let client = HoloClient::new("test-key".to_string()).with_base_url(format!("http://{addr}"));
        let router = ModelRouter::new(client.clone(), client, crate::ratelimit::RateLimiter::new(60, 60));
        (RgaaAgent::new(router), handle)
    }

    #[test]
    fn test_agent_creation() {
        let router = ModelRouter::new_placeholder();
        let agent = RgaaAgent::new(router);
        assert!(agent.model_router.rate_limiter().config().tactical_rpm > 0);
    }

    #[tokio::test]
    async fn test_run_ia_assiste_returns_results_for_all_criteria() {
        let (agent, _handle) = agent_with_mock(
            r#"{"verdict":"pass","confidence":0.9,"justification":"ok"}"#,
        );
        let criteria: Vec<Criterion> = vec![
            Criterion {
                id: "2.2",
                title: "Test Criterion 2.2",
                classification: Classification::IaAssiste,
                wcag_refs: "1.3.1",
            },
            Criterion {
                id: "8.6",
                title: "Test Criterion 8.6",
                classification: Classification::IaAssiste,
                wcag_refs: "2.4.2",
            },
        ];
        let context = sample_context();

        let results = agent.run_ia_assiste(&criteria, &context, None).await;

        assert_eq!(results.len(), 2);
        assert!(results.contains_key("2.2"));
        assert!(results.contains_key("8.6"));
    }

    #[tokio::test]
    async fn test_evaluate_criterion_returns_parsed_verdict() {
        let (agent, _handle) = agent_with_mock(
            r#"{"verdict":"pass","confidence":0.95,"justification":"Titre pertinent"}"#,
        );
        let criterion = Criterion {
            id: "8.6",
            title: "Titre de page pertinent",
            classification: Classification::IaAssiste,
            wcag_refs: "2.4.2",
        };
        let context = sample_context();

        let result = agent.evaluate_criterion(&criterion, &context, None).await;
        assert_eq!(result.source, "agent");
        assert!(result.justification.is_some());
        assert_eq!(result.status, CriterionStatus::Pass);
        assert_eq!(result.confidence, Some(0.95));
    }

    #[tokio::test]
    async fn test_evaluate_criterion_fail_verdict() {
        let (agent, _handle) = agent_with_mock(
            r#"{"verdict":"fail","confidence":0.8,"justification":"Pas de titre"}"#,
        );
        let criterion = Criterion {
            id: "8.6",
            title: "Titre de page pertinent",
            classification: Classification::IaAssiste,
            wcag_refs: "2.4.2",
        };
        let context = sample_context();

        let result = agent.evaluate_criterion(&criterion, &context, None).await;
        assert_eq!(result.status, CriterionStatus::Fail);
        assert_eq!(result.confidence, Some(0.8));
    }

    #[tokio::test]
    async fn test_evaluate_criterion_low_confidence_needs_review() {
        let (agent, _handle) = agent_with_mock(
            r#"{"verdict":"pass","confidence":0.3,"justification":"Incertain"}"#,
        );
        let criterion = Criterion {
            id: "2.2",
            title: "Test",
            classification: Classification::IaAssiste,
            wcag_refs: "1.3.1",
        };
        let context = sample_context();

        let result = agent.evaluate_criterion(&criterion, &context, None).await;
        assert_eq!(result.status, CriterionStatus::NeedsReview);
    }

    #[tokio::test]
    async fn test_run_ia_assiste_with_ia_assiste_criteria_only() {
        let (agent, _handle) = agent_with_mock(
            r#"{"verdict":"pass","confidence":0.9,"justification":"ok"}"#,
        );
        let ia_criteria = RgaaCriteria::ia_assiste();
        let context = sample_context();

        let results = agent.run_ia_assiste(&ia_criteria, &context, None).await;

        // Should have one result per IA-assisted criterion
        assert_eq!(results.len(), ia_criteria.len());
        for criterion in &ia_criteria {
            assert!(results.contains_key(criterion.id));
        }
    }

    // === NEW TESTS FOR BUILDER PATTERN AND CONFIG ===

    #[test]
    fn test_rig_agent_config_defaults() {
        let config = RigAgentConfig::default();
        assert_eq!(config.model, "holo3-1-35b-a3b");
        assert_eq!(config.max_concurrent, 5);
        assert!(config.criteria_filter.is_none());
    }

    #[test]
    fn test_rig_agent_config_custom_values() {
        let filter = vec!["1.3".to_string(), "11.2".to_string()];
        let config = RigAgentConfig {
            model: "custom-model".to_string(),
            max_concurrent: 10,
            criteria_filter: Some(filter.clone()),
        };
        assert_eq!(config.model, "custom-model");
        assert_eq!(config.max_concurrent, 10);
        assert_eq!(config.criteria_filter, Some(filter));
    }

    #[test]
    fn test_agent_builder_default_config() {
        let builder = AgentBuilder::new();
        let config = builder.build_config();
        assert_eq!(config, RigAgentConfig::default());
    }

    #[test]
    fn test_agent_builder_chaining() {
        let filter = vec!["1.3".to_string()];
        let config = AgentBuilder::new()
            .model("test-model")
            .max_concurrent(3)
            .criteria_filter(filter.clone())
            .build_config();

        assert_eq!(config.model, "test-model");
        assert_eq!(config.max_concurrent, 3);
        assert_eq!(config.criteria_filter, Some(filter));
    }

    #[test]
    fn test_agent_builder_builds_agent() {
        let agent = AgentBuilder::new().build();
        assert!(agent.model_router.rate_limiter().config().tactical_rpm > 0);
    }

    #[test]
    fn test_create_simple_agent() {
        let agent = create_simple_agent();
        assert!(agent.model_router.rate_limiter().config().tactical_rpm > 0);
    }

    #[tokio::test]
    async fn test_agent_builder_builds_working_agent() {
        let (agent, _handle) = agent_with_mock(
            r#"{"verdict":"pass","confidence":0.9,"justification":"ok"}"#,
        );

        let context = sample_context();
        let criterion = Criterion {
            id: "2.2",
            title: "Test",
            classification: Classification::IaAssiste,
            wcag_refs: "1.3.1",
        };

        let result = agent.evaluate_criterion(&criterion, &context, None).await;
        assert_eq!(result.status, CriterionStatus::Pass);
    }
}
