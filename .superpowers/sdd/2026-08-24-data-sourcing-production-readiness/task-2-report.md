# Task 2: Implement axe-core rule fetcher

## Status: DONE

## Summary

Implemented the axe-core rule fetcher in `rgaa-data` crate. The fetcher downloads the axe-core `rule-descriptions.md` file from GitHub and parses its markdown tables into structured `AxeRule` objects.

## Changes

- **`crates/rgaa-data/src/fetch.rs`**: Expanded `AxeRule` struct with `impact`, `tags`, `help`, `help_url` fields. Replaced `todo!()` with actual HTTP fetch + parse pipeline.
- **`crates/rgaa-data/src/parse.rs`**: Implemented markdown table parser that extracts rule ID (from markdown links), description, impact, tags, and help URL from the axe-core rule descriptions page.

## Commit

- `b2b27d8` — Add axe-core rule fetcher to rgaa-data crate

## Tests

- 3 unit tests pass: `parse_simple_table`, `skips_non_table_lines`, `handle_plain_id_without_link`
- `cargo clippy` clean

## Verification

- `cargo run -p rgaa-data` fetched and saved **105 axe-core rules** to `crates/rgaa-core/data/rgaa-4.1.2/axe_rules.json`
- Output validated as correct JSON with all expected fields populated

## Deviations from Brief

- Parser uses `contains("---")` to skip separator rows (handles both `|---` and `| :---` styles)
- Extracts `help_url` from markdown links in Rule ID column (brief left it empty)
- Sets `help` = description (brief left it empty)
- Added 3 unit tests not in brief

## Concerns

- Repository has corrupted git objects (empty files in `.git/objects/`) — unrelated to this task, pre-existing issue
