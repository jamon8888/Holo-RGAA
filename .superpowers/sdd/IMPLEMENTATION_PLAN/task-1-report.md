# M1 Implementation Report — Stop the Bleeding

**Status:** DONE_WITH_CONCERNS

## What Changed

### M1.1 — `calculate_compliance` formula fix
- **File:** `crates/rgaa-orchestrator/src/pipeline.rs:18-33`
- Changed formula from `C / (total - NA)` to `C / (C + NC)` where NC = Fail + Error
- Removed unused `total` parameter
- Updated call site to not pass `total`
- Updated test assertion: `needs_review_is_not_excluded_from_compliance_denominator` now expects 100% (Pass only, NeedsReview excluded from denominator)

### M1.2 — audit_one fallback → NotTested
- **File:** `crates/rgaa-orchestrator/src/pipeline.rs:173,177`
- Changed default status from `CriterionStatus::Pass` to `CriterionStatus::NotTested`
- Changed justification to `"Not tested — no automated check covered this criterion"`

### M1.3 — gap-fix script return value fix
- **File:** `crates/rgaa-obscura/src/lib.rs:1161-1174`
- Changed `build_gap_fix_script` to capture and return the snippet's actual result (`const r = {snippet}; return JSON.stringify(r)`) instead of discarding it and returning `{ success: true }`
- Error fallback now returns `{ pass: false, details: e.message, nodes: 0 }` matching `GapFixRules::parse_results` expectations

### M1.4 — HoloClient::new → Result
- **File:** `crates/rgaa-holo/Cargo.toml` — added `rgaa-core = { path = "../rgaa-core" }`
- **File:** `crates/rgaa-holo/src/client.rs:60-77`
- Added `use rgaa_core::RgaaError;`
- Changed `new(api_key: String) -> Self` to `new(api_key: String) -> Result<Self, RgaaError>`
- Replaced `.expect()` with `.map_err(|e| RgaaError::Holo3(e.to_string()))?`
- Removed `#[must_use]` attribute
- Updated doc comment from `# Panics` to `# Errors`
- Updated all callers: 5 test sites in `client.rs` and 2 calls in `rgaa-agent/src/models.rs:74-76`

### M1.5 — HoloClient::evaluate → Result<HoloResponse, RgaaError>
- **File:** `crates/rgaa-holo/src/client.rs`
- Changed `evaluate`, `evaluate_multimodal`, `evaluate_with_messages` return types from `Result<_, String>` to `Result<_, RgaaError>`
- Changed final error in `evaluate_with_messages` from `Err(format!(...))` to `Err(RgaaError::Holo3(format!(...)))`
- Changed base64 validation error to use `RgaaError::Holo3`
- Fixed test assertion: `result.unwrap_err().contains("base64")` → `result.unwrap_err().to_string().contains("base64")`

### M1.6 — prompts build_for_criterion simplified
- **File:** `crates/rgaa-holo/src/prompts.rs:293-335`
- Replaced broken `build_for_criterion` (which split on `-` instead of `.` and mapped single integers to nonsense groups) with a simple delegation: `Self::build(criterion_id, context)`
- Deleted `get_base_criterion` and `get_criterion_focus` methods entirely (42 lines removed)

### M1.7 — axe_mapper::map → Result
- **File:** `crates/rgaa-rules/src/axe_mapper.rs`
- Added `use rgaa_core::RgaaError;`
- Changed return type from `HashMap<String, CriterionResult>` to `Result<HashMap<String, CriterionResult>, RgaaError>`
- Replaced `unwrap_or_default()` with `map_err(|e| RgaaError::AxeCore(...))?`
- Changed return from `results` to `Ok(results)`
- Updated callers:
  - `crates/rgaa-orchestrator/src/pipeline.rs:101` — added `.map_err(|e| e.to_string())?`
  - `crates/rgaa-obscura/src/lib.rs:96` — added `.map_err(|error| ObscuraError::Evaluation(error.to_string()))?`

## Test Results

```
rgaa-core:      15/15 passed
rgaa-holo:      9/9 client tests passed (7 prompts tests: 5 passed, 2 pre-existing failures)
rgaa-obscura:   13/13 passed (excluding 1 pre-existing network-dependent test)
rgaa-rules:     compiled OK (no test module)
rgaa-orchestrator: BLOCKED — pre-existing borrow checker error prevents compilation
```

### Pre-existing failures (NOT introduced by this work):
1. **`rgaa-orchestrator` compilation** — `E0597: session does not live long enough` at `pipeline.rs:95`. The `MutexGuard` from `tool_ctx.session().lock().await` is dropped before the cloned `bridge` is used. This error exists on the base branch before any M1 changes.
2. **`prompts::tests::test_build_prompt`** — assertion expects `"H1: Titre principal"` but `format_page_context` wraps text in `<<<UNTRUSTED PAGE CONTENT>>>` delimiters. Pre-existing.
3. **`prompts::tests::test_format_page_context`** — same delimiter issue. Pre-existing.
4. **`tests::test_run_axe_with_broken_script_surfaces_error`** — requires `obscura` binary in PATH. Pre-existing.

## Concerns & Deviations

1. **orchestrator can't compile**: The pre-existing `E0597` borrow error at `pipeline.rs:94-96` means the orchestrator crate cannot be built or tested. This blocks verification of M1.1 and M1.2 changes in context. The fix is straightforward (hold the lock guard in the block, or restructure) but was out of scope for M1.

2. **`Result<_, String>` ecosystem**: The orchestrator uses `String` as its error type throughout, while we now return `RgaaError` from `AxeMapper::map`. This required `.map_err(|e| e.to_string())` at the call site. The orchestrator should ideally be migrated to use `RgaaError` (or a crate-level error type) in Phase 2.

3. **`build_gap_fix_script` change**: The snippet now returns `{ pass, details, nodes }` — this is correct per `GapFixRules::parse_results` expectations. However, we should verify that all existing snippets in `GapFixRules::snippets()` actually return this shape (Phase 3 concern).

4. **`build_for_criterion` simplification**: The extra group/focus note logic was removed entirely. Per the controller's ruling, this is intentional — the broken logic (splitting on `-` instead of `.`) was worse than no special logic. The catalog-based Phase 3 approach will provide proper criterion grouping.
