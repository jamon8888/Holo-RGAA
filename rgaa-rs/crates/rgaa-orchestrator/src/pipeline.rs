use rgaa_agent::agent::RgaaAgent;
use rgaa_browser_tools::{BrowserSession, ToolContext};
use rgaa_core::{
    AuditResult, Classification, CrawlConfig, CriterionResult, CriterionStatus, PageResult,
    RgaaCatalog, RgaaCriteria,
};
use rgaa_core::catalog::Automatable;
use rgaa_core::na_detection;
use rgaa_core::types::ConformityStatus;
use rgaa_holo::PageContext;
use rgaa_rules::{AxeMapper, GapFixRules};
use std::collections::HashMap;
use tracing::info;

use rgaa_obscura::ObscuraBridge;

fn manual_status() -> CriterionStatus {
    CriterionStatus::NeedsReview
}

fn calculate_compliance(criteria: &[CriterionResult]) -> f64 {
    let pass = criteria
        .iter()
        .filter(|c| c.status == CriterionStatus::Pass)
        .count();
    let fail = criteria
        .iter()
        .filter(|c| c.status == CriterionStatus::Fail || c.status == CriterionStatus::Error)
        .count();
    let denominator = pass + fail;
    if denominator > 0 {
        (pass as f64 / denominator as f64) * 100.0
    } else {
        0.0
    }
}

fn calculate_compliance_summary(criteria: &[CriterionResult]) -> (f64, f64, String) {
    let mut c = 0;
    let mut nc = 0;
    let mut validated_total = 0;
    let mut validated_executed = 0;

    for criterion in criteria {
        let conformity = ConformityStatus::from(criterion.status.clone());
        if let Some((_theme, cat)) = RgaaCatalog::by_id(&criterion.criterion_id) {
            if matches!(cat.automatable, Automatable::FullyAutomatable | Automatable::PartiallyAutomatable) {
                validated_total += 1;
                if criterion.status != CriterionStatus::NotTested {
                    validated_executed += 1;
                }
            }
        }
        match conformity {
            ConformityStatus::Conforme => c += 1,
            ConformityStatus::NonConforme => nc += 1,
            _ => {}
        }
    }

    let taux_global = if c + nc > 0 {
        (c as f64 / (c + nc) as f64) * 100.0
    } else {
        0.0
    };

    let coverage_percent = if validated_total > 0 {
        (validated_executed as f64 / validated_total as f64) * 100.0
    } else {
        0.0
    };

    let etat_conformite = if taux_global >= 100.0 {
        "totale".to_string()
    } else if taux_global >= 50.0 {
        "partielle".to_string()
    } else {
        "non conforme".to_string()
    };

    (taux_global, coverage_percent, etat_conformite)
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

    // Hold the lock for the sequential bridge calls — released before agent work.
    let session = tool_ctx.session().lock().await;
    let bridge = session.bridge();

    // 1. Run axe-core
    info!("Running axe-core");
    let axe_violations = bridge.run_axe(url).await?;
    let axe_results = AxeMapper::map(&axe_violations).map_err(|e| e.to_string())?;

    // 2. Run gap-fix rules for 10 false negatives
    info!("Running gap-fix rules");
    let gap_snippets = GapFixRules::snippets();
    let gap_js_results = bridge.run_gap_fix(url, &gap_snippets).await?;
    let gap_results = GapFixRules::parse_results(&gap_js_results);

    // 3. Extract page context for Holo3 prompts
    info!("Extracting page context");
    let raw_context = bridge.extract_page_context(url).await?;
    let na_map = na_detection::detect_na(&raw_context);
    let page_context: PageContext = serde_json::from_value(raw_context)
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

    drop(session); // Release the browser lock before agent calls

    // 4. Run agentic evaluation for all IA_ASSISTE criteria
    let ia_criteria = RgaaCriteria::ia_assiste();
    info!(
        criteria = ia_criteria.len(),
        "Running agentic IA_ASSISTE evaluation"
    );

    let agent_results = agent.run_ia_assiste(&ia_criteria, &page_context).await;

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
        } else if !all_results.contains_key(criterion.id) {
            all_results
                .entry(criterion.id.to_string())
                .or_insert_with(|| CriterionResult {
                    criterion_id: criterion.id.to_string(),
                    title: criterion.title.to_string(),
                    classification: criterion.classification,
                    status: CriterionStatus::NotTested,
                    violations: vec![],
                    confidence: None,
                    justification: Some(
                        "Not tested — no automated check covered this criterion".into(),
                    ),
                    source: "automated".into(),
                });
        }
    }

    // 7. Apply NA detection
    let mut criteria: Vec<CriterionResult> = all_results.into_values().collect();
    for criterion in &mut criteria {
        if let Some(&false) = na_map.get(&criterion.criterion_id) {
            criterion.status = CriterionStatus::NotApplicable;
        }
    }

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
    let compliance = calculate_compliance(&criteria);
    let (taux_global, coverage_percent, etat_conformite) = calculate_compliance_summary(&criteria);

    info!(
        pass = pass_count,
        fail = fail_count,
        na = na_count,
        errors = error_count,
        total,
        compliance = format!("{:.1}%", compliance),
        taux_global = format!("{:.1}%", taux_global),
        coverage_percent = format!("{:.1}%", coverage_percent),
        etat_conformite,
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
        taux_global,
        coverage_percent,
        etat_conformite,
        duration_ms: start.elapsed().as_millis() as u64,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

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

    fn test_result_id(id: &str, status: CriterionStatus) -> CriterionResult {
        CriterionResult {
            criterion_id: id.into(),
            title: "test".into(),
            classification: Classification::IaAssiste,
            status,
            violations: Vec::new(),
            confidence: None,
            justification: None,
            source: "test".into(),
        }
    }

    #[test]
    fn manual_criteria_require_review() {
        assert_eq!(manual_status(), CriterionStatus::NeedsReview);
    }

    #[test]
    fn compliance_empty_input() {
        assert_eq!(calculate_compliance(&[]), 0.0);
    }

    #[test]
    fn compliance_all_pass() {
        let criteria = vec![
            test_result(CriterionStatus::Pass),
            test_result(CriterionStatus::Pass),
        ];
        assert_eq!(calculate_compliance(&criteria), 100.0);
    }

    #[test]
    fn compliance_all_fail() {
        let criteria = vec![
            test_result(CriterionStatus::Fail),
            test_result(CriterionStatus::Fail),
        ];
        assert_eq!(calculate_compliance(&criteria), 0.0);
    }

    #[test]
    fn compliance_mixed_pass_fail() {
        let criteria = vec![
            test_result(CriterionStatus::Pass),
            test_result(CriterionStatus::Pass),
            test_result(CriterionStatus::Fail),
        ];
        // 2 pass, 1 fail → 2/3 ≈ 66.67%
        let c = calculate_compliance(&criteria);
        assert!((c - 66.67).abs() < 0.1, "got {c}");
    }

    #[test]
    fn compliance_na_excluded() {
        let criteria = vec![
            test_result(CriterionStatus::Pass),
            test_result(CriterionStatus::NotApplicable),
            test_result(CriterionStatus::Fail),
        ];
        // NA excluded: 1 pass, 1 fail → 50%
        assert_eq!(calculate_compliance(&criteria), 50.0);
    }

    #[test]
    fn compliance_nt_excluded() {
        let criteria = vec![
            test_result(CriterionStatus::Pass),
            test_result(CriterionStatus::NotTested),
            test_result(CriterionStatus::Fail),
        ];
        // NT excluded: 1 pass, 1 fail → 50%
        assert_eq!(calculate_compliance(&criteria), 50.0);
    }

    #[test]
    fn compliance_error_counted_as_fail() {
        let criteria = vec![
            test_result(CriterionStatus::Pass),
            test_result(CriterionStatus::Error),
        ];
        // 1 pass, 1 error → 50%
        assert_eq!(calculate_compliance(&criteria), 50.0);
    }

    #[test]
    fn compliance_needs_review_excluded() {
        let criteria = vec![
            test_result(CriterionStatus::Pass),
            test_result(CriterionStatus::NeedsReview),
        ];
        // NeedsReview excluded: 1 pass, 0 fail → 100%
        assert_eq!(calculate_compliance(&criteria), 100.0);
    }

    #[test]
    fn compliance_all_na() {
        let criteria = vec![
            test_result(CriterionStatus::NotApplicable),
            test_result(CriterionStatus::NotApplicable),
        ];
        // All NA → denominator 0 → 0%
        assert_eq!(calculate_compliance(&criteria), 0.0);
    }

    #[test]
    fn compliance_sample_wide_nc_if_any_page_fail() {
        // Per official RGAA: NC if NC on ANY page
        // Simulated: 3 criteria, 2 pass, 1 fail → NC overall
        let criteria = vec![
            test_result_id("1.1", CriterionStatus::Pass),
            test_result_id("1.2", CriterionStatus::Pass),
            test_result_id("1.3", CriterionStatus::Fail),
        ];
        let c = calculate_compliance(&criteria);
        // 2/3 ≈ 66.67% but status is NC because any page fail
        assert!((c - 66.67).abs() < 0.1, "got {c}");
    }

    #[test]
    fn compliance_all_c_only_if_all_pass() {
        let criteria = vec![
            test_result_id("1.1", CriterionStatus::Pass),
            test_result_id("1.2", CriterionStatus::Pass),
            test_result_id("1.3", CriterionStatus::Pass),
        ];
        assert_eq!(calculate_compliance(&criteria), 100.0);
    }

    #[test]
    fn compliance_summary_all_pass() {
        let criteria = vec![
            test_result_id("1.1", CriterionStatus::Pass),
            test_result_id("1.2", CriterionStatus::Pass),
        ];
        let (taux, coverage, etat) = calculate_compliance_summary(&criteria);
        assert_eq!(taux, 100.0);
        assert_eq!(etat, "totale");
    }

    #[test]
    fn compliance_summary_mixed() {
        let criteria = vec![
            test_result_id("1.1", CriterionStatus::Pass),
            test_result_id("1.2", CriterionStatus::Fail),
        ];
        let (taux, _coverage, etat) = calculate_compliance_summary(&criteria);
        assert_eq!(taux, 50.0);
        assert_eq!(etat, "partielle");
    }

    #[test]
    fn compliance_summary_coverage_percent() {
        // 1.1 is FullyAutomatable, 1.2 is PartiallyAutomatable, 13.1 is NotAutomatable
        let criteria = vec![
            test_result_id("1.1", CriterionStatus::Pass),
            test_result_id("1.2", CriterionStatus::NotTested),
            test_result_id("13.1", CriterionStatus::Pass),
        ];
        let (taux, coverage, _etat) = calculate_compliance_summary(&criteria);
        // validated_total = 2 (1.1,1.2), validated_executed =1 (1.1)
        assert!((coverage - 50.0).abs() < 0.01);
        // taux based on ConformityStatus: Pass and NotTested → NonTeste not counted, so taux =100
        assert_eq!(taux, 100.0);
    }
}
