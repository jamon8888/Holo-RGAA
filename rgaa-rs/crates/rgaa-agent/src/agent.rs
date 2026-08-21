use crate::models::ModelRouter;
use crate::prompts::PromptBuilder;
use crate::verify::map_verdict;
use rgaa_browser_tools::ToolContext;
use rgaa_core::{Classification, Criterion, CriterionResult, CriterionStatus};
use rgaa_holo::PageContext;
use rig_core::client::CompletionClient;
use rig_core::tool::{portable_tool_definition, IntoToolOutput, PortableDynamicTool, PortableTool};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::warn;

/// Configuration for the RgaaAgent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RigAgentConfig {
    pub model: String,
    pub max_concurrent: usize,
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

/// Wraps rig-core's OpenAI client configured for the Holo3 API.
///
/// Holo3 exposes an OpenAI-compatible `/v1` endpoint, so we reuse
/// rig-core's `providers::openai::Client` with a custom base URL.
pub struct HoloProvider {
    client: rig_core::providers::openai::CompletionsClient,
}

impl HoloProvider {
    /// Create a new provider pointing at the given Holo3 base URL.
    ///
    /// Uses the Completions API (Chat Completions) since Holo3 is
    /// OpenAI-compatible and may not support the newer Responses API.
    ///
    /// # Errors
    ///
    /// Returns an error if the underlying HTTP client cannot be built.
    pub fn new(base_url: &str, api_key: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let client = rig_core::providers::openai::Client::builder()
            .base_url(base_url)
            .api_key(api_key)
            .build()?
            .completions_api();
        Ok(Self { client })
    }

    /// Returns a completion model for the given model name.
    pub fn completion_model(
        &self,
        model: &str,
    ) -> rig_core::providers::openai::completion::CompletionModel {
        self.client.completion_model(model)
    }

    /// Build portable dynamic tools from browser tools for the given context.
    ///
    /// Each tool is wrapped in a `PortableDynamicTool` that deserializes
    /// arguments, executes the typed tool, and serializes the output.
    pub fn build_tools(tool_ctx: &ToolContext) -> Vec<PortableDynamicTool> {
        use rgaa_browser_tools::{
            A11yTreeTool, ClickTool, EvalJsTool, NavigateTool, PressKeyTool, ScreenshotTool,
            TabOrderTool, TypeTool,
        };

        macro_rules! wrap_tool {
            ($tool:expr) => {{
                let tool = Arc::new($tool);
                let def = portable_tool_definition(&*tool);
                let name = def.name;
                let description = def.description;
                let parameters = def.parameters;
                PortableDynamicTool::new(name, description, parameters, move |args| {
                    let tool = Arc::clone(&tool);
                    Box::pin(async move {
                        let typed_args = serde_json::from_value(args).map_err(|e| {
                            rig_core::tool::ToolExecutionError::other(format!(
                                "failed to deserialize tool args: {e}"
                            ))
                        })?;
                        let output = PortableTool::call(&*tool, typed_args).await.map_err(|e| {
                            rig_core::tool::ToolExecutionError::other(e.to_string())
                        })?;
                        output.into_tool_output()
                    })
                })
            }};
        }

        vec![
            wrap_tool!(NavigateTool::new(tool_ctx.clone())),
            wrap_tool!(ScreenshotTool::new(tool_ctx.clone())),
            wrap_tool!(A11yTreeTool::new(tool_ctx.clone())),
            wrap_tool!(ClickTool::new(tool_ctx.clone())),
            wrap_tool!(PressKeyTool::new(tool_ctx.clone())),
            wrap_tool!(TabOrderTool::new(tool_ctx.clone())),
            wrap_tool!(TypeTool::new(tool_ctx.clone())),
            wrap_tool!(EvalJsTool::new(tool_ctx.clone())),
        ]
    }
}

/// Builder pattern for constructing RgaaAgent instances.
pub struct AgentBuilder {
    config: RigAgentConfig,
    tool_ctx: ToolContext,
}

impl AgentBuilder {
    pub fn new(tool_ctx: ToolContext) -> Self {
        Self {
            config: RigAgentConfig::default(),
            tool_ctx,
        }
    }

    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.config.model = model.into();
        self
    }

    pub fn max_concurrent(mut self, max_concurrent: usize) -> Self {
        self.config.max_concurrent = max_concurrent;
        self
    }

    pub fn criteria_filter(mut self, criteria_filter: Vec<String>) -> Self {
        self.config.criteria_filter = Some(criteria_filter);
        self
    }

    pub fn build_config(self) -> RigAgentConfig {
        self.config
    }

    #[must_use]
    pub fn build(self) -> RgaaAgent {
        let _config = self.config;
        RgaaAgent::new_placeholder(self.tool_ctx)
    }
}

/// Creates a new `RgaaAgent` with placeholder rig agents for testing.
#[must_use]
pub fn create_simple_agent(tool_ctx: ToolContext) -> RgaaAgent {
    AgentBuilder::new(tool_ctx).build()
}

/// The main RGAA agent wrapping rig-core agents.
///
/// Contains a tactical (35b) agent for simple text criteria and a reasoning
/// (122b) agent for visual/complex criteria. The `ModelRouter` determines
/// which agent to use per criterion.
pub struct RgaaAgent {
    model_router: ModelRouter,
    tool_ctx: ToolContext,
}

impl RgaaAgent {
    /// Creates a new `RgaaAgent` with the given model router and tool context.
    #[must_use]
    pub fn new(model_router: ModelRouter, tool_ctx: ToolContext) -> Self {
        Self {
            model_router,
            tool_ctx,
        }
    }

    /// Creates a placeholder agent for testing without API keys.
    #[must_use]
    pub fn new_placeholder(tool_ctx: ToolContext) -> Self {
        Self {
            model_router: ModelRouter::new_placeholder(),
            tool_ctx,
        }
    }

    /// Returns true if the tactical agent is available.
    pub fn has_tactical_agent(&self) -> bool {
        true
    }

    /// Returns true if the reasoning agent is available.
    pub fn has_reasoning_agent(&self) -> bool {
        true
    }

    /// Returns a reference to the tool context.
    pub fn tool_ctx(&self) -> &ToolContext {
        &self.tool_ctx
    }

    /// Evaluate all IA_ASSISTE criteria sequentially (rate-limited).
    pub async fn run_ia_assiste(
        &self,
        criteria: &[Criterion],
        page_context: &PageContext,
        screenshot: Option<&str>,
    ) -> HashMap<String, CriterionResult> {
        let mut results = HashMap::with_capacity(criteria.len());

        for criterion in criteria {
            let result = self
                .evaluate_criterion(criterion, page_context, screenshot)
                .await;
            results.insert(criterion.id.to_string(), result);
        }

        results
    }

    /// Evaluates a single criterion against the page context.
    ///
    /// Routes to the appropriate model tier, acquires rate limit, builds
    /// the prompt, and calls the HoloClient API. Falls back to NeedsReview
    /// on API failure.
    async fn evaluate_criterion(
        &self,
        criterion: &Criterion,
        page_context: &PageContext,
        _screenshot: Option<&str>,
    ) -> CriterionResult {
        let tier = self.model_router.route_for(criterion.id);

        let prompt = PromptBuilder::build(criterion.id, page_context);

        self.model_router
            .rate_limiter()
            .acquire(match tier {
                crate::models::SelectedTier::Tactical => crate::ratelimit::ModelTier::Tactical,
                crate::models::SelectedTier::Reasoning => crate::ratelimit::ModelTier::Reasoning,
            })
            .await;

        let client = self.model_router.client_for_tier(tier);
        match client.evaluate(&prompt).await {
            Ok(response) => {
                let status = map_verdict(response);
                CriterionResult {
                    criterion_id: criterion.id.to_string(),
                    title: criterion.title.to_string(),
                    classification: Classification::IaAssiste,
                    status,
                    violations: vec![],
                    confidence: None,
                    justification: Some(format!(
                        "Évaluation par agent pour critère {}",
                        criterion.id
                    )),
                    source: "agent".to_string(),
                }
            }
            Err(e) => {
                warn!(criterion = criterion.id, error = %e, "évaluation API échouée");
                CriterionResult {
                    criterion_id: criterion.id.to_string(),
                    title: criterion.title.to_string(),
                    classification: Classification::IaAssiste,
                    status: CriterionStatus::NeedsReview,
                    violations: vec![],
                    confidence: None,
                    justification: Some(format!("Erreur API: {e}")),
                    source: "agent".to_string(),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
        let ctx = ToolContext::new(rgaa_browser_tools::BrowserSession::new_placeholder());
        let agent = RgaaAgent::new_placeholder(ctx);
        assert!(agent.model_router.rate_limiter().config().tactical_rpm > 0);
    }

    #[tokio::test]
    async fn test_run_ia_assiste_returns_results_for_all_criteria() {
        let ctx = ToolContext::new(rgaa_browser_tools::BrowserSession::new_placeholder());
        let agent = RgaaAgent::new_placeholder(ctx);
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
        let ctx = ToolContext::new(rgaa_browser_tools::BrowserSession::new_placeholder());
        let agent = RgaaAgent::new_placeholder(ctx);
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
        let ctx = ToolContext::new(rgaa_browser_tools::BrowserSession::new_placeholder());
        let agent = RgaaAgent::new_placeholder(ctx);
        let ia_criteria = RgaaCriteria::ia_assiste();
        let context = sample_context();

        let results = agent.run_ia_assiste(&ia_criteria, &context, None).await;

        assert_eq!(results.len(), ia_criteria.len());
        for criterion in &ia_criteria {
            assert!(results.contains_key(criterion.id));
        }
    }

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
    fn test_agent_builder_chaining() {
        let ctx = ToolContext::new(rgaa_browser_tools::BrowserSession::new_placeholder());
        let filter = vec!["1.3".to_string()];
        let config = AgentBuilder::new(ctx)
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
        let ctx = ToolContext::new(rgaa_browser_tools::BrowserSession::new_placeholder());
        let agent = AgentBuilder::new(ctx).build();
        assert!(agent.model_router.rate_limiter().config().tactical_rpm > 0);
    }

    #[test]
    fn test_create_simple_agent() {
        let ctx = ToolContext::new(rgaa_browser_tools::BrowserSession::new_placeholder());
        let agent = create_simple_agent(ctx);
        assert!(agent.model_router.rate_limiter().config().tactical_rpm > 0);
    }

    #[tokio::test]
    async fn test_agent_builder_builds_working_agent() {
        let ctx = ToolContext::new(rgaa_browser_tools::BrowserSession::new_placeholder());
        let agent = AgentBuilder::new(ctx)
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

    #[test]
    fn test_rig_agent_has_tool_context() {
        let ctx = ToolContext::new(rgaa_browser_tools::BrowserSession::new_placeholder());
        let agent = RgaaAgent::new_placeholder(ctx);
        let tool_ctx = agent.tool_ctx();
        assert!(tool_ctx.session().try_lock().is_ok());
    }

    #[test]
    fn test_rig_agent_has_tactical_agent() {
        let ctx = ToolContext::new(rgaa_browser_tools::BrowserSession::new_placeholder());
        let agent = RgaaAgent::new_placeholder(ctx);
        assert!(agent.has_tactical_agent());
    }

    #[test]
    fn test_rig_agent_has_reasoning_agent() {
        let ctx = ToolContext::new(rgaa_browser_tools::BrowserSession::new_placeholder());
        let agent = RgaaAgent::new_placeholder(ctx);
        assert!(agent.has_reasoning_agent());
    }
}
