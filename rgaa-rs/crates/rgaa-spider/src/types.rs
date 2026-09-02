use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct CrawlSiteArgs {
    pub url: String,
    #[serde(default)]
    pub max_pages: Option<u32>,
    #[serde(default)]
    pub max_depth: Option<u32>,
    #[serde(default)]
    pub respect_robots_txt: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PageSummary {
    pub url: String,
    pub html: String,
    pub links: Vec<String>,
    pub status_code: u16,
    pub truncated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CrawlStats {
    pub pages_crawled: usize,
    pub duration_ms: u64,
    pub blocked_by_robots: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CrawlSiteOutput {
    pub pages: Vec<PageSummary>,
    pub total_discovered: usize,
    pub crawl_stats: CrawlStats,
}
