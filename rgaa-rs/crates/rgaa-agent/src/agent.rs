use crate::models::ModelRouter;
use crate::prompts::PromptBuilder;
use rgaa_core::{Classification, Criterion, CriterionResult, CriterionStatus};
use rgaa_holo::PageContext;
use std::collections::HashMap;

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
    /// permit, and builds the evaluation prompt. Currently returns a placeholder
    /// result pending full HoloClient integration.
    ///
    /// # Arguments
    ///
    /// * `criterion` - The criterion to evaluate.
    /// * `page_context` - The page context containing extracted HTML information.
    /// * `_screenshot` - Optional base64-encoded screenshot for visual evaluation.
    ///
    /// # Returns
    ///
    /// A `CriterionResult` with the evaluation outcome (currently a placeholder).
    async fn evaluate_criterion(
        &self,
        criterion: &Criterion,
        page_context: &PageContext,
        _screenshot: Option<&str>,
    ) -> CriterionResult {
        let tier = self.model_router.route_for(criterion.id);

        // Build prompt with criterion definition
        let _prompt = PromptBuilder::build(criterion.id, page_context);

        // Acquire rate limit permit
        self.model_router
            .rate_limiter()
            .acquire(match tier {
                crate::models::SelectedTier::Tactical => crate::ratelimit::ModelTier::Tactical,
                crate::models::SelectedTier::Reasoning => crate::ratelimit::ModelTier::Reasoning,
            })
            .await;

        // TODO: In production, this calls HoloClient::evaluate(prompt)
        // For now, return a placeholder
        CriterionResult {
            criterion_id: criterion.id.to_string(),
            title: criterion.title.to_string(),
            classification: Classification::IaAssiste,
            status: CriterionStatus::NeedsReview,
            violations: vec![],
            confidence: None,
            justification: Some("Agent integration pending".to_string()),
            source: "agent".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::ModelRouter;
    use rgaa_core::RgaaCriteria;
    use rgaa_holo::PageContext;

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

    #[test]
    fn test_agent_creation() {
        let router = ModelRouter::new_placeholder();
        let agent = RgaaAgent::new(router);
        assert!(agent.model_router.rate_limiter().config().tactical_rpm > 0);
    }

    #[tokio::test]
    async fn test_run_ia_assiste_returns_results_for_all_criteria() {
        let router = ModelRouter::new_placeholder();
        let agent = RgaaAgent::new(router);
        let criteria: Vec<Criterion> = vec![
            Criterion {
                id: "1.3",
                title: "Test Criterion 1.3",
                classification: Classification::IaAssiste,
                wcag_refs: "1.1.1",
            },
            Criterion {
                id: "11.2",
                title: "Test Criterion 11.2",
                classification: Classification::IaAssiste,
                wcag_refs: "2.4.6",
            },
        ];
        let context = sample_context();

        let results = agent.run_ia_assiste(&criteria, &context, None).await;

        assert_eq!(results.len(), 2);
        assert!(results.contains_key("1.3"));
        assert!(results.contains_key("11.2"));
    }

    #[tokio::test]
    async fn test_evaluate_criterion_returns_placeholder() {
        let router = ModelRouter::new_placeholder();
        let agent = RgaaAgent::new(router);
        let criterion = Criterion {
            id: "1.3",
            title: "Alternative textuelle pertinente",
            classification: Classification::IaAssiste,
            wcag_refs: "1.1.1, 4.1.2",
        };
        let context = sample_context();

        let result = agent.evaluate_criterion(&criterion, &context, None).await;

        assert_eq!(result.status, CriterionStatus::NeedsReview);
        assert_eq!(result.source, "agent");
        assert!(result.justification.is_some());
    }

    #[tokio::test]
    async fn test_run_ia_assiste_with_ia_assiste_criteria_only() {
        let router = ModelRouter::new_placeholder();
        let agent = RgaaAgent::new(router);
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
        let agent = AgentBuilder::new()
            .model("holo3-1-35b-a3b")
            .max_concurrent(2)
            .build();

        let context = sample_context();
        let criterion = Criterion {
            id: "1.3",
            title: "Test",
            classification: Classification::IaAssiste,
            wcag_refs: "1.1.1",
        };

        let result = agent.evaluate_criterion(&criterion, &context, None).await;
        assert_eq!(result.status, CriterionStatus::NeedsReview);
    }
}
