//! Network-dependent tests for the streaming crawler. Skipped unless
//! RUN_E2E=1 is set (e.g. in the CI `e2e` job), matching the convention in
//! rgaa-orchestrator's full_audit/obscura_audit E2E tests.

use rgaa_spider::{CrawlSiteArgs, SpiderTool};
use rig_core::tool::PortableTool;

fn e2e_enabled() -> bool {
    if std::env::var("RUN_E2E").ok().as_deref() != Some("1") {
        eprintln!("skipping rgaa-spider E2E (set RUN_E2E=1 to enable)");
        return false;
    }
    true
}

#[tokio::test]
async fn crawl_streams_pages_with_unchanged_output_shape() {
    if !e2e_enabled() {
        return;
    }

    let tool = SpiderTool::new();
    let args = CrawlSiteArgs {
        url: "https://example.com".to_string(),
        max_pages: Some(1),
        max_depth: Some(0),
        respect_robots_txt: Some(false),
        concurrency_limit: None,
        request_delay_ms: None,
        request_timeout_ms: None,
        crawl_timeout_ms: Some(30_000),
        retry_budget: None,
        url_blacklist: None,
    };

    let output = tool.call(args).await.expect("crawl should succeed");

    assert!(
        !output.pages.is_empty(),
        "should discover at least one page"
    );
    assert!(
        output.total_discovered >= output.crawl_stats.pages_crawled,
        "total_discovered should count at least as many pages as were kept"
    );
    assert!(
        !output.crawl_stats.timed_out,
        "a generous 30s budget against example.com should not time out"
    );

    let page = &output.pages[0];
    assert!(page.url.contains("example.com"));
    assert!(!page.html.is_empty());
    assert_eq!(page.status_code, 200);
}

#[tokio::test]
async fn crawl_never_hangs_past_its_timeout_budget() {
    if !e2e_enabled() {
        return;
    }

    let tool = SpiderTool::new();
    let args = CrawlSiteArgs {
        url: "https://example.com".to_string(),
        max_pages: Some(5),
        max_depth: Some(2),
        respect_robots_txt: Some(false),
        concurrency_limit: None,
        request_delay_ms: None,
        request_timeout_ms: None,
        // Deliberately tiny — either spider's own crawl_timeout honors this,
        // or the outer belt-and-suspenders guard (budget + 5s) does. Either
        // way the call must return well under the guard's hard ceiling
        // instead of hanging the caller.
        crawl_timeout_ms: Some(1),
        retry_budget: None,
        url_blacklist: None,
    };

    let result = tokio::time::timeout(std::time::Duration::from_secs(15), tool.call(args)).await;

    assert!(
        result.is_ok(),
        "crawl_site must return well within its timeout budget, not hang the caller"
    );
}

#[tokio::test]
async fn crawl_filters_blacklisted_urls() {
    if !e2e_enabled() {
        return;
    }

    let tool = SpiderTool::new();
    let args = CrawlSiteArgs {
        url: "https://example.com".to_string(),
        max_pages: Some(5),
        max_depth: Some(0),
        respect_robots_txt: Some(false),
        concurrency_limit: None,
        request_delay_ms: None,
        request_timeout_ms: None,
        crawl_timeout_ms: Some(30_000),
        retry_budget: None,
        url_blacklist: Some(vec!["example.com".to_string()]),
    };

    let output = tool.call(args).await.expect("crawl should succeed");

    assert!(
        output.pages.is_empty(),
        "blacklisting the only reachable URL should leave no pages"
    );
}
