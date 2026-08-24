# Performance Optimization Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Optimize rgaa-rs workspace for latency, throughput, memory efficiency, and deployment readiness across all 12 crates.

**Architecture:** Cache static data with `OnceLock`, parallelize batch audits with `futures::stream`, replace `Result<T, String>` with typed errors, tune obscura V8/axe-core config, and add profiling/benchmark infrastructure.

**Tech Stack:** Rust 1.80+, tokio, futures, serde, tracing, criterion, parking_lot, axum

**Spec:** `docs/superpowers/specs/2026-08-24-performance-optimization-design.md`

## Global Constraints

- Rust edition 2024, rust-version 1.85
- `unsafe_code = "warn"`, `clippy::all = "warn"`, `clippy::pedantic = "warn"`
- All new code must have unit tests
- No `.unwrap()` in production code — use `?` or `expect()` for invariants
- Commit after each task with descriptive message

---

## Task 1: Cache `RgaaCriteria::all()` with `OnceLock`

**Files:**
- Modify: `rgaa-rs/crates/rgaa-core/src/criteria.rs:126-136`

**Interfaces:**
- Consumes: `CLASSIFICATION` const (line 14), `RgaaCatalog::title()` (line 82)
- Produces: `&'static [Criterion]` from `all()`

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn test_all_criteria_returns_same_reference() {
    let first = RgaaCriteria::all();
    let second = RgaaCriteria::all();
    std::ptr::eq(first.as_ptr(), second.as_ptr());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p rgaa-core test_all_criteria_returns_same_reference`
Expected: FAIL (references differ)

- [ ] **Step 3: Implement OnceLock cache**

```rust
use std::sync::OnceLock;

static CRITERIA_CACHE: OnceLock<Vec<Criterion>> = OnceLock::new();

impl RgaaCriteria {
    pub fn all() -> &'static [Criterion] {
        CRITERIA_CACHE.get_or_init(|| {
            CLASSIFICATION
                .iter()
                .map(|(id, classification)| Criterion {
                    id,
                    title: RgaaCatalog::title(id).unwrap_or("Unknown").to_string(),
                    classification: *classification,
                    wcag_refs: "",  // Populated from catalog if needed
                })
                .collect()
        })
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p rgaa-core test_all_criteria_returns_same_reference`
Expected: PASS

- [ ] **Step 5: Run all rgaa-core tests**

Run: `cargo test -p rgaa-core`
Expected: All 35+ tests pass

- [ ] **Step 6: Commit**

```bash
git add rgaa-rs/crates/rgaa-core/src/criteria.rs
git commit -m "perf: cache RgaaCriteria::all() with OnceLock"
```

---

## Task 2: Cache `rgaa_to_axe_map()` with `OnceLock`

**Files:**
- Modify: `rgaa-rs/crates/rgaa-rules/src/axe_mapper.rs:54-238`

**Interfaces:**
- Consumes: `AxeMapper::map()` (line 10) calls `rgaa_to_axe_map()`
- Produces: `&'static IndexMap<String, Vec<String>>`

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn test_rgaa_to_axe_map_returns_same_reference() {
    let first = AxeMapper::rgaa_to_axe_map();
    let second = AxeMapper::rgaa_to_axe_map();
    std::ptr::eq(first.as_ptr(), second.as_ptr());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p rgaa-rules test_rgaa_to_axe_map_returns_same_reference`
Expected: FAIL

- [ ] **Step 3: Implement OnceLock cache**

```rust
use std::sync::OnceLock;

static AXE_MAP_CACHE: OnceLock<IndexMap<String, Vec<String>>> = OnceLock::new();

impl AxeMapper {
    pub fn rgaa_to_axe_map() -> &'static IndexMap<String, Vec<String>> {
        AXE_MAP_CACHE.get_or_init(|| {
            let mut map = IndexMap::new();
            // ... existing mapping logic ...
            map
        })
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p rgaa-rules test_rgaa_to_axe_map_returns_same_reference`
Expected: PASS

- [ ] **Step 5: Run all rgaa-rules tests**

Run: `cargo test -p rgaa-rules`
Expected: All 7+ tests pass

- [ ] **Step 6: Commit**

```bash
git add rgaa-rs/crates/rgaa-rules/src/axe_mapper.rs
git commit -m "perf: cache rgaa_to_axe_map() with OnceLock"
```

---

## Task 3: Add `concurrency` field to `CrawlConfig`

**Files:**
- Modify: `rgaa-rs/crates/rgaa-core/src/types.rs:63-80`

**Interfaces:**
- Consumes: `CrawlConfig` struct
- Produces: `concurrency: usize` field with default 4

- [ ] **Step 1: Add field to struct**

```rust
pub struct CrawlConfig {
    pub max_pages: usize,
    pub max_depth: usize,
    pub respect_robots: bool,
    pub sample_mode: bool,
    pub concurrency: usize,  // NEW
}

impl Default for CrawlConfig {
    fn default() -> Self {
        Self {
            max_pages: 50,
            max_depth: 5,
            respect_robots: true,
            sample_mode: false,
            concurrency: 4,  // NEW
        }
    }
}
```

- [ ] **Step 2: Update test**

```rust
#[test]
fn test_crawl_config_default_concurrency() {
    let config = CrawlConfig::default();
    assert_eq!(config.concurrency, 4);
}
```

- [ ] **Step 3: Run tests**

Run: `cargo test -p rgaa-core`
Expected: All tests pass

- [ ] **Step 4: Commit**

```bash
git add rgaa-rs/crates/rgaa-core/src/types.rs
git commit -m "feat: add concurrency field to CrawlConfig"
```

---

## Task 4: Parallelize `run_batch` with `futures::stream`

**Files:**
- Modify: `rgaa-rs/crates/rgaa-orchestrator/src/pipeline.rs:50-75`
- Modify: `rgaa-rs/crates/rgaa-orchestrator/Cargo.toml` (add `futures` dependency)

**Interfaces:**
- Consumes: `CrawlConfig.concurrency`, `audit_one()` function
- Produces: Parallel audit execution

- [ ] **Step 1: Add futures dependency**

```toml
[dependencies]
futures = "0.3"
```

- [ ] **Step 2: Implement parallel batch**

```rust
pub async fn run_batch(
    urls: &[String],
    config: &CrawlConfig,
) -> Result<Vec<AuditResult>, RgaaError> {
    let bridge = ObscuraBridge::from_env();
    bridge.start_server().await?;
    
    let mut results = Vec::with_capacity(urls.len());
    let mut stream = futures::stream::iter(urls)
        .map(|url| {
            let bridge = bridge.clone();
            let config = config.clone();
            async move {
                let session = BrowserSession::new(bridge);
                let ctx = ToolContext::new(session);
                audit_one(url, &ctx, &config).await
            }
        })
        .buffer_unordered(config.concurrency);
    
    while let Some(result) = stream.next().await {
        results.push(result?);
    }
    
    bridge.stop_server().await;
    Ok(results)
}
```

- [ ] **Step 3: Add `Clone` to `ObscuraBridge`**

```rust
#[derive(Clone)]
pub struct ObscuraBridge {
    // ... existing fields
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p rgaa-orchestrator`
Expected: All tests pass

- [ ] **Step 5: Commit**

```bash
git add rgaa-rs/crates/rgaa-orchestrator/src/pipeline.rs rgaa-rs/crates/rgaa-orchestrator/Cargo.toml rgaa-rs/crates/rgaa-obscura/src/lib.rs
git commit -m "perf: parallelize run_batch with futures::stream"
```

---

## Task 5: Make agent concurrency configurable

**Files:**
- Modify: `rgaa-rs/crates/rgaa-agent/src/agent.rs:119-141`
- Modify: `rgaa-rs/crates/rgaa-agent/src/config.rs:8-36`

**Interfaces:**
- Consumes: `AgentConfig.tactical_rpm`
- Produces: Dynamic `buffer_unordered(concurrency)` value

- [ ] **Step 1: Add concurrency computation**

```rust
impl AgentConfig {
    pub fn agent_concurrency(&self) -> usize {
        let base = self.tactical_rpm / 15;
        base.clamp(1, 16)
    }
}
```

- [ ] **Step 2: Update `run_ia_assiste`**

```rust
pub async fn run_ia_assiste(
    &self,
    page_context: &PageContext,
    config: &AgentConfig,
) -> Result<IndexMap<String, CriterionResult>, AgentError> {
    let concurrency = config.agent_concurrency();
    let criteria = RgaaCriteria::ia_assiste();
    
    let mut results = futures::stream::iter(criteria)
        .map(|criterion| {
            let page_context = page_context.clone();
            async move {
                let result = self.evaluate_criterion(criterion, &page_context).await?;
                Ok((criterion.id.to_string(), result))
            }
        })
        .buffer_unordered(concurrency)
        .try_collect()
        .await?;
    
    Ok(results)
}
```

- [ ] **Step 3: Add test**

```rust
#[test]
fn test_agent_concurrency_derived_from_rpm() {
    let config = AgentConfig::default();
    // tactical_rpm=20 -> 20/15=1 -> clamp(1,16)=1
    assert_eq!(config.agent_concurrency(), 1);
    
    let config = AgentConfig {
        tactical_rpm: 120,
        ..Default::default()
    };
    // 120/15=8 -> clamp(1,16)=8
    assert_eq!(config.agent_concurrency(), 8);
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p rgaa-agent`
Expected: All tests pass

- [ ] **Step 5: Commit**

```bash
git add rgaa-rs/crates/rgaa-agent/src/agent.rs rgaa-rs/crates/rgaa-agent/src/config.rs
git commit -m "perf: make agent concurrency configurable from tactical_rpm"
```

---

## Task 6: Reduce lock scope in `audit_one`

**Files:**
- Modify: `rgaa-rs/crates/rgaa-orchestrator/src/pipeline.rs:83-122`

**Interfaces:**
- Consumes: `ToolContext.session()` mutex
- Produces: Scoped lock pattern

- [ ] **Step 1: Refactor lock usage**

```rust
pub async fn audit_one(
    url: &str,
    ctx: &ToolContext,
    config: &CrawlConfig,
) -> Result<AuditResult, RgaaError> {
    // Lock for axe-core only
    let axe_result = {
        let session = ctx.session().lock().await;
        session.bridge().run_axe(url).await?
    };
    // Lock released here
    
    // Lock for gap-fix only
    let gap_fix_result = {
        let session = ctx.session().lock().await;
        session.bridge().run_gap_fix(url).await?
    };
    // Lock released here
    
    // Lock for page context only
    let page_context = {
        let session = ctx.session().lock().await;
        session.bridge().extract_page_context(url).await?
    };
    // Lock released here
    
    // ... rest of pipeline
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p rgaa-orchestrator`
Expected: All tests pass

- [ ] **Step 3: Commit**

```bash
git add rgaa-rs/crates/rgaa-orchestrator/src/pipeline.rs
git commit -m "perf: reduce lock scope in audit_one with scoped pattern"
```

---

## Task 7: Replace `Result<T, String>` with `RgaaError`

**Files:**
- Modify: `rgaa-rs/crates/rgaa-core/src/error.rs:3-33`
- Modify: `rgaa-rs/crates/rgaa-orchestrator/src/pipeline.rs` (return types)

**Interfaces:**
- Consumes: `RgaaError` enum
- Produces: Typed error returns

- [ ] **Step 1: Add missing variants to `RgaaError`**

```rust
#[derive(Debug, thiserror::Error)]
pub enum RgaaError {
    // ... existing variants
    #[error("pipeline error: {0}")]
    Pipeline(String),
    
    #[error("agent error: {0}")]
    Agent(String),
}
```

- [ ] **Step 2: Update pipeline return types**

```rust
pub async fn audit_one(
    url: &str,
    ctx: &ToolContext,
    config: &CrawlConfig,
) -> Result<AuditResult, RgaaError> {
    // ... implementation
}
```

- [ ] **Step 3: Add `From` impl for `ObscuraError`**

```rust
impl From<ObscuraError> for RgaaError {
    fn from(e: ObscuraError) -> Self {
        RgaaError::Browser(e.to_string())
    }
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test --workspace`
Expected: All tests pass

- [ ] **Step 5: Commit**

```bash
git add rgaa-rs/crates/rgaa-core/src/error.rs rgaa-rs/crates/rgaa-orchestrator/src/pipeline.rs
git commit -m "refactor: replace Result<T,String> with RgaaError in pipeline"
```

---

## Task 8: Add `#[must_use]` annotations

**Files:**
- Modify: `rgaa-rs/crates/rgaa-rules/src/axe_mapper.rs:10`
- Modify: `rgaa-rs/crates/rgaa-rules/src/gap_fix.rs`
- Modify: `rgaa-rs/crates/rgaa-holo/src/client.rs`

**Interfaces:**
- Consumes: Fallible functions
- Produces: Compiler warnings on unused Results

- [ ] **Step 1: Add annotations**

```rust
#[must_use]
pub fn map(...) -> IndexMap<String, CriterionResult> { ... }

#[must_use]
pub fn snippets(...) -> HashMap<String, &'static str> { ... }

#[must_use]
pub async fn evaluate(...) -> Result<HoloResponse, RgaaError> { ... }
```

- [ ] **Step 2: Run tests**

Run: `cargo test --workspace`
Expected: All tests pass

- [ ] **Step 3: Commit**

```bash
git add rgaa-rs/crates/rgaa-rules/src/axe_mapper.rs rgaa-rs/crates/rgaa-rules/src/gap_fix.rs rgaa-rs/crates/rgaa-holo/src/client.rs
git commit -m "style: add #[must_use] to fallible functions"
```

---

## Task 9: Optimize release profile

**Files:**
- Modify: `rgaa-rs/Cargo.toml:53-58`

**Interfaces:**
- Consumes: Cargo profile configuration
- Produces: Optimized binary

- [ ] **Step 1: Update release profile**

```toml
[profile.release]
opt-level = 3
lto = "fat"
codegen-units = 1
panic = "abort"
strip = "symbols"
```

- [ ] **Step 2: Add profiling profile**

```toml
[profile.profiling]
inherits = "release"
debug = "line-tables-only"
strip = "none"
```

- [ ] **Step 3: Build and verify**

Run: `cargo build --release`
Expected: Smaller binary

- [ ] **Step 4: Commit**

```bash
git add rgaa-rs/Cargo.toml
git commit -m "perf: optimize release profile with fat LTO"
```

---

## Task 10: Add graceful shutdown to API

**Files:**
- Modify: `rgaa-rs/crates/rgaa-api/src/main.rs:321-358`
- Modify: `rgaa-rs/crates/rgaa-api/Cargo.toml` (add `tower-http`)

**Interfaces:**
- Consumes: `tokio::signal::ctrl_c()`
- Produces: Graceful shutdown handling

- [ ] **Step 1: Add tower-http dependency**

```toml
[dependencies]
tower-http = { version = "0.5", features = ["timeout"] }
```

- [ ] **Step 2: Implement graceful shutdown**

```rust
#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/health", get(health_check))
        .layer(TimeoutLayer::new(Duration::from_secs(30)));
    
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .unwrap();
    
    tracing::info!("Server started on port 3000");
    
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };
    
    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };
    
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();
    
    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
    
    tracing::info!("Shutdown signal received, starting graceful shutdown");
}
```

- [ ] **Step 3: Run tests**

Run: `cargo test -p rgaa-api`
Expected: All tests pass

- [ ] **Step 4: Commit**

```bash
git add rgaa-rs/crates/rgaa-api/src/main.rs rgaa-rs/crates/rgaa-api/Cargo.toml
git commit -m "feat: add graceful shutdown to API server"
```

---

## Task 11: Configure database connection pooling

**Files:**
- Modify: `rgaa-rs/crates/rgaa-api/src/main.rs:329-332`

**Interfaces:**
- Consumes: `PgPoolOptions`
- Produces: Configurable pool

- [ ] **Step 1: Update pool configuration**

```rust
let max_conn: u32 = std::env::var("DATABASE_MAX_CONNECTIONS")
    .unwrap_or_else(|_| "10".to_string())
    .parse()
    .expect("DATABASE_MAX_CONNECTIONS must be a number");

let pool = PgPoolOptions::new()
    .max_connections(max_conn)
    .min_connections(2)
    .connect(&database_url)
    .await
    .expect("Failed to connect to database");
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p rgaa-api`
Expected: All tests pass

- [ ] **Step 3: Commit**

```bash
git add rgaa-rs/crates/rgaa-api/src/main.rs
git commit -m "perf: configure database connection pooling"
```

---

## Task 12: Add structured tracing with timing

**Files:**
- Modify: `rgaa-rs/crates/rgaa-orchestrator/src/pipeline.rs:83`
- Modify: `rgaa-rs/crates/rgaa-agent/src/agent.rs:67`

**Interfaces:**
- Consumes: `#[tracing::instrument]`
- Produces: Timing data in logs

- [ ] **Step 1: Add instrument attributes**

```rust
#[tracing::instrument(skip_all, fields(url = %url))]
pub async fn audit_one(
    url: &str,
    ctx: &ToolContext,
    config: &CrawlConfig,
) -> Result<AuditResult, RgaaError> {
    let start = std::time::Instant::now();
    // ... implementation
    tracing::info!(elapsed_ms = start.elapsed().as_millis(), "audit complete");
    Ok(result)
}
```

- [ ] **Step 2: Add instrument to `evaluate_criterion`**

```rust
#[tracing::instrument(skip_all, fields(criterion = %criterion.id))]
pub async fn evaluate_criterion(
    &self,
    criterion: &Criterion,
    page_context: &PageContext,
) -> Result<CriterionResult, AgentError> {
    // ... implementation
}
```

- [ ] **Step 3: Run tests**

Run: `cargo test --workspace`
Expected: All tests pass

- [ ] **Step 4: Commit**

```bash
git add rgaa-rs/crates/rgaa-orchestrator/src/pipeline.rs rgaa-rs/crates/rgaa-agent/src/agent.rs
git commit -m "feat: add structured tracing with timing"
```

---

## Task 13: Add Criterion benchmarks

**Files:**
- Modify: `rgaa-rs/Cargo.toml` (add criterion)
- Create: `rgaa-rs/benches/performance.rs`

**Interfaces:**
- Consumes: `criterion` crate
- Produces: Benchmark results

- [ ] **Step 1: Add criterion dependency**

```toml
[workspace.dependencies]
criterion = { version = "0.5", features = ["html_reports"] }

[[bench]]
name = "performance"
harness = false
```

- [ ] **Step 2: Create benchmark file**

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rgaa_core::criteria::RgaaCriteria;
use rgaa_rules::axe_mapper::AxeMapper;

fn bench_rgaa_criteria_all(c: &mut Criterion) {
    c.bench_function("rgaa_criteria_all", |b| {
        b.iter(|| black_box(RgaaCriteria::all()))
    });
}

fn bench_axe_mapper_map(c: &mut Criterion) {
    let violations = r#"{"violations":[]}"#;
    c.bench_function("axe_mapper_map", |b| {
        b.iter(|| black_box(AxeMapper::map(violations)))
    });
}

criterion_group!(benches, bench_rgaa_criteria_all, bench_axe_mapper_map);
criterion_main!(benches);
```

- [ ] **Step 3: Run benchmarks**

Run: `cargo bench`
Expected: Benchmarks execute and produce HTML reports

- [ ] **Step 4: Commit**

```bash
git add rgaa-rs/Cargo.toml rgaa-rs/benches/performance.rs
git commit -m "feat: add criterion benchmarks for hot paths"
```

---

## Task 14: Optimize axe-core configuration

**Files:**
- Modify: `rgaa-rs/crates/rgaa-obscura/src/lib.rs:659-662`

**Interfaces:**
- Consumes: axe-core CDN script
- Produces: Optimized axe config

- [ ] **Step 1: Update axe-core config**

```rust
pub async fn run_axe(&self, url: &str) -> Result<String, ObscuraError> {
    let axe_script = self.fetch_axe_core().await?;
    
    let config = serde_json::json!({
        "resultTypes": ["violations", "incomplete"],
        "elementRef": false,
        "reporter": "v2",
        "runOnly": {
            "type": "tag",
            "values": ["wcag2a", "wcag2aa", "wcag21a", "wcag21aa"]
        }
    });
    
    self.run_axe_with_script(url, &axe_script, &config.to_string()).await
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p rgaa-obscura`
Expected: All tests pass

- [ ] **Step 3: Commit**

```bash
git add rgaa-rs/crates/rgaa-obscura/src/lib.rs
git commit -m "perf: optimize axe-core config with resultTypes filtering"
```

---

## Task 15: Document obscura tuning

**Files:**
- Create: `rgaa-rs/crates/rgaa-obscura/TUNING.md`

**Interfaces:**
- Consumes: obscura CLI reference
- Produces: Deployment documentation

- [ ] **Step 1: Create tuning guide**

```markdown
# Obscura Performance Tuning

## Worker Configuration

Use `--workers N` where N = CPU cores for true V8 parallelism:

```bash
obscura serve --workers $(nproc)
```

## V8 Memory Tuning

Default V8 flags on 64-bit:
- `--max-old-space-size=4096`
- `--max-semi-space-size=4`
- `--optimize-for-size`

Override for memory-constrained hosts:
```bash
obscura serve --v8-flags "--max-old-space-size=2048"
```

## Timeout Configuration

Environment variables:
- `OBSCURA_NAV_TIMEOUT_MS=60000` (per-navigation)
- `OBSCURA_CDP_COMMAND_TIMEOUT_MS=30000` (per-CDP-command)
- `OBSCURA_FETCH_TIMEOUT_MS=20000` (scripted fetch/XHR)
- `OBSCURA_SCRIPT_DEADLINE_MS=60000` (heavy SPA budget)
```

- [ ] **Step 2: Commit**

```bash
git add rgaa-rs/crates/rgaa-obscura/TUNING.md
git commit -m "docs: add obscura performance tuning guide"
```

---

## Verification

After completing all tasks, run:

```bash
# Run all tests
cargo test --workspace

# Run clippy
cargo clippy --workspace --all-targets

# Run benchmarks
cargo bench

# Build release
cargo build --release

# Check binary size
ls -lh target/release/rgaa-cli
```

Expected:
- All tests pass
- No clippy warnings
- Benchmarks produce HTML reports
- Release binary ~10MB
