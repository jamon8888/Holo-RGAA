// End-to-end check of the Obscura-backed audit path. Requires the `obscura`
// binary and `obscura-worker` on PATH, and network access to example.com.

use rgaa_core::CrawlConfig;
use rgaa_orchestrator::Orchestrator;

#[tokio::test]
async fn test_run_batch_obscura() {
    // Full-pipeline E2E via the Obscura CDP backend: requires the `obscura`
    // binary on PATH, network access, and a Holo3 API key. Skipped unless
    // RUN_E2E=1 is set (e.g. in the CI `e2e` job).
    if std::env::var("RUN_E2E").as_deref() != Some("1") {
        eprintln!("skipping obscura_audit E2E (set RUN_E2E=1 to enable)");
        return;
    }

    let config = CrawlConfig {
        max_pages: 1,
        max_depth: 0,
        respect_robots: false,
        sample_mode: false,
    };

    let urls = vec!["https://example.com".to_string()];
    let results = Orchestrator::run_batch(&urls, &config).await;

    assert!(
        results.is_ok(),
        "run_batch (obscura) should succeed: {:?}",
        results.err()
    );

    let map = results.unwrap();
    assert_eq!(map.len(), 1, "one audit result per URL");
    let audit = &map["https://example.com"];
    assert_eq!(audit.url, "https://example.com");
    assert!(
        audit.total_criteria > 0,
        "should evaluate at least one criterion"
    );
}
