use crate::models::ModelRouter;
use crate::prompts::PromptBuilder;
use rgaa_core::{Classification, Criterion, CriterionResult, CriterionStatus};
use rgaa_holo::PageContext;
use std::collections::HashMap;

pub struct RgaaAgent {
    model_router: ModelRouter,
}

impl RgaaAgent {
    pub fn new(model_router: ModelRouter) -> Self {
        Self { model_router }
    }

    /// Evaluate all IA_ASSISTE criteria sequentially (rate-limited).
    ///
    /// Returns a map of criterion_id → CriterionResult.
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
}
