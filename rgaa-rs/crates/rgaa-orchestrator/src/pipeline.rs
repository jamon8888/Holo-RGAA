use std::collections::HashMap;
use std::sync::Arc;
use rgaa_core::{AuditResult, CriterionResult, CriterionStatus, CrawlConfig, PageResult, RgaaCriteria, Classification};
use rgaa_rules::{AxeMapper, GapFixRules};
use rgaa_holo::{HoloClient, PromptBuilder, PageContext};
use tokio::sync::Semaphore;
use tokio::task::JoinSet;
use tracing::{info, error};

/// Maximum number of concurrent Holo3 evaluations per audited URL. Bounded so we
/// don't trip API rate limits while still parallelizing the 27 IA_ASSISTE calls.
const HOLO3_CONCURRENCY: usize = 12;

use rgaa_obscura::ObscuraBridge;

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
        let api_key = std::env::var("HOLO3_API_KEY")
            .unwrap_or_else(|_| "hk-a73b030c64aac335fc3651c280c95694beb8df95c4a5d8b1".into());
        let holo = Arc::new(HoloClient::new(api_key));
        let mut results = HashMap::new();
        for url in urls {
            let audit = audit_one(&bridge, &holo, url, config).await?;
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
    bridge: &ObscuraBridge,
    holo_client: &Arc<HoloClient>,
    url: &str,
    _config: &CrawlConfig,
) -> Result<AuditResult, String> {
    let start = std::time::Instant::now();
    info!(url, "Starting audit");

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

    // 4. Run Holo3 for all IA_ASSISTE criteria concurrently (bounded).
    //
    // The 27 evaluations are independent; running them sequentially was the main
    // performance bottleneck (each is a network round-trip to the LLM API). We
    // spawn them all and bound concurrency with a semaphore so we parallelize
    // without hammering the API into rate limits.
    let ia_criteria = RgaaCriteria::ia_assiste();
    info!(criteria = ia_criteria.len(), "Running Holo3 IA_ASSISTE evaluation");
    let sem = Arc::new(Semaphore::new(HOLO3_CONCURRENCY));
    let mut set = JoinSet::new();
    for criterion in &ia_criteria {
        let client = Arc::clone(holo_client);
        let sem = Arc::clone(&sem);
        let prompt = PromptBuilder::build(criterion.id, &page_context);
        let criterion_id = criterion.id.to_string();
        let title = criterion.title.to_string();
        set.spawn(async move {
            let _permit = sem
                .acquire()
                .await
                .expect("Holo3 semaphore closed unexpectedly");
            let res = client.evaluate(&prompt).await;
            (criterion_id, title, res)
        });
    }

    let mut holo_results = HashMap::new();
    while let Some(joined) = set.join_next().await {
        let (criterion_id, title, res) = joined.expect("Holo3 task panicked");
        match res {
            Ok(response) => {
                let status = match response.verdict.to_lowercase().as_str() {
                    "pass" | "conforme" => CriterionStatus::Pass,
                    "fail" | "non_conforme" => CriterionStatus::Fail,
                    _ => CriterionStatus::Na,
                };
                holo_results.insert(criterion_id.clone(), CriterionResult {
                    criterion_id,
                    title,
                    classification: Classification::IaAssiste,
                    status,
                    violations: vec![],
                    confidence: Some(response.confidence),
                    justification: Some(response.justification),
                    source: "holo3".into(),
                });
            }
            Err(e) => {
                error!(criterion_id = %criterion_id, error = %e, "Holo3 evaluation failed");
                holo_results.insert(criterion_id.clone(), CriterionResult {
                    criterion_id,
                    title,
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

    // 5. Merge results
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
