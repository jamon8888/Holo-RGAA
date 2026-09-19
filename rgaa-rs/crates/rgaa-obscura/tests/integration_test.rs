use rgaa_obscura::ObscuraBridge;
use rgaa_obscura::{
    AnalyzeConfig, AnalyzeRequest, PreScanAction, ScreenshotConfig, ScreenshotFormat,
    ScreenshotPolicy, Viewport,
};
use rgaa_obscura::{GuidedStep, GuidedTest, TerminationReason};

#[tokio::test]
#[ignore = "requires the obscura substrate binary + network access"]
async fn test_obscura_bridge_sync() {
    // Single-audit callers route through the batch entry point (N=1) — this is
    // the live production path, not a one-off; see pipeline::audit_one.
    let bridge = ObscuraBridge::new();
    let urls = vec!["https://example.com".to_string()];
    let result = bridge.extract_page_context_batch(&urls, 1).await;
    println!("Page context result: {:?}", result);
    assert!(
        result.is_ok(),
        "Failed to extract page context: {:?}",
        result.err()
    );

    let mut context_by_url = result.unwrap();
    let context = context_by_url
        .remove("https://example.com")
        .expect("batch result must contain the requested URL");
    println!("Context: {:?}", context);
    assert!(
        context.get("title").is_some(),
        "Missing title in page context"
    );
}

#[tokio::test]
#[ignore = "requires the obscura substrate binary + network access"]
async fn test_obscura_bridge_axe_via_cdp() {
    // Single-audit callers route through the batch entry point (N=1) — this is
    // the live production path, not a one-off; see pipeline::audit_one.
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
    let urls = vec!["https://example.com".to_string()];
    let result = bridge.run_axe_batch(&urls, 1).await;
    println!("Axe result: {:?}", result);

    // Stop server
    bridge.stop_server().await;

    assert!(result.is_ok(), "Failed to run axe: {:?}", result.err());

    // The returned string must be valid JSON and a JSON array (violations).
    let mut results_by_url = result.unwrap();
    let ax = results_by_url
        .remove("https://example.com")
        .expect("batch result must contain the requested URL");
    let parsed: serde_json::Value =
        serde_json::from_str(&ax).expect("axe result must be parseable JSON");
    assert!(parsed.is_array(), "axe result must be a JSON array");
}

#[tokio::test]
#[ignore = "requires the obscura substrate binary + network access"]
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
#[ignore = "requires the obscura substrate binary + network access"]
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
#[ignore = "requires the obscura substrate binary + network access"]
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
#[ignore = "requires the obscura substrate binary + network access"]
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
        screenshot: ScreenshotConfig {
            policy: ScreenshotPolicy::Always,
            format: ScreenshotFormat::Png,
            inline: Some(true),
            save_to: None,
        },
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
#[ignore = "requires the obscura substrate binary + network access"]
async fn test_guided_test_captures_trace_tree_screenshot_and_mapping() {
    let mut bridge = ObscuraBridge::new().with_port(9228);
    bridge
        .start_server()
        .await
        .expect("failed to start guided-test CDP server");
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
    bridge.stop_server().await;

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
    let screenshot = result
        .evidence
        .iter()
        .find(|evidence| evidence.kind == "screenshot")
        .expect("screenshot evidence");
    assert!(std::fs::read(&screenshot.path)
        .expect("read screenshot evidence")
        .starts_with(&[137, 80, 78, 71, 13, 10, 26, 10]));
}

#[tokio::test]
#[ignore = "requires the obscura substrate binary + network access"]
async fn test_guided_stateful_ax_ref_fill_and_observed_state() {
    let mut bridge = ObscuraBridge::new().with_port(9229);
    bridge
        .start_server()
        .await
        .expect("failed to start stateful guided-test CDP server");
    let url = "data:text/html,%3C!doctype%20html%3E%3Cform%3E%3Clabel%3EName%3Cinput%20aria-label%3D%22Name%22%20name%3D%22name%22%3E%3C/label%3E%3C/form%3E";
    let test = GuidedTest {
        id: "worker-stateful-fill".into(),
        version: 1,
        preconditions: vec!["form is loaded".into()],
        steps: vec![
            GuidedStep::Navigate { url: url.into() },
            GuidedStep::AccessibilityTree,
            GuidedStep::FillRef {
                reference: "ax-role=textbox;name=Name".into(),
                value: "Ada".into(),
            },
            GuidedStep::AssertState {
                expected: serde_json::json!({
                    "values": [{"id": "", "name": "name", "value": "Ada"}]
                }),
            },
        ],
        criterion_mapping: vec!["11.1".into()],
        evidence_requirements: vec!["tree".into()],
    };

    let result = bridge
        .run_guided_test(&test)
        .await
        .expect("stateful guided run returns an envelope");
    let targets: serde_json::Value = reqwest::get("http://127.0.0.1:9229/json/list")
        .await
        .expect("read CDP targets")
        .json()
        .await
        .expect("parse CDP targets");
    bridge.stop_server().await;

    assert!(result.is_pass(), "state did not persist: {result:?}");
    assert_eq!(result.completed_steps, 4);
    assert_eq!(result.terminated_reason, TerminationReason::Completed);
    assert!(!targets
        .as_array()
        .expect("target list is an array")
        .iter()
        .any(|target| target.get("url").and_then(serde_json::Value::as_str) == Some(url)));
}

// Live: label-aware pre-scan fill on a labeled third-party form (v0.2.2
// substrate resolves the wrapping-label association; the multiline value
// exercises newline/quote-safe insertion).
#[tokio::test]
#[ignore = "requires the obscura substrate binary + network access"]
async fn test_prescan_fill_on_labeled_form_completes_without_label_findings() {
    let mut bridge = ObscuraBridge::new().with_port(9230);
    bridge
        .start_server()
        .await
        .expect("failed to start fill-verification CDP server");

    let config = AnalyzeConfig {
        pre_scan_actions: vec![PreScanAction::Fill {
            selector: "input[name=custname]".into(),
            value: "Ada \"Lovelace\"\nAnalytical Engine".into(),
        }],
        timeout_ms: 30_000,
        retry_limit: 1,
        ..Default::default()
    };
    let request = AnalyzeRequest {
        url: "https://httpbin.org/forms/post".into(),
        config,
    };

    let result = bridge.analyze(&request).await;
    bridge.stop_server().await;

    let result = result.expect("labeled-form analysis should be accepted");
    assert!(
        result.completed,
        "fill must not break completion: {result:?}"
    );
    assert!(
        result.errors.is_empty(),
        "fill returned errors: {:?}",
        result.errors
    );
    assert!(
        result.obscura_version.is_some(),
        "result must carry substrate version"
    );
    assert!(
        !result
            .findings
            .iter()
            .any(|finding| finding.rule == "label"),
        "wrapping-label input must not raise label findings: {:?}",
        result.findings
    );
}

// Live: a pre-scan submit click must settle the navigation before axe runs,
// so the audit observes the post-submit page instead of racing it.
#[tokio::test]
#[ignore = "requires the obscura substrate binary + network access"]
async fn test_prescan_submit_click_audits_post_navigation_page() {
    let mut bridge = ObscuraBridge::new().with_port(9231);
    bridge
        .start_server()
        .await
        .expect("failed to start submit-settle CDP server");

    let config = AnalyzeConfig {
        pre_scan_actions: vec![
            PreScanAction::Fill {
                selector: "input[name=custname]".into(),
                value: "Ada".into(),
            },
            PreScanAction::Click {
                selector: "input[type=submit], button[type=submit], button:not([type])".into(),
            },
        ],
        timeout_ms: 30_000,
        retry_limit: 1,
        ..Default::default()
    };
    let request = AnalyzeRequest {
        url: "https://httpbin.org/forms/post".into(),
        config,
    };

    let result = bridge.analyze(&request).await;
    bridge.stop_server().await;

    let result = result.expect("submit flow analysis should be accepted");
    assert!(
        result.completed,
        "submit must settle before axe runs: {result:?}"
    );
    assert!(
        result.errors.is_empty(),
        "submit flow returned errors: {:?}",
        result.errors
    );
    assert!(
        result
            .evidence
            .iter()
            .any(|evidence| evidence.kind == "dom_snapshot"),
        "post-navigation page must leave evidence"
    );
}
