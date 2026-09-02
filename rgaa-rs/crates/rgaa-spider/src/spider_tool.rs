use crate::error::SpiderError;
use crate::types::{CrawlSiteArgs, CrawlSiteOutput, CrawlStats, PageSummary};
use rig_core::tool::PortableTool;
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
        "Crawl a website and discover pages for RGAA accessibility auditing. \
         Returns raw HTML and links for each discovered page so the LLM can \
         determine which pages are RGAA-relevant (mandatory pages, forms, \
         navigation, etc.)."
            .to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::to_value(schemars::schema_for!(CrawlSiteArgs)).expect("valid schema")
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let start = std::time::Instant::now();
        let max_pages = args.max_pages.unwrap_or(20);
        let max_depth = args.max_depth.unwrap_or(3);
        let respect_robots = args.respect_robots_txt.unwrap_or(true);

        let mut website = Website::new(&args.url);
        website
            .with_depth(max_depth as usize)
            .with_limit(max_pages)
            .with_respect_robots_txt(respect_robots)
            .with_user_agent(Some("RGAA-Audit-Bot/1.0"));

        website.crawl().await;

        let pages_raw = website.get_pages();
        let mut pages = Vec::new();
        let mut total_discovered = 0;
        let blocked_count = website.get_extra_links().len();

        if let Some(raw_pages) = pages_raw {
            for page in raw_pages.iter().take(max_pages as usize) {
                total_discovered += 1;
                let url = page.get_url().to_string();
                let raw_html = page.get_html();
                let truncated = raw_html.len() > HTML_TRUNCATE_LEN;
                let html = if truncated {
                    raw_html[..HTML_TRUNCATE_LEN].to_string()
                } else {
                    raw_html
                };

                pages.push(PageSummary {
                    url,
                    html,
                    links: Vec::new(),
                    status_code: page.status_code.as_u16(),
                    truncated,
                });
            }
        }

        let duration_ms = start.elapsed().as_millis() as u64;

        let crawl_stats = CrawlStats {
            pages_crawled: pages.len(),
            duration_ms,
            blocked_by_robots: blocked_count,
        };

        Ok(CrawlSiteOutput {
            pages,
            total_discovered,
            crawl_stats,
        })
    }
}
