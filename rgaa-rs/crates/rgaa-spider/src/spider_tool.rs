use crate::error::SpiderError;
use crate::types::{CrawlSiteArgs, CrawlSiteOutput, CrawlStats, PageSummary};
use rig_core::tool::PortableTool;
use serde::{Deserialize, Serialize};
use spider::website::Website;
use std::time::Duration;
use tokio::sync::broadcast::error::RecvError;

const HTML_TRUNCATE_LEN: usize = 50_000;
/// Broadcast channel capacity for `Website::subscribe` — a small buffer is
/// enough since we drain it continuously while the crawl runs concurrently.
const SUBSCRIBE_CAPACITY: usize = 16;

/// True when `url` matches any blacklist pattern (plain substring match).
fn is_blacklisted(url: &str, patterns: &[String]) -> bool {
    patterns.iter().any(|pat| url.contains(pat.as_str()))
}

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
        let crawl_timeout = args.crawl_timeout_ms.map(Duration::from_millis);

        let mut website = Website::new(&args.url);
        website
            .with_depth(max_depth as usize)
            .with_limit(max_pages)
            .with_respect_robots_txt(respect_robots)
            .with_user_agent(Some("RGAA-Audit-Bot/1.0"));

        if let Some(limit) = args.concurrency_limit {
            website.with_concurrency_limit(Some(limit));
        }
        if let Some(delay) = args.request_delay_ms {
            website.with_delay(delay);
        }
        if let Some(timeout_ms) = args.request_timeout_ms {
            website.with_request_timeout(Some(Duration::from_millis(timeout_ms)));
        }
        if let Some(timeout) = crawl_timeout {
            website.with_crawl_timeout(Some(timeout));
        }
        if let Some(retry) = args.retry_budget {
            website.with_retry(retry);
        }
        // Filtered on receipt below rather than via `Website::with_blacklist_url`,
        // which takes the crawler's own `CompactString` type — a version-pinned
        // transitive dep not worth taking on directly for a substring filter.
        let url_blacklist = args.url_blacklist.clone().unwrap_or_default();

        // Stream page summaries as they're crawled instead of buffering the
        // whole crawl: subscribe before starting, then drain the broadcast
        // channel concurrently with the crawl task. `total_discovered` counts
        // every page the crawler saw; `pages` is capped at `max_pages`.
        let mut rx = website.subscribe(SUBSCRIBE_CAPACITY);
        let crawl_handle = tokio::spawn(async move {
            website.crawl().await;
            let blocked_by_robots = website.get_extra_links().len();
            // Drop `website` (and its broadcast sender) here, at the end of
            // the task's own execution, rather than returning it: the sender
            // must go away as soon as the crawl finishes so `drain`'s
            // `rx.recv()` observes the channel close promptly. Returning
            // `website` instead would keep it (and the sender) alive inside
            // the still-unpolled JoinHandle result, deadlocking `drain`.
            drop(website);
            blocked_by_robots
        });
        let crawl_abort = crawl_handle.abort_handle();

        let mut pages = Vec::with_capacity(max_pages as usize);
        let mut total_discovered: usize = 0;
        let drain = async {
            loop {
                match rx.recv().await {
                    Ok(page) => {
                        let url = page.get_url().to_string();
                        if is_blacklisted(&url, &url_blacklist) {
                            continue;
                        }
                        total_discovered += 1;
                        if pages.len() < max_pages as usize {
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
                    // Channel closes once the crawl finishes and drops its sender.
                    Err(RecvError::Closed) => break,
                    // We fell behind the broadcast buffer — keep draining rather
                    // than treating a slow consumer as a crawl failure.
                    Err(RecvError::Lagged(_)) => continue,
                }
            }
        };

        // Belt-and-suspenders: `with_crawl_timeout` makes the crawl itself
        // stop at the budget, but an outer timeout guarantees this call never
        // hangs the caller even if that internal timeout is bypassed (e.g. a
        // single pathological connect that ignores it).
        let outer_budget = crawl_timeout.map(|t| t + Duration::from_secs(5));
        let (timed_out, blocked_count) = if let Some(budget) = outer_budget {
            match tokio::time::timeout(budget, futures::future::join(drain, crawl_handle)).await {
                Ok((_, Ok(blocked_by_robots))) => (false, blocked_by_robots),
                Ok((_, Err(_join_err))) => (false, 0),
                Err(_elapsed) => {
                    // Outer guard fired: the internal crawl_timeout didn't stop
                    // the crawl on its own. Abort the spawned task explicitly
                    // so it doesn't keep running (and holding a worker) after
                    // this call has already returned to its caller.
                    crawl_abort.abort();
                    (true, 0)
                }
            }
        } else {
            let (_, joined) = futures::future::join(drain, crawl_handle).await;
            (false, joined.unwrap_or(0))
        };

        let duration_ms = start.elapsed().as_millis() as u64;

        let crawl_stats = CrawlStats {
            pages_crawled: pages.len(),
            duration_ms,
            blocked_by_robots: blocked_count,
            timed_out,
        };

        Ok(CrawlSiteOutput {
            pages,
            total_discovered,
            crawl_stats,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blacklist_matches_substring() {
        let patterns = vec!["/admin".to_string(), "logout".to_string()];
        assert!(is_blacklisted("https://example.com/admin/users", &patterns));
        assert!(is_blacklisted("https://example.com/user/logout", &patterns));
    }

    #[test]
    fn blacklist_rejects_non_matching_url() {
        let patterns = vec!["/admin".to_string()];
        assert!(!is_blacklisted("https://example.com/home", &patterns));
    }

    #[test]
    fn empty_blacklist_matches_nothing() {
        assert!(!is_blacklisted("https://example.com/admin", &[]));
    }
}
