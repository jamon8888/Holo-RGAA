use rgaa_core::CrawlConfig;
use rgaa_orchestrator::Orchestrator;

#[tokio::test]
async fn test_full_audit_example_com() {
    // Full-pipeline E2E: requires a Holo3 API key and network access to the
    // target page. Skipped unless RUN_E2E=1 is set (e.g. in the CI `e2e` job).
    if std::env::var("RUN_E2E").ok().as_deref() != Some("1") {
        eprintln!("skipping full_audit E2E (set RUN_E2E=1 to enable)");
        return;
    }

    let config = CrawlConfig {
        max_pages: 1,
        max_depth: 0,
        respect_robots: false,
        sample_mode: false,
    };

    let result = Orchestrator::run("https://example.com", &config).await;

    eprintln!("Audit result: {:?}", result);

    assert!(
        result.is_ok(),
        "Audit should complete without error: {:?}",
        result.err()
    );

    let audit = result.unwrap();

    assert_eq!(audit.url, "https://example.com", "URL should match input");

    assert!(
        audit.total_criteria > 0,
        "Should evaluate at least one criterion"
    );

    assert!(
        audit.overall_compliance >= 0.0 && audit.overall_compliance <= 100.0,
        "Compliance rate should be between 0.0 and 100.0, got: {}",
        audit.overall_compliance
    );

    assert!(
        !audit.pages.is_empty(),
        "Should have at least one page result"
    );
}
