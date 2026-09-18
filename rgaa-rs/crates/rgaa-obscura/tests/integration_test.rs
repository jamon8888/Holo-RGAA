//! Live browser tests for the channel-based native worker.
//!
//! All tests require a real browser and network access, hence `ignore`:
//! they document intended end-to-end behavior, not CI gates.

use rgaa_obscura::ObscuraBridge;
use rgaa_obscura::{AnalyzeConfig, AnalyzeRequest, Viewport};
use rgaa_obscura::{GuidedStep, GuidedTest, TerminationReason};

async fn live_bridge() -> ObscuraBridge {
    ObscuraBridge::new()
        .await
        .expect("browser backend must start")
}

#[tokio::test]
#[ignore = "requires browser + network"]
async fn test_obscura_bridge_sync() {
    let bridge = live_bridge().await;
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
#[ignore = "requires browser + network"]
async fn test_obscura_bridge_axe() {
    let bridge = live_bridge().await;

    // Run axe-core
    let result = bridge.run_axe("https://example.com").await;
    println!("Axe result: {:?}", result);

    assert!(result.is_ok(), "Failed to run axe: {:?}", result.err());

    // The returned string must be valid JSON and a JSON array (violations).
    let ax = result.unwrap();
    let parsed: serde_json::Value =
        serde_json::from_str(&ax).expect("axe result must be parseable JSON");
    assert!(parsed.is_array(), "axe result must be a JSON array");
}

#[tokio::test]
#[ignore = "requires browser + network"]
async fn test_obscura_bridge_axe_batch_multiple_urls() {
    let bridge = live_bridge().await;

    let urls = vec![
        "https://example.com".to_string(),
        "https://example.org".to_string(),
    ];
    let results = bridge.run_axe_batch(&urls, 2).await;
    println!("Batch axe results: {:?}", results);

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
#[ignore = "requires browser + network"]
async fn test_obscura_bridge_extract_page_context_batch() {
    let bridge = live_bridge().await;

    let urls = vec![
        "https://example.com".to_string(),
        "https://example.org".to_string(),
    ];
    let results = bridge.extract_page_context_batch(&urls, 2).await;
    println!("Batch page context results: {:?}", results);

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

/// Performance/timing regression guard for the batch path.
///
/// `run_axe_batch` must process every URL (not just the first) and must finish
/// within a generous bound so a regression to sequential execution is caught.
#[tokio::test]
#[ignore = "requires browser + network"]
async fn test_obscura_bridge_axe_batch_performance() {
    let bridge = live_bridge().await;

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
#[ignore = "requires browser + network"]
async fn test_structured_analyze_applies_configuration() {
    let bridge = live_bridge().await;

    let config = AnalyzeConfig {
        viewport: Viewport {
            width: 375,
            height: 812,
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

    let result = result.expect("structured analysis request should be accepted");
    assert!(
        result.completed,
        "configured analysis must complete: {result:?}"
    );
    assert!(
        result.errors.is_empty(),
        "configured analysis returned errors: {:?}",
        result.errors
    );
    assert!(
        result.obscura_version.is_some(),
        "result must carry substrate version"
    );
}

#[tokio::test]
#[ignore = "requires browser + network"]
async fn test_guided_test_captures_trace_and_tree() {
    let bridge = live_bridge().await;
    let test = GuidedTest {
        id: "worker-keyboard-flow".into(),
        version: 1,
        preconditions: vec!["page is reachable".into()],
        steps: vec![
            GuidedStep::Navigate {
                url: "https://example.com".into(),
            },
            GuidedStep::AccessibilityTree,
            // Screenshot capture is unsupported in this substrate: the step is
            // skipped gracefully instead of failing on placeholder bytes.
            GuidedStep::Screenshot,
        ],
        criterion_mapping: vec!["12.9".into()],
        evidence_requirements: vec!["tree".into()],
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
}

#[tokio::test]
#[ignore = "requires browser + network"]
async fn test_guided_stateful_fill_and_observed_state() {
    let bridge = live_bridge().await;
    let url = "data:text/html,%3C!doctype%20html%3E%3Cform%3E%3Clabel%3EName%3Cinput%20aria-label%3D%22Name%22%20name%3D%22name%22%3E%3C/label%3E%3C/form%3E";
    let test = GuidedTest {
        id: "worker-stateful-fill".into(),
        version: 1,
        preconditions: vec!["form is loaded".into()],
        steps: vec![
            GuidedStep::Navigate { url: url.into() },
            GuidedStep::AccessibilityTree,
            GuidedStep::FillRef {
                reference: "input[name=name]".into(),
                value: "Ada".into(),
            },
        ],
        criterion_mapping: vec!["11.1".into()],
        evidence_requirements: vec!["tree".into()],
    };

    let result = bridge
        .run_guided_test(&test)
        .await
        .expect("stateful guided run returns an envelope");

    assert!(result.is_pass(), "state did not persist: {result:?}");
    assert_eq!(result.completed_steps, 3);
    assert_eq!(result.terminated_reason, TerminationReason::Completed);

    // The accessibility tree does not observe the input value: assert the
    // filled value directly on the live bridge after the guided run.
    // `run_guided_test` clones the browser handle, so the bridge stays usable.
    let value = bridge
        .eval_js("document.querySelector('input[name=name]').value")
        .await
        .expect("input value is readable");
    assert_eq!(value, serde_json::Value::String("Ada".into()));
}

// Live: analysis of a labeled third-party form completes and carries the
// substrate version (pre-scan actions are not executed by the native worker).
#[tokio::test]
#[ignore = "requires browser + network"]
async fn test_form_analysis_completes_without_label_findings() {
    let bridge = live_bridge().await;

    let config = AnalyzeConfig {
        timeout_ms: 30_000,
        retry_limit: 1,
        ..Default::default()
    };
    let request = AnalyzeRequest {
        url: "https://httpbin.org/forms/post".into(),
        config,
    };

    let result = bridge.analyze(&request).await;

    let result = result.expect("labeled-form analysis should be accepted");
    assert!(result.completed, "analysis must complete: {result:?}");
    assert!(
        result.errors.is_empty(),
        "analysis returned errors: {:?}",
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
