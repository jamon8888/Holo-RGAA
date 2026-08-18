# Claude Code - rgaa-rs Project Guidelines

RGAA 4.1.2 accessibility audit platform — Rust workspace with 7 crates.

## Project Overview

- **rgaa-core**: Domain types + 106 RGAA criteria catalog
- **rgaa-rules**: axe-core violation mapping + gap-fix JS snippets
- **rgaa-holo**: Holo3 LLM client for AI-assisted evaluation
- **rgaa-browser**, **rgaa-orchestrator**, **rgaa-storage**, **rgaa-api**: Empty stubs

## Codebase-Specific Fixes

### Fix: `HoloClient::new()` panics on TLS failure

```rust
// BAD (crates/rgaa-holo/src/client.rs:68)
pub fn new(api_key: &str, api_url: &str) -> Self {
    let http_client = Client::builder()
        .timeout(Duration::from_secs(60))
        .build()
        .expect("Failed to create HTTP client"); // PANICS!
    Self { http_client, api_key: api_key.to_string(), api_url: api_url.to_string() }
}

// GOOD: return Result
pub fn new(api_key: &str, api_url: &str) -> Result<Self, RgaaError> {
    let http_client = Client::builder()
        .timeout(Duration::from_secs(60))
        .build()
        .map_err(|e| RgaaError::Holo3(format!("HTTP client init failed: {e}")))?;
    Ok(Self { http_client, api_key: api_key.to_string(), api_url: api_url.to_string() })
}
```

### Fix: `HoloClient::evaluate()` returns `Result<_, String>`

```rust
// BAD (crates/rgaa-holo/src/client.rs)
pub async fn evaluate(&self, prompt: &str) -> Result<HoloResponse, String> { ... }

// GOOD: use the defined error type
pub async fn evaluate(&self, prompt: &str) -> Result<HoloResponse, RgaaError> { ... }
```

### Fix: `AxeMapper::map()` silently drops JSON errors

```rust
// BAD (crates/rgaa-rules/src/axe_mapper.rs:12-13)
let violations: Vec<AxeViolation> = serde_json::from_str(violations_json)
    .unwrap_or_default(); // silently returns empty vec on bad JSON

// GOOD: log or return error
let violations: Vec<AxeViolation> = serde_json::from_str(violations_json)
    .map_err(|e| {
        tracing::warn!(error = %e, "failed to parse axe violations JSON");
        e
    })?;
```

### Fix: Double clone in `AxeMapper::map()`

```rust
// BAD (crates/rgaa-rules/src/axe_mapper.rs:19-28)
results.insert(rgaa_id.clone(), CriterionResult {
    criterion_id: rgaa_id.clone(), // cloned twice!
    // ...
});

// GOOD: clone once for key, reference for value
let result = CriterionResult {
    criterion_id: rgaa_id.clone(),
    // ...
};
results.insert(rgaa_id, result);
```

### Fix: `RgaaCriteria::all()` allocates on every call

```rust
// BAD (crates/rgaa-core/src/criteria.rs:14-123)
pub fn all() -> Vec<Criterion> {
    vec![ /* 106 items */ ] // allocated every call
}

// GOOD: use OnceLock for static data
use std::sync::OnceLock;

pub fn all() -> &'static [Criterion] {
    static CRITERIA: OnceLock<Vec<Criterion>> = OnceLock::new();
    CRITERIA.get_or_init(|| vec![ /* 106 items */ ])
}
```

### Fix: Non-deterministic HashMap ordering

```rust
// BAD (crates/rgaa-rules/src/axe_mapper.rs)
use std::collections::HashMap;
let results: HashMap<String, CriterionResult> = HashMap::new(); // order varies

// GOOD: use IndexMap for deterministic output
use indexmap::IndexMap;
let results: IndexMap<String, CriterionResult> = IndexMap::new();
```

## Error Handling Pattern

```rust
// Library errors (rgaa-core/src/error.rs)
#[derive(Error, Debug)]
pub enum RgaaError {
    #[error("Crawl error: {0}")]
    Crawl(String),
    #[error("Browser error: {0}")]
    Browser(String),
    #[error("Holo3 API error: {0}")]
    Holo3(String),
    // ... 5 more variants
}
pub type Result<T> = std::result::Result<T, RgaaError>;

// Add this to enable ? in HoloClient
impl From<reqwest::Error> for RgaaError {
    fn from(e: reqwest::Error) -> Self {
        RgaaError::Holo3(e.to_string())
    }
}
```

## Unit-Struct Pattern

All stateless services use unit structs with associated functions:

```rust
pub struct AxeMapper;

impl AxeMapper {
    pub fn map(violations_json: &str) -> IndexMap<String, CriterionResult> {
        // ...
    }
}

pub struct PromptBuilder;

impl PromptBuilder {
    pub fn build(context: &EvaluationContext) -> Result<String, RgaaError> {
        // ...
    }
}
```

## Testing Gaps

| Crate | Has Tests | Needs |
|-------|-----------|-------|
| rgaa-core | No | `RgaaCriteria::all()`, `find()`, `deterministic()` |
| rgaa-rules | No | `AxeMapper::map()`, `GapFixRules::snippets()` |
| rgaa-holo | Yes (client, prompts) | `PromptBuilder::build()` e2e, async `evaluate()` |

```rust
// Add async tests
#[tokio::test]
async fn evaluate_returns_valid_response() {
    let client = HoloClient::new("test-key", "http://localhost:8080").unwrap();
    let result = client.evaluate("test prompt").await;
    assert!(result.is_ok());
}

// Add benchmarks
use criterion::{criterion_group, criterion_main, Criterion};

fn bench_axe_mapper(c: &mut Criterion) {
    let json = std::fs::read_to_string("test_fixtures/axe_output.json").unwrap();
    c.bench_function("axe_mapper_map", |b| {
        b.iter(|| AxeMapper::map(&json));
    });
}

criterion_group!(benches, bench_axe_mapper);
criterion_main!(benches);
```

## Observability

```rust
// Use structured tracing (already done in rgaa-holo)
info!(attempt, max_retries = MAX_RETRIES, "Calling Holo3 API");
warn!(attempt, backoff_ms = backoff, "Rate limited, backing off");
error!(status = status.as_u16(), body = %body, "API error");

// Add instrument spans
#[tracing::instrument(name = "evaluate", skip(self))]
pub async fn evaluate(&self, prompt: &str) -> Result<HoloResponse, RgaaError> { ... }
```

## Performance Checklist

1. [ ] Profiled in `--release` mode (dev builds are 10-100x slower)
2. [ ] `RgaaCriteria::all()` uses `OnceLock` not repeated allocation
3. [ ] `AxeMapper::map()` uses `IndexMap` not `HashMap` for deterministic output
4. [ ] `HoloClient` returns `RgaaError` not `String`
5. [ ] No `.unwrap()` or `.expect()` in production paths
6. [ ] `#[must_use]` on all `Result`-returning public functions

## Quick Commands

```bash
# Build
cargo build --workspace
cargo build --workspace --release

# Test
cargo test --workspace
cargo test --workspace -- --nocapture

# Lint
cargo clippy --workspace --all-targets
cargo fmt --check

# Profile
RUSTFLAGS="-C force-frame-pointers=yes" cargo build --profile profiling
cargo flamegraph --release
```
