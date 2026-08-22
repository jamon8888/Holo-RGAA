use rgaa_agent::agent::RgaaAgent;
use rgaa_browser_tools::{BrowserSession, ToolContext};
use rgaa_core::{
    AuditResult, Classification, CrawlConfig, CriterionResult, CriterionStatus, PageResult,
    RgaaCriteria,
};
use rgaa_holo::PageContext;
use rgaa_rules::{AxeMapper, GapFixRules};
use std::collections::HashMap;
use tracing::info;

use rgaa_obscura::ObscuraBridge;

fn manual_status() -> CriterionStatus {
    CriterionStatus::NeedsReview
}

fn calculate_compliance(criteria: &[CriterionResult], total: usize) -> f64 {
    let pass_count = criteria
        .iter()
        .filter(|criterion| criterion.status == CriterionStatus::Pass)
        .count();
    let na_count = criteria
        .iter()
        .filter(|criterion| criterion.status == CriterionStatus::NotApplicable)
        .count();
    let denominator = total.saturating_sub(na_count);
    if denominator > 0 {
        (pass_count as f64 / denominator as f64) * 100.0
    } else {
        0.0
    }
}

pub struct Orchestrator;

impl Orchestrator {
    /// Audit a single URL. Behavior is identical to the pre-batch implementation:
    /// it runs the per-URL pipeline and returns a single [`AuditResult`].
    pub async fn run(url: &str, config: &CrawlConfig) -> Result<AuditResult, String> {
        let mut results = Self::run_batch(&[url.to_string()], config).await?;
        results
            .remove(url)
            .ok_or_else(|| format!("audit result missing for {url}"))
    }

    /// Audit multiple URLs, returning one [`AuditResult`] per URL keyed by the URL.
    /// The Obscura CDP server is started once before the loop and stopped via
    /// [`ObscuraBridge`] `Drop` after the loop completes.
    pub async fn run_batch(
        urls: &[String],
        config: &CrawlConfig,
    ) -> Result<HashMap<String, AuditResult>, String> {
        let bridge = {
            let mut b = ObscuraBridge::new();
            b.start_server().await?;
            b
        };

        // Create shared tool context from browser session
        let session = BrowserSession::new(bridge);
        let tool_ctx = ToolContext::new(session);

        let agent_config = rgaa_agent::config::AgentConfig::from_env()
            .map_err(|e| format!("invalid agent configuration: {e}"))?;
        let agent = rgaa_agent::agent::RgaaAgent::new(&agent_config)
            .await
            .map_err(|e| format!("failed to create agent: {e}"))?;
        let mut results = HashMap::new();
        for url in urls {
            let audit = audit_one(&agent, &tool_ctx, url, config).await?;
            results.insert(url.clone(), audit);
        }
        Ok(results)
    }
}

/// Run the full per-URL audit pipeline against a browser bridge.
///
/// This is the single source of truth for the audit logic; both [`Orchestrator::run`]
/// and [`Orchestrator::run_batch`] route through it so a single URL produces
/// identical results regardless of entry point.
async fn audit_one(
    agent: &RgaaAgent,
    tool_ctx: &ToolContext,
    url: &str,
    _config: &CrawlConfig,
) -> Result<AuditResult, String> {
    let start = std::time::Instant::now();
    info!(url, "Starting audit");

    // Get the bridge without holding the mutex across awaits
    let bridge = {
        let session = tool_ctx.session().lock().await;
        session.bridge().clone()
    };

    // 1. Run axe-core
    info!("Running axe-core");
    let axe_violations = bridge.run_axe(url).await?;
    let axe_results = AxeMapper::map(&axe_violations);

    // 2. Run gap-fix rules for 10 false negatives
    info!("Running gap-fix rules");
    let gap_snippets = GapFixRules::snippets();
    let gap_js_results = bridge.run_gap_fix(url, &gap_snippets).await?;
    let gap_results = GapFixRules::parse_results(&gap_js_results);

    // 3. Extract page context for Holo3 prompts
    info!("Extracting page context");
    let page_context: PageContext = serde_json::from_value(bridge.extract_page_context(url).await?)
        .unwrap_or(PageContext {
            title: None,
            lang: None,
            headings: vec![],
            images: vec![],
            iframes: vec![],
            links: vec![],
            forms: vec![],
            media: vec![],
            navigation: vec![],
        });

    // 4. Run agentic evaluation for all IA_ASSISTE criteria
    let ia_criteria = RgaaCriteria::ia_assiste();
    info!(
        criteria = ia_criteria.len(),
        "Running agentic IA_ASSISTE evaluation"
    );

    let agent_results = agent
        .run_ia_assiste(&ia_criteria, &page_context)
        .await;

    let mut holo_results = HashMap::new();
    for (criterion_id, result) in agent_results {
        holo_results.insert(criterion_id, result);
    }

    // 5. Merge results
    let mut all_results: HashMap<String, CriterionResult> = HashMap::new();
    all_results.extend(axe_results);
    all_results.extend(gap_results);
    all_results.extend(holo_results);

    // 6. Ensure every criterion has an entry.
    //
    // Déterministe criteria not flagged by axe-core/gap-fix (and not already
    // present from Holo3) are conforming for the automated checks -> Pass, so
    // the compliance rate reflects the full 106-criterion catalog instead of
    // only the criteria that produced a violation.
    // Manuel criteria always require human review -> NeedsReview.
    let all_criteria = RgaaCriteria::all();
    for criterion in &all_criteria {
        if criterion.classification == Classification::Manuel {
            all_results
                .entry(criterion.id.to_string())
                .or_insert_with(|| CriterionResult {
                    criterion_id: criterion.id.to_string(),
                    title: criterion.title.to_string(),
                    classification: Classification::Manuel,
                    status: manual_status(),
                    violations: vec![],
                    confidence: None,
                    justification: Some("Manual verification required".into()),
                    source: "manual".into(),
                });
        } else if !all_results.contains_key(&criterion.id.to_string()) {
            all_results
                .entry(criterion.id.to_string())
                .or_insert_with(|| CriterionResult {
                    criterion_id: criterion.id.to_string(),
                    title: criterion.title.to_string(),
                    classification: criterion.classification.clone(),
                    status: CriterionStatus::Pass,
                    violations: vec![],
                    confidence: None,
                    justification: Some(
                        "No violation detected by automated checks (axe-core + gap-fix)".into(),
                    ),
                    source: "automated".into(),
                });
        }
    }

    // 7. Calculate compliance rate
    let criteria: Vec<CriterionResult> = all_results.into_values().collect();
    let pass_count = criteria
        .iter()
        .filter(|c| c.status == CriterionStatus::Pass)
        .count();
    let fail_count = criteria
        .iter()
        .filter(|c| c.status == CriterionStatus::Fail)
        .count();
    let na_count = criteria
        .iter()
        .filter(|c| c.status == CriterionStatus::NotApplicable)
        .count();
    let error_count = criteria
        .iter()
        .filter(|c| c.status == CriterionStatus::Error)
        .count();
    let total = RgaaCriteria::count();
    let compliance = calculate_compliance(&criteria, total);

    info!(
        pass = pass_count,
        fail = fail_count,
        na = na_count,
        errors = error_count,
        total,
        compliance = format!("{:.1}%", compliance),
        "Audit complete"
    );

    Ok(AuditResult {
        audit_id: uuid::Uuid::new_v4().to_string(),
        url: url.to_string(),
        pages: vec![PageResult {
            url: url.to_string(),
            title: page_context.title,
            criteria,
            compliance_rate: compliance,
            crawl_depth: 0,
        }],
        total_criteria: total,
        passed: pass_count,
        failed: fail_count,
        na: na_count,
        overall_compliance: compliance,
        duration_ms: start.elapsed().as_millis() as u64,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manual_criteria_require_review() {
        assert_eq!(manual_status(), CriterionStatus::NeedsReview);
    }

    #[test]
    fn needs_review_is_not_excluded_from_compliance_denominator() {
        let criteria = vec![
            test_result(CriterionStatus::Pass),
            test_result(CriterionStatus::NeedsReview),
        ];

        assert_eq!(calculate_compliance(&criteria, 2), 50.0);
    }

    fn test_result(status: CriterionStatus) -> CriterionResult {
        CriterionResult {
            criterion_id: "1.1".into(),
            title: "test".into(),
            classification: Classification::IaAssiste,
            status,
            violations: Vec::new(),
            confidence: None,
            justification: None,
            source: "test".into(),
        }
    }
}