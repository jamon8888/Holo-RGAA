//! `verify_fix` behaviour (ticket #164): the tool re-runs the injected
//! analysis service over the corrected pages and hands the outcome to the
//! shared baseline diff. These tests drive the real handler with stub
//! services, so a regression in the wiring — not just in the pure
//! categorisation — fails them.

use rgaa_core::{AuditBundle, AuditConfig, Citation, CriterionResult, CriterionStatus, Finding};
use rgaa_mcp::{
    AnalyzeService, FileVerificationStatus, GuidedService, McpFailure, NoOpStorageService,
    OrchestrationService, RemediationServiceImpl, ToolServer, VerifyFixRequest, VerifyFixResponse,
};
use rmcp::handler::server::wrapper::Parameters;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

const PAGE: &str = "https://example.test/contact";

struct StubAnalyze {
    findings: Vec<Finding>,
    delay: Duration,
}

impl AnalyzeService for StubAnalyze {
    fn analyze(
        &self,
        request: rgaa_obscura::AnalyzeRequest,
    ) -> Pin<
        Box<
            dyn std::future::Future<Output = Result<rgaa_obscura::AnalyzePageResult, McpFailure>>
                + Send
                + '_,
        >,
    > {
        Box::pin(async move {
            tokio::time::sleep(self.delay).await;
            Ok(rgaa_obscura::AnalyzePageResult {
                url: request.url,
                findings: self.findings.clone(),
                evidence: Vec::new(),
                errors: Vec::new(),
                completed: true,
                duration_ms: 1,
                igt: None,
                obscura_version: None,
            })
        })
    }
}

struct RefusingGuided;
impl GuidedService for RefusingGuided {
    fn run(
        &self,
        _test: rgaa_obscura::GuidedTest,
    ) -> Pin<
        Box<
            dyn std::future::Future<Output = Result<rgaa_obscura::GuidedRunResult, McpFailure>>
                + Send
                + '_,
        >,
    > {
        Box::pin(async { Err(McpFailure::unsupported("guided stub")) })
    }
}

fn server(findings: Vec<Finding>, delay: Duration) -> ToolServer {
    ToolServer::new(
        Arc::new(StubAnalyze { findings, delay }),
        Arc::new(RemediationServiceImpl::default()),
        Arc::new(RefusingGuided),
        Arc::new(OrchestrationService::new()),
        Arc::new(NoOpStorageService),
    )
}

fn finding(id: &str, target: &str, criterion: &str) -> Finding {
    let mut f = Finding::new(id);
    f.rule = "image-alt".into();
    f.url = PAGE.into();
    f.target = target.into();
    f.criterion_id = Some(criterion.into());
    f.status = CriterionStatus::Fail;
    f
}

/// Reference audit with two open findings on [`PAGE`], and a RAG-backed
/// verdict (one carrying citations) for criterion `1.1`.
fn reference(citations: Vec<Citation>) -> serde_json::Value {
    let mut bundle = AuditBundle::new("audit-ref", PAGE, AuditConfig::default());
    bundle.findings.push(finding("f-alt", "#hero img", "1.1"));
    bundle
        .findings
        .push(finding("f-label", "#search input", "11.1"));
    bundle.pages.push(rgaa_core::PageAudit {
        page_id: "p1".into(),
        url: PAGE.into(),
        title: None,
        criteria: vec![CriterionResult {
            criterion_id: "1.1".into(),
            title: "Images porteuses d'information".into(),
            classification: rgaa_core::Classification::IaAssiste,
            status: CriterionStatus::Fail,
            violations: Vec::new(),
            confidence: Some(0.9),
            justification: Some("alt manquant".into()),
            source: "agent".into(),
            citations,
            considered_sources: Vec::new(),
            tests: Vec::new(),
            automated_verdict: None,
            verdict_basis: Vec::new(),
            evidence: Vec::new(),
            confidence_calibration_version: None,
            review_required: false,
            review_reason: None,
            verified_status: None,
            review_events: Vec::new(),
        }],
        findings: Vec::new(),
        errors: Vec::new(),
        completed: true,
        duration_ms: 1,
    });
    serde_json::to_value(bundle).expect("bundle serialises")
}

fn request(reference_audit: serde_json::Value, timeout_ms: Option<u64>) -> VerifyFixRequest {
    VerifyFixRequest {
        reference_audit,
        files: vec![rgaa_mcp::CorrectedFileInput {
            path: "src/Hero.tsx".into(),
            url: PAGE.into(),
        }],
        per_file_timeout_ms: timeout_ms,
    }
}

async fn call(server: &ToolServer, request: VerifyFixRequest) -> VerifyFixResponse {
    server
        .verify_fix(Parameters(request))
        .await
        .expect("verify_fix succeeds")
        .0
}

fn ids(findings: &[rgaa_mcp::VerifiedFindingDto]) -> Vec<&str> {
    findings.iter().map(|f| f.finding.id.as_str()).collect()
}

#[tokio::test]
async fn a_finding_gone_after_the_fix_is_fixed_while_a_surviving_one_remains() {
    // The rescan still reports the label problem, so only the alt problem
    // was actually fixed.
    let server = server(
        vec![finding("f-label", "#search input", "11.1")],
        Duration::ZERO,
    );
    let response = call(&server, request(reference(Vec::new()), None)).await;

    assert_eq!(ids(&response.fixed), vec!["f-alt"]);
    assert_eq!(ids(&response.remaining), vec!["f-label"]);
    assert!(response.new_findings.is_empty());
    assert!(response.unverified.is_empty());
    assert_eq!(response.files.len(), 1);
    assert_eq!(response.files[0].status, FileVerificationStatus::Verified);
}

#[tokio::test]
async fn a_finding_absent_from_the_reference_is_reported_as_new() {
    let server = server(
        vec![finding("f-contrast", "#footer a", "3.2")],
        Duration::ZERO,
    );
    let response = call(&server, request(reference(Vec::new()), None)).await;

    assert_eq!(ids(&response.new_findings), vec!["f-contrast"]);
    // Both reference findings disappeared from the rescan.
    assert_eq!(response.fixed.len(), 2);
}

#[tokio::test]
async fn citations_behind_a_rag_verdict_reach_the_response() {
    let citation = Citation::referentiel("1.1.1", "RGAA-4.1.2");
    let server = server(vec![finding("f-alt", "#hero img", "1.1")], Duration::ZERO);
    let response = call(&server, request(reference(vec![citation]), None)).await;

    let alt = response
        .remaining
        .iter()
        .find(|f| f.finding.id == "f-alt")
        .expect("the alt finding is still open");
    assert_eq!(
        alt.citations,
        vec![rgaa_mcp::CitationDto::Referentiel {
            test_id: "1.1.1".into(),
            referentiel_version: "RGAA-4.1.2".into(),
        }],
        "a RAG-backed verdict must not lose its evidence on the way out"
    );
    // A criterion with no retrieval behind it stays uncited rather than
    // borrowing another criterion's sources.
    let label = response
        .fixed
        .iter()
        .find(|f| f.finding.id == "f-label")
        .expect("the label finding was fixed");
    assert!(label.citations.is_empty());
}

#[tokio::test(start_paused = true)]
async fn a_page_that_outruns_its_budget_is_unverified_rather_than_fixed() {
    // The analyzer never answers within the budget. Reporting its reference
    // findings as `fixed` would tell the caller an unchecked page was
    // remediated, so they must land in `unverified` instead.
    let server = server(Vec::new(), Duration::from_secs(120));
    let response = call(&server, request(reference(Vec::new()), Some(50))).await;

    assert_eq!(response.files[0].status, FileVerificationStatus::TimedOut);
    assert!(response.fixed.is_empty(), "{:?}", ids(&response.fixed));
    assert!(response.remaining.is_empty());
    assert!(response.new_findings.is_empty());
    assert_eq!(ids(&response.unverified), vec!["f-alt", "f-label"]);
}

#[tokio::test]
async fn the_per_file_budget_cannot_be_raised_above_thirty_seconds() {
    // Clamped, not honoured: the ceiling protects the server, so a caller
    // asking for ten minutes still gets cut off at thirty seconds.
    assert_eq!(rgaa_mcp::MAX_PER_FILE_TIMEOUT_MS, 30_000);
    let server = server(Vec::new(), Duration::ZERO);
    let response = call(&server, request(reference(Vec::new()), Some(600_000))).await;
    assert_eq!(response.files[0].status, FileVerificationStatus::Verified);
}

#[tokio::test]
async fn a_reference_audit_that_is_not_a_bundle_is_rejected_as_invalid_input() {
    let server = server(Vec::new(), Duration::ZERO);
    let error = server
        .verify_fix(Parameters(request(serde_json::json!({"nope": 1}), None)))
        .await
        // `Json<_>` is not Debug, so the Ok side is discarded before
        // `expect_err` can ask to print it.
        .map(|_| ())
        .expect_err("a non-bundle reference must be rejected");
    assert_eq!(error.data.as_ref().unwrap()["code"], "INVALID_INPUT");
}

#[tokio::test]
async fn an_empty_file_list_is_rejected_rather_than_reported_as_all_fixed() {
    let server = server(Vec::new(), Duration::ZERO);
    let error = server
        .verify_fix(Parameters(VerifyFixRequest {
            reference_audit: reference(Vec::new()),
            files: Vec::new(),
            per_file_timeout_ms: None,
        }))
        .await
        // `Json<_>` is not Debug, so the Ok side is discarded before
        // `expect_err` can ask to print it.
        .map(|_| ())
        .expect_err("an empty batch must be rejected");
    assert_eq!(error.data.as_ref().unwrap()["code"], "INVALID_INPUT");
}
