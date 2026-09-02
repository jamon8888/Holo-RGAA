# Spider + RGAA Integration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Integrate spider crawler as a Rig tool so the RGAA agent can autonomously discover pages and prioritize them for accessibility auditing.

**Architecture:** Spider runs as a `PortableTool` in the Rig agent. `crawl_site()` returns `Vec<PageSummary>` to the LLM, which then requests audits on selected URLs via the existing pipeline.

**Tech Stack:** spider 2.53, rig-core 0.42, tokio, schemars, serde

**Spec:** `docs/superpowers/specs/2026-08-31-spider-rgaa-design.md`

---

## Global Constraints

- Rust edition 2021 (workspace default), `rust-version = "1.80"`
- Spider feature: `reqwest_native_tls_tls` (native-tls, no WASM)
- HTML truncation: 50,000 chars per page
- Max pages default: 20, max depth default: 3
- `#[derive(Serialize, Deserialize, JsonSchema)]` on all public types

---

## Task Map

| Task | Deliverable | Files |
|------|-------------|-------|
| 1 | `rgaa-spider` crate skeleton | `crates/rgaa-spider/Cargo.toml`, `src/lib.rs`, `src/error.rs` |
| 2 | `SpiderTool` implementation | `src/tool.rs`, extend `src/lib.rs` |
| 3 | `SpiderTool` integrated into `RgaaAgent` | `rgaa-agent/src/agent.rs`, `rgaa-agent/Cargo.toml` |
| 4 | Page discovery prompt section | `rgaa-agent/src/prompts.rs` |

---

## Task 1: Create `rgaa-spider` crate

**Files:**
- Create: `rgaa-rs/crates/rgaa-spider/Cargo.toml`
- Create: `rgaa-rs/crates/rgaa-spider/src/lib.rs`
- Create: `rgaa-rs/crates/rgaa-spider/src/error.rs`
- Modify: `rgaa-rs/Cargo.toml` (add workspace member)
- Modify: `rgaa-rs/Cargo.toml` (add workspace dependency for spider)

**Interfaces:**
- Produces: `rgaa_spider::SpiderTool`, `rgaa_spider::SpiderError`, `rgaa_spider::{CrawlSiteArgs, PageSummary, CrawlSiteOutput, CrawlStats}`

- [ ] **Step 1: Create directory structure**

```bash
mkdir -p rgaa-rs/crates/rgaa-spider/src
```

- [ ] **Step 2: Write `Cargo.toml`**

```toml
[package]
description = "Spider crawler integration for RGAA agentic evaluation"
license = "MIT"
name = "rgaa-spider"
version = "0.1.0"
edition = "2021"
rust-version.workspace = true

[dependencies]
spider = { version = "2.53", default-features = false, features = ["reqwest_native_tls_tls"] }
tokio = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
schemars = { workspace = true }
thiserror = { workspace = true }
tracing = { workspace = true }
```

- [ ] **Step 3: Write `src/error.rs`**

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SpiderError {
    #[error("spider crawl failed: {0}")]
    CrawlFailed(String),

    #[error("channel receive error")]
    ChannelError,

    #[error("invalid URL: {0}")]
    InvalidUrl(String),

    #[error("page fetch failed: {0}")]
    PageFetchFailed(String),
}
```

- [ ] **Step 4: Write `src/lib.rs`** (initial skeleton — tool implementation in Task 2)

```rust
//! Spider crawler integration for RGAA.
//!
//! Provides a Rig `PortableTool` that crawls a website and returns discovered
//! pages as `PageSummary` structs for LLM-based page selection.

pub mod error;

pub use error::SpiderError;
```

- [ ] **Step 5: Add crate to workspace `Cargo.toml`**

Add `"crates/rgaa-spider"` to `members` array in `rgaa-rs/Cargo.toml`.

Add to `[workspace.dependencies]`:
```toml
spider = { version = "2.53", default-features = false, features = ["reqwest_native_tls_tls"] }
```

- [ ] **Step 6: Verify compilation**

```bash
cd rgaa-rs && cargo check -p rgaa-spider 2>&1
```

Expected: compiles with warnings (unused imports/exports OK for now).

- [ ] **Step 7: Commit**

```bash
git add rgaa-rs/crates/rgaa-spider rgaa-rs/Cargo.toml
git commit -m "feat(rgaa-spider): add spider crate skeleton"
```

---

## Task 2: Implement `SpiderTool`

**Files:**
- Create: `rgaa-rs/crates/rgaa-spider/src/tool.rs`
- Modify: `rgaa-rs/crates/rgaa-spider/src/lib.rs`

**Interfaces:**
- Consumes: nothing (standalone tool)
- Produces: `SpiderTool`, `CrawlSiteArgs`, `PageSummary`, `CrawlSiteOutput`, `CrawlStats`

- [ ] **Step 1: Write `src/tool.rs`**

```rust
use crate::error::SpiderError;
use crate::{CrawlSiteArgs, CrawlSiteOutput, CrawlStats, PageSummary};
use rig_core::tool::PortableTool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use spider::website::Website;

const HTML_TRUNCATE_LEN: usize = 50_000;
const DEFAULT_MAX_PAGES: u32 = 20;
const DEFAULT_MAX_DEPTH: u32 = 3;

/// PortableTool that crawls a website and returns discovered pages.
pub struct SpiderTool;

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct CrawlStats {
    pub pages_crawled: usize,
    pub duration_ms: u64,
    pub blocked_by_robots: usize,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct PageSummary {
    pub url: String,
    pub html: String,
    pub links: Vec<String>,
    pub status_code: u16,
    pub truncated: bool,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct CrawlSiteOutput {
    pub pages: Vec<PageSummary>,
    pub total_discovered: usize,
    pub crawl_stats: CrawlStats,
}

impl SpiderTool {
    pub fn new() -> Self {
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

    type Args = CrawlSiteArgs;

    type Output = CrawlSiteOutput;

    type Error = SpiderError;

    fn description(&self) -> String {
        "Crawl a website to discover pages for RGAA accessibility auditing. \
         Returns raw HTML and links for each discovered page so the LLM can \
         determine which pages are RGAA-relevant (mandatory pages, forms, \
         navigation, etc.).".to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::to_value(schemars::schema_for!(CrawlSiteArgs))
            .expect("valid schema")
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let start = std::time::Instant::now();
        let max_pages = args.max_pages.unwrap_or(DEFAULT_MAX_PAGES);
        let max_depth = args.max_depth.unwrap_or(DEFAULT_MAX_DEPTH);
        let respect_robots = args.respect_robots_txt.unwrap_or(true);

        let website = Website::new(&args.url)
            .with_limit(max_pages)
            .with_depth(max_depth)
            .with_respect_robots_txt(respect_robots)
            .with_user_agent(Some("RGAA-Audit-Bot/1.0".to_string()))
            .build()
            .map_err(|e| SpiderError::CrawlFailed(e.to_string()))?;

        let (tx, mut rx) = tokio::sync::mpsc::channel(16);

        let mut blocked_count: std::sync::atomic::AtomicUsize =
            std::sync::atomic::AtomicUsize::new(0);

        let website_clone = website.clone();
        let tx_clone = tx.clone();

        let handler = tokio::spawn(async move {
            let mut rx = website_clone.subscribe(16);
            while let Ok(page) = rx.recv().await {
                let url = page.get_url().to_string();
                let status = page.status_code();
                let links: Vec<String> = page.links().map(|l| l.to_string()).collect();

                let html = if let Ok(html) = page.html() {
                    if html.len() > HTML_TRUNCATE_LEN {
                        blocked_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        (html[..HTML_TRUNCATE_LEN].to_string(), true)
                    } else {
                        (html, false)
                    }
                } else {
                    (String::new(), false)
                };

                let summary = PageSummary {
                    url,
                    html: html.0,
                    links,
                    status_code: status,
                    truncated: html.1,
                };

                if tx_clone.send(summary).await.is_err() {
                    break;
                }
            }
        });

        website.crawl().await;

        let _ = handler.await;

        let mut pages = Vec::new();
        while let Ok(page) = rx.try_recv() {
            pages.push(page);
        }

        let elapsed = start.elapsed().as_millis() as u64;

        Ok(CrawlSiteOutput {
            total_discovered: pages.len(),
            crawl_stats: CrawlStats {
                pages_crawled: pages.len(),
                duration_ms: elapsed,
                blocked_by_robots: blocked_count.load(std::sync::atomic::Ordering::Relaxed),
            },
            pages,
        })
    }
}
```

- [ ] **Step 2: Write `src/lib.rs`** (final version)

```rust
//! Spider crawler integration for RGAA.
//!
//! Provides a Rig `PortableTool` that crawls a website and returns discovered
//! pages as `PageSummary` structs for LLM-based page selection.

pub mod error;
pub mod tool;

pub use error::SpiderError;
pub use tool::{SpiderTool, CrawlSiteArgs, CrawlSiteOutput, CrawlStats, PageSummary};
```

- [ ] **Step 3: Add `CrawlSiteArgs` to `tool.rs`**

Add before `SpiderTool`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CrawlSiteArgs {
    /// Starting URL to crawl
    pub url: String,
    /// Maximum number of pages to crawl (default: 20)
    #[serde(default)]
    pub max_pages: Option<u32>,
    /// Maximum crawl depth (default: 3)
    #[serde(default)]
    pub max_depth: Option<u32>,
    /// Respect robots.txt rules (default: true)
    #[serde(default)]
    pub respect_robots_txt: Option<bool>,
}
```

- [ ] **Step 4: Verify compilation**

```bash
cd rgaa-rs && cargo check -p rgaa-spider 2>&1
```

Expected: compiles cleanly (warnings OK).

- [ ] **Step 5: Commit**

```bash
git add rgaa-rs/crates/rgaa-spider/src/
git commit -m "feat(rgaa-spider): implement SpiderTool PortableTool"
```

---

## Task 3: Integrate `SpiderTool` into `RgaaAgent`

**Files:**
- Modify: `rgaa-rs/crates/rgaa-agent/Cargo.toml`
- Modify: `rgaa-rs/crates/rgaa-agent/src/lib.rs`
- Modify: `rgaa-rs/crates/rgaa-agent/src/agent.rs`

**Interfaces:**
- Consumes: `rgaa_spider::SpiderTool`
- Produces: `RgaaAgent` with `crawl_site` tool registered

- [ ] **Step 1: Add `rgaa-spider` dependency to `rgaa-agent/Cargo.toml`**

Add new line:
```toml
rgaa-spider = { path = "../rgaa-spider" }
```

- [ ] **Step 2: Register `SpiderTool` in `RgaaAgent::new()`**

In `rgaa-rs/crates/rgaa-agent/src/agent.rs`, change the agent builder from:

```rust
let agent = client
    .agent(config.model.as_str())
    .preamble("You are an RGAA accessibility expert. Evaluate criteria and provide verdicts.")
    .build();
```

To:

```rust
let agent = client
    .agent(config.model.as_str())
    .preamble("You are an RGAA accessibility expert. Evaluate criteria and provide verdicts.")
    .tool(rgaa_spider::SpiderTool::new())
    .build();
```

- [ ] **Step 3: Verify compilation**

```bash
cd rgaa-rs && cargo check -p rgaa-agent 2>&1
```

Expected: compiles cleanly.

- [ ] **Step 4: Commit**

```bash
git add rgaa-rs/crates/rgaa-agent/src/ rgaa-rs/crates/rgaa-agent/Cargo.toml
git commit -m "feat(rgaa-agent): register SpiderTool in agent builder"
```

---

## Task 4: Add page discovery section to system prompt

**Files:**
- Modify: `rgaa-rs/crates/rgaa-agent/src/prompts.rs`

**Interfaces:**
- Consumes: existing `PromptBuilder`
- Produces: `PromptBuilder` with new `page_discovery_preamble()` method

- [ ] **Step 1: Add `page_discovery_preamble()` to `prompts.rs`**

Add after `PromptBuilder` impl block:

```rust
impl PromptBuilder {
    // ... existing methods ...

    /// Returns the page discovery preamble injected into the agent preamble.
    ///
    /// Explains how to use `crawl_site` and the RGAA mandatory page types.
    pub fn page_discovery_preamble() -> &'static str {
        r#"## Page Discovery for RGAA Auditing

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
- Always respect robots.txt unless auditing requires ignoring it (explain why in your reasoning)"#
    }
}
```

- [ ] **Step 2: Use the preamble in `RgaaAgent::new()`**

In `rgaa-rs/crates/rgaa-agent/src/agent.rs`, change the preamble in the agent builder to:

```rust
use crate::prompts::PromptBuilder;

let agent = client
    .agent(config.model.as_str())
    .preamble(&format!(
        "You are an RGAA accessibility expert. Evaluate criteria and provide verdicts.\n\n{}",
        PromptBuilder::page_discovery_preamble()
    ))
    .tool(rgaa_spider::SpiderTool::new())
    .build();
```

- [ ] **Step 3: Verify compilation**

```bash
cd rgaa-rs && cargo check -p rgaa-agent 2>&1
```

Expected: compiles cleanly.

- [ ] **Step 4: Commit**

```bash
git add rgaa-rs/crates/rgaa-agent/src/prompts.rs rgaa-rs/crates/rgaa-agent/src/agent.rs
git commit -m "feat(rgaa-agent): add page discovery preamble to agent prompt"
```

---

## Spec Coverage Check

- [x] Spider as Rig PortableTool → Task 1, 2
- [x] SpiderTool registered in RgaaAgent → Task 3
- [x] System prompt with mandatory pages + selection strategy → Task 4
- [x] PageSummary with html, links, url, status_code → Task 2
- [x] HTML truncation at 50KB → Task 2
- [x] CrawlStats (pages_crawled, duration_ms, blocked_by_robots) → Task 2
- [x] max_pages, max_depth, respect_robots_txt config → Task 2

## Type Consistency Check

- `SpiderTool::call()` returns `CrawlSiteOutput` ✓
- `CrawlSiteOutput.pages` is `Vec<PageSummary>` ✓
- `PageSummary::truncated` is `bool` ✓
- `CrawlSiteArgs.url` is `String` ✓
- All types derive `Serialize, Deserialize, JsonSchema` ✓
