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
- Add `#[serde(borrow)]` on borrowed fields or use `#[serde(with = "...")]` for custom serde
- Pre-allocate `Vec` with `with_capacity()` where size is known (106 criteria, 77 axe rules)
- Use `clone_from()` in merge loop to reuse existing allocations
- Use `write!()` into reusable `String` buffer instead of `format!()` in hot paths

**Files:**
- `rgaa-core/src/types.rs` — `CriterionResult` struct
- `rgaa-orchestrator/src/pipeline.rs:139-182` — merge loop

**Acceptance:**
- Zero-copy for static criterion data
- `Vec` allocations use `with_capacity(106)` for criteria collection
- No new heap allocations in merge loop for existing entries

---

## 2. Concurrency Optimization

### 2.1 Parallel batch audits

**Problem:** `run_batch` processes URLs sequentially in a `for` loop (`pipeline.rs:70`).

**Solution:**
- Use `futures::stream::iter(urls).map(|url| audit_one(...)).buffer_unordered(concurrency).collect()`
- Add `concurrency: usize` field to `CrawlConfig` (default: 4)
- Browser lock acquired/released per-URL, not held across batch
- `ObscuraBridge` is config-only (strings + optional Child), safe to clone per task

**Files:**
- `rgaa-orchestrator/src/pipeline.rs:50-75` — `run_batch` method
- `rgaa-core/src/types.rs:64-69` — `CrawlConfig` struct

**Acceptance:**
- Batch of 10 URLs with concurrency=4 completes in ~3x single-URL time (not 10x)
- Each URL audit is independent

### 2.2 Configurable agent parallelism

**Problem:** `buffer_unordered(4)` hardcoded in `run_ia_assiste` (`agent.rs:136`).

**Solution:**
- Derive concurrency from `AgentConfig.tactical_rpm`: `concurrency = max(1, min(tactical_rpm / 15, 16))`
- Cap at 16 to prevent overwhelming the API
- Pass as parameter to `run_ia_assiste`
- Default: `tactical_rpm=20` → concurrency=1 (conservative), user can increase

**Files:**
- `rgaa-agent/src/agent.rs:119-141` — `run_ia_assiste` method
- `rgaa-agent/src/config.rs` — `AgentConfig` struct

**Acceptance:**
- Changing `tactical_rpm` in config changes parallelism
- No rate limit violations
- Concurrency capped at 16 to prevent API overload

### 2.3 Reduce lock scope in pipeline

**Problem:** `tool_ctx.session().lock().await` holds Mutex during axe + gap-fix + page context extraction (`pipeline.rs:93`).

**Solution:**
- Lock once for axe-core, release immediately
- Lock again for gap-fix, release immediately
- Lock again for page context, release immediately
- Each browser call is independent; no need to hold lock across all three
- Use scoped lock pattern: `{ let guard = session.lock().await; /* use */ } // guard dropped`

**Files:**
- `rgaa-orchestrator/src/pipeline.rs:83-122` — `audit_one` function

**Acceptance:**
- Mutex held for <100ms per lock acquisition (vs ~3s currently)
- Other tasks can interleave between browser calls
- No `MutexGuard` held across `.await` points

---

## 3. Error Handling

### 3.1 Replace `Result<T, String>` with `RgaaError`

**Problem:** `pipeline.rs` and callers return `Result<T, String>`, losing error context.

**Solution:**
- Add specific `RgaaError` variants: `Browser(String)`, `Agent(String)`, `Pipeline(String)`
- `pipeline.rs`: Return `Result<AuditResult, RgaaError>`
- Map `ObscuraError` → `RgaaError` at orchestration boundary using `From` impl
- Remove `.unwrap_or_default()` on JSON parse; use `.map_err()?`
- Preserve error chains with `#[source]` attribute

**Files:**
- `rgaa-core/src/error.rs` — add variants + `From<ObscuraError>`
- `rgaa-orchestrator/src/pipeline.rs` — change return types

**Acceptance:**
- All pipeline functions return `Result<T, RgaaError>`
- Malformed JSON produces `Err`, not silent empty vec
- Error chains preserved for debugging

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

**Trade-off:** ~2-3 min longer compile, 2-10% runtime improvement.

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
- Add `tower_http::timeout::TimeoutLayer` for individual request timeouts (30s default)

**Files:** `rgaa-api/src/main.rs:321-358`

**Acceptance:**
- Ctrl+C triggers graceful shutdown
- In-flight requests complete before exit
- Long-running requests timeout after 30s

### 4.3 Database connection pooling

**Problem:** `PgPool::connect()` uses default pool config (no limit).

**Solution:**
- Use `PgPoolOptions::new().max_connections(max_conn).min_connections(2)`
- Read `max_conn` from `DATABASE_MAX_CONNECTIONS` env (default: 10)
- `min_connections(2)` keeps warm pool for low-latency first request

**Files:** `rgaa-api/src/main.rs:329-332`

**Acceptance:**
- Pool respects configured max connections
- No connection exhaustion under load
- Warm pool reduces first-request latency

---

## 5. Observability & Profiling

### 5.1 Structured tracing with timing

**Problem:** Logs lack timing data; hard to identify bottlenecks.

**Solution:**
- Add `#[tracing::instrument(skip_all, fields(url = %url))]` on `audit_one`
- Add `#[tracing::instrument(skip_all, fields(criterion = %criterion.id))]` on `evaluate_criterion`
- Log durations: `info!(elapsed_ms = start.elapsed().as_millis(), "operation complete")`
- Use tracing spans for nested operations

**Files:**
- `rgaa-orchestrator/src/pipeline.rs:83` — `audit_one`
- `rgaa-agent/src/agent.rs:67` — `evaluate_criterion`

**Acceptance:**
- Each pipeline stage logs duration with URL/criterion context
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

Add `criterion` to workspace dependencies:
```toml
[workspace.dependencies]
criterion = { version = "0.5", features = ["html_reports"] }
```

**Files:**
- `rgaa-rs/Cargo.toml` — add `criterion` dependency
- `rgaa-rs/benches/` — new directory with benchmark files

**Acceptance:**
- `cargo bench` runs all benchmarks
- Results stored for comparison
- CI job runs benchmarks on PRs and reports regression

---

## 6. Obscura & axe-core Tuning (from documentation research)

### 6.1 Obscura worker and V8 configuration

**Problem:** Default obscura settings (1 worker, default V8 heap) don't leverage available CPU cores or memory tuning.

**Solution:**
- Use `--workers N` where N = CPU cores for true V8 parallelism (each worker = separate V8 isolate)
- Default V8 flags on 64-bit: `--max-old-space-size=4096 --max-semi-space-size=4 --optimize-for-size`
- Override for memory-constrained hosts: `--max-old-space-size=2048`
- Override for heavy SPAs: `--max-old-space-size=8192 --max-semi-space-size=8`
- Tune timeouts via env vars:
  - `OBSCURA_NAV_TIMEOUT_MS=60000` (per-navigation, default 30000)
  - `OBSCURA_CDP_COMMAND_TIMEOUT_MS=30000` (per-CDP-command V8 deadline, default 60000)
  - `OBSCURA_FETCH_TIMEOUT_MS=20000` (scripted fetch/XHR, default 30000)
  - `OBSCURA_SCRIPT_DEADLINE_MS=60000` (heavy SPA script execution budget, default 30000)

**Impact:** obscura achieves ~30 MB per process vs Chrome's 200+ MB; ~21x faster median latency; 10-17% RSS reduction with `--optimize-for-size`.

**Files:**
- `rgaa-obscura/src/lib.rs` — `AXE_CORE_CDN`, `ObscuraBridge` config
- Deployment config / CLI args — workers, V8 flags, env vars

**Acceptance:**
- `obscura serve --workers N` runs N parallel worker processes
- V8 flags tunable via `--v8-flags` CLI arg
- Timeout env vars configurable per deployment

### 6.2 axe-core result filtering

**Problem:** Running `axe.run()` without `resultTypes` returns passes, inapplicable, violations, and incomplete — wasting bandwidth and parsing time.

**Solution:**
- Use `resultTypes: ['violations', 'incomplete']` in axe-core config to skip passes/inapplicable
- Use `elementRef: false` to avoid serializing the full live DOM element (returns selector string only)
- Use `reporter: 'v2'` for faster JSON output (strips pass details)
- Freeze axe-core config to prevent order-dependent flake: `Object.freeze(config)`
- Use `runOnly.type: 'tag'` with `values: ['wcag2a', 'wcag2aa']` for targeted rule execution

**Impact:** Reduces axe-core output size by ~40-60%, faster JSON serialization, lower memory during evaluation.

**Files:**
- `rgaa-obscura/src/lib.rs` — axe-core injection and config
- `rgaa-core/data/rgaa-4.1.2/axe_mapping.json` — rule mapping

**Acceptance:**
- axe-core runs with `resultTypes` filtering
- `elementRef: false` reduces DOM serialization overhead
- Config frozen to prevent flake

---

## 7. Additional Optimizations (from skills review)

### 7.1 Faster mutex implementation

**Problem:** `std::sync::Mutex` has overhead for uncontended locks.

**Solution:** Consider `parking_lot::Mutex` for hot paths:
- `ToolContext` session mutex
- Any mutex in agent evaluation loop

**Trade-off:** Adds dependency, but 2-5x faster for uncontended locks.

**Files:** `rgaa-browser-tools/src/session.rs`, `Cargo.toml`

**Acceptance:**
- Benchmarks show improvement for lock-heavy paths

### 7.2 Integer-keyed map optimization

**Problem:** `HashMap<String, Vec<String>>` for axe mapping uses string keys.

**Solution:** For purely internal maps with integer-like keys, consider `FxHashMap` or `AHash`:
- Faster hashing for integer-like strings
- Lower allocation overhead

**Trade-off:** Adds dependency, but 10-20% faster for integer keys.

**Files:** `rgaa-rules/src/axe_mapper.rs`

**Acceptance:**
- Benchmarks show improvement for mapping operations

### 7.3 CI benchmark regression

**Problem:** No automated detection of performance regressions.

**Solution:** Add GitHub Actions job:
```yaml
- name: Run benchmarks
  run: cargo bench --workspace 2>&1 | tee bench-output.txt
- name: Compare with baseline
  # Compare against main branch baseline
```

**Files:** `.github/workflows/bench.yml` (new)

**Acceptance:**
- PRs show benchmark comparison
- Regressions flagged as warnings

---

## Implementation Order

1. **Memory** (Section 1) — zero-risk, immediate wins
2. **Error Handling** (Section 3) — enables safe concurrency changes
3. **Concurrency** (Section 2) — depends on error types
4. **Obscura & axe-core Tuning** (Section 6) — external tool configuration
5. **Build & Deployment** (Section 4) — independent
6. **Observability** (Section 5) — independent, last
7. **Additional Optimizations** (Section 7) — based on benchmarks

## Success Metrics

| Metric | Current (est.) | Target |
|--------|----------------|--------|
| Single URL audit | ~8s | ~5s |
| 10-URL batch (concurrency=4) | ~80s | ~20s |
| Peak memory (single audit) | ~50MB | ~30MB |
| Binary size (release) | ~15MB | ~10MB |
| `RgaaCriteria::all()` calls/s | ~1K | ~100K (cached) |
| axe-core output size | 100% | ~40-60% (resultTypes filtering) |
| obscura RSS per process | ~50MB | ~30MB (V8 tuning) |
