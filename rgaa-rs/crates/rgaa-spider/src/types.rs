use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct CrawlSiteArgs {
    pub url: String,
    #[serde(default = "default_max_pages")]
    pub max_pages: Option<u32>,
    #[serde(default = "default_max_depth")]
    pub max_depth: Option<u32>,
    #[serde(default = "default_respect_robots")]
    pub respect_robots_txt: Option<bool>,
}

fn default_max_pages() -> Option<u32> {
    Some(20)
}

fn default_max_depth() -> Option<u32> {
    Some(3)
}

fn default_respect_robots() -> Option<bool> {
    Some(true)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageSummary {
    pub url: String,
    pub html: String,
    pub links: Vec<String>,
    pub status_code: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrawlStats {
    pub pages_crawled: usize,
    pub duration_ms: u64,
    pub blocked_by_robots: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrawlSiteOutput {
    pub pages: Vec<PageSummary>,
    pub total_discovered: usize,
    pub crawl_stats: CrawlStats,
}
