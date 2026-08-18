# Task 4: Playwright Child Process Bridge

## Status
✅ Complete

## Files Created/Modified
- `crates/rgaa-browser/Cargo.toml` — Updated with dependencies (rgaa-core, rgaa-rules, tokio, serde, serde_json, tracing, anyhow)
- `crates/rgaa-browser/src/playwright.rs` — PlaywrightBridge struct with async methods
- `crates/rgaa-browser/src/js/interaction.js` — Ported from interaction-audit.js (703→444 lines)
- `crates/rgaa-browser/src/lib.rs` — Re-exports playwright module

## Architecture
Hybrid approach:
- **Embedded scripts**: axe-core, gap-fix, and page-context scripts are inline in Rust methods
- **Separate file**: interaction.js kept as separate file, read at runtime via `std::fs::read_to_string`

## Methods Implemented
| Method | Description |
|--------|-------------|
| `new()` | Constructor |
| `run_axe(url)` | Launches Playwright, runs axe-core, returns violations JSON |
| `run_gap_fix(url, snippets)` | Runs each gap-fix snippet on the page |
| `run_interaction(url)` | Runs interaction tests from interaction.js |
| `extract_page_context(url)` | Extracts PageContext for Holo3 prompts |
| `run_node_script(script)` | Spawns `node -e <script>`, captures stdout |

## Test Summary
- `cargo check -p rgaa-browser` — ✅ Passed (warnings only)
- No unit tests (async require #[tokio::test], added to backlog)

## Concerns
1. **Node.js dependency**: Requires Node.js and Playwright installed at runtime
2. **No timeout handling**: Long-running scripts could hang; consider adding timeout to `run_node_script`
3. **Script injection**: URL and snippets are interpolated into JS strings; needs escaping for special characters
4. **No stderr capture in results**: Errors from Node are returned as Err but not logged structurally
