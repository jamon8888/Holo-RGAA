# Task 3 Report: Build axe-core → RGAA mapping validator

**Status:** DONE

## What was implemented

Created the axe-core → RGAA mapping validator in `rgaa-data` crate:

- **`validate.rs`**: `validate_mapping()` function that cross-references 77 hardcoded axe-core rule IDs against fetched axe-core rules, producing `MappingEntry` structs with `Provenance` metadata (source, validator, timestamp, notes)
- **`parse.rs`**: Added `load_existing_mapping()` returning all 77 criterion-to-axe-rule mappings as `HashMap<String, Vec<String>>`
- **`main.rs`**: Wired validation into the pipeline — fetches axe-core rules, validates mapping, writes `axe_mapping.json`

## Output

- `crates/rgaa-core/data/rgaa-4.1.2/axe_mapping.json` — 77 entries with provenance
- 23 entries have all rules valid against axe-core 4.9.1
- 54 entries have at least one invalid rule (legacy axe rule IDs no longer present in current axe-core), correctly flagged in `provenance.notes`

## Tests

- 6 tests pass (3 parse, 3 validate)
- Clippy clean

## Commits

- `f533818` — Add axe-core mapping validator with provenance tracking

## Concerns

The 54 entries with invalid rules are expected — the original axe-core rule IDs from the legacy poc.js include many rules that were renamed or removed in axe-core 4.9.1 (e.g., `longdesc`, `deprecated-element`, `fieldset`, `keyboard-trap`, `three-flashes`). This is exactly what the validator is designed to surface. The valid rules are kept in the `axe_rules` field, invalid ones are only noted in `provenance.notes`.
