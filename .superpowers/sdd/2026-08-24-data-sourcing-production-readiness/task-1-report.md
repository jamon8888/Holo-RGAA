# Task 1: Create `rgaa-data` build crate scaffold

## Status: DONE

## Files Created
- `crates/rgaa-data/Cargo.toml` — binary crate with reqwest, tokio, serde, anyhow, tracing dependencies
- `crates/rgaa-data/src/main.rs` — async main with tracing, fetches axe-core rules, writes JSON to `crates/rgaa-core/data/rgaa-4.1.2/`
- `crates/rgaa-data/src/fetch.rs` — stub with `AxeRule` struct and `axe_core_rules()` function (todo! placeholder)
- `crates/rgaa-data/src/parse.rs` — stub module
- `crates/rgaa-data/src/validate.rs` — stub module

## Files Modified
- `Cargo.toml` — added `"crates/rgaa-data"` to workspace members

## Compilation
`cargo check -p rgaa-data` passes cleanly.

## Commit
`17843da` — "Add rgaa-data build crate scaffold for automated data sourcing"

## Notes
- Added `serde::Serialize` derive to `AxeRule` (not in task brief but required for `serde_json::to_string_pretty` to compile).
- `fetch::axe_core_rules()` is a `todo!()` — implementation comes in later tasks.
- Repo has pre-existing corrupt git objects (empty pack entries). Commit succeeded despite this.
