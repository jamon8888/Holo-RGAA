use rgaa_obscura::ObscuraBridge;
use rgaa_obscura::{AnalyzeConfig, AnalyzeRequest, PreScanAction, ScreenshotPolicy, Viewport};
use rgaa_obscura::{GuidedStep, GuidedTest, TerminationReason};

#[tokio::test]
async fn test_obscura_bridge_sync() {
    let bridge = ObscuraBridge::new();
    let result = bridge.extract_page_context("https://example.com").await;
    println!("Page context result: {:?}", result);
    assert!(
        result.is_ok(),
        "Failed to extract page context: {:?}",
        result.err()
    );

    let context = result.unwrap();
    println!("Context: {:?}", context);
    assert!(
        context.get("title").is_some(),
        "Missing title in page context"
    );
}

#[tokio::test]
async fn test_obscura_bridge_axe_via_cdp() {
    let mut bridge = ObscuraBridge::new().with_port(9223);

    // Start CDP server
    let server_result = bridge.start_server().await;
    println!("Server start result: {:?}", server_result);
    assert!(
        server_result.is_ok(),
        "Failed to start server: {:?}",
        server_result.err()
    );

    // Run axe-core
    let result = bridge.run_axe("https://example.com").await;
    println!("Axe result: {:?}", result);

    // Stop server
    bridge.stop_server().await;

    assert!(result.is_ok(), "Failed to run axe: {:?}", result.err());

    // The returned string must be valid JSON and a JSON array (violations).
    let ax = result.unwrap();
    let parsed: serde_json::Value =
        serde_json::from_str(&ax).expect("axe result must be parseable JSON");
    assert!(parsed.is_array(), "axe result must be a JSON array");
}

#[tokio::test]
async fn test_obscura_bridge_axe_batch_multiple_urls() {
    let mut bridge = ObscuraBridge::new().with_port(9224);

    let server_result = bridge.start_server().await;
    println!("Server start result: {:?}", server_result);
    assert!(
        server_result.is_ok(),
        "Failed to start server: {:?}",
        server_result.err()
    );

    let urls = vec![
        "https://example.com".to_string(),
        "https://example.org".to_string(),
    ];
    let results = bridge.run_axe_batch(&urls, 2).await;
    println!("Batch axe results: {:?}", results);

    bridge.stop_server().await;

    assert!(
        results.is_ok(),
        "Failed to run axe batch: {:?}",
        results.err()
    );
    let results = results.unwrap();
    assert_eq!(results.len(), 2, "axe batch must return one entry per URL");
    for (url, ax) in &results {
        let parsed: serde_json::Value = serde_json::from_str(ax)
            .unwrap_or_else(|e| panic!("axe batch result for {url} must be JSON: {e}"));
        assert!(
            parsed.is_array(),
            "axe batch result for {url} must be a JSON array"
        );
    }
}

#[tokio::test]
async fn test_obscura_bridge_extract_page_context_batch() {
    let mut bridge = ObscuraBridge::new().with_port(9225);

    let server_result = bridge.start_server().await;
    assert!(
        server_result.is_ok(),
        "Failed to start server: {:?}",
        server_result.err()
    );

    let urls = vec![
        "https://example.com".to_string(),
        "https://example.org".to_string(),
    ];
    let results = bridge.extract_page_context_batch(&urls, 2).await;
    println!("Batch page context results: {:?}", results);

    bridge.stop_server().await;

    assert!(
        results.is_ok(),
        "Failed to extract page context batch: {:?}",
        results.err()
    );
    let results = results.unwrap();
    assert_eq!(
        results.len(),
        2,
        "page context batch must return one entry per URL"
    );
    for (url, ctx) in &results {
        assert!(
            ctx.get("title").is_some(),
            "missing title in page context for {url}"
        );
    }
}

/// Performance/timing regression guard for the concurrent batch path.
///
/// `run_axe_batch` must process every URL (not just the first) and must finish
/// within a generous bound so a regression to sequential execution is caught.
#[tokio::test]
async fn test_obscura_bridge_axe_batch_performance() {
    // run_axe_batch drives per-URL CDP sessions, so the CDP server must be up.
    let mut bridge = ObscuraBridge::new().with_port(9226);
    assert!(
        bridge.start_server().await.is_ok(),
        "failed to start CDP server"
    );

    let urls: Vec<String> = vec![
        "https://example.com".to_string(),
        "https://example.com/page-1".to_string(),
        "https://example.com/page-2".to_string(),
        "https://example.org".to_string(),
    ];

    let start = std::time::Instant::now();
    let results = bridge.run_axe_batch(&urls, 4).await;
    let elapsed = start.elapsed();
    println!("axe batch of {} urls took {:?}", urls.len(), elapsed);

    bridge.stop_server().await;

    assert!(results.is_ok(), "axe batch failed: {:?}", results.err());
    let results = results.unwrap();
    assert_eq!(
        results.len(),
        urls.len(),
        "axe batch must return one entry per URL"
    );
    for (url, ax) in &results {
        let parsed: serde_json::Value = serde_json::from_str(ax)
            .unwrap_or_else(|e| panic!("axe batch result for {url} must be JSON: {e}"));
        assert!(
            parsed.is_array(),
            "axe batch result for {url} must be a JSON array"
        );
    }

    // Guard against a regression to sequential/blocking execution.
    assert!(
        elapsed.as_secs() < 60,
        "axe batch unexpectedly slow: {elapsed:?}"
    );
}

#[tokio::test]
async fn test_structured_analyze_applies_configuration_and_captures_evidence() {
    let mut bridge = ObscuraBridge::new().with_port(9227);
    assert!(
        bridge.start_server().await.is_ok(),
        "failed to start CDP server"
    );

    let config = AnalyzeConfig {
        viewport: Viewport {
            width: 375,
            height: 812,
        },
        selector: Some("body".into()),
        pre_scan_actions: vec![PreScanAction::Click {
            selector: "body".into(),
        }],
        screenshot_policy: ScreenshotPolicy::Always,
        timeout_ms: 30_000,
        retry_limit: 1,
        ..Default::default()
    };
    let request = AnalyzeRequest {
        url: "https://example.com".into(),
        config,
    };

    let result = bridge.analyze(&request).await;
    bridge.stop_server().await;

    let result = result.expect("structured analysis request should be accepted");
    assert!(
        result.completed,
        "configured analysis must complete with evidence: {result:?}"
    );
    assert!(
        result.errors.is_empty(),
        "configured analysis returned errors: {:?}",
        result.errors
    );
    assert!(result
        .evidence
        .iter()
        .any(|evidence| evidence.kind == "dom_snapshot"));
    assert!(result
        .evidence
        .iter()
        .any(|evidence| evidence.kind == "screenshot"));
}

#[tokio::test]
async fn test_guided_test_captures_trace_tree_screenshot_and_mapping() {
    let bridge = ObscuraBridge::new();
    let test = GuidedTest {
        id: "worker-keyboard-flow".into(),
        version: 1,
        preconditions: vec!["page is reachable".into()],
        steps: vec![
            GuidedStep::Navigate {
                url: "https://example.com".into(),
            },
            GuidedStep::AccessibilityTree,
            GuidedStep::Screenshot,
        ],
        criterion_mapping: vec!["12.9".into()],
        evidence_requirements: vec!["tree".into(), "screenshot".into()],
    };

    let result = bridge
        .run_guided_test(&test)
        .await
        .expect("guided run returns an envelope");

    assert_eq!(result.terminated_reason, TerminationReason::Completed);
    assert_eq!(result.action_trace.len(), 3);
    assert_eq!(result.criterion_mapping, vec!["12.9"]);
    assert!(result
        .evidence
        .iter()
        .any(|evidence| evidence.kind == "tree"));
    assert!(result
        .evidence
        .iter()
        .any(|evidence| evidence.kind == "screenshot"));
}
