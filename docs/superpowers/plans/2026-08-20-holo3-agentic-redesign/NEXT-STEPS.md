# Holo3 Agentic Redesign — Next Steps Report

**Date:** 2026-08-21
**Status:** Implementation phase complete, production readiness pending

---

## Executive Summary

The 13-task implementation plan is complete. Two new crates (`rgaa-browser-tools`, `rgaa-agent`) were created, the orchestrator was wired to use the rig agent, and integration tests verify core functionality. However, several items remain before this can ship to production.

---

## Priority 1: Critical Blockers

### 1.1 Connect Browser Tools to CDP

**Current state:** All 9 tool types (`NavigateTool`, `ScreenshotTool`, `A11yTreeTool`, etc.) return `Err("not yet connected to CDP")`.

**What's needed:**
- Wire `BrowserMcpServer::dispatch()` to call actual tool `execute()` methods
- Implement CDP command execution in each tool using `ObscuraBridge`
- Add error propagation from CDP failures to tool responses

**Effort:** 2-3 days
**Blocks:** Any real browser-based auditing

### 1.2 Wire `evaluate_criterion` to Production LLM

**Current state:** `RgaaAgent::evaluate_criterion()` returns a placeholder verdict.

**What's needed:**
- Connect to `HoloClient::evaluate_multimodal()` for actual LLM calls
- Handle API errors, timeouts, and malformed responses
- Parse LLM JSON output into `CriterionResult`

**Effort:** 1-2 days
**Blocks:** Any real IA_ASSISTE evaluation

### 1.3 Fix Rate Limiter Token Refill

**Current state:** Tokens refill in bulk every 60 seconds, causing request clustering.

**What's needed:**
- Implement smooth token-per-interval refill (token bucket semantics)
- Replace 1s coarse backoff with exact time-to-next-token calculation
- Make rates configurable via `RateLimitConfig`

**Effort:** 0.5 days
**Blocks:** None (works but suboptimal)

---

## Priority 2: Important Improvements

### 2.1 Extract Shared Page Context Renderer

**Current state:** `rgaa-agent::prompts` and `rgaa-holo::prompts` have near-identical page context rendering code.

**What's needed:**
- Extract `format_page_context()` into `rgaa-holo` as a public function
- Both crates call the shared function
- Add test coverage for edge cases (missing fields, empty arrays)

**Effort:** 0.5 days
**Risk:** Maintenance drift if not fixed

### 2.2 Add Drift-Prevention Test for Criterion Definitions

**Current state:** `criteria_defs.rs` could silently diverge from `rgaa-core` catalog.

**What's needed:**
- Write a test that loads `RgaaCriteria::ia_assiste()` and verifies all 27 IDs exist in `criteria_defs`
- Run in CI to catch catalog changes

**Effort:** 0.5 days
**Risk:** Silent regressions in LLM evaluation

### 2.3 Add `#[must_use]` Annotations

**Current state:** Public constructors and important methods lack `#[must_use]`.

**What's needed:**
- Add `#[must_use]` to:
  - `HoloClient::new()`, `HoloClient::evaluate()`, `HoloClient::evaluate_multimodal()`
  - `RgaaAgent::new()`, `AgentBuilder::build()`, `create_simple_agent()`
  - `RateLimiter::new()`, `ModelRouter::new()`
  - `BrowserSession::new()`, all tool `execute()` methods

**Effort:** 0.5 days
**Risk:** Accidentally discarding rate-limited handles

### 2.4 Make `RateLimiterInner` Private

**Current state:** `RateLimiterInner` is `pub` but only used internally via `Arc`.

**What's needed:**
- Change to `pub(crate)` or private
- Verify no external code depends on it

**Effort:** 5 minutes
**Risk:** None

---

## Priority 3: Code Quality

### 3.1 Remove Dead Code in `assert_state.rs`

**Current state:** `_wrapped` is computed but unused.

**What's needed:**
- Either remove the computation or use the value
- Add `#[allow(dead_code)]` if intentionally stubbed

**Effort:** 5 minutes

### 3.2 Add Tests for Helper Methods

**Current state:** `focused_element()`, `focusable_elements()` in `AXTree` are untested. `build_with_image()` has no test.

**What's needed:**
- Add unit tests for `focused_element()` with various role combinations
- Add unit test for `build_with_image()` verifying screenshot section presence
- Add edge case tests (empty tree, no focused element, multiple focusable)

**Effort:** 1 day

### 3.3 Add Input Validation for Base64 Images

**Current state:** `evaluate_multimodal()` accepts any `&str` as image data.

**What's needed:**
- Validate base64 encoding before sending to API
- Return clear error message on invalid input
- Consider adding MIME type parameter (currently hardcoded to PNG)

**Effort:** 0.5 days

### 3.4 Reduce Rate Limiter Test Duration

**Current state:** `test_rate_limiter_enforces_throttle` takes ~60s due to refill window.

**What's needed:**
- Use higher token budget in test config (e.g., 100 tokens/s)
- Or mock the limiter for unit tests, keep slow test as integration

**Effort:** 0.5 days

---

## Priority 4: Architecture & Scale

### 4.1 Add Benchmarks

**Current state:** No benchmarks exist for hot paths.

**What's needed:**
- `criterion` benchmarks for `AxeMapper::map()`
- `criterion` benchmarks for `RgaaCriteria::all()`
- `criterion` benchmarks for prompt building
- Track allocation counts in benchmarks

**Effort:** 1 day

### 4.2 Add Integration Tests Across Crates

**Current state:** Only `rgaa-agent` has integration tests.

**What's needed:**
- Test `rgaa-orchestrator` → `rgaa-agent` → `rgaa-holo` pipeline with mocked HTTP
- Test `rgaa-browser-tools` MCP server with mock CDP
- Test `rgaa-agent` → `rgaa-core` criterion coverage

**Effort:** 2 days

### 4.3 Add Doc Comments with `# Errors` / `# Panics`

**Current state:** Public APIs lack error documentation.

**What's needed:**
- Document error conditions for all fallible methods
- Document panic conditions (if any)
- Add `# Examples` with runnable code (use `?` not `.unwrap()`)

**Effort:** 1 day

### 4.4 Consider `IndexMap` for Deterministic Output

**Current state:** `AxeMapper::map()` uses `HashMap` — iteration order non-deterministic.

**What's needed:**
- Replace with `IndexMap` for insertion-order preservation
- Ensures reproducible audit results and test stability

**Effort:** 0.5 days

---

## Priority 5: Production Hardening

### 5.1 Add Retry Logic with Exponential Backoff

**Current state:** `HoloClient` has retry logic but it's not used in agent evaluation.

**What's needed:**
- Wire retry logic into `evaluate_criterion()`
- Add jitter to prevent thundering herd
- Respect `429 Too Many Requests` with `Retry-After` header

**Effort:** 1 day

### 5.2 Add Structured Error Types

**Current state:** Many functions return `Result<_, String>`.

**What's needed:**
- Define proper error enums with `thiserror`
- Add `From<E>` implementations for `?` operator
- Preserve error chains with `#[source]`

**Effort:** 1 day

### 5.3 Add Observability

**Current state:** Some tracing exists but inconsistent.

**What's needed:**
- Add `#[tracing::instrument]` on async methods
- Add span context for LLM calls (model, criterion, duration)
- Log errors with full source chain

**Effort:** 1 day

### 5.4 Add Configuration File Support

**Current state:** API keys read from env vars, no config file.

**What's needed:**
- Support TOML/YAML config file
- CLI flags override config file
- Environment variables override everything

**Effort:** 1-2 days

---

## Recommended Execution Order

```
Week 1: Priority 1 (blockers)
  Day 1-2: Connect browser tools to CDP
  Day 3: Wire evaluate_criterion to production LLM
  Day 4: Fix rate limiter token refill
  Day 5: Priority 2 quick wins (drift test, must_use, dead code)

Week 2: Priority 3 + 4
  Day 1-2: Add integration tests across crates
  Day 3: Add benchmarks
  Day 4: Add doc comments
  Day 5: Add IndexMap, extract shared renderer

Week 3: Priority 5 (production hardening)
  Day 1-2: Add retry logic and error types
  Day 3: Add observability
  Day 4-5: Add config file support
```

---

## Success Criteria

Before shipping to production, verify:

- [ ] All 9 browser tools connect to CDP and execute real commands
- [ ] `evaluate_criterion` makes real LLM calls and parses responses
- [ ] Rate limiter uses smooth token-per-interval refill
- [ ] All public APIs have `#[must_use]` and doc comments with `# Errors`
- [ ] Integration tests cover orchestrator → agent → holo pipeline
- [ ] Benchmarks exist for hot paths
- [ ] No `String` error types in public APIs
- [ ] All `unwrap()` calls in production code replaced with `?` or `expect()`
- [ ] `cargo clippy --workspace --all-targets` clean
- [ ] `cargo test --workspace` passes in < 60s

---

## Risk Register

| Risk | Impact | Mitigation |
|------|--------|------------|
| Holo3 API changes break evaluation | High | Pin API version, add contract tests |
| Rate limiter too aggressive | Medium | Make rates configurable, add metrics |
| Browser tools fail on complex pages | Medium | Add fallback to text-only evaluation |
| Criterion definitions drift from RGAA | High | Drift-prevention test in CI |
| LLM responses malformed | High | Robust JSON parsing with fallbacks |
