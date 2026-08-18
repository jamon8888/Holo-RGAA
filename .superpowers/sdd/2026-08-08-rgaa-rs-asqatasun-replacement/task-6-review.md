# Task 6 Review: rgaa-orchestrator — Main Audit Pipeline

## Verdict: Spec ❌ | Quality ❌

---

## Spec Compliance

### Pipeline Flow ✅
The 7-step pipeline correctly implements the spec order:
1. axe-core via PlaywrightBridge → `AxeMapper::map()`
2. Gap-fix rules → `GapFixRules::snippets()` + `parse_results()`
3. Page context extraction → `bridge.extract_page_context()`
4. Holo3 IA_ASSISTE → loop over `RgaaCriteria::ia_assiste()` (27 criteria)
5. Merge results → `HashMap::extend()`
6. MANUEL criteria → insert 7.5 as `Na`
7. Compliance calculation → pass/(total-na)*100

### Criterion Coverage ✅
- 77 deterministic: handled by AxeMapper (initialized as PASS, downgraded on violations)
- 27 IA_ASSISTE: handled by Holo3 loop
- 1 MANUEL (7.5): inserted as `CriterionStatus::Na`

### Crate Dependencies ✅
All 5 required crates are declared: `rgaa-core`, `rgaa-rules`, `rgaa-holo`, `rgaa-browser`, `rgaa-storage`.

### ❌ Fails: Holo3 Verdict Mapping (Critical)
**Pipeline** (`pipeline.rs:56-59`) matches on `"CONFORME"` / `"NON_CONFORME"`.
**HoloClient tests** (`client.rs:203-228`) use verdicts `"pass"`, `"fail"`, `"na"`.

The `HoloResponse.verdict` field is a free-form `String` — the LLM may return either pattern, but the prompt (`SYSTEM_PROMPT` in client.rs) does not specify which format to use. The `_ => CriterionStatus::Na` fallback means any `"pass"`/`"fail"` verdict maps to Na, silently discarding valid LLM results. **All 27 IA_ASSISTE criteria will likely be Na.**

Fix: match on both patterns, or normalize the verdict in `HoloClient`.

---

## Quality Issues

### ❌ Hardcoded API Key (`pipeline.rs:17`)
```rust
.unwrap_or_else(|_| "hk-a73b030c64aac335fc3651c280c95694beb8df95c4a5d8b1".into());
```
A fallback API key in source code is a security risk. Should fail if `HOLO3_API_KEY` is unset, or use a dedicated config mechanism.

### ❌ `RgaaCriteria::all()` Called Repeatedly Without Caching (`pipeline.rs:95,117`)
`RgaaCriteria::all()` allocates 106 `Criterion` structs each call. The pipeline calls it 3 times:
- `RgaaCriteria::ia_assiste()` (line 49) → calls `all()` internally
- `RgaaCriteria::all()` (line 95) → MANUEL iteration
- `RgaaCriteria::count()` (line 117) → calls `all()` internally

That's 3 × 106 allocations per audit. Use `OnceLock<Vec<Criterion>>` or call `all()` once and derive everything from it.

### ⚠️ `HashMap` for Result Merging — Non-Deterministic Order (`pipeline.rs:89`)
Results are merged into a `HashMap<String, CriterionResult>`, then collected into a `Vec`. Iteration order of `HashMap` is non-deterministic, making audit output non-reproducible. Use `IndexMap` or `BTreeMap` for deterministic ordering.

### ⚠️ `Result<AuditResult, String>` Error Type (`pipeline.rs:11`)
The pipeline returns `Result<_, String>` instead of `Result<_, RgaaError>`. The `RgaaError` enum exists with variants for Browser, AxeCore, Holo3, etc. Using `String` errors loses structured error information and prevents callers from matching on error types.

### ⚠️ `_config` Parameter Unused (`pipeline.rs:11`)
`_config: &CrawlConfig` is accepted but ignored. While the report acknowledges this (crawl deferred), the prefix `_` hides the fact that no validation or stub behavior exists. Consider `todo!()` or a log warning if config fields are non-default.

### ⚠️ No Unit Tests
The report states "N/A (orchestrator is integration-focused)" but there are testable units:
- Compliance calculation logic (pass/fail/na/total)
- MANUEL criteria insertion
- Result merging behavior
- Verdict-to-status mapping

At minimum, test the compliance calculation and verdict mapping.

### ⚠️ `rgaa-storage` Dependency Declared But Unused
`rgaa-storage` is in `Cargo.toml` but not imported or used anywhere in the pipeline. Dead dependency increases compile time.

### ⚠️ `error_count` Computed But Not in `AuditResult` (`pipeline.rs:116,153`)
`error_count` is counted and logged but not stored in the `AuditResult` struct. The `AuditResult` type has no `errors` field. Either add it to the type or remove the misleading variable.

---

## Summary

| Category | Verdict | Key Issue |
|----------|---------|-----------|
| Pipeline flow | ✅ | Correct 7-step order |
| Criterion coverage | ✅ | 77 + 27 + 1 = 106 |
| Holo3 verdict mapping | ❌ | "pass"/"fail" never matched → all 27 IA_ASSISTE = Na |
| API key handling | ❌ | Hardcoded fallback in source |
| Error type | ❌ | `String` instead of `RgaaError` |
| Memory allocation | ❌ | 3× `all()` = 308 allocations per audit |
| Determinism | ❌ | `HashMap` ordering |
| Test coverage | ❌ | Zero tests for testable logic |

**Spec: ❌** — Holo3 verdict mapping bug means 27 IA_ASSISTE criteria silently fail.

**Quality: ❌** — Hardcoded secret, wrong error type, excessive allocations, non-deterministic output, no tests.
