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
    /// Max in-flight requests. Unset uses the crawler's own default.
    #[serde(default)]
    pub concurrency_limit: Option<usize>,
    /// Politeness delay between requests, in milliseconds. Unset means no delay.
    #[serde(default)]
    pub request_delay_ms: Option<u64>,
    /// Per-request timeout, in milliseconds. Unset uses the crawler's own default.
    #[serde(default)]
    pub request_timeout_ms: Option<u64>,
    /// Whole-crawl wall-clock budget, in milliseconds. When it elapses, the
    /// crawl stops and whatever pages were discovered so far are returned
    /// with `crawl_stats.timed_out = true` instead of hanging the caller.
    #[serde(default)]
    pub crawl_timeout_ms: Option<u64>,
    /// Per-request retry budget. Unset uses the crawler's own default.
    #[serde(default)]
    pub retry_budget: Option<u8>,
    /// URL substrings/patterns to exclude from the crawl.
    #[serde(default)]
    pub url_blacklist: Option<Vec<String>>,
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
    /// True when `crawl_timeout_ms` elapsed before the crawl finished on its
    /// own — `pages` holds a partial-but-marked result, not a hang.
    #[serde(default)]
    pub timed_out: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CrawlSiteOutput {
    pub pages: Vec<PageSummary>,
    pub total_discovered: usize,
    pub crawl_stats: CrawlStats,
}
