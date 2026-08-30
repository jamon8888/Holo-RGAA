# Production Readiness Plan

**Goal:** Make Holo-RGAA production-ready for real accessibility audits.

## Current State

- M3 fidelity work complete (T1-T6)
- Data sourcing complete (T1-T13 from prior plan)
- Workspace: `feat/rig-agentic-loop`
- BASE: `7fd3ba6`

## Critical Gaps (Must Fix)

### Task 1: Fix PageContext Extraction
**Problem:** `iframes` and `media` fields missing from extraction → criteria 2.1, 4.x always NA.

**Files to modify:**
- `rgaa-obscura/src/bridge.rs` — add `extract_page_context` to include iframes and media
- `rgaa-holo/src/lib.rs` — update `PageContext` struct with `iframes` and `media` fields

**Steps:**
1. Update `PageContext` struct in rgaa-holo to include `iframes: Vec<Iframe>` and `media: Vec<Media>`
2. Update `bridge.extract_page_context` to extract these from the page
3. Update NA detection to use the actual fields instead of always NA
4. Add tests

**Test:** `cargo test -p rgaa-core -p rgaa-holo`

---

### Task 2: Implement Integration Test
**Problem:** Never tested end-to-end on a real page.

**Files to create:**
- `rgaa-orchestrator/tests/integration_test.rs`

**Steps:**
1. Create integration test that runs against a test HTML page
2. Use `mockito` or a local file server for testing
3. Verify the full pipeline runs: axe-core → gap-fix → NA detection → compliance calc
4. Verify AuditResult has correct fields populated

**Test:** `cargo test -p rgaa-orchestrator --test integration_test`

---

### Task 3: Wire Holo3 for PartiallyAutomatable
**Problem:** Agent only evaluates IA_ASSISTE criteria, not the human-review portion of PartiallyAutomatable.

**Files to modify:**
- `rgaa-orchestrator/src/pipeline.rs` — modify step 4 to also handle PartiallyAutomatable criteria
- `rgaa-agent/src/agent.rs` — add method for PartiallyAutomatable evaluation

**Steps:**
1. Identify PartiallyAutomatable criteria that need human review
2. Add prompt for human-review portion
3. Wire into pipeline alongside IA_ASSISTE evaluation

**Test:** `cargo test -p rgaa-orchestrator`

---

## Important Gaps (Should Fix)

### Task 4: Implement Storage Layer
**Problem:** Results only in-memory, can't persist audits.

**Files to create/modify:**
- `rgaa-storage/src/lib.rs` — implement `Storage` trait
- `rgaa-storage/src/postgres.rs` — PostgreSQL implementation
- `rgaa-orchestrator/src/pipeline.rs` — save results to storage

**Steps:**
1. Define `Storage` trait with `save_audit`, `get_audit`, `list_audits`
2. Implement PostgreSQL storage using `sqlx`
3. Add connection pooling
4. Wire into `Orchestrator::run_batch` to save results

**Test:** `cargo test -p rgaa-storage`

---

### Task 5: Implement MCP Server Tools
**Problem:** `rgaa-mcp` has scaffold but no actual tools.

**Files to modify:**
- `rgaa-mcp/src/tools/*.rs` — implement actual tools
- `rgaa-mcp/src/lib.rs` — register tools

**Steps:**
1. Implement `audit_url` tool — runs full audit
2. Implement `get_audit_result` tool — retrieves saved audit
3. Implement `list_criteria` tool — lists RGAA criteria
4. Add proper error handling and response formatting

**Test:** `cargo test -p rgaa-mcp`

---

### Task 6: Implement HTTP API
**Problem:** `rgaa-api` is empty scaffold.

**Files to modify:**
- `rgaa-api/src/lib.rs` — implement API routes
- `rgaa-api/src/routes.rs` — add routes

**Steps:**
1. Add `POST /audit` — run audit on URL
2. Add `GET /audit/{id}` — get audit result
3. Add `GET /criteria` — list criteria
4. Add `GET /health` — health check

**Test:** `cargo test -p rgaa-api`

---

### Task 7: Error Handling & Retry Logic
**Problem:** LLM calls fail fast, no resilience.

**Files to modify:**
- `rgaa-holo/src/client.rs` — add retry with backoff
- `rgaa-agent/src/agent.rs` — add circuit breaker

**Steps:**
1. Add exponential backoff retry (5 attempts, 429 handling)
2. Add circuit breaker for consecutive failures
3. Return structured errors instead of `String`

**Test:** `cargo test -p rgaa-holo`

---

## Minor Gaps (Nice to Have)

### Task 8: Rate Limiting on LLM
**Problem:** Unbounded LLM calls, cost risk.

**Files to modify:**
- `rgaa-agent/src/ratelimit.rs` — implement rate limiter

---

### Task 9: Audit Log Persistence
**Problem:** No audit trail.

**Files to modify:**
- `rgaa-storage/` — add audit log table

---

### Task 10: HTML Report Generation
**Problem:** Only JSON output, stakeholders can't view.

**Files to create:**
- `rgaa-cli/src/report.rs` — add HTML report generation

---

### Task 11: User-Friendly CLI
**Problem:** Developer-focused, not user-friendly.

**Files to modify:**
- `rgaa-cli/src/commands/` — improve help text, add examples

---

## Global Constraints

- No new crates — only modify existing ones
- No new dependencies without justification
- Must pass `cargo check --workspace`
- Must have tests for new code
- Keep backward compatibility with existing API

## Test Commands

After each task:
```bash
cargo check --workspace
cargo test -p <affected-crate>
```

Final:
```bash
cargo clippy --workspace --all-targets
cargo test --workspace
```
