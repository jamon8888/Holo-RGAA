# Task 6 Fix Report: Holo3 Verdict Mapping

**Date:** 2026-08-08
**File:** `rgaa-rs/crates/rgaa-orchestrator/src/pipeline.rs:56-59`
**Commit:** pending (fix: Holo3 verdict mapping)

## Bug

Pipeline matched Holo3 verdicts as `"CONFORME"`/`"NON_CONFORME"` but the `HoloClient` SYSTEM_PROMPT (`client.rs:14`) instructs the LLM to return `"pass"`, `"fail"`, or `"na"`. Tests in `client.rs:203-229` confirmed the actual values are lowercase.

**Impact:** All 27 IA_ASSISTE criteria silently mapped to `CriterionStatus::Na`, producing a false compliance rate.

## Fix

Changed `pipeline.rs:56-59` from:

```rust
let status = match response.verdict.as_str() {
    "CONFORME" => CriterionStatus::Pass,
    "NON_CONFORME" => CriterionStatus::Fail,
    _ => CriterionStatus::Na,
};
```

To:

```rust
let status = match response.verdict.to_lowercase().as_str() {
    "pass" | "conforme" => CriterionStatus::Pass,
    "fail" | "non_conforme" => CriterionStatus::Fail,
    _ => CriterionStatus::Na,
};
```

## Verification

- `cargo check -p rgaa-orchestrator` — passes (clean, no warnings)
- Case-insensitive matching handles both English (`pass`/`fail`) and French (`conforme`/`non_conforme`) variants

## Root Cause

Mismatch between LLM system prompt (English verdicts) and pipeline matcher (French verdicts only). The LLM was returning the correct values but the pipeline never recognized them.
