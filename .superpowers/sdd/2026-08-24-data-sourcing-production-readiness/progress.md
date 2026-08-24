# SDD ledger — plan: docs/superpowers/plans/2026-08-24-data-sourcing-production-readiness.md

Branch: feat/rig-agentic-loop
BASE: 3865542 (Add implementation plan)

## Plan Scan

| Tasks | Shared File/Interface | Producer → Consumer | Finding | Ruling |
|-------|----------------------|---------------------|---------|--------|
| T1→T2 | `rgaa-data/src/main.rs`, `rgaa-data/src/fetch.rs` | T1 creates scaffold → T2 adds fetch module | Clean — T2 creates new files, T1's main.rs skeleton is extended | — |
| T1→T3 | `rgaa-data/Cargo.toml` | T1 creates → T3 modifies (adds chrono) | Plan says "no new deps" but T3 adds chrono — already fixed in plan (hardcoded date) | OK — no chrono needed |
| T2→T3 | `Vec<AxeRule>` type | T2 produces → T3 consumes | Clean — type defined in fetch.rs, used in validate.rs | — |
| T3→T4 | `rgaa-data/src/parse.rs` | T3 adds `load_existing_mapping()` → T4 adds `parse_criteres_automatability()` | Clean — both add to same file but different functions | — |
| T5 standalone | `criteria_defs.rs` | Independent of Phase 1 | Clean — no dependencies on rgaa-data | — |
| T6→T7-T11 | `ToolContext`, `BrowserSession` | T6 implements NavigateTool → T7-T11 implement other tools | Clean — all tools use same context, no conflicts | — |
| T12→T13 | `tests/corpus/` | T12 creates structure → T13 expands | Clean — T13 adds more files to same dirs | — |

## Tasks

Task 1: complete (commits 3865542..17843da, review clean)
- Important: not using workspace = true for deps (defer to later tasks)
- Important: tracing-subscriber missing env-filter feature (defer)

Task 2: complete (commits 17843da..b2b27d8, review clean)
- Minor: unnecessary clone in parse.rs (deferred)
- Minor: fragile link extraction (acceptable given axe-core formatting)

Task 3: complete (commits b2b27d8..f533818, review clean)
- Important: 54/77 axe rule IDs are legacy (data quality issue, not code issue)
- Important: hardcoded date in provenance (deferred)
- Important: Result return type on infallible function (deferred)

