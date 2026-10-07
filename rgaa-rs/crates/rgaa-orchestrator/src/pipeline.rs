use crate::{merge, site_comparison};
use rgaa_agent::agent::RgaaAgent;
use rgaa_browser_tools::{BrowserSession, ToolContext};
use rgaa_core::catalog::Automatable;
use rgaa_core::na_detection;
use rgaa_core::{
    AuditResult, Classification, CrawlConfig, CriterionResult, CriterionStatus, EnginePlan,
    PageResult, PlanEngine, RgaaCatalog, RgaaCriteria,
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
                if existing.status == CriterionStatus::Fail {
                    let note = format!(
                        "Site-level comparison also found: {}; sample_complete={}, sampled_pages={}, failed_pages={}",
                        site_result.details,
                        site_result.sample_complete,
                        site_result.sampled_pages,
                        site_result.failed_pages
                    );
                    existing
                        .justification
                        .get_or_insert_with(String::new)
                        .push_str(&format!("; {note}"));
                    if !existing
                        .considered_sources
                        .iter()
                        .any(|s| s == "site-comparison")
                    {
                        existing
                            .considered_sources
                            .push("site-comparison".to_string());
                    }
                    continue;
                }
                existing.status = site_result.status.clone();
                existing.source = "site-comparison".to_string();
                existing.justification = Some(format!(
                    "{}; sample_complete={}, sampled_pages={}, failed_pages={}",
                    site_result.details,
                    site_result.sample_complete,
                    site_result.sampled_pages,
                    site_result.failed_pages
                ));
                existing.violations.clear();
                existing.tests.clear();
            } else {
                page.criteria.push(CriterionResult {
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
                });
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
        audit_complete: false,
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

    // 2. Run gap-fix rules for 10 false negatives
    on_phase(AuditPhase::GapFix);
    info!("Running gap-fix rules");
    let gap_snippets = GapFixRules::snippets();
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
    let gap_results = GapFixRules::parse_results(&gap_js_results);

    // Obscura keyboard actions are limited to Tab key-down/up events. The
    // observation is shared by criteria 12.8 and 12.9; no activation key or
    // pointer click is sent.
    let keyboard_results = match bridge.observe_keyboard(&url).await {
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

    // 4. Route only the criteria owned by Holo in the audited EnginePlan.
    // Axe and deterministic evidence already available for this page suppress
    // a Holo request when it produced a conclusive result.
    on_phase(AuditPhase::AgentIaAssiste);
    let prior_results: Vec<CriterionResult> = axe_results
        .values()
        .chain(gap_results.values())
        .chain(keyboard_results.values())
        .cloned()
        .collect();
    let holo_criteria = select_holo_candidates(&prior_results);
    info!(
        criteria = holo_criteria.len(),
        "Running one deduplicated Holo batch pipeline for Holo-owned criteria"
    );

    let agent_results = agent
        .clone()
        .run_ia_assiste(holo_criteria, page_context.clone())
        .await;

    let holo_results = agent_results;

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
            .chain(holo_results),
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
        audit_complete: false,
    })
}
