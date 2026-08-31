use crate::error::SpiderError;
use crate::types::{CrawlSiteArgs, CrawlSiteOutput, CrawlStats, PageSummary};
use rig_core::tool::PortableTool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use spider::website::Website;

const HTML_TRUNCATE_LEN: usize = 50_000;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpiderTool;

impl SpiderTool {
    pub const fn new() -> Self {
        Self
    }
}

impl Default for SpiderTool {
    fn default() -> Self {
        Self::new()
    }
}

impl PortableTool for SpiderTool {
    const NAME: &'static str = "crawl_site";
    type Error = SpiderError;
    type Args = CrawlSiteArgs;
    type Output = CrawlSiteOutput;

    fn description(&self) -> String {
        "Crawl a website and discover pages for RGAA accessibility auditing".to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::to_value(schemars::schema_for!(CrawlSiteArgs)).expect("valid schema")
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let max_pages = args.max_pages.unwrap_or(20);
        let max_depth = args.max_depth.unwrap_or(3);
        let respect_robots = args.respect_robots_txt.unwrap_or(true);

        let start = std::time::Instant::now();
        let mut website = Website::new(&args.url);
        website = website.max_depth(max_depth);
        website = website.respect_robots_txt(respect_robots);

        let page_count = std::sync::atomic::AtomicUsize::new(0);
        let blocked_count = std::sync::atomic::AtomicUsize::new(0);

        let page_tx = async_channel::bounded(256);
        let page_rx = page_tx.1.clone();

        let website_clone = website.clone();
        let page_tx_clone = page_tx.clone();
        let page_count_clone = page_count.clone();
        let blocked_count_clone = blocked_count.clone();

        website = website.spawn(page_tx.0);

        website.crawl().await;

        let mut pages = Vec::new();

        while let Ok(page) = page_rx.recv().await {
            if page_count_clone.load(std::sync::atomic::Ordering::Relaxed) >= max_pages as usize {
                break;
            }

            let status_code = page.status().code().as_u16();
            let url = page.url().to_string();

            let html = if page.html().len() > HTML_TRUNCATE_LEN {
                page.html()[..HTML_TRUNCATE_LEN].to_string()
            } else {
                page.html().to_string()
            };

            let links: Vec<String> = page
                .links()
                .iter()
                .filter_map(|l| l.as_ref().map(|s| s.to_string()))
                .collect();

            let page_summary = PageSummary {
                url,
                html,
                links,
                status_code,
            };

            pages.push(page_summary);
            page_count_clone.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }

        let duration_ms = start.elapsed().as_millis() as u64;

        let crawl_stats = CrawlStats {
            pages_crawled: page_count.load(std::sync::atomic::Ordering::Relaxed),
            duration_ms,
            blocked_by_robots: blocked_count.load(std::sync::atomic::Ordering::Relaxed),
        };

        let total_discovered = pages.len();

        Ok(CrawlSiteOutput {
            pages,
            total_discovered,
            crawl_stats,
        })
    }
}
