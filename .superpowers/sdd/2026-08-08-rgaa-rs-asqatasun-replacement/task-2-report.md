# Task 2 Report: rgaa-rules — axe-core Mapping + Gap-Fix Rules

**Status:** ✅ Completed  
**Date:** 2026-08-08  
**Crate:** `rgaa-rules`

## Summary

Created the `rgaa-rules` crate that provides axe-core mapping and gap-fix rules for the 10 known false negatives identified in the comparison data.

## Files Created

1. **`rgaa-rs/crates/rgaa-rules/Cargo.toml`** — Package configuration with dependencies on rgaa-core, serde, serde_json
2. **`rgaa-rs/crates/rgaa-rules/src/axe_mapper.rs`** — AxeMapper struct mapping axe-core violations to RGAA criteria
3. **`rgaa-rs/crates/rgaa-rules/src/gap_fix.rs`** — GapFixRules struct with JS snippets for 10 gap-fix criteria
4. **`rgaa-rs/crates/rgaa-rules/src/lib.rs`** — Re-exports AxeMapper and GapFixRules

## Implementation Details

### axe_mapper.rs
- **AxeMapper::map(violations_json: &str) -> HashMap<String, CriterionResult>**
  - Parses axe-core violations JSON
  - Maps violations to 77 RGAA criteria using the mapping from `poc.js`
  - Initializes all mapped criteria as PASS, then marks as FAIL if violations found
  - Returns HashMap of criterion_id → CriterionResult

- **rgaa_to_axe_map()**: 77 criteria mapped from RGAA to axe-core rule IDs

### gap_fix.rs
- **GapFixRules::snippets() -> HashMap<String, &str>**
  - Returns 10 JavaScript snippets for gap-fix criteria (1.1, 1.2, 2.1, 3.2, 6.1, 8.3, 8.5, 11.1, 11.4, 12.7)
  - Each snippet returns JSON: `{ "pass": bool, "details": string, "nodes": number }`
  - Targets the 10 real false negatives from comparison data

- **GapFixRules::parse_results(js_results: &HashMap<String, Value>) -> HashMap<String, CriterionResult>**
  - Parses JS execution results into CriterionResults
  - Maps pass/fail status and creates violations for failures

## Verification

```bash
cargo check -p rgaa-rules
# Compiled successfully with no errors
```

## Concerns

None. The implementation follows the plan exactly and compiles successfully.

## Commit

Pending: "feat: axe-core mapping + 10 gap-fix rules for real false negatives"