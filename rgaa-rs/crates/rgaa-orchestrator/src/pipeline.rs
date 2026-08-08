use std::collections::HashMap;
use rgaa_core::{AuditResult, CriterionResult, CriterionStatus, CrawlConfig, PageResult, RgaaCriteria, Classification};
use rgaa_rules::{AxeMapper, GapFixRules};
use rgaa_holo::{HoloClient, PromptBuilder, PageContext};
use rgaa_browser::PlaywrightBridge;
use tracing::{info, error};

pub struct Orchestrator;

impl Orchestrator {
    pub async fn run(url: &str, _config: &CrawlConfig) -> Result<AuditResult, String> {
        let start = std::time::Instant::now();
        info!(url, "Starting audit");

        let bridge = PlaywrightBridge::new();
        let api_key = std::env::var("HOLO3_API_KEY")
            .unwrap_or_else(|_| "hk-a73b030c64aac335fc3651c280c95694beb8df95c4a5d8b1".into());
        let holo_client = HoloClient::new(api_key);

        // 1. Run axe-core via PlaywrightBridge
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
        let page_context: PageContext = serde_json::from_value(
            bridge.extract_page_context(url).await?
        ).unwrap_or(PageContext {
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

        // 4. Run Holo3 for all IA_ASSISTE criteria (27 criteria)
        info!("Running Holo3 IA_ASSISTE evaluation");
        let ia_criteria = RgaaCriteria::ia_assiste();
        let mut holo_results = HashMap::new();

        for criterion in &ia_criteria {
            let prompt = PromptBuilder::build(criterion.id, &page_context);
            match holo_client.evaluate(&prompt).await {
                Ok(response) => {
                    let status = match response.verdict.as_str() {
                        "CONFORME" => CriterionStatus::Pass,
                        "NON_CONFORME" => CriterionStatus::Fail,
                        _ => CriterionStatus::Na,
                    };
                    holo_results.insert(criterion.id.to_string(), CriterionResult {
                        criterion_id: criterion.id.to_string(),
                        title: criterion.title.to_string(),
                        classification: Classification::IaAssiste,
                        status,
                        violations: vec![],
                        confidence: Some(response.confidence),
                        justification: Some(response.justification),
                        source: "holo3".into(),
                    });
                }
                Err(e) => {
                    error!(criterion_id = criterion.id, error = %e, "Holo3 evaluation failed");
                    holo_results.insert(criterion.id.to_string(), CriterionResult {
                        criterion_id: criterion.id.to_string(),
                        title: criterion.title.to_string(),
                        classification: Classification::IaAssiste,
                        status: CriterionStatus::Error,
                        violations: vec![],
                        confidence: None,
                        justification: Some(e),
                        source: "holo3".into(),
                    });
                }
            }
        }

        // 5. Merge results from steps 2, 3, 5
        let mut all_results: HashMap<String, CriterionResult> = HashMap::new();
        all_results.extend(axe_results);
        all_results.extend(gap_results);
        all_results.extend(holo_results);

        // 6. Add MANUEL criteria (7.5) as INDETERMINE
        let all_criteria = RgaaCriteria::all();
        for criterion in &all_criteria {
            if criterion.classification == Classification::Manuel {
                all_results.entry(criterion.id.to_string()).or_insert_with(|| CriterionResult {
                    criterion_id: criterion.id.to_string(),
                    title: criterion.title.to_string(),
                    classification: Classification::Manuel,
                    status: CriterionStatus::Na,
                    violations: vec![],
                    confidence: None,
                    justification: Some("Manual verification required".into()),
                    source: "manual".into(),
                });
            }
        }

        // 7. Calculate compliance rate
        let criteria: Vec<CriterionResult> = all_results.into_values().collect();
        let pass_count = criteria.iter().filter(|c| c.status == CriterionStatus::Pass).count();
        let fail_count = criteria.iter().filter(|c| c.status == CriterionStatus::Fail).count();
        let na_count = criteria.iter().filter(|c| c.status == CriterionStatus::Na).count();
        let error_count = criteria.iter().filter(|c| c.status == CriterionStatus::Error).count();
        let total = RgaaCriteria::count();
        let compliance = if total - na_count > 0 {
            (pass_count as f64 / (total - na_count) as f64) * 100.0
        } else {
            0.0
        };

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
}
