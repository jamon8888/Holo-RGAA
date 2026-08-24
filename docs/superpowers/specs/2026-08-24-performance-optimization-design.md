# Performance Optimization Design

> Comprehensive performance optimization for rgaa-rs workspace targeting
> latency, throughput, memory efficiency, and deployment readiness.

**Date:** 2026-08-24
**Status:** Approved for implementation
**Scope:** All 12 crates in rgaa-rs workspace

---

## 1. Memory Optimization

### 1.1 Cache static data with `OnceLock`

**Problem:** `RgaaCriteria::all()` allocates 106 `Criterion` structs on every call. `rgaa_to_axe_map()` rebuilds 77-entry `IndexMap` per `AxeMapper::map()`.

**Solution:**
- `rgaa-core/src/criteria.rs`: Cache `Vec<Criterion>` in `static INSTANCE: OnceLock<Vec<Criterion>>`
- `rgaa-rules/src/axe_mapper.rs`: Cache `IndexMap<String, Vec<String>>` in `static MAPPING: OnceLock<IndexMap<...>>`

**Files:**
- `rgaa-core/src/criteria.rs:126` — `all()` method
- `rgaa-rules/src/axe_mapper.rs:54` — `rgaa_to_axe_map()` method

**Acceptance:**
- `RgaaCriteria::all()` returns same `&'static [Criterion]` on repeated calls
- `AxeMapper::map()` reuses cached mapping

### 1.2 Reduce string allocations

**Problem:** `CriterionResult` fields use `String` with repeated `.to_string()` on static data.

**Solution:**
- Use `Cow<'static, str>` for `criterion_id`, `title`, `source` where values are static
- Pre-allocate `Vec` with `with_capacity()` where size is known (106 criteria, 77 axe rules)

**Files:**
- `rgaa-core/src/types.rs` — `CriterionResult` struct
- `rgaa-orchestrator/src/pipeline.rs:139-182` — merge loop

**Acceptance:**
- Zero-copy for static criterion data
- `Vec` allocations use `with_capacity(106)` for criteria collection

---

## 2. Concurrency Optimization

### 2.1 Parallel batch audits

**Problem:** `run_batch` processes URLs sequentially in a `for` loop (`pipeline.rs:70`).

**Solution:**
- Use `futures::stream::iter(urls).map(|url| audit_one(...)).buffer_unordered(concurrency).collect()`
- Add `concurrency: usize` field to `CrawlConfig` (default: 4)
- Browser lock acquired/released per-URL, not held across batch

**Files:**
- `rgaa-orchestrator/src/pipeline.rs:50-75` — `run_batch` method
- `rgaa-core/src/types.rs:64-69` — `CrawlConfig` struct

**Acceptance:**
- Batch of 10 URLs with concurrency=4 completes in ~3x single-URL time (not 10x)
- Each URL audit is independent

### 2.2 Configurable agent parallelism

**Problem:** `buffer_unordered(4)` hardcoded in `run_ia_assiste` (`agent.rs:136`).

**Solution:**
- Derive concurrency from `AgentConfig.tactical_rpm`: `concurrency = max(1, tactical_rpm / 60)`
- Pass as parameter to `run_ia_assiste`
- Default: `tactical_rpm=20` → concurrency=4 (matches current)

**Files:**
- `rgaa-agent/src/agent.rs:119-141` — `run_ia_assiste` method
- `rgaa-agent/src/config.rs` — `AgentConfig` struct

**Acceptance:**
- Changing `tactical_rpm` in config changes parallelism
- No rate limit violations

### 2.3 Reduce lock scope in pipeline

**Problem:** `tool_ctx.session().lock().await` holds Mutex during axe + gap-fix + page context extraction (`pipeline.rs:93`).

**Solution:**
- Lock once for axe-core, release immediately
- Lock again for gap-fix, release immediately
- Lock again for page context, release immediately
- Each browser call is independent; no need to hold lock across all three

**Files:**
- `rgaa-orchestrator/src/pipeline.rs:83-122` — `audit_one` function

**Acceptance:**
- Mutex held for <100ms per lock acquisition (vs ~3s currently)
- Other tasks can interleave between browser calls

---

## 3. Error Handling

### 3.1 Replace `Result<T, String>` with `RgaaError`

**Problem:** `pipeline.rs` and callers return `Result<T, String>`, losing error context.

**Solution:**
- Add `RgaaError::Pipeline(String)` variant for orchestration errors
- `pipeline.rs`: Return `Result<AuditResult, RgaaError>`
- Map `ObscuraError` → `RgaaError` at orchestration boundary
- Remove `.unwrap_or_default()` on JSON parse; use `.map_err()?`

**Files:**
- `rgaa-core/src/error.rs` — add `Pipeline` variant
- `rgaa-orchestrator/src/pipeline.rs` — change return types

**Acceptance:**
- All pipeline functions return `Result<T, RgaaError>`
- Malformed JSON produces `Err`, not silent empty vec

### 3.2 `#[must_use]` annotations

**Problem:** Fallible functions can have their `Result` silently ignored.

**Solution:** Add `#[must_use]` to:
- `AxeMapper::map()`
- `GapFixRules::snippets()`
- `HoloClient::evaluate()`
- `RgaaCriteria::all()` (already `&[Criterion]`, no change needed)

**Files:**
- `rgaa-rules/src/axe_mapper.rs:10`
- `rgaa-rules/src/gap_fix.rs`
- `rgaa-holo/src/client.rs`

**Acceptance:**
- Compiler warns on unused `Result` from these functions

---

## 4. Build & Deployment

### 4.1 Release profile optimization

**Problem:** `lto = "thin"` leaves optimization on the table.

**Solution:**
```toml
[profile.release]
opt-level = 3
lto = "fat"           # Whole-program LTO
codegen-units = 1     # Single codegen unit
panic = "abort"       # Smaller binary
strip = "symbols"     # Remove debug symbols
```

**Trade-off:** ~2-3 min longer compile, 5-15% runtime improvement.

**Files:** `rgaa-rs/Cargo.toml:52-57`

**Acceptance:**
- `cargo build --release` produces smaller binary
- Runtime benchmarks show improvement

### 4.2 Graceful shutdown for API

**Problem:** API server has no shutdown handling; kills connections on SIGTERM.

**Solution:**
- Use `tokio::signal::ctrl_c()` + `axum::serve(...).with_graceful_shutdown()`
- Drop database pool on shutdown
- Log shutdown events

**Files:** `rgaa-api/src/main.rs:321-358`

**Acceptance:**
- Ctrl+C triggers graceful shutdown
- In-flight requests complete before exit

### 4.3 Database connection pooling

**Problem:** `PgPool::connect()` uses default pool config (no limit).

**Solution:**
- Use `PgPoolOptions::new().max_connections(max_conn)`
- Read `max_conn` from `DATABASE_MAX_CONNECTIONS` env (default: 10)

**Files:** `rgaa-api/src/main.rs:329-332`

**Acceptance:**
- Pool respects configured max connections
- No connection exhaustion under load

---

## 5. Observability & Profiling

### 5.1 Structured tracing with timing

**Problem:** Logs lack timing data; hard to identify bottlenecks.

**Solution:**
- Add `#[tracing::instrument(skip_all)]` on `audit_one`, `run_ia_assiste`, `AxeMapper::map`
- Log durations: `info!(elapsed_ms = start.elapsed().as_millis(), "operation complete")`
- Use tracing spans for nested operations

**Files:**
- `rgaa-orchestrator/src/pipeline.rs:83` — `audit_one`
- `rgaa-agent/src/agent.rs:67` — `evaluate_criterion`

**Acceptance:**
- Each pipeline stage logs duration
- Total audit time visible in logs

### 5.2 Profiling profile

**Problem:** No profile for benchmarking without distorting results.

**Solution:**
```toml
[profile.profiling]
inherits = "release"
debug = "line-tables-only"
strip = "none"
```

Enables `samply`, `cargo flamegraph`, `perf` without recompiling.

**Files:** `rgaa-rs/Cargo.toml`

**Acceptance:**
- `cargo build --profile profiling` produces benchmarkable binary

### 5.3 Criterion benchmarks

**Problem:** No automated performance regression detection.

**Solution:** Add benchmarks for hot paths:
- `AxeMapper::map()` — 77-entry mapping
- `RgaaCriteria::all()` — 106-criterion allocation
- `calculate_compliance()` — compliance computation
- `PromptBuilder::build()` — string construction

**Files:** `rgaa-rs/benches/` (new directory)

**Acceptance:**
- `cargo bench` runs all benchmarks
- Results stored for comparison

---

## Implementation Order

1. **Memory** (Section 1) — zero-risk, immediate wins
2. **Error Handling** (Section 3) — enables safe concurrency changes
3. **Concurrency** (Section 2) — depends on error types
4. **Build & Deployment** (Section 4) — independent
5. **Observability** (Section 5) — independent, last

## Success Metrics

| Metric | Current (est.) | Target |
|--------|----------------|--------|
| Single URL audit | ~8s | ~5s |
| 10-URL batch (concurrency=4) | ~80s | ~20s |
| Peak memory (single audit) | ~50MB | ~30MB |
| Binary size (release) | ~15MB | ~10MB |
| `RgaaCriteria::all()` calls/s | ~1K | ~100K (cached) |
