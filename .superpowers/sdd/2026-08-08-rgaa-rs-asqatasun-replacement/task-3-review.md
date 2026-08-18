# Task 3 Review: Holo3 Client with Retry + JSON Extraction

**Reviewer:** opencode
**Date:** 2026-08-08

## Spec ✅

All global constraints met:

| Requirement | Status | Evidence |
|---|---|---|
| POST https://api.hcompany.ai/v1/chat/completions | ✅ | `client.rs:6` |
| Model `holo3-1-35b-a3b` | ✅ | `client.rs:7` |
| 5 retry attempts | ✅ | `MAX_RETRIES = 5`, `client.rs:8` |
| Exponential backoff on 429 | ✅ | `1s → 2s → 4s → 8s → 16s`, `client.rs:131` |
| JSON: direct parse → code blocks → regex | ✅ | `extract_json()` chain, `client.rs:167-184` |
| PageContext types for prompt building | ✅ | `prompts.rs:3-67` |

## Quality ❌

### Bugs

**1. `build_for_criterion` logic error** (`prompts.rs:199-213`)

```rust
pub fn build_for_criterion(criterion_id: &str, context: &PageContext) -> String {
    let prefix = criterion_id.split('-').next().unwrap_or(criterion_id);
    let base_criterion = Self::get_base_criterion(prefix);

    if base_criterion != criterion_id {  // BUG: comparing "1" != "1.1" always true
```

`get_base_criterion` takes the prefix (e.g. `"1"`) and returns `"1"`. Then it compares `"1" != "1.1"` — always true for sub-criteria. The intent was likely to check if the criterion is a sub-criterion of a group, but this logic makes `build_for_criterion` always append the group note, even for base criteria.

**2. Dead code** (`client.rs:42-56`)

`ChatResponse`, `Choice`, `MessageContent` are defined but never constructed — the API response is never deserialized into these types. This is dead code.

### Unused Dependencies

- `anyhow` in Cargo.toml — never imported
- `rgaa-core` in Cargo.toml — never imported

### Missing Edge Case Tests

- No test for retry exhaustion (all 5 attempts fail)
- No test for 429 backoff timing
- No test for API error responses
- `extract_from_code_block` doesn't handle ```` ```json ```` with a leading space

### Minor Issues

- `extract_with_regex` pattern assumes flat JSON — won't match if `justification` contains nested objects
- `expect("Failed to create HTTP client")` on `Client::builder().build()` could panic in production — should return `Result`

## Verdict

| Category | Verdict |
|---|---|
| Spec Compliance | ✅ Pass |
| Code Quality | ❌ Fail |

The `build_for_criterion` logic bug and dead code prevent a clean pass. 8/8 tests pass, but tests don't cover the buggy path.
