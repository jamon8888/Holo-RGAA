# Task 4 Fix Report

## Status: ✅ All 4 issues fixed

## Changes Made

### Issue 1: Missing pub use re-export
- **File:** `rgaa-rs/crates/rgaa-browser/src/lib.rs`
- **Change:** Added `pub use playwright::PlaywrightBridge;`
- **Reason:** Re-export for ergonomic access

### Issue 2: URL injection vulnerability
- **File:** `rgaa-rs/crates/rgaa-browser/src/playwright.rs`
- **Change:** Added `escape_js_string()` helper that escapes `\`, `'`, `"`, `\n`, `\r`
- **Applied to:** `run_axe`, `run_gap_fix`, `run_interaction`, `extract_page_context`
- **Reason:** Prevents JS injection via malformed URLs

### Issue 3: std::fs blocks async runtime
- **File:** `rgaa-rs/crates/rgaa-browser/src/playwright.rs`
- **Change:** Replaced `std::fs::read_to_string` with `tokio::fs::read_to_string(...).await` in `run_interaction`
- **Reason:** Prevents blocking the async runtime

### Issue 4: No timeout on child process
- **File:** `rgaa-rs/crates/rgaa-browser/src/playwright.rs`
- **Change:** Wrapped `Command::output()` in `tokio::time::timeout(Duration::from_secs(60), ...)`
- **Added imports:** `std::time::Duration`, `tokio::time::timeout`
- **Reason:** Prevents indefinite hang on stuck Node.js processes

## Verification

```bash
$ cargo check -p rgaa-browser
Finished `dev` profile [unoptimized + debuginfo] target(s) in 3.72s
```

No errors or warnings.

## Commits

```
fix: URL escaping, async file read, process timeout, re-export
```
