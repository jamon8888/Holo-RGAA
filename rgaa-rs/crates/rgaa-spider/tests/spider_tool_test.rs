//! End-to-end tests for the streaming crawler, against a local HTTP server (no
//! outbound network, so a flaky runner cannot fail them). Skipped unless
//! RUN_E2E=1 is set (CI job `E2E (spider, serveur local)`), matching the
//! convention in rgaa-orchestrator's full_audit/obscura_audit E2E tests.

use rgaa_spider::{CrawlSiteArgs, SpiderTool};
use rig_core::tool::PortableTool;

fn e2e_enabled() -> bool {
    if std::env::var("RUN_E2E").ok().as_deref() != Some("1") {
        // Même garde que `rgaa_test_corpus::e2e::enabled` : en CI, perdre RUN_E2E
        // doit faire échouer le job, pas le laisser passer à vide.
        assert!(
            std::env::var("RGAA_E2E_REQUIRED").ok().as_deref() != Some("1"),
            "RGAA_E2E_REQUIRED=1 mais RUN_E2E n'est pas 1 : ces tests se sauteraient en silence"
        );
        eprintln!("skipping rgaa-spider E2E (set RUN_E2E=1 to enable)");
        return false;
    }
    true
}

/// Serves one fixed HTML page on 127.0.0.1 until the test process exits, and
/// returns its base URL. A crawl of it must not depend on any external site.
async fn local_site() -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind a local port");
    let addr = listener.local_addr().expect("local address");
    tokio::spawn(async move {
        loop {
            let Ok((mut stream, _)) = listener.accept().await else {
                return;
            };
            tokio::spawn(async move {
                use tokio::io::{AsyncReadExt, AsyncWriteExt};
                let mut buf = [0u8; 2048];
                let _ = stream.read(&mut buf).await;
                let body = "<!doctype html><html lang=\"fr\"><head><title>Local</title></head>\
                            <body><h1>Local</h1></body></html>";
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\n\
                     Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = stream.write_all(response.as_bytes()).await;
            });
        }
    });
    format!("http://{addr}")
}

#[tokio::test]
async fn crawl_streams_pages_with_unchanged_output_shape() {
    if !e2e_enabled() {
        return;
    }

    let tool = SpiderTool::new();
    let args = CrawlSiteArgs {
        url: local_site().await,
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
        "a generous 30s budget against a local page should not time out"
    );

    let page = &output.pages[0];
    assert!(page.url.contains("127.0.0.1"));
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
        url: local_site().await,
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
        url: local_site().await,
        max_pages: Some(5),
        max_depth: Some(0),
        respect_robots_txt: Some(false),
        concurrency_limit: None,
        request_delay_ms: None,
        request_timeout_ms: None,
        crawl_timeout_ms: Some(30_000),
        retry_budget: None,
        url_blacklist: Some(vec!["127.0.0.1".to_string()]),
    };

    let output = tool.call(args).await.expect("crawl should succeed");

    assert!(
        output.pages.is_empty(),
        "blacklisting the only reachable URL should leave no pages"
    );
}
