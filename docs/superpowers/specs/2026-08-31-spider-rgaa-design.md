# Spider + RGAA Integration Design

## Status

Approved for implementation.

## Overview

Integrate the `spider` crawler as a Rig tool so the RGAA agent can autonomously discover and prioritize pages for accessibility auditing. Spider crawls a site, returns raw HTML + URLs to the LLM, which then selects RGAA-relevant pages for full audit via the existing pipeline.

## Architecture

```
SpiderTool (Rig PortableTool)
  └── crawl_site(url, max_pages, max_depth, respect_robots)
        └── Vec<PageSummary { url, html, links }>
              │
              ▼
        LLM: select RGAA-relevant pages
              │
              ▼
        audit_one() per selected URL (existing pipeline)
              │
              ▼
        Merged AuditResult
```

## New Crate: rgaa-spider

Path: `crates/rgaa-spider/`

### Dependencies

```toml
[package]
name = "rgaa-spider"
edition = "2024"
rust-version = "1.85"

[dependencies]
spider = { version = "2.53", features = ["reqwest_native_tls_tls"] }
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
schemars = "0.8"
tracing = "0.1"
```

### Types

```rust
// Tool input
pub struct CrawlSiteArgs {
    pub url: String,
    pub max_pages: Option<u32>,       // default 20
    pub max_depth: Option<u32>,       // default 3
    pub respect_robots_txt: Option<bool>, // default true
}

// Tool output
#[derive(Serialize, Deserialize)]
pub struct PageSummary {
    pub url: String,
    pub html: String,           // raw HTML, truncated if large
    pub links: Vec<String>,    // discovered hrefs
    pub status_code: u16,
}

#[derive(Serialize, Deserialize)]
pub struct CrawlSiteOutput {
    pub pages: Vec<PageSummary>,
    pub total_discovered: usize,
    pub crawl_stats: CrawlStats,
}

pub struct CrawlStats {
    pub pages_crawled: usize,
    pub duration_ms: u64,
    pub blocked_by_robots: usize,
}
```

### SpiderTool Implementation

```rust
pub struct SpiderTool;

impl PortableTool for SpiderTool {
    const NAME: str = "crawl_site";
    type Args = CrawlSiteArgs;
    type Output = CrawlSiteOutput;
    type Error = SpiderError;

    fn description() -> String { "...".to_string() }
    fn parameters() -> serde_json::Value { schemars::schema_for!(CrawlSiteArgs) }

    async fn call(args: CrawlSiteArgs) -> Result<CrawlSiteOutput, SpiderError> {
        // 1. Build Website with Configuration
        // 2. Set channel buffer for page stream
        // 3. Call website.crawl().await
        // 4. Collect pages up to max_pages
        // 5. Truncate html to e.g. 50KB per page to avoid huge tool responses
        // 6. Return CrawlSiteOutput
    }
}
```

## rgaa-agent Changes

### New tool registration

In `rgaa-agent/src/lib.rs`, add `SpiderTool` to the agent builder:

```rust
let agent = client
    .agent(model)
    .preamble(system_prompt)
    .tool(SpiderTool)
    // existing tools...
    .build();
```

### System prompt additions

In `rgaa-agent/src/prompts.rs`, add a `PageDiscoveryContext` section to the agent preamble:

```
## Page Discovery for RGAA Auditing

When auditing a website, you must first discover which pages exist before
you can evaluate their accessibility. Use the `crawl_site` tool to
discover pages on the target domain.

### RGAA Mandatory Pages (all sites)

These 5 page types are mandatory for RGAA compliance on ANY website:
1. **Home page** (/) - the main entry point
2. **Contact or Search** - user communication or site search
3. **Legal mentions** (/mentions-legales, /legal, /cgv, /privacy) - CGU, privacy policy, cookies
4. **Sitemap** (/sitemap, /plan-du-site) - site structure overview
5. **Login or Account** (/login, /connexion, /account, /mon-compte) - user authentication

### Site-Specific Pages

After crawling, analyze page content to detect:
- Forms with more than 3 fields (likely registration, checkout, contact)
- Pages with video/audio players
- Pages with data tables
- Pages with modal/dialog interactions
- Search result pages
- Error pages (404, 500)

### Page Selection Strategy

1. Call `crawl_site` with the base URL
2. Review discovered pages and their content
3. Always include the 5 mandatory page types if found
4. Add site-specific pages identified above
5. Request a full RGAA audit on each selected page via the `audit_page` tool
6. Include 2-3 random low-depth pages for surprise coverage

### Crawl Configuration

- Start with `max_pages=30, max_depth=3` for initial discovery
- If the site is large (>100 pages), first crawl at depth 1 to find key pages, then recrawl sections as needed
- Always respect robots.txt unless auditing requires ignoring it (explain why in your reasoning)
```

## rgaa-orchestrator Changes

No structural changes. `Orchestrator::run_batch()` remains the entry point for single-URL audits. Spider discovery feeds URLs into `run_batch()` or directly into `audit_one()`.

## Data Flow

```
User: "Audit example.com for RGAA compliance"

Agent calls crawl_site(url="https://example.com", max_pages=30, max_depth=3)
  → SpiderTool returns 30 PageSummary { url, html, links }
  → LLM analyzes pages, selects 8 RGAA-relevant URLs
  → LLM calls audit_page() for each selected URL (8 parallel)
  → Each audit_page() → Orchestrator::audit_one() → AxeMapper + HoloClient + Merge
  → Results returned to LLM
  → LLM synthesizes into final AuditResult
```

## Error Handling

| Error | Handling |
|-------|----------|
| Spider fails to connect | Return error to LLM, LLM decides retry or skip |
| No pages discovered | Return empty list, LLM prompts user for explicit URLs |
| Page HTML too large | Truncate to 50KB, include `truncated: true` flag |
| robots.txt blocks all | Return crawl_stats with blocked count, proceed with allowed pages |

## HTML Truncation

To avoid huge tool responses, truncate each page's HTML to 50,000 characters. Store first 50KB — this is sufficient for the LLM to understand page purpose and structure for page selection. The full HTML is NOT needed at this stage; axe-core fetches the page fresh during audit.

## Testing

- **Unit**: `SpiderTool::call()` against a local `localhost` server
- **Integration**: crawl a known test site, verify page discovery and truncation
- **Agent integration**: end-to-end test with mock HoloClient

## Out of Scope

- Storing crawl state across sessions (stateless per run)
- Distributed crawling
- Custom link rewrite callbacks
- Chrome/CDP rendering in spider (JS rendering handled by ObscuraBridge during audit)
