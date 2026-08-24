# Data-First Production Readiness Design

> Strategy to make Holo-RGAA production-ready by sourcing curated data from official
> RGAA references, validating existing mappings, implementing browser tools, and
> building a test corpus.

## Context

The codebase has 106 RGAA criteria cataloged but significant data gaps:
- Only 28/106 criteria have IA-assiste definitions (for Holo3 prompts)
- axe-core mapping (77 criteria) has undocumented provenance
- Only ~20/106 classifications are confirmed with justification
- All 9 browser tools are TODO stubs
- No test corpus exists

**Production use case:** Full stack — CLI (`rgaa-cli analyze`), API (`rgaa-api`), CI/CD integration.

**Approach:** Data-First (data sourcing → classification validation → browser tools → test corpus).

---

## Phase 1: Data Sourcing Pipeline

### Sources

| Source | URL | Data |
|--------|-----|------|
| RGAA 4.1.2 referentiel | `github.com/DISIC/accessibilite.numerique.gouv.fr` (`RGAA/criteres.json`) | Already vendored |
| RGAA glossary | Same repo, `RGAA/glossaire.json` | Term definitions |
| RGAA methodology | Same repo, `RGAA/methodologie.json` | Test methodology |
| axe-core rules | `github.com/dequelabs/axe-core` (`lib/rules/index.js`) | Rule descriptions |
| WCAG techniques | `w3.org/WAI/WCAG21/Techniques/` | Technique codes |

### Output files

```
rgaa-core/data/rgaa-4.1.2/
  criteres.json          (existing)
  glossaire.json         (NEW)
  methodologie.json      (NEW)
  automatability.json    (NEW: criterion_id → { validated_tests, total_tests })
  axe_mapping.json       (NEW: axe_rule_id → criterion_id[], with provenance)

rgaa-core/data/wcag/
  techniques.json        (NEW: technique code → description, criteria)
```

### Implementation

A `rgaa-data` build crate or build script that:
1. Fetches sources via `reqwest` (cached locally)
2. Parses JSON into typed structs
3. Validates cross-references (every criterion has automatability, every axe rule maps to valid criteria)
4. Outputs structured JSON files

---

## Phase 2: IA-Assiste Definitions (All 106 Criteria)

### Current state

`rgaa-agent/src/criteria_defs.rs` has 28 definitions with:
- `id`, `title`, `wcag_refs`, `definition` (French question text)
- `VISUAL_CRITERIA` (14 criteria routed to reasoning model)

### Plan

1. **Source:** `criteres.json` has `title` + `tests` for each criterion
2. **Automated generation:** Parse `criteres.json` → extract title + tests → generate `CriterionDefinition` entries for all 106
3. **Manual enrichment:** Keep curated text for existing 28; auto-generate from catalog for other 78
4. **VISUAL_CRITERIA expansion:** Use automatability data to identify which criteria require visual understanding

### New structure

```rust
const DEFINITIONS: &[CriterionDefinition] = &[
    CriterionDefinition {
        id: "1.1",
        title: "Chaque image porteuse d'information a-t-elle une alternative textuelle ?",
        wcag_refs: "1.1.1",
        definition: "Pour chaque image ...",
        automatable: true,
        test_count: 7,
    },
    // ... 105 more
];
```

---

## Phase 3: axe-core Mapping Validation + Documentation

### Current state

`axe_mapper.rs` has `rgaa_to_axe_map()` with 77 entries, comment "From existing poc.js". No validation, no provenance.

### Validation approach

1. Cross-reference each axe rule against axe-core rule descriptions
2. Cross-reference each mapped criterion against RGAA test definitions
3. Document provenance for each mapping entry
4. Identify gaps: the 29 unmapped criteria classified as no-rule/visual/human

### Output

`axe_mapping.json` with validated mappings + provenance:

```json
{
  "criterion_id": "1.1",
  "axe_rules": ["image-alt", "input-image-alt"],
  "provenance": {
    "source": "axe-core 4.9.1 rule-descriptions.md",
    "validated_by": "automated cross-reference",
    "validated_at": "2026-08-24"
  }
}
```

---

## Phase 4: Browser Tools (CDP Implementation)

### Tools to implement

| Tool | CDP Command | Complexity |
|------|-------------|------------|
| `navigate` | `Page.navigate` | Low |
| `eval_js` | `Runtime.evaluate` | Low |
| `screenshot` | `Page.captureScreenshot` | Medium |
| `a11y_tree` | `Accessibility.getFullAXTree` | High |
| `click` | `Input.dispatchMouseEvent` + `DOM.querySelector` | Medium |
| `type_input` | `Input.dispatchKeyEvent` + `Input.insertText` | Medium |
| `press_key` | `Input.dispatchKeyEvent` | Low |
| `tab_order` | `Accessibility.getFullAXTree` + filter | Medium |
| `assert_state` | `Runtime.evaluate` + DOM inspection | Medium |

### Architecture

Each tool follows the same pattern:
```rust
impl CdpTool for SomeTool {
    fn execute(&self, ctx: &ToolContext, params: Value) -> Result<Value, BrowserError> {
        // 1. Build CDP command
        // 2. Send via ctx.session.send_command()
        // 3. Parse response
        // 4. Return structured result
    }
}
```

---

## Phase 5: Test Corpus (Sample HTML Pages)

### Structure

```
rgaa-core/tests/corpus/
  1.1/img-alt-present.html       (PASS)
  1.1/img-alt-missing.html       (FAIL)
  1.1/img-alt-empty.html         (FAIL)
  1.2/img-decorative.html        (PASS)
  ...
```

### Each file contains

- Minimal HTML with the specific criterion being tested
- Comment: `<!-- EXPECTED: Pass | Fail | NA -->`
- Comment explaining why

### Usage

1. **Unit tests:** Parse HTML, run evaluation, assert expected status
2. **Integration tests:** Serve HTML via local HTTP, run full pipeline
3. **Regression tests:** Catch behavior changes

### Scope

Start with 77 axe-mapped criteria (automated tests), then expand to IA-assiste and Manuel.

---

## Execution Order

1. Phase 1: Data sourcing pipeline (~2-3 days)
2. Phase 2: IA-assiste definitions expansion (~1-2 days)
3. Phase 3: axe-core mapping validation (~1 day)
4. Phase 4: Browser tools (~3-4 days)
5. Phase 5: Test corpus (~2 days)

**Total estimated effort: ~8-10 days**

---

## Success Criteria

- [ ] All 106 criteria have automatability data
- [ ] All 106 criteria have IA-assiste definitions
- [ ] All 77 axe-core mappings are validated with provenance
- [ ] All 9 browser tools return real data (not TODO stubs)
- [ ] Test corpus covers all 77 axe-mapped criteria
- [ ] `cargo test --workspace` passes
- [ ] `rgaa-cli analyze <url>` runs end-to-end and produces a valid AuditBundle
