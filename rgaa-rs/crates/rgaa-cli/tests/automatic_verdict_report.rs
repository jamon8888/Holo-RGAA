use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use chrono::Utc;
use rgaa_agent::agent::RgaaAgent;
use rgaa_agent::config::AgentConfig;
use rgaa_cli::commands::report::{run as render_cli_report, ReportArgs};
use rgaa_cli::commands::review::apply_review;
use rgaa_cli::commands::CommonArgs;
use rgaa_core::test_plan::{CoverageLevel, TestRoutePlan};
use rgaa_core::{
    AuditBundle, AuditResult, Classification, CriterionResult, CriterionStatus, PageResult,
    RgaaCriteria, TestOutcome, VerdictBasis, Violation,
};
use rgaa_orchestrator::{merge_candidates, pipeline::validate_automatic_verdict_coverage};
use rgaa_report::{compute_audit_metrics, ReportFormat};

/// A bounded local OpenAI-compatible endpoint. Its response uses the real
/// Task 4 batch JSON contract and is consumed by the unchanged agent parser.
struct MockProvider {
    stop: Arc<AtomicBool>,
    requests: Arc<AtomicUsize>,
    server: Option<JoinHandle<()>>,
    base_url: String,
}

impl MockProvider {
    fn start(response: String, status: u16) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("local mock should bind");
        listener
            .set_nonblocking(true)
            .expect("local mock should be nonblocking");
        let address = listener.local_addr().expect("local address");
        let stop = Arc::new(AtomicBool::new(false));
        let requests = Arc::new(AtomicUsize::new(0));
        let thread_stop = Arc::clone(&stop);
        let thread_requests = Arc::clone(&requests);
        let server = thread::spawn(move || {
            while !thread_stop.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok((stream, _)) => {
                        thread_requests.fetch_add(1, Ordering::Relaxed);
                        let _ = serve_completion(stream, &response, status);
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(2));
                    }
                    Err(_) => break,
                }
            }
        });
        Self {
            stop,
            requests,
            server: Some(server),
            base_url: format!("http://{address}/v1"),
        }
    }

    fn request_count(&self) -> usize {
        self.requests.load(Ordering::Relaxed)
    }
}

impl Drop for MockProvider {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(server) = self.server.take() {
            let _ = server.join();
        }
    }
}

fn serve_completion(mut stream: TcpStream, content: &str, status: u16) -> std::io::Result<()> {
    stream.set_read_timeout(Some(Duration::from_secs(3)))?;
    let mut request = Vec::new();
    let mut chunk = [0_u8; 4096];
    loop {
        let read = stream.read(&mut chunk)?;
        if read == 0 {
            break;
        }
        request.extend_from_slice(&chunk[..read]);
        if let Some(header_end) = request.windows(4).position(|window| window == b"\r\n\r\n") {
            let headers = String::from_utf8_lossy(&request[..header_end]);
            let body_len = headers
                .lines()
                .find_map(|line| {
                    let (key, value) = line.split_once(':')?;
                    key.eq_ignore_ascii_case("content-length")
                        .then(|| value.trim().parse::<usize>().ok())
                        .flatten()
                })
                .unwrap_or(0);
            if request.len() >= header_end + 4 + body_len {
                break;
            }
        }
    }

    let body = serde_json::json!({
        "id": "mock-completion",
        "object": "chat.completion",
        "model": "mock-model",
        "created": 0,
        "usage": {"prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2},
        "choices": [{
            "index": 0,
            "message": {"role": "assistant", "content": content},
            "finish_reason": "stop"
        }]
    })
    .to_string();
    let reason = if status == 200 { "OK" } else { "Unavailable" };
    write!(
        stream,
        "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )?;
    stream.flush()
}

fn provider_response() -> String {
    let routes = TestRoutePlan::builtin();
    let items: Vec<_> = RgaaCriteria::all()
        .iter()
        .map(|criterion| {
            let tests: Vec<_> = routes
                .routes()
                .iter()
                .filter(|route| {
                    route.criterion_id == criterion.id && route.fallback == "holo_estimate"
                })
                .map(|route| {
                    serde_json::json!({
                        "test_key": route.test_key,
                        "verdict": "pass",
                        "justification": "mock model observed the supplied page context"
                    })
                })
                .collect();
            serde_json::json!({
                "criterion_id": criterion.id,
                "tests": tests,
                "verdict": "pass",
                "justification": "mock automatic estimate",
                "confidence": if criterion.id == "1.1" { 0.25 } else { 0.82 },
                "review_required": true,
                "evidence": []
            })
        })
        .collect();
    serde_json::to_string(&items).expect("response JSON")
}

fn test_agent(base_url: String) -> AgentConfig {
    AgentConfig {
        provider: "local-mock".to_owned(),
        base_url,
        api_key: "mock-only".to_owned(),
        model: "mock-model".to_owned(),
        model_tactical: "mock-model".to_owned(),
        model_reasoning: "mock-model".to_owned(),
        tactical_rpm: 0,
        reasoning_rpm: 0,
        agent_concurrency: 4,
        timeout: Duration::from_secs(5),
        ..AgentConfig::default()
    }
}

fn page_context(url: &str) -> rgaa_holo::PageContext {
    rgaa_holo::PageContext {
        title: Some(format!("Mock audit page {url}")),
        lang: Some("fr".to_owned()),
        headings: vec![],
        images: vec![],
        iframes: vec![],
        links: vec![],
        forms: vec![],
        media: vec![],
        navigation: vec![],
    }
}

/// Applies route-owned deterministic tests to the provider's actual estimates.
/// Complete routes are tested by the deterministic fake; partial routes retain
/// the Task 4 estimate and evidence gap.
fn merge_deterministic_routes(
    estimate: CriterionResult,
    finding: Option<(&str, &str)>,
) -> CriterionResult {
    let route_plan = TestRoutePlan::builtin();
    let mut deterministic = estimate.clone();
    deterministic.source = "axe-core".to_owned();
    deterministic.status = if finding.is_some() {
        CriterionStatus::Fail
    } else {
        CriterionStatus::NeedsReview
    };
    deterministic.automated_verdict = None;
    deterministic.raw_confidence = None;
    deterministic.confidence = None;
    deterministic.confidence_calibration_version = None;
    deterministic.verdict_basis = vec![VerdictBasis::Axe];
    deterministic.evidence.clear();
    deterministic.review_required = false;
    deterministic.review_reason = None;
    deterministic.verified_status = None;
    deterministic.tests.clear();
    deterministic.violations.clear();

    let mut complete_keys = Vec::new();
    for route in route_plan
        .routes()
        .iter()
        .filter(|route| route.criterion_id == estimate.criterion_id)
    {
        if route.coverage == CoverageLevel::Complete {
            complete_keys.push(route.test_key.clone());
            let failed = finding.is_some_and(|(key, _)| key == route.test_key);
            deterministic.tests.push(TestOutcome {
                test_key: route.test_key.clone(),
                status: if failed {
                    CriterionStatus::Fail
                } else {
                    CriterionStatus::Pass
                },
                source: "axe-core".to_owned(),
                evidence: Some("deterministic mock observed the test target".to_owned()),
            });
        }
    }
    let mut estimate = estimate;
    estimate
        .tests
        .retain(|outcome| !complete_keys.contains(&outcome.test_key));

    if let Some((test_key, description)) = finding {
        deterministic.violations.push(Violation {
            rule_id: "mock-deterministic-finding".to_owned(),
            impact: "serious".to_owned(),
            description: description.to_owned(),
            nodes_affected: 1,
        });
        debug_assert!(deterministic
            .tests
            .iter()
            .any(|test| { test.test_key == test_key && test.status == CriterionStatus::Fail }));
    }

    if deterministic.tests.is_empty() {
        estimate
    } else {
        merge_candidates(vec![deterministic, estimate]).expect("both candidates are present")
    }
}

fn chosen_finding_route() -> (String, String) {
    let route = TestRoutePlan::builtin()
        .routes()
        .iter()
        .find(|route| route.coverage == CoverageLevel::Complete)
        .expect("catalog has a complete route");
    (route.criterion_id.clone(), route.test_key.clone())
}

fn audit_result(pages: Vec<PageResult>) -> AuditResult {
    let metrics = compute_audit_metrics(&pages);
    AuditResult {
        audit_id: "mock-rgaa-audit".to_owned(),
        url: "https://example.test".to_owned(),
        total_criteria: pages.iter().map(|page| page.criteria.len()).sum(),
        passed: 0,
        failed: 0,
        na: 0,
        overall_compliance: metrics.verified_compliance_percent,
        taux_global: metrics.verified_compliance_percent,
        coverage_percent: 0.0,
        automatic_verdict_coverage_percent: metrics.automatic_verdict_coverage_percent,
        test_evidence_coverage_percent: metrics.test_evidence_coverage_percent,
        verified_compliance_percent: metrics.verified_compliance_percent,
        etat_conformite: "Non Conforme".to_owned(),
        duration_ms: 1,
        audit_complete: !pages.is_empty()
            && pages
                .iter()
                .all(|page| validate_automatic_verdict_coverage(&page.criteria).is_ok()),
        pages,
    }
}

fn assert_cli_report_does_not_claim_conformance(audit: &AuditResult) {
    let directory = tempfile::tempdir().expect("temporary report directory");
    let input = directory.path().join("audit-bundle.json");
    let output = directory.path().join("report.html");
    let mut bundle = AuditBundle::from(audit.clone());
    // The current conversion stores each violation at both bundle and page
    // scope, while validation requires finding IDs to be unique globally.
    // Keep the page-scoped copies that the HTML renderer uses.
    bundle.findings.clear();
    std::fs::write(&input, serde_json::to_vec(&bundle).expect("bundle JSON"))
        .expect("bundle fixture should be written");
    let expected_coverage = format!("{:.1}%", audit.automatic_verdict_coverage_percent);
    let native_html = rgaa_report::render(&bundle, ReportFormat::Html)
        .expect("native report renderer should succeed");
    assert!(native_html.contains(&expected_coverage));
    assert!(native_html.contains("Non Conforme"));
    assert!(!native_html.contains("status-badge pass\">Conforme</span>"));

    let exit = render_cli_report(ReportArgs {
        common: CommonArgs {
            config: None,
            output: Some(output.clone()),
            format: Some("html".to_owned()),
            audit_id: None,
            log_file: None,
        },
        input: Some(input),
        audit_id: None,
    })
    .expect("CLI report renderer should succeed");
    assert_eq!(exit, 0);
    let html = std::fs::read_to_string(output).expect("HTML report should be written");
    assert!(html.contains("Couverture des verdicts automatiques"));
    assert!(html.contains("Conformité vérifiée"));
    assert!(html.contains(&expected_coverage));
    assert!(html.contains("Non Conforme"));
    assert!(!html.contains("status-badge pass\">Conforme</span>"));
    if !audit.audit_complete {
        // AuditBundle (the CLI report input) does not carry AuditResult's
        // audit_complete flag. The report must still expose the sub-100%
        // prediction coverage and must never label the outage as conforming.
        assert!(audit.automatic_verdict_coverage_percent < 100.0);
        assert!(!html.contains("audit_complete\">true"));
    }
}

#[tokio::test]
async fn four_page_mock_audit_keeps_predictions_evidence_and_human_review_separate() {
    let provider = MockProvider::start(provider_response(), 200);
    let agent = RgaaAgent::new(&test_agent(provider.base_url.clone()))
        .await
        .expect("agent should use the local mock endpoint");
    let (finding_criterion, finding_key) = chosen_finding_route();
    let urls = [
        "https://example.test/",
        "https://example.test/contact",
        "https://example.test/services",
        "https://example.test/accessibilite",
    ];
    let mut pages = Vec::new();

    for (page_index, url) in urls.iter().enumerate() {
        let estimates = agent
            .run_automatic_estimates(RgaaCriteria::all(), &page_context(url), &[])
            .await;
        assert_eq!(estimates.len(), 106, "every catalog ID should be returned");
        let mut criteria = Vec::with_capacity(106);
        for criterion in RgaaCriteria::all() {
            let estimate = estimates
                .get(criterion.id)
                .expect("the agent returns every requested criterion")
                .clone();
            assert!(estimate.automated_verdict.is_some(), "{}", criterion.id);
            if criterion.id == "1.1" {
                assert_eq!(estimate.raw_confidence, Some(0.25));
                assert!(estimate.review_required);
            }
            let finding = (page_index == 0 && criterion.id == finding_criterion)
                .then_some((finding_key.as_str(), "mock deterministic failure"));
            criteria.push(merge_deterministic_routes(estimate, finding));
        }

        if page_index == 0 {
            let human_review = criteria
                .iter_mut()
                .find(|criterion| criterion.criterion_id == "1.1")
                .expect("criterion 1.1 exists");
            let original_prediction = human_review.automated_verdict;
            let original_raw_confidence = human_review.raw_confidence;
            let original_calibrated_confidence = human_review.confidence;
            apply_review(
                human_review,
                CriterionStatus::Fail,
                "Auditeur RGAA",
                Utc::now(),
                "l’image observée ne possède pas d’alternative adaptée",
            )
            .expect("human review should be recorded");
            assert_eq!(human_review.verified_status, Some(CriterionStatus::Fail));
            assert_eq!(human_review.automated_verdict, original_prediction);
            assert_eq!(human_review.raw_confidence, original_raw_confidence);
            assert_eq!(human_review.confidence, original_calibrated_confidence);
            assert_eq!(human_review.review_events.len(), 1);
        }

        pages.push(PageResult {
            url: (*url).to_owned(),
            title: Some(format!("Mock page {page_index}")),
            criteria,
            compliance_rate: 0.0,
            crawl_depth: page_index as u32,
        });
    }

    assert!(
        provider.request_count() >= 4 * 20,
        "expected batched local calls"
    );
    let audit = audit_result(pages);
    let row_count: usize = audit.pages.iter().map(|page| page.criteria.len()).sum();
    let verdict_count: usize = audit
        .pages
        .iter()
        .flat_map(|page| &page.criteria)
        .filter(|criterion| criterion.automated_verdict.is_some())
        .count();
    assert_eq!(row_count, 4 * 106);
    assert_eq!(verdict_count, 424);
    assert!(audit.audit_complete);
    assert_eq!(audit.automatic_verdict_coverage_percent, 100.0);
    let complete_route_count = TestRoutePlan::builtin()
        .routes()
        .iter()
        .filter(|route| route.coverage == CoverageLevel::Complete)
        .count();
    let expected_evidence_percent = 100.0 * (4 * complete_route_count) as f64 / (4 * 258) as f64;
    assert_eq!(
        audit.test_evidence_coverage_percent,
        expected_evidence_percent
    );
    assert!(audit.test_evidence_coverage_percent < 100.0);
    assert!(audit.verified_compliance_percent < 100.0);
    assert!(audit.pages[0]
        .criteria
        .iter()
        .any(|criterion| !criterion.violations.is_empty()));
    let manual_estimate = audit.pages[0]
        .criteria
        .iter()
        .find(|result| {
            RgaaCriteria::find(&result.criterion_id)
                .is_some_and(|criterion| criterion.classification == Classification::Manuel)
        })
        .expect("at least one manually classified criterion exists");
    assert!(manual_estimate.automated_verdict.is_some());
    assert!(manual_estimate
        .verdict_basis
        .contains(&VerdictBasis::ModelEstimate));
    let report = rgaa_report::render(&AuditBundle::from(audit.clone()), ReportFormat::Html)
        .expect("native report should render");
    assert!(report.contains("Couverture des verdicts automatiques"));
    assert!(report.contains("100.0%"));
    assert!(report.contains("estimation"));
    assert_cli_report_does_not_claim_conformance(&audit);
}

#[tokio::test]
async fn one_required_provider_failure_remains_incomplete_in_metrics_and_cli_report() {
    let healthy = MockProvider::start(provider_response(), 200);
    let agent = RgaaAgent::new(&test_agent(healthy.base_url.clone()))
        .await
        .expect("agent should use the local healthy endpoint");
    let mut estimates = agent
        .run_automatic_estimates(
            RgaaCriteria::all(),
            &page_context("https://example.test/outage"),
            &[],
        )
        .await;
    assert!(estimates
        .values()
        .all(|result| result.automated_verdict.is_some()));

    let missing = RgaaCriteria::find("1.1")
        .expect("catalog criterion")
        .clone();
    let outage = MockProvider::start("provider unavailable".to_owned(), 503);
    let outage_agent = RgaaAgent::new(&test_agent(outage.base_url.clone()))
        .await
        .expect("agent should initialize against outage mock");
    let failed = outage_agent
        .run_automatic_estimates(
            std::slice::from_ref(&missing),
            &page_context("outage page"),
            &[],
        )
        .await;
    let unresolved = failed
        .get(missing.id)
        .expect("requested ID remains visible");
    assert_eq!(unresolved.automated_verdict, None);
    assert_eq!(unresolved.verified_status, None);
    assert!(unresolved.review_required);
    assert!(outage.request_count() >= 1);
    estimates.insert(missing.id.to_owned(), unresolved.clone());

    let criteria = RgaaCriteria::all()
        .iter()
        .map(|criterion| estimates.get(criterion.id).unwrap().clone())
        .collect();
    let incomplete = audit_result(vec![PageResult {
        url: "https://example.test/outage".to_owned(),
        title: Some("Provider outage fixture".to_owned()),
        criteria,
        compliance_rate: 0.0,
        crawl_depth: 0,
    }]);
    assert!(!incomplete.audit_complete);
    assert!(incomplete.automatic_verdict_coverage_percent < 100.0);
    assert!(validate_automatic_verdict_coverage(&incomplete.pages[0].criteria).is_err());
    assert_cli_report_does_not_claim_conformance(&incomplete);
}
