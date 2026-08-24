# Task 3: Build axe-core → RGAA mapping validator

**Files:**
- Create: `rgaa-rs/crates/rgaa-data/src/validate.rs`
- Modify: `rgaa-rs/crates/rgaa-data/src/main.rs`
- Modify: `rgaa-rs/crates/rgaa-data/src/parse.rs` (add `load_existing_mapping()`)

**Interfaces:**
- Consumes: `Vec<AxeRule>` from Task 2 (already implemented in `fetch.rs`)
- Consumes: `criteres.json` (already vendored at `rgaa-core/data/rgaa-4.1.2/criteres.json`)
- Produces: `axe_mapping.json` with provenance

## Steps

- [ ] **Step 1: Write validate.rs**

Create `/home/jamin/Documents/Holo-RGAA/rgaa-rs/crates/rgaa-data/src/validate.rs`:

```rust
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MappingEntry {
    pub criterion_id: String,
    pub axe_rules: Vec<String>,
    pub provenance: Provenance,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Provenance {
    pub source: String,
    pub validated_by: String,
    pub validated_at: String,
    pub notes: String,
}

pub fn validate_mapping(
    axe_rules: &[super::fetch::AxeRule],
    existing_mapping: &HashMap<String, Vec<String>>,
) -> Vec<MappingEntry> {
    let axe_ids: std::collections::HashSet<&str> = axe_rules.iter().map(|r| r.id.as_str()).collect();
    let mut entries = Vec::new();

    for (criterion_id, rule_ids) in existing_mapping {
        let valid_rules: Vec<String> = rule_ids
            .iter()
            .filter(|r| axe_ids.contains(r.as_str()))
            .cloned()
            .collect();
        let invalid_rules: Vec<&String> = rule_ids
            .iter()
            .filter(|r| !axe_ids.contains(r.as_str()))
            .collect();

        let notes = if invalid_rules.is_empty() {
            format!("All {} axe rules validated", valid_rules.len())
        } else {
            format!(
                "Invalid rules found: {:?}",
                invalid_rules.iter().map(|r| r.as_str()).collect::<Vec<_>>()
            )
        };

        entries.push(MappingEntry {
            criterion_id: criterion_id.clone(),
            axe_rules: valid_rules,
            provenance: Provenance {
                source: "axe-core 4.9.1 rule-descriptions.md".to_string(),
                validated_by: "automated cross-reference".to_string(),
                validated_at: "2026-08-24".to_string(),
                notes,
            },
        });
    }
    entries.sort_by(|a, b| a.criterion_id.cmp(&b.criterion_id));
    entries
}
```

- [ ] **Step 2: Add `load_existing_mapping()` to parse.rs**

This function returns the 77 existing axe-core → RGAA mappings. Hardcode them as a `HashMap<String, Vec<String>>` based on the entries in `rgaa-rs/crates/rgaa-rules/src/axe_mapper.rs` function `rgaa_to_axe_map()`.

The mapping is:
- "1.1" → ["image-alt", "input-image-alt"]
- "1.2" → ["image-alt", "image-redundant-alt"]
- "1.5" → ["image-alt"]
- "1.6" → ["image-alt", "longdesc"]
- "1.8" → ["image-text"]
- "1.9" → ["figure-caption"]
- "2.1" → ["iframe-title"]
- "3.2" → ["color-contrast"]
- "3.3" → ["color-contrast"]
- "4.1" → ["audio-description", "video-description"]
- "4.3" → ["video-caption"]
- "4.5" → ["audio-description", "video-description"]
- "4.7" → ["video-description", "audio-description"]
- "4.8" → ["video-description", "audio-description"]
- "4.10" → ["audio-control"]
- "4.11" → ["keyboard", "keyboard-trap"]
- "4.12" → ["keyboard", "keyboard-trap"]
- "4.13" → ["video-description", "audio-description"]
- "5.1" → ["table-header"]
- "5.4" → ["table-header"]
- "5.6" → ["table-header", "td-headers-attr"]
- "5.7" → ["td-headers-attr", "th-has-data-cells"]
- "5.8" → ["layout-table"]
- "6.1" → ["link-name", "link-purpose-in-context"]
- "6.2" → ["link-name"]
- "7.1" → ["keyboard", "keyboard-trap", "focus-order"]
- "7.3" → ["keyboard", "keyboard-trap", "focus-visible"]
- "7.4" → ["on-focus", "on-input"]
- "8.1" → ["doctype"]
- "8.2" → ["html-has-lang", "html-lang-valid"]
- "8.3" → ["html-has-lang"]
- "8.5" → ["page-title"]
- "8.7" → ["lang"]
- "8.9" → ["layout-table", "deprecated-element"]
- "8.10" → ["focus-order", "meaningful-sequence"]
- "9.1" → ["heading-order", "landmark-one-main", "region"]
- "9.3" → ["list", "listitem"]
- "9.4" → ["blockquote"]
- "10.1" → ["deprecated-element"]
- "10.2" → ["color-contrast", "image-alt"]
- "10.4" → ["resize-text"]
- "10.5" → ["color-contrast"]
- "10.6" → ["link-in-text-block"]
- "10.7" → ["focus-visible"]
- "10.8" → ["aria-hidden-focus", "hidden-content"]
- "10.9" → ["color-contrast", "image-alt"]
- "10.11" → ["reflow"]
- "10.12" → ["text-spacing"]
- "10.13" → ["focus-visible", "keyboard"]
- "10.14" → ["keyboard"]
- "11.1" → ["label", "label-title-only", "input-image-alt"]
- "11.4" → ["label"]
- "11.5" → ["fieldset"]
- "11.6" → ["fieldset"]
- "11.11" → ["error-suggestion"]
- "11.12" → ["error-prevention"]
- "11.13" → ["autocomplete"]
- "12.1" → ["landmark-one-main", "region"]
- "12.2" → ["consistent-navigation"]
- "12.4" → ["landmark-one-main", "region"]
- "12.5" → ["consistent-navigation"]
- "12.6" → ["landmark-one-main", "region", "bypass"]
- "12.7" → ["bypass", "skip-link"]
- "12.9" → ["keyboard-trap"]
- "12.10" → ["character-key-shortcuts"]
- "12.11" → ["keyboard"]
- "13.1" → ["timing-adjustable", "pause-stop-hide"]
- "13.2" → ["on-focus"]
- "13.3" → ["document-title", "pdf"]
- "13.4" → ["document-title", "pdf"]
- "13.5" → ["image-alt", "non-text-content"]
- "13.7" → ["three-flashes"]
- "13.8" → ["pause-stop-hide", "timing-adjustable"]
- "13.9" → ["orientation"]
- "13.10" → ["pointer-gestures"]
- "13.11" → ["pointer-cancellation"]
- "13.12" → ["motion-actuation"]

- [ ] **Step 3: Wire validate into main.rs**

Add to main.rs after the axe-core fetch:
```rust
tracing::info!("Validating axe-core → RGAA mapping...");
let existing_mapping = parse::load_existing_mapping()?;
let validated = validate::validate_mapping(&axe_rules, &existing_mapping);
let mapping_json = serde_json::to_string_pretty(&validated)?;
std::fs::write(out_dir.join("axe_mapping.json"), &mapping_json)?;
tracing::info!("Saved {} validated mappings", validated.len());
```

- [ ] **Step 4: Run and verify**

Run: `cargo run -p rgaa-data`
Expected: `axe_mapping.json` with 77 validated entries

- [ ] **Step 5: Verify output**

Run: `python3 -c "import json; d=json.load(open('crates/rgaa-core/data/rgaa-4.1.2/axe_mapping.json')); print(f'{len(d)} entries'); print(json.dumps(d[0], indent=2))"`
Expected: 77 entries, first entry has provenance

- [ ] **Step 6: Commit**

```bash
git add rgaa-rs/crates/rgaa-data/
git commit -m "Add axe-core mapping validator with provenance tracking"
```
