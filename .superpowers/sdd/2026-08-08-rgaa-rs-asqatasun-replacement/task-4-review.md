# Task 4 Review: Playwright Child Process Bridge

## Verdict

**Spec:** ❌ **Quality:** ❌

---

## Spec Compliance

### Files

| Expected | Status | Notes |
|----------|--------|-------|
| `Cargo.toml` | ✅ | Dependencies match spec |
| `src/lib.rs` | ⚠️ | Missing `pub use playwright::PlaywrightBridge;` re-export |
| `src/playwright.rs` | ✅ | Created |
| `src/js/axe-runner.js` | ❌ | Not created — spec requires separate file |
| `src/js/gap-fix-runner.js` | ❌ | Not created — spec requires separate file |
| `src/js/interaction.js` | ✅ | Created (552 lines vs spec's ~100) |

### Struct Architecture

- **Spec:** `PlaywrightBridge { js_dir: String }` with `js_dir` set in constructor
- **Actual:** `PlaywrightBridge;` (unit struct, no field)
- **Impact:** `run_interaction` uses `env!("CARGO_MANIFEST_DIR")` at compile time instead of `self.js_dir` at runtime. Functionally equivalent but deviates from spec.

### Method Signatures

| Method | Spec | Actual | Match |
|--------|------|--------|-------|
| `run_axe(url)` | `Result<String, String>` | `Result<String, String>` | ✅ |
| `run_gap_fix(url, snippets)` | `Result<HashMap<String, Value>, String>` | `Result<HashMap<String, Value>, String>` | ✅ |
| `run_interaction(url)` | `Result<HashMap<String, Value>, String>` | `Result<HashMap<String, Value>, String>` | ✅ |
| `extract_page_context(url)` | `Result<serde_json::Value, String>` | `Result<serde_json::Value, String>` | ✅ |
| `run_node_script(script)` | `Result<String, String>` | `Result<String, String>` | ✅ |

### interaction.js

- **Spec:** `runInteractionTests(page)` — 1 parameter
- **Actual:** `runInteractionTests(page, url)` — 2 parameters
- **Impact:** `playwright.rs` passes `url` as second arg. Signature mismatch with spec.
- **Criteria coverage:** Spec lists 10.7, 12.8, 12.9, 10.11. Actual covers 10.7, 12.8, 12.9, 12.11, 9.3, 10.11, 10.12, 11.x. More comprehensive — acceptable deviation.

### lib.rs Re-export

Spec requires `pub use playwright::PlaywrightBridge;`. Only `pub mod playwright;` is present. Downstream crates must use `rgaa_browser::playwright::PlaywrightBridge` instead of `rgaa_browser::PlaywrightBridge`.

---

## Quality Issues

### Critical

1. **`run_axe` calls `axe.run()` twice** (`playwright.rs:22-29`) — First call gets rules (discarded), second gets violations. Should call once and reuse result.

2. **`run_gap_fix` launches new Chromium per snippet** (`playwright.rs:44-70`) — Each of 10 snippets spawns a full browser. Should reuse a single browser instance across snippets.

3. **URL interpolation without escaping** (`playwright.rs:20,51,88,112`) — URLs with single quotes or backticks break the JS. Needs proper escaping or `JSON.stringify()`.

### Medium

4. **`std::fs::read_to_string` in async fn** (`playwright.rs:81`) — Blocks the tokio thread. Use `tokio::fs::read_to_string`.

5. **No timeout on `run_node_script`** (`playwright.rs:135-154`) — `Command::output()` waits indefinitely. Add `tokio::time::timeout`.

6. **`Result<_, String>` error type** — Inconsistent with `RgaaError` from rgaa-core. Other crates use `thiserror`. Consider `Result<T, RgaaError>` with `From` impls.

7. **`run_gap_fix` error handling diverges from spec** — Spec silently ignores errors and continues. Actual propagates with `?`, failing the entire operation on first snippet error.

### Low

8. **No unit tests** — Report acknowledges this. At minimum, test `run_node_script` with a simple `console.log("ok")` script.

9. **`info!("Running Node.js script")` on every call** (`playwright.rs:136`) — Too noisy for 10+ invocations. Use `debug!` or log only on first call.

10. **`HashMap` for results** — Iteration order non-deterministic. Use `IndexMap` for reproducible output (per AGENTS.md).

---

## Positive Notes

- Clean architecture: `run_node_script` is a good abstraction for the child process pattern
- `interaction.js` is significantly more thorough than the spec version (552 lines vs ~100), covering 9.3, 10.12, 11.x, 12.11
- Proper error logging with `warn!` on stderr in `run_node_script`
- `extract_page_context` returns structured data matching `PageContext` shape from rgaa-holo

---

## Required Fixes

1. Add `pub use playwright::PlaywrightBridge;` to `lib.rs`
2. Fix `run_axe` to call `axe.run()` once
3. Refactor `run_gap_fix` to reuse browser across snippets
4. Escape URL interpolation in JS templates
5. Replace `std::fs::read_to_string` with `tokio::fs::read_to_string`

## Recommended Fixes

6. Add timeout to `run_node_script`
7. Switch to `RgaaError` error type
8. Add unit tests
9. Use `IndexMap` for deterministic output
