use crate::{merge, site_comparison};
use rgaa_agent::agent::RgaaAgent;
use rgaa_browser_tools::{BrowserSession, ToolContext};
use rgaa_core::catalog::Automatable;
use rgaa_core::na_detection;
use rgaa_core::test_plan::CoverageLevel;
use rgaa_core::types::{is_deterministic_source, TestOutcome, VerdictBasis};
use rgaa_core::{
    AuditResult, Classification, CrawlConfig, Criterion, CriterionResult, CriterionStatus,
    EnginePlan, PageResult, PlanEngine, RgaaCatalog, RgaaCriteria,
};
use rgaa_holo::PageContext;
use rgaa_rules::{AxeMapper, GapFixRules};
use rgaa_spider::{CrawlSiteArgs, SpiderTool};
use rgaa_storage::Storage;
use rig_core::tool::PortableTool;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;
use tracing::info;

use rgaa_obscura::ObscuraBridge;

/// Notified once per URL as a batch progresses, with that URL and its
/// outcome. Shared across every in-flight audit task, hence `Arc`.
pub type BatchObserver = Arc<dyn Fn(&str, Result<&AuditResult, &str>) + Send + Sync>;

/// At most this many audits run concurrently in a batch — the operating
/// point #42 locks in (1000-audit batches, 8 concurrent).
const MAX_CONCURRENT_AUDITS: usize = 8;

/// Pipeline stage reported to progress callbacks, in execution order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditPhase {
    Axe,
    GapFix,
    PageContext,
    AgentIaAssiste,
    AgentPartial,
    Merging,
}

impl AuditPhase {
    fn index(self) -> usize {
        match self {
            AuditPhase::Axe => 0,
            AuditPhase::GapFix => 1,
            AuditPhase::PageContext => 2,
            AuditPhase::AgentIaAssiste => 3,
            AuditPhase::AgentPartial => 4,
            AuditPhase::Merging => 5,
        }
    }

    /// Short human-readable label for progress displays.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            AuditPhase::Axe => "Running axe-core checks...",
            AuditPhase::GapFix => "Running gap-fix rules...",
            AuditPhase::PageContext => "Extracting page context...",
            AuditPhase::AgentIaAssiste => "Agentic IA_ASSISTE evaluation...",
            AuditPhase::AgentPartial => "Partially-automatable evaluation...",
            AuditPhase::Merging => "Merging results...",
        }
    }

    /// Fraction complete when this phase starts (6 phases, evenly weighted).
    #[must_use]
    pub fn progress(self) -> f32 {
        self.index() as f32 / 6.0
    }
}

fn partially_automatable_status() -> CriterionStatus {
    CriterionStatus::NeedsReview
}

fn manual_status() -> CriterionStatus {
    CriterionStatus::NeedsReview
}

fn calculate_compliance(criteria: &[CriterionResult]) -> f64 {
    rgaa_report::compliance_rate(criteria)
}

fn calculate_compliance_summary(criteria: &[CriterionResult]) -> (f64, f64, String) {
    let m = rgaa_report::compute_metrics(criteria, &rgaa_report::RGAA_41);
    (m.taux_global, m.coverage_percent, m.etat_conformite)
}

/// Catalog criteria whose automatic estimate or routed test outcome is missing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoverageError {
    pub missing_criterion_ids: Vec<String>,
}

impl std::fmt::Display for CoverageError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "automatic verdict coverage is incomplete for {} criterion(s): {}",
            self.missing_criterion_ids.len(),
            self.missing_criterion_ids.join(", ")
        )
    }
}

impl std::error::Error for CoverageError {}

/// Ensure every catalog criterion has a prediction and every routed test has
/// an outcome from an allowed source. Complete routes require actual
/// deterministic evidence; model estimates may cover partial routes only.
pub fn validate_automatic_verdict_coverage(
    results: &[CriterionResult],
) -> Result<(), CoverageError> {
    let mut by_id = HashMap::with_capacity(results.len());
    let mut duplicate_ids = std::collections::HashSet::new();
    for result in results {
        if by_id.insert(result.criterion_id.as_str(), result).is_some() {
            duplicate_ids.insert(result.criterion_id.as_str());
        }
    }

    let mut missing = Vec::new();
    for criterion in RgaaCriteria::all() {
        let id = criterion.id;
        let Some(result) = by_id.get(id).copied() else {
            missing.push(id.to_owned());
            continue;
        };
        let mut complete = !duplicate_ids.contains(id) && result.automated_verdict.is_some();
        if let Some(test_map) = RgaaCatalog::tests(id) {
            let mut has_complete_route = false;
            for test_key in test_map.keys() {
                let Some(route) = EnginePlan::route_test(id, test_key) else {
                    complete = false;
                    continue;
                };
                has_complete_route |= route.coverage == CoverageLevel::Complete;
                let outcome_present = result.tests.iter().any(|outcome| {
                    outcome.test_key == *test_key && outcome_is_allowed(outcome, route.coverage)
                });
                complete &= outcome_present;
            }
            // A criterion-wide deterministic failure is valid verified
            // evidence, but it does not identify which complete-route test
            // failed. Keep the audit open until a mechanism supplies that key.
            if has_complete_route
                && result.status == CriterionStatus::Fail
                && !result.tests.iter().any(|outcome| {
                    outcome.status == CriterionStatus::Fail
                        && is_deterministic_source(&outcome.source)
                })
            {
                complete = false;
            }
            let site_comparison_failed = (result.source == "site-comparison"
                && result.status == CriterionStatus::Fail)
                || result
                    .justification
                    .as_deref()
                    .is_some_and(|justification| {
                        justification.contains("Site-level comparison returned Fail:")
                    });
            if result.status == CriterionStatus::Fail
                && site_comparison_failed
                && !result.tests.iter().any(|outcome| {
                    outcome.status == CriterionStatus::Fail && outcome.source == "site-comparison"
                })
            {
                complete = false;
            }
        } else {
            complete = false;
        }
        if !complete {
            missing.push(id.to_owned());
        }
    }

    if missing.is_empty() {
        Ok(())
    } else {
        Err(CoverageError {
            missing_criterion_ids: missing,
        })
    }
}

fn outcome_is_allowed(outcome: &TestOutcome, coverage: CoverageLevel) -> bool {
    let decided = matches!(
        outcome.status,
        CriterionStatus::Pass | CriterionStatus::Fail | CriterionStatus::NotApplicable
    );
    if !decided {
        return false;
    }
    if outcome.status == CriterionStatus::NotApplicable {
        return is_deterministic_source(&outcome.source);
    }
    match coverage {
        CoverageLevel::Complete => is_deterministic_source(&outcome.source),
        CoverageLevel::Partial => {
            is_deterministic_source(&outcome.source) || outcome.source == "agent-estimate"
        }
    }
}

fn criterion_has_routed_mechanism(criterion_id: &str, mechanism_id: &str) -> bool {
    RgaaCatalog::tests(criterion_id).is_some_and(|tests| {
        tests.keys().any(|test_key| {
            EnginePlan::route_test(criterion_id, test_key)
                .is_some_and(|route| route.mechanisms.iter().any(|id| id == mechanism_id))
        })
    })
}

fn mechanism_id(prefix: &str, criterion_id: &str) -> String {
    format!("{prefix}-{}", criterion_id.replace('.', "-"))
}

fn routed_gap_fix_snippets() -> HashMap<String, String> {
    GapFixRules::snippets()
        .iter()
        .filter(|(criterion_id, _)| {
            criterion_has_routed_mechanism(criterion_id, &mechanism_id("gapfix", criterion_id))
        })
        .map(|(criterion_id, snippet)| (criterion_id.clone(), (*snippet).to_owned()))
        .collect()
}

fn admit_routed_results(
    results: impl IntoIterator<Item = (String, CriterionResult)>,
    mechanism_prefix: &str,
) -> HashMap<String, CriterionResult> {
    results
        .into_iter()
        .filter(|(criterion_id, _)| {
            criterion_has_routed_mechanism(
                criterion_id,
                &mechanism_id(mechanism_prefix, criterion_id),
            )
        })
        .collect()
}

fn pages_have_complete_automatic_coverage(pages: &[PageResult]) -> bool {
    !pages.is_empty()
        && pages
            .iter()
            .all(|page| validate_automatic_verdict_coverage(&page.criteria).is_ok())
}

fn record_site_comparison_evidence(
    result: &mut CriterionResult,
    site_result: &site_comparison::SiteCriterionObservation,
) {
    let details = format!(
        "{}; sample_complete={}, sampled_pages={}, failed_pages={}",
        site_result.details,
        site_result.sample_complete,
        site_result.sampled_pages,
        site_result.failed_pages
    );
    let existing_failure = result.status == CriterionStatus::Fail;
    let site_finding = format!(
        "Site-level comparison returned {:?}: {details}",
        site_result.status
    );
    if existing_failure {
        result
            .justification
            .get_or_insert_with(String::new)
            .push_str(&format!("; {site_finding}"));
    } else {
        result.status = site_result.status.clone();
        result.source = "site-comparison".to_string();
        result.justification = Some(site_finding);
    }

    if !result
        .considered_sources
        .iter()
        .any(|source| source == "site-comparison")
    {
        result
            .considered_sources
            .push("site-comparison".to_string());
    }
    let evidence = format!("site-comparison: {details}");
    if !result.verdict_basis.contains(&VerdictBasis::Deterministic) {
        result.verdict_basis.push(VerdictBasis::Deterministic);
    }
    if matches!(
        site_result.status,
        CriterionStatus::Pass | CriterionStatus::Fail | CriterionStatus::NotApplicable
    ) {
        result.verified_status = Some(site_result.status.clone());

        // The site comparison can identify a test outcome only when the
        // catalog has one key for this criterion. For multi-test criteria,
        // preserve the aggregate evidence and let the coverage gate stay open
        // on a Fail rather than attributing it to every test.
        if let Some(test_map) = RgaaCatalog::tests(&result.criterion_id) {
            if test_map.len() == 1 {
                let test_key = test_map.keys().next().expect("a single test key exists");
                if !result.tests.iter().any(|outcome| {
                    outcome.test_key == *test_key
                        && outcome.source == "site-comparison"
                        && outcome.status == site_result.status
                }) {
                    result.tests.push(TestOutcome {
                        test_key: test_key.clone(),
                        status: site_result.status.clone(),
                        source: "site-comparison".into(),
                        evidence: Some(evidence),
                    });
                }
            }
        }
    }
}

/// Materialize per-test `Pass` outcomes only when an executed complete
/// mechanism explicitly returned a criterion-wide pass. A criterion-wide
/// failure does not reveal which test failed, so it stays aggregate evidence.
fn attach_complete_mechanism_passes<'a>(
    results: impl IntoIterator<Item = &'a mut CriterionResult>,
) {
    for result in results {
        if result.status != CriterionStatus::Pass {
            continue;
        }
        let Some(test_map) = RgaaCatalog::tests(&result.criterion_id) else {
            continue;
        };
        for test_key in test_map.keys() {
            let Some(route) = EnginePlan::route_test(&result.criterion_id, test_key) else {
                continue;
            };
            let mechanism = match result.source.as_str() {
                "axe-core" => mechanism_id("axe", &result.criterion_id),
                "gap-fix" => mechanism_id("gapfix", &result.criterion_id),
                _ => continue,
            };
            if route.coverage != CoverageLevel::Complete
                || !route.mechanisms.iter().any(|id| id == &mechanism)
            {
                continue;
            }
            if !result
                .tests
                .iter()
                .any(|test| test.test_key == *test_key && test.source == result.source)
            {
                result.tests.push(TestOutcome {
                    test_key: test_key.clone(),
                    status: CriterionStatus::Pass,
                    source: result.source.clone(),
                    evidence: Some(format!(
                        "{} returned a pass under its complete-coverage contract",
                        result.source
                    )),
                });
            }
        }
    }
}

fn mark_deterministically_not_applicable(result: &mut CriterionResult) {
    result.status = CriterionStatus::NotApplicable;
    result.verified_status = Some(CriterionStatus::NotApplicable);
    if !result.verdict_basis.contains(&VerdictBasis::Deterministic) {
        result.verdict_basis.push(VerdictBasis::Deterministic);
    }
    if let Some(test_map) = RgaaCatalog::tests(&result.criterion_id) {
        for test_key in test_map.keys() {
            if !result.tests.iter().any(|test| {
                test.test_key == *test_key
                    && test.status == CriterionStatus::NotApplicable
                    && is_deterministic_source(&test.source)
            }) {
                result.tests.push(TestOutcome {
                    test_key: test_key.clone(),
                    status: CriterionStatus::NotApplicable,
                    source: "automated".into(),
                    evidence: Some(
                        "deterministic applicability detector marked criterion not applicable"
                            .into(),
                    ),
                });
            }
        }
    }
}

pub struct Orchestrator {
    storage: Option<Arc<dyn Storage>>,
}

/// Events emitted by the audit pipeline for progress tracking.
#[derive(Debug, Clone)]
pub enum AuditEvent {
    Phase(AuditPhase),
    Done(Result<AuditResult, String>),
}

impl Default for Orchestrator {
    fn default() -> Self {
        Self::new()
    }
}

impl Orchestrator {
    pub fn new() -> Self {
        Self { storage: None }
    }

    pub fn with_storage(storage: Arc<dyn Storage>) -> Self {
        Self {
            storage: Some(storage),
        }
    }

    /// Audit a single URL. Behavior is identical to the pre-batch implementation:
    /// it runs the per-URL pipeline and returns a single [`AuditResult`].
    pub async fn run(&self, url: &str, config: &CrawlConfig) -> Result<AuditResult, String> {
        let mut results = self.run_batch(&[url.to_string()], config).await?;
        results
            .remove(url)
            .ok_or_else(|| format!("audit result missing for {url}"))
    }

    /// Audit a URL with full crawl support.
    /// If config.sample_mode is true, uses RGAA mandatory 7-page sampling.
    /// Otherwise, crawls the site up to max_pages/max_depth.
    /// Returns a single AuditResult with all pages and site-wide aggregated metrics.
    pub async fn run_crawl_and_audit(
        &self,
        url: &str,
        config: &CrawlConfig,
    ) -> Result<AuditResult, String> {
        run_crawl_and_audit(self, url, config).await
    }

    /// Audit an explicit, already-discovered list of page URLs, aggregated
    /// into one site-wide [`AuditResult`] the same way
    /// [`Orchestrator::run_crawl_and_audit`] does. For callers that discover
    /// pages another way (e.g. a site's `sitemap.xml`) rather than the RGAA
    /// sample or spider-crawl discovery built into `run_crawl_and_audit`.
    pub async fn run_explicit_audit(
        &self,
        url: &str,
        urls: Vec<String>,
        config: &CrawlConfig,
    ) -> Result<AuditResult, String> {
        run_explicit_audit(self, url, urls, config).await
    }

    /// Audit multiple URLs, returning one [`AuditResult`] per URL keyed by the
    /// URL (successful audits only — a failed URL is logged and skipped, not
    /// allowed to abort the rest of the batch).
    ///
    /// Runs up to [`MAX_CONCURRENT_AUDITS`] audits concurrently under one
    /// shared semaphore rather than one at a time: each audit gets its own
    /// [`BrowserSession`] (via [`ObscuraBridge::handle`], a handle to the
    /// same running server — not a `BrowserSession`/mutex shared across every
    /// in-flight audit), and is persisted to storage as soon as it completes
    /// instead of being held in an accumulator until the whole batch is
    /// done — a crash or kill mid-batch loses only the audits still
    /// in-flight, not every audit that had already finished. The Obscura CDP
    /// server itself is started once before the fan-out and stopped via
    /// [`ObscuraBridge`] `Drop` after every audit has finished.
    pub async fn run_batch(
        &self,
        urls: &[String],
        config: &CrawlConfig,
    ) -> Result<HashMap<String, AuditResult>, String> {
        self.run_batch_observed(urls, config, Arc::new(|_, _| {}))
            .await
    }

    /// [`Orchestrator::run_batch`], plus a callback fired the moment each
    /// individual URL finishes (before the batch as a whole returns).
    ///
    /// Exists because `run_batch` only ever hands back the whole map at the
    /// end: a caller tracking per-URL progress — the REST batch endpoints
    /// (#167) — otherwise had no way to observe anything between "batch
    /// started" and "batch finished", and would have had to fan URLs out
    /// itself, duplicating this concurrency control. The observer runs on
    /// the audit's own task while its semaphore permit is still held, so a
    /// slow observer throttles the fan-out rather than racing ahead of it;
    /// keep it cheap (a store write, not an audit).
    pub async fn run_batch_observed(
        &self,
        urls: &[String],
        config: &CrawlConfig,
        observer: BatchObserver,
    ) -> Result<HashMap<String, AuditResult>, String> {
        use futures::stream::{self, StreamExt};

        let bridge = {
            let mut b = ObscuraBridge::from_env();
            b.start_server().await?;
            b
        };

        let agent_config = rgaa_agent::config::AgentConfig::from_env()
            .map_err(|e| format!("invalid agent configuration: {e}"))?;
        let agent = Arc::new(
            rgaa_agent::agent::RgaaAgent::new(&agent_config)
                .await
                .map_err(|e| format!("failed to create agent: {e}"))?,
        );

        let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT_AUDITS));
        let storage = self.storage.clone();

        let outcomes = stream::iter(urls.iter().cloned())
            .map(|url| {
                let agent = Arc::clone(&agent);
                let tool_ctx = ToolContext::new(BrowserSession::new(bridge.handle()));
                let semaphore = Arc::clone(&semaphore);
                let storage = storage.clone();
                let config = config.clone();
                let observer = Arc::clone(&observer);
                async move {
                    let _permit = semaphore
                        .acquire()
                        .await
                        .expect("semaphore is never closed");

                    let outcome =
                        audit_one(agent, tool_ctx, url.clone(), config, Arc::new(|_| {})).await;

                    if let Ok(audit) = &outcome {
                        if let Some(storage) = &storage {
                            if let Err(e) = storage.save_audit(audit).await {
                                tracing::warn!(url, error = %e, "failed to save audit to storage");
                            }
                        }
                    }

                    // Fired after the persist attempt, so an observer that
                    // marks the URL done can never advertise a result that
                    // is not yet readable from storage.
                    observer(&url, outcome.as_ref().map_err(String::as_str));

                    (url, outcome)
                }
            })
            .buffer_unordered(MAX_CONCURRENT_AUDITS)
            .collect::<Vec<_>>()
            .await;

        let mut results = HashMap::new();
        for (url, outcome) in outcomes {
            match outcome {
                Ok(audit) => {
                    results.insert(url, audit);
                }
                Err(e) => {
                    tracing::warn!(url, error = %e, "audit failed; excluded from batch results");
                }
            }
        }

        Ok(results)
    }

    /// Audit a single URL with progress reporting.
    pub async fn run_with_progress(
        &self,
        url: &str,
        config: &CrawlConfig,
        on_phase: impl Fn(AuditPhase) + Send + Sync + 'static,
    ) -> Result<AuditResult, String> {
        let mut results = self
            .run_batch_with_progress(&[url.to_string()], config, on_phase)
            .await?;
        results
            .remove(url)
            .ok_or_else(|| format!("audit result missing for {url}"))
    }

    /// Audit multiple URLs with progress reporting.
    ///
    /// Unlike [`Orchestrator::run_batch`], this runs URLs sequentially — a
    /// single `on_phase` callback can't meaningfully report progress for
    /// several audits running concurrently at once.
    pub async fn run_batch_with_progress(
        &self,
        urls: &[String],
        config: &CrawlConfig,
        on_phase: impl Fn(AuditPhase) + Send + Sync + 'static,
    ) -> Result<HashMap<String, AuditResult>, String> {
        let bridge = {
            let mut b = ObscuraBridge::from_env();
            b.start_server().await?;
            b
        };

        let session = BrowserSession::new(bridge);
        let tool_ctx = ToolContext::new(session);

        let agent_config = rgaa_agent::config::AgentConfig::from_env()
            .map_err(|e| format!("invalid agent configuration: {e}"))?;
        let agent = rgaa_agent::agent::RgaaAgent::new(&agent_config)
            .await
            .map_err(|e| format!("failed to create agent: {e}"))?;

        let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT_AUDITS));
        let on_phase = Arc::new(on_phase);
        let mut handles = Vec::new();

        for url in urls {
            let permit = semaphore.clone().acquire_owned().await.unwrap();
            let agent = agent.clone();
            let tool_ctx = tool_ctx.clone();
            let config = config.clone();
            let url = url.clone();
            let on_phase = on_phase.clone();
            handles.push(tokio::spawn(async move {
                let _permit = permit;
                let url_for_result = url.clone();
                let audit = audit_one(Arc::new(agent), tool_ctx, url, config, on_phase).await?;
                Ok::<(String, AuditResult), String>((url_for_result, audit))
            }));
        }

        let mut results = HashMap::new();
        for handle in handles {
            let (url, audit) = handle.await.map_err(|e| e.to_string())??;
            results.insert(url, audit);
        }

        if let Some(storage) = &self.storage {
            for (url, audit) in &results {
                if let Err(e) = storage.save_audit(audit).await {
                    tracing::warn!(url, error = %e, "failed to save audit to storage");
                }
            }
        }

        Ok(results)
    }
}

/// Audit a URL with full crawl support.
/// If config.sample_mode is true, uses RGAA mandatory 7-page sampling.
/// Otherwise, crawls the site up to max_pages/max_depth.
/// Returns a single AuditResult with all pages and site-wide aggregated metrics.
pub async fn run_crawl_and_audit(
    orchestrator: &Orchestrator,
    url: &str,
    config: &CrawlConfig,
) -> Result<AuditResult, String> {
    let start = std::time::Instant::now();

    let urls = if config.sample_mode {
        discover_rgaa_sample_pages(url, config).await?
    } else {
        let spider_args = CrawlSiteArgs {
            url: url.to_string(),
            max_pages: Some(config.max_pages as u32),
            max_depth: Some(config.max_depth),
            respect_robots_txt: Some(config.respect_robots),
            concurrency_limit: None,
            request_delay_ms: None,
            request_timeout_ms: None,
            crawl_timeout_ms: None,
            retry_budget: None,
            url_blacklist: None,
        };
        let output = SpiderTool::new()
            .call(spider_args)
            .await
            .map_err(|e| e.to_string())?;
        output.pages.into_iter().map(|p| p.url).collect()
    };

    audit_discovered_urls(orchestrator, url, urls, config, start).await
}

/// Audits an explicit, already-discovered list of page URLs and aggregates
/// them into one site-wide [`AuditResult`] — the same aggregation
/// [`run_crawl_and_audit`] does for its own (RGAA-sample or spider-crawl)
/// discovery, shared here for callers that discover pages another way, e.g.
/// from a site's `sitemap.xml`.
pub async fn run_explicit_audit(
    orchestrator: &Orchestrator,
    url: &str,
    urls: Vec<String>,
    config: &CrawlConfig,
) -> Result<AuditResult, String> {
    let start = std::time::Instant::now();
    audit_discovered_urls(orchestrator, url, urls, config, start).await
}

async fn audit_discovered_urls(
    orchestrator: &Orchestrator,
    url: &str,
    urls: Vec<String>,
    config: &CrawlConfig,
    start: std::time::Instant,
) -> Result<AuditResult, String> {
    let discovered_count = urls.len();
    // Cap at max_pages
    let urls: Vec<String> = urls.into_iter().take(config.max_pages).collect();

    if urls.is_empty() {
        return Err("no pages to audit".to_string());
    }

    let failures = Arc::new(std::sync::Mutex::new(HashMap::<String, String>::new()));
    let observed_failures = Arc::clone(&failures);
    let observer: BatchObserver = Arc::new(move |page_url, outcome| {
        if let Err(error) = outcome {
            if let Ok(mut failures) = observed_failures.lock() {
                failures.insert(page_url.to_string(), error.to_string());
            }
        }
    });
    let mut batch_results = orchestrator
        .run_batch_observed(&urls, config, observer)
        .await?;
    let failures = failures
        .lock()
        .map(|failures| failures.clone())
        .unwrap_or_default();

    // Extract PageResults in the caller's requested order — run_batch
    // returns a HashMap, whose iteration order is arbitrary and would
    // otherwise silently discard a meaningful input order (e.g. sitemap
    // priority ranking) that report/export consumers rely on.
    let mut all_pages = Vec::new();
    for page_url in &urls {
        if let Some(audit) = batch_results.remove(page_url) {
            all_pages.extend(audit.pages);
        } else if let Some(error) = failures.get(page_url) {
            all_pages.push(failed_page_result(page_url, error));
        } else {
            all_pages.push(failed_page_result(
                page_url,
                "audit did not return a result or an error callback",
            ));
        }
    }

    // RGAA 12.1/12.2/12.4/12.5 are scoped to a set of pages. Capture their
    // normalized signals in one Obscura scrape for this exact crawl sample.
    // This is local browser instrumentation; it does not create Holo requests.
    let site_context = ObscuraBridge::extract_page_context_batch(
        ObscuraBridge::from_env().binary_path().to_string(),
        urls.clone(),
        1,
    )
    .await;
    let site_contexts = match site_context {
        Ok(contexts) => contexts,
        Err(error) => {
            tracing::warn!(error = %error, "site-level Obscura observation failed; criteria remain unresolved");
            HashMap::new()
        }
    };
    let page_observations: Vec<site_comparison::PageObservation> = urls
        .iter()
        .filter_map(|page_url| site_contexts.get(page_url))
        .filter_map(|value| serde_json::from_value(value.clone()).ok())
        .collect();
    let failed_pages = urls
        .iter()
        .filter(|page_url| {
            failures.contains_key(*page_url) || !site_contexts.contains_key(*page_url)
        })
        .count();
    let sample_complete = discovered_count < config.max_pages
        && failed_pages == 0
        && page_observations.len() == urls.len();
    let site_results = site_comparison::compare_site(
        &page_observations,
        urls.len(),
        failed_pages,
        sample_complete,
    );
    for site_result in site_results {
        let Some(criterion) = RgaaCriteria::all()
            .iter()
            .find(|criterion| criterion.id == site_result.criterion_id)
        else {
            continue;
        };
        for page in &mut all_pages {
            if let Some(existing) = page
                .criteria
                .iter_mut()
                .find(|result| result.criterion_id == site_result.criterion_id)
            {
                record_site_comparison_evidence(existing, &site_result);
                existing.violations.clear();
            } else {
                let mut result = CriterionResult {
                    criterion_id: criterion.id.to_string(),
                    title: criterion.title.to_string(),
                    classification: criterion.classification,
                    status: site_result.status.clone(),
                    violations: vec![],
                    confidence: None,
                    justification: Some(format!(
                        "{}; sample_complete={}, sampled_pages={}, failed_pages={}",
                        site_result.details,
                        site_result.sample_complete,
                        site_result.sampled_pages,
                        site_result.failed_pages
                    )),
                    source: "site-comparison".to_string(),
                    citations: vec![],
                    considered_sources: vec![],
                    tests: vec![],
                    automated_verdict: None,
                    verdict_basis: Vec::new(),
                    evidence: Vec::new(),
                    confidence_calibration_version: None,
                    review_required: false,
                    review_reason: None,
                    verified_status: None,
                    review_events: Vec::new(),
                };
                record_site_comparison_evidence(&mut result, &site_result);
                page.criteria.push(result);
            }
            page.compliance_rate = calculate_compliance(&page.criteria);
        }
    }

    // Site-wide aggregation
    let (taux_global, coverage_percent, etat_conformite) = aggregate_site_compliance(&all_pages);

    // Flatten all criteria for totals
    let all_criteria: Vec<CriterionResult> =
        all_pages.iter().flat_map(|p| p.criteria.clone()).collect();

    let total = RgaaCriteria::count();
    let pass_count = all_criteria
        .iter()
        .filter(|c| c.status == CriterionStatus::Pass)
        .count();
    let fail_count = all_criteria
        .iter()
        .filter(|c| c.status == CriterionStatus::Fail)
        .count();
    let na_count = all_criteria
        .iter()
        .filter(|c| c.status == CriterionStatus::NotApplicable)
        .count();
    let _error_count = all_criteria
        .iter()
        .filter(|c| c.status == CriterionStatus::Error)
        .count();
    let compliance = calculate_compliance(&all_criteria);

    let audit_complete = pages_have_complete_automatic_coverage(&all_pages);

    Ok(AuditResult {
        audit_id: uuid::Uuid::new_v4().to_string(),
        url: url.to_string(),
        pages: all_pages,
        total_criteria: total,
        passed: pass_count,
        failed: fail_count,
        na: na_count,
        overall_compliance: compliance,
        taux_global,
        coverage_percent,
        etat_conformite,
        duration_ms: start.elapsed().as_millis() as u64,
        audit_complete,
    })
}

fn select_holo_candidates(prior_results: &[CriterionResult]) -> Vec<rgaa_core::Criterion> {
    let settled: std::collections::HashSet<&str> = prior_results
        .iter()
        .filter(|result| {
            matches!(
                result.status,
                CriterionStatus::Pass | CriterionStatus::Fail | CriterionStatus::NotApplicable
            )
        })
        .map(|result| result.criterion_id.as_str())
        .collect();

    RgaaCriteria::all()
        .iter()
        .filter(|criterion| EnginePlan::primary(criterion.id) == Some(PlanEngine::Holo))
        .filter(|criterion| !settled.contains(criterion.id))
        .cloned()
        .collect()
}

fn failed_page_result(url: &str, error: &str) -> PageResult {
    let criteria = RgaaCriteria::all()
        .iter()
        .map(|criterion| CriterionResult {
            criterion_id: criterion.id.to_string(),
            title: criterion.title.to_string(),
            classification: criterion.classification,
            status: CriterionStatus::NotTested,
            violations: vec![],
            confidence: None,
            justification: Some(format!("Page audit failed: {error}")),
            source: "audit-error".to_string(),
            citations: vec![],
            considered_sources: vec![],
            tests: vec![],
            automated_verdict: None,
            verdict_basis: Vec::new(),
            evidence: Vec::new(),
            confidence_calibration_version: None,
            review_required: false,
            review_reason: None,
            verified_status: None,
            review_events: Vec::new(),
        })
        .collect();
    PageResult {
        url: url.to_string(),
        title: None,
        criteria,
        compliance_rate: 0.0,
        crawl_depth: 0,
    }
}

/// Discover RGAA mandatory 7 sample pages.
/// Returns URLs for: Accueil, Contact, Mentions légales, Accessibilité, Aide, Plan du site, Authentification (if exists).
async fn discover_rgaa_sample_pages(
    base_url: &str,
    config: &CrawlConfig,
) -> Result<Vec<String>, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .map_err(|e| e.to_string())?;

    let base = base_url.trim_end_matches('/');
    let mut pages = vec![base.to_string()]; // 1. Accueil

    let patterns = [
        (
            "contact",
            vec!["/contact", "/contactez-nous", "/nous-contacter"],
        ),
        (
            "mentions_legales",
            vec!["/mentions-legales", "/mentions-legales"],
        ),
        (
            "accessibilite",
            vec![
                "/accessibilite",
                "/declaration-accessibilite",
                "/accessibilite",
            ],
        ),
        ("aide", vec!["/aide", "/help", "/faq"]),
        ("plan_site", vec!["/plan-du-site", "/sitemap", "/plan-site"]),
    ];

    for (_name, paths) in patterns {
        for path in paths {
            let test_url = format!("{}{}", base, path);
            if let Ok(resp) = client.head(&test_url).send().await {
                if resp.status().is_success() || resp.status().is_redirection() {
                    pages.push(test_url);
                    break;
                }
            }
        }
    }

    // 7. Auth - only add if ANY auth path exists
    let auth_paths = vec![
        "/connexion",
        "/login",
        "/authentification",
        "/identification",
    ];
    for path in auth_paths {
        let test_url = format!("{}{}", base, path);
        if let Ok(resp) = client.head(&test_url).send().await {
            if resp.status().is_success() || resp.status().is_redirection() {
                pages.push(test_url);
                break;
            }
        }
    }

    // Fallback: if < 7 pages, shallow spider crawl
    if pages.len() < 7 {
        let spider_args = CrawlSiteArgs {
            url: base.to_string(),
            max_pages: Some(20),
            max_depth: Some(1),
            respect_robots_txt: Some(config.respect_robots),
            concurrency_limit: None,
            request_delay_ms: None,
            request_timeout_ms: None,
            crawl_timeout_ms: None,
            retry_budget: None,
            url_blacklist: None,
        };
        if let Ok(output) = SpiderTool::new().call(spider_args).await {
            for page in output.pages {
                if pages.len() >= config.max_pages.min(7) {
                    break;
                }
                if !pages.contains(&page.url) {
                    pages.push(page.url);
                }
            }
        }
    }

    pages.truncate(config.max_pages.min(7));
    Ok(pages)
}

/// Aggregate site-wide compliance per RGAA official rule:
/// A criterion is NonConforme for the entire site if it fails on ANY page of the sample.
/// Returns (taux_global, coverage_percent, etat_conformite).
pub fn aggregate_site_compliance(page_results: &[PageResult]) -> (f64, f64, String) {
    use std::collections::HashMap;

    // Group criterion results by criterion_id across all pages
    let mut criterion_statuses: HashMap<String, Vec<CriterionStatus>> = HashMap::new();
    let mut criterion_classifications: HashMap<String, Classification> = HashMap::new();
    let mut validated_total = 0;
    let mut validated_executed = 0;

    for page in page_results {
        for criterion in &page.criteria {
            criterion_statuses
                .entry(criterion.criterion_id.clone())
                .or_default()
                .push(criterion.status.clone());
            criterion_classifications
                .insert(criterion.criterion_id.clone(), criterion.classification);
        }
    }

    // Apply RGAA rule: NC if ANY page has Fail/Error
    let mut conforme = 0;
    let mut non_conforme = 0;

    for (criterion_id, statuses) in criterion_statuses {
        let classification = criterion_classifications
            .get(&criterion_id)
            .copied()
            .unwrap_or(Classification::Manuel);

        // Skip Manuel criteria from taux calculation (they're NonTeste)
        if classification == Classification::Manuel {
            continue;
        }

        // Count for coverage
        if let Some((_theme, cat)) = RgaaCatalog::by_id(&criterion_id) {
            if matches!(
                cat.automatable,
                Automatable::FullyAutomatable | Automatable::PartiallyAutomatable
            ) {
                validated_total += 1;
                if rgaa_report::is_validated(&statuses) {
                    validated_executed += 1;
                }
            }
        }

        let has_fail_or_error = statuses
            .iter()
            .any(|s| matches!(s, CriterionStatus::Fail | CriterionStatus::Error));
        let all_pass = statuses.iter().all(|s| matches!(s, CriterionStatus::Pass));
        let all_na = statuses
            .iter()
            .all(|s| matches!(s, CriterionStatus::NotApplicable));

        if all_na {
            continue; // NA excluded from denominator
        }

        if has_fail_or_error {
            non_conforme += 1;
        } else if all_pass {
            conforme += 1;
        } else {
            // Mixed Pass/NeedsReview/NotTested → NonTeste (excluded from taux)
            continue;
        }
    }

    let taux_global = if conforme + non_conforme > 0 {
        (conforme as f64 / (conforme + non_conforme) as f64) * 100.0
    } else {
        0.0
    };

    let coverage_percent = if validated_total > 0 {
        (validated_executed as f64 / validated_total as f64) * 100.0
    } else {
        0.0
    };

    let etat_conformite = if taux_global >= 100.0 {
        "totale"
    } else if taux_global >= 50.0 {
        "partielle"
    } else {
        "non conforme"
    }
    .to_string();

    (taux_global, coverage_percent, etat_conformite)
}

/// Run the full per-URL audit pipeline against a browser bridge.
///
/// This is the single source of truth for the audit logic; both [`Orchestrator::run`]
/// and [`Orchestrator::run_batch`] route through it so a single URL produces
/// identical results regardless of entry point.
/// This is the single source of truth for the audit logic; both [`Orchestrator::run`]
/// and [`Orchestrator::run_batch`] route through it so a single URL produces
/// identical results regardless of entry point.
///
/// Takes owned values (not references) so the returned future is `Send` for
/// all lifetimes — required by `tokio::spawn` and `buffer_unordered`.
async fn audit_one(
    agent: Arc<RgaaAgent>,
    tool_ctx: ToolContext,
    url: String,
    _config: CrawlConfig,
    on_phase: Arc<dyn Fn(AuditPhase) + Send + Sync>,
) -> Result<AuditResult, String> {
    let start = std::time::Instant::now();
    info!(url, "Starting audit");

    // Extract the bridge Arc and release the parking_lot lock before any .await.
    // parking_lot::MutexGuard is not Send, so holding it across .await would
    // prevent buffer_unordered from spawning the future concurrently.
    let bridge = {
        let session_arc = tool_ctx.session().clone();
        let guard = session_arc.lock();
        let b = guard.bridge();
        drop(guard);
        b
    };

    // Single-element slice so every browser call below routes through the same
    // batch entry point multi-URL callers use — one live path, no per-page/
    // snippet one-off browser process spawns on the audit path.
    let urls = vec![url.clone()];

    // 1. Run axe-core
    on_phase(AuditPhase::Axe);
    info!("Running axe-core");
    let mut axe_by_url = bridge.clone().run_axe_batch(urls.clone(), 1).await?;
    let axe_violations = axe_by_url
        .remove(url.as_str())
        .ok_or_else(|| format!("axe-core produced no result for {url}"))?;
    let axe_results = AxeMapper::map(&axe_violations).map_err(|e| e.to_string())?;
    let mut axe_results = admit_routed_results(axe_results, "axe");
    attach_complete_mechanism_passes(axe_results.values_mut());

    // 2. Run gap-fix rules for 10 false negatives
    on_phase(AuditPhase::GapFix);
    info!("Running gap-fix rules");
    let gap_snippets = routed_gap_fix_snippets();
    // clippy's `--all-targets` (dev-profile) check reports the `&` here as a
    // needless borrow, but the actual `[profile.test]` build (cargo test /
    // nextest, and thus CI) requires it — `gap_snippets` alone fails to
    // type-check there with "expected `&HashMap<_, &_>`, found `HashMap<_,
    // &_>`". Keeping the borrow so the real test build stays green.
    #[allow(clippy::needless_borrow)]
    let mut gap_by_url = ObscuraBridge::run_gap_fix_batch(
        bridge.binary_path().to_string(),
        urls.clone(),
        gap_snippets
            .iter()
            .map(|(k, v)| (k.clone(), v.to_string()))
            .collect(),
        1,
    )
    .await?;
    let gap_js_results = gap_by_url.remove(url.as_str()).unwrap_or_default();
    let mut gap_results =
        admit_routed_results(GapFixRules::parse_results(&gap_js_results), "gapfix");
    attach_complete_mechanism_passes(gap_results.values_mut());

    // Obscura keyboard actions are limited to Tab key-down/up events. The
    // observation is shared by criteria 12.8 and 12.9; no activation key or
    // pointer click is sent.
    let keyboard_is_routed = ["12.8", "12.9"].iter().any(|criterion_id| {
        criterion_has_routed_mechanism(criterion_id, &mechanism_id("keyboard", criterion_id))
    });
    let keyboard_results = if keyboard_is_routed {
        match bridge.observe_keyboard(&url).await {
            Ok(observation) => {
                let issue_rules: Vec<String> = observation
                    .keyboard
                    .issues
                    .iter()
                    .map(|issue| issue.rule.clone())
                    .collect();
                GapFixRules::parse_keyboard_observation(
                    &observation.keyboard.status,
                    &issue_rules,
                    observation.keyboard.igt_elements.len(),
                )
            }
            Err(error) => {
                tracing::warn!(url, error = %error, "Obscura keyboard probe failed; retain static review results");
                HashMap::new()
            }
        }
    } else {
        HashMap::new()
    };

    // 3. Extract page context for Holo3 prompts
    on_phase(AuditPhase::PageContext);
    info!("Extracting page context");
    let mut context_by_url = ObscuraBridge::extract_page_context_batch(
        bridge.binary_path().to_string(),
        urls.clone(),
        1,
    )
    .await?;
    let raw_context = context_by_url
        .remove(url.as_str())
        .ok_or_else(|| format!("page context extraction produced no result for {url}"))?;
    let na_map = na_detection::detect_na(&raw_context);
    // A malformed page context must fail the audit, not silently evaluate as
    // an empty page — an all-empty PageContext would otherwise sail through
    // every criterion and produce green-looking verdicts over no real data.
    let page_context: PageContext = serde_json::from_value(raw_context).map_err(|e| {
        tracing::warn!(url, error = %e, "malformed page context; failing audit instead of evaluating empty data");
        format!("malformed page context for {url}: {e}")
    })?;

    // 4. Request an automatic estimate for every catalog criterion, including
    // criteria owned by a deterministic engine or a human. The plan grouping
    // keeps each primary route visible while the estimator still covers every
    // routed test key, not just unresolved criterion-level leftovers.
    on_phase(AuditPhase::AgentIaAssiste);
    let prior_results: Vec<CriterionResult> = axe_results
        .values()
        .chain(gap_results.values())
        .chain(keyboard_results.values())
        .cloned()
        .collect();
    let mut by_engine: HashMap<PlanEngine, Vec<Criterion>> = HashMap::new();
    for criterion in RgaaCriteria::all() {
        if let Some(engine) = EnginePlan::primary(criterion.id) {
            by_engine.entry(engine).or_default().push(criterion.clone());
        }
    }
    let mut ordered_criteria = Vec::with_capacity(RgaaCriteria::count());
    for engine in [
        PlanEngine::AxeCore,
        PlanEngine::Deterministic,
        PlanEngine::Holo,
        PlanEngine::Human,
    ] {
        if let Some(criteria) = by_engine.remove(&engine) {
            info!(engine = ?engine, criteria = criteria.len(), "Queueing automatic estimates");
            ordered_criteria.extend(criteria);
        }
    }
    let estimate_results = agent
        .run_automatic_estimates(&ordered_criteria, &page_context, &prior_results)
        .await;

    // Retain the progress event for consumers expecting six ordered phases;
    // this is bookkeeping only and intentionally issues no second Holo call.
    on_phase(AuditPhase::AgentPartial);

    // 5. Merge results
    //
    // Precedence is decided by `merge::merge_results`, not by the order the
    // sources are listed here: an errored evaluation never overwrites a
    // verdict, and deterministic evidence outranks an LLM verdict. The order
    // below only fixes the order of `considered_sources` on each winner.
    on_phase(AuditPhase::Merging);
    let mut all_results: HashMap<String, CriterionResult> = merge::merge_results(
        axe_results
            .into_iter()
            .chain(gap_results)
            .chain(keyboard_results)
            .chain(estimate_results),
    );

    // 6. Ensure every criterion has an entry, so the result always spans the
    // full 106-criterion catalog.
    //
    // Silence is not evidence: a Déterministe criterion that no mechanism
    // flagged (and that Holo3 did not decide) is `NotTested`, never `Pass`.
    // Only a mechanism that can actually fail the criterion may pass it (#199,
    // #201). `NotTested` and `NeedsReview` are both left out of `taux_global`,
    // so the rate is optimistic by exactly the criteria nobody decided.
    // Manuel criteria always require human review -> NeedsReview.
    // PartiallyAutomatable criteria need human review for un-covered portions
    // -> NeedsReview.
    let all_criteria = RgaaCriteria::all();
    for criterion in all_criteria {
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
                    citations: vec![],
                    considered_sources: vec![],
                    tests: vec![],
                    automated_verdict: None,
                    verdict_basis: Vec::new(),
                    evidence: Vec::new(),
                    confidence_calibration_version: None,
                    review_required: false,
                    review_reason: None,
                    verified_status: None,
                    review_events: Vec::new(),
                });
        } else if !all_results.contains_key(criterion.id) {
            let is_partially_automatable = RgaaCatalog::by_id(criterion.id)
                .is_some_and(|(_, cat)| cat.automatable == Automatable::PartiallyAutomatable);

            let (status, justification, source) = if is_partially_automatable {
                (
                    partially_automatable_status(),
                    "Partially automatable — human review required for uncovered portions".into(),
                    "partially-automatable".into(),
                )
            } else {
                (
                    CriterionStatus::NotTested,
                    "Not tested — no automated check covered this criterion".into(),
                    "automated".into(),
                )
            };

            all_results
                .entry(criterion.id.to_string())
                .or_insert_with(|| CriterionResult {
                    criterion_id: criterion.id.to_string(),
                    title: criterion.title.to_string(),
                    classification: criterion.classification,
                    status,
                    violations: vec![],
                    confidence: None,
                    justification: Some(justification),
                    source,
                    citations: vec![],
                    considered_sources: vec![],
                    tests: vec![],
                    automated_verdict: None,
                    verdict_basis: Vec::new(),
                    evidence: Vec::new(),
                    confidence_calibration_version: None,
                    review_required: false,
                    review_reason: None,
                    verified_status: None,
                    review_events: Vec::new(),
                });
        }
    }

    // 7. Apply NA detection
    let mut criteria: Vec<CriterionResult> = all_results.into_values().collect();
    for criterion in &mut criteria {
        if let Some(&false) = na_map.get(criterion.criterion_id.as_str()) {
            mark_deterministically_not_applicable(criterion);
        }
    }

    let coverage_result = validate_automatic_verdict_coverage(&criteria);
    if let Err(error) = &coverage_result {
        tracing::warn!(missing = ?error.missing_criterion_ids, "automatic verdict coverage is incomplete");
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
        "Audit finished"
    );

    Ok(AuditResult {
        audit_id: uuid::Uuid::new_v4().to_string(),
        url: url.clone(),
        pages: vec![PageResult {
            url: url.clone(),
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
        audit_complete: coverage_result.is_ok(),
    })
}

#[cfg(test)]
mod routing_tests {
    use super::*;
    use rgaa_core::types::Violation;

    fn deterministic_result(
        criterion_id: &str,
        status: CriterionStatus,
        source: &str,
    ) -> CriterionResult {
        let criterion = RgaaCriteria::find(criterion_id).expect("criterion exists");
        CriterionResult {
            criterion_id: criterion_id.to_string(),
            title: criterion.title.clone(),
            classification: criterion.classification,
            status,
            violations: Vec::<Violation>::new(),
            confidence: None,
            justification: None,
            source: source.to_string(),
            citations: vec![],
            considered_sources: vec![],
            tests: vec![],
            automated_verdict: None,
            verdict_basis: Vec::new(),
            evidence: Vec::new(),
            confidence_calibration_version: None,
            review_required: false,
            review_reason: None,
            verified_status: None,
            review_events: Vec::new(),
        }
    }

    fn complete_prediction_set() -> Vec<CriterionResult> {
        let mut results: HashMap<String, CriterionResult> = RgaaCriteria::all()
            .iter()
            .map(|criterion| {
                let mut result = deterministic_result(
                    criterion.id,
                    CriterionStatus::NeedsReview,
                    "agent-estimate",
                );
                result.automated_verdict = Some(rgaa_core::AutomatedVerdict::Pass);
                (criterion.id.to_owned(), result)
            })
            .collect();

        for (criterion_id, test_key) in RgaaCatalog::all_test_keys() {
            let route = EnginePlan::route_test(&criterion_id, &test_key)
                .expect("every canonical test has a route");
            let source = if route.coverage == CoverageLevel::Complete {
                "axe-core"
            } else {
                "agent-estimate"
            };
            results
                .get_mut(&criterion_id)
                .expect("criterion belongs to catalog")
                .tests
                .push(TestOutcome {
                    test_key,
                    status: CriterionStatus::Pass,
                    source: source.into(),
                    evidence: None,
                });
        }
        results.into_values().collect()
    }

    #[test]
    fn only_unique_holo_primary_routes_without_a_deterministic_verdict_are_dispatched() {
        let determined = [deterministic_result(
            "1.2",
            CriterionStatus::Fail,
            "axe-core",
        )];

        let candidates = select_holo_candidates(&determined);
        let ids: std::collections::HashSet<&str> =
            candidates.iter().map(|criterion| criterion.id).collect();

        assert_eq!(candidates.len(), 31);
        assert_eq!(ids.len(), candidates.len());
        assert!(!ids.contains("1.2"));
        assert!(ids.contains("3.1"));
        assert!(!ids.contains("4.2"));
    }

    #[test]
    fn failed_page_is_retained_with_every_criterion_not_tested() {
        let page = failed_page_result("https://example.test/forms", "navigation timed out");

        assert_eq!(page.url, "https://example.test/forms");
        assert_eq!(page.criteria.len(), 106);
        assert!(page.criteria.iter().all(|criterion| {
            criterion.status == CriterionStatus::NotTested
                && criterion.source == "audit-error"
                && criterion
                    .justification
                    .as_deref()
                    .is_some_and(|reason| reason.contains("navigation timed out"))
        }));
    }

    #[test]
    fn automatic_verdict_coverage_requires_every_routed_test_outcome() {
        let error = validate_automatic_verdict_coverage(&[])
            .expect_err("an empty page result cannot cover the catalog");
        assert_eq!(error.missing_criterion_ids.len(), RgaaCriteria::count());
        assert!(error.missing_criterion_ids.iter().any(|id| id == "4.2"));
    }

    #[test]
    fn complete_routes_require_deterministic_test_evidence() {
        let mut results = complete_prediction_set();
        assert!(validate_automatic_verdict_coverage(&results).is_ok());

        let result = results
            .iter_mut()
            .find(|result| result.criterion_id == "1.1")
            .expect("criterion 1.1 exists");
        result.tests[0].source = "agent-estimate".into();

        let error = validate_automatic_verdict_coverage(&results)
            .expect_err("model estimates alone cannot close a complete route");
        assert_eq!(error.missing_criterion_ids, vec!["1.1"]);
    }

    #[test]
    fn aggregate_failure_without_a_test_key_keeps_the_audit_incomplete() {
        let mut results = complete_prediction_set();
        let result = results
            .iter_mut()
            .find(|result| result.criterion_id == "1.1")
            .expect("criterion 1.1 exists");
        result.status = CriterionStatus::Fail;

        let error = validate_automatic_verdict_coverage(&results)
            .expect_err("aggregate failure does not identify a test key");
        assert!(error.missing_criterion_ids.iter().any(|id| id == "1.1"));
    }

    #[test]
    fn every_criterion_needs_an_automatic_prediction() {
        let mut results = complete_prediction_set();
        results
            .iter_mut()
            .find(|result| result.criterion_id == "4.2")
            .expect("criterion 4.2 exists")
            .automated_verdict = None;

        let error = validate_automatic_verdict_coverage(&results)
            .expect_err("missing model output must stay incomplete");
        assert_eq!(error.missing_criterion_ids, vec!["4.2"]);
    }

    #[test]
    fn complete_mechanism_pass_materializes_routes_but_failure_does_not_guess_test_keys() {
        let mut results: HashMap<String, CriterionResult> = HashMap::new();
        let pass = deterministic_result("1.1", CriterionStatus::Pass, "axe-core");
        results.insert("1.1".into(), pass);
        let fail = deterministic_result("1.2", CriterionStatus::Fail, "gap-fix");
        results.insert("1.2".into(), fail);

        attach_complete_mechanism_passes(results.values_mut());

        assert_eq!(results["1.1"].tests.len(), 8);
        assert!(results["1.1"]
            .tests
            .iter()
            .all(|test| { test.status == CriterionStatus::Pass && test.source == "axe-core" }));
        assert!(results["1.2"].tests.is_empty());
    }

    #[test]
    fn route_plan_controls_mechanism_dispatch_and_result_admission() {
        assert!(criterion_has_routed_mechanism("1.1", "axe-1-1"));
        assert!(!criterion_has_routed_mechanism("1.2", "axe-1-2"));
        assert!(criterion_has_routed_mechanism("1.2", "gapfix-1-2"));

        let admitted = admit_routed_results(
            [
                (
                    "1.1".to_string(),
                    deterministic_result("1.1", CriterionStatus::Fail, "axe-core"),
                ),
                (
                    "1.2".to_string(),
                    deterministic_result("1.2", CriterionStatus::Fail, "axe-core"),
                ),
            ],
            "axe",
        );
        assert_eq!(admitted.len(), 1);
        assert!(admitted.contains_key("1.1"));

        let snippets = routed_gap_fix_snippets();
        assert!(snippets.contains_key("1.1"));
        assert!(snippets.contains_key("1.2"));
        assert!(!snippets.contains_key("10.1"));
        assert!(!snippets.contains_key("1.9"));
        assert!(!criterion_has_routed_mechanism("12.8", "keyboard-12-8"));
    }

    #[test]
    fn multi_page_audit_is_complete_only_when_every_page_passes_coverage_gate() {
        let complete = PageResult {
            url: "https://example.test/complete".into(),
            title: None,
            criteria: complete_prediction_set(),
            compliance_rate: 0.0,
            crawl_depth: 0,
        };
        let mut incomplete = complete.clone();
        incomplete.url = "https://example.test/incomplete".into();
        incomplete
            .criteria
            .iter_mut()
            .find(|result| result.criterion_id == "4.2")
            .expect("criterion 4.2 exists")
            .automated_verdict = None;

        assert!(pages_have_complete_automatic_coverage(
            std::slice::from_ref(&complete)
        ));
        assert!(pages_have_complete_automatic_coverage(&[
            complete.clone(),
            complete.clone()
        ]));
        assert!(!pages_have_complete_automatic_coverage(&[
            complete, incomplete
        ]));
        assert!(!pages_have_complete_automatic_coverage(&[]));
    }

    #[test]
    fn keyed_site_failure_preserves_model_pass_and_is_counted_as_deterministic() {
        let mut results = complete_prediction_set();
        let result = results
            .iter_mut()
            .find(|result| result.criterion_id == "12.1")
            .expect("criterion 12.1 exists");
        let site_failure = site_comparison::SiteCriterionObservation {
            criterion_id: "12.1",
            status: CriterionStatus::Fail,
            details: "one observed page lacks a navigation system".into(),
            sampled_pages: 2,
            failed_pages: 0,
            sample_complete: true,
        };

        record_site_comparison_evidence(result, &site_failure);

        assert_eq!(result.status, CriterionStatus::Fail);
        assert_eq!(result.verified_status, Some(CriterionStatus::Fail));
        assert_eq!(
            result.automated_verdict,
            Some(rgaa_core::AutomatedVerdict::Pass)
        );
        assert!(result.verdict_basis.contains(&VerdictBasis::Deterministic));
        assert!(result
            .considered_sources
            .contains(&"site-comparison".into()));
        assert!(result.tests.iter().any(|outcome| {
            outcome.test_key == "1"
                && outcome.source == "site-comparison"
                && outcome.status == CriterionStatus::Fail
        }));
        assert_eq!(
            rgaa_core::reduce_test_outcomes(&result.tests, &["1".to_string()]),
            Some(CriterionStatus::Fail)
        );
        assert!(validate_automatic_verdict_coverage(&results).is_ok());
    }

    #[test]
    fn unkeyed_site_failure_for_multi_test_criterion_keeps_coverage_incomplete() {
        let mut results = complete_prediction_set();
        let result = results
            .iter_mut()
            .find(|result| result.criterion_id == "12.4")
            .expect("criterion 12.4 exists");
        let site_failure = site_comparison::SiteCriterionObservation {
            criterion_id: "12.4",
            status: CriterionStatus::Fail,
            details: "site-wide sitemap placement signatures differ".into(),
            sampled_pages: 2,
            failed_pages: 0,
            sample_complete: true,
        };

        record_site_comparison_evidence(result, &site_failure);

        assert_eq!(result.status, CriterionStatus::Fail);
        assert_eq!(
            result.automated_verdict,
            Some(rgaa_core::AutomatedVerdict::Pass)
        );
        assert!(!result
            .tests
            .iter()
            .any(|outcome| outcome.source == "site-comparison"));
        let error = validate_automatic_verdict_coverage(&results)
            .expect_err("an unkeyed site failure cannot be closed by model rows");
        assert!(error.missing_criterion_ids.contains(&"12.4".to_string()));
    }

    #[test]
    fn site_needs_review_does_not_block_an_existing_mechanism_failure() {
        let mut results = complete_prediction_set();
        let result = results
            .iter_mut()
            .find(|result| result.criterion_id == "12.4")
            .expect("criterion 12.4 exists");
        result.status = CriterionStatus::Fail;
        result.source = "gap-fix".into();
        result.considered_sources.push("gap-fix".into());
        let site_review = site_comparison::SiteCriterionObservation {
            criterion_id: "12.4",
            status: CriterionStatus::NeedsReview,
            details: "target relevance still requires review".into(),
            sampled_pages: 2,
            failed_pages: 0,
            sample_complete: true,
        };

        record_site_comparison_evidence(result, &site_review);

        assert_eq!(result.status, CriterionStatus::Fail);
        assert!(result
            .considered_sources
            .contains(&"site-comparison".into()));
        assert!(result
            .justification
            .as_deref()
            .is_some_and(|text| text.contains("Site-level comparison returned NeedsReview:")));
        assert!(validate_automatic_verdict_coverage(&results).is_ok());
    }

    #[test]
    fn deterministic_na_updates_verified_status_without_replacing_model_prediction() {
        let mut result =
            deterministic_result("4.2", CriterionStatus::NeedsReview, "agent-estimate");
        result.automated_verdict = Some(rgaa_core::AutomatedVerdict::Fail);
        result.verdict_basis = vec![VerdictBasis::ModelEstimate];

        mark_deterministically_not_applicable(&mut result);

        assert_eq!(result.status, CriterionStatus::NotApplicable);
        assert_eq!(result.verified_status, Some(CriterionStatus::NotApplicable));
        assert_eq!(
            result.automated_verdict,
            Some(rgaa_core::AutomatedVerdict::Fail)
        );
        assert!(result.verdict_basis.contains(&VerdictBasis::Deterministic));
        assert_eq!(result.tests.len(), RgaaCatalog::tests("4.2").unwrap().len());
    }
}
