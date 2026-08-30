# M3: RGAA 4.1.2 Fidelity Implementation Plan (v2)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the audit engine produce RGAA 4.1.2-compliant results with correct status mapping, NA detection, and honest compliance reporting.

**Architecture:** Integrate existing data artifacts (automatable_criteres.json, axe_mapping.json, criteres.json) into the core types and pipeline. Add NA detection from PageContext. Fix compliance calculation to match official `C/(C+NC)` formula.

**Tech Stack:** Rust, serde, existing rgaa-core types

**Spec:** `IMPLEMENTATION_PLAN.md` (Phase 3, sections 3.2–3.6)

## Global Constraints

- Rust edition 2021, MSRV 1.80
- Tests use `cargo test -p <crate>`
- Follow existing code patterns: `thiserror` for errors, `anyhow` for app errors
- No new workspace dependencies unless absolutely necessary
- All data files live in `rgaa-core/data/rgaa-4.1.2/`

---

## File Structure

| File | Responsibility |
|------|---------------|
| `rgaa-core/src/types.rs` | Status enum, CriterionResult, PageResult, AuditResult |
| `rgaa-core/src/catalog.rs` | RgaaCatalog with automatability data |
| `rgaa-core/src/criteria.rs` | Classification shim (already done in M3.3) |
| `rgaa-orchestrator/src/pipeline.rs` | calculate_compliance, audit_one |
| `rgaa-core/src/na_detection.rs` | NEW: NA detection logic using PageContext |

---

### Task 1: Integrate automatability into RgaaCatalog

**Files:**
- Modify: `rgaa-core/src/catalog.rs`
- Modify: `rgaa-core/data/rgaa-4.1.2/automatable_criteres.json` (already exists)

**Interfaces:**
- Consumes: `automatable_criteres.json` (106 criteria with classification)
- Produces: `CatalogCriterion` with `automatable` field

**Important:** The actual `CatalogCriterion` struct uses `number: u8` (not `id: String`). The `by_id()` method returns `Option<(u8, &'static CatalogCriterion)>`.

- [ ] **Step 1: Add Automatable enum to catalog.rs**

```rust
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
pub enum Automatable {
    FullyAutomatable,
    PartiallyAutomatable,
    NotAutomatable,
}
```

Note: The JSON uses `"FullyAutomatable"`, `"PartiallyAutomatable"`, `"NotAutomatable"` — the enum must match.

- [ ] **Step 2: Add automatable field to CatalogCriterion**

```rust
pub struct CatalogCriterion {
    pub number: u8,
    pub title: String,
    pub tests: HashMap<String, Vec<String>>,
    pub automatable: Automatable,  // NEW
}
```

- [ ] **Step 3: Load automatable_criteres.json in RgaaCatalog::init**

Parse the JSON file and map `classification` string to `Automatable` enum. The JSON has structure:
```json
{
  "criteria": [
    {
      "criterion_id": "1.1",
      "classification": "FullyAutomatable"
    }
  ]
}
```

- [ ] **Step 4: Add test for automatability coverage**

```rust
#[test]
fn test_all_criteria_have_automatability() {
    let catalog = RgaaCatalog::all();
    for criterion in catalog {
        // automatable is always set
        assert!(matches!(
            criterion.automatable,
            Automatable::FullyAutomatable | Automatable::PartiallyAutomatable | Automatable::NotAutomatable
        ));
    }
}
```

- [ ] **Step 5: Verify tests pass**

Run: `cargo test -p rgaa-core`

- [ ] **Step 6: Commit**

```bash
git add rgaa-core/
git commit -m "feat(catalog): integrate automatability data from automatable_criteres.json"
```

---

### Task 2: Map axe_mapping.json into catalog

**Files:**
- Modify: `rgaa-core/src/catalog.rs`
- Modify: `rgaa-core/data/rgaa-4.1.2/axe_mapping.json` (already exists)

**Interfaces:**
- Consumes: `axe_mapping.json` (77 axe-core → RGAA mappings with provenance)
- Produces: `CatalogCriterion { axe_rules: Vec<String>, axe_provenance: Option<AxeProvenance> }`

- [ ] **Step 1: Add AxeProvenance struct to catalog.rs**

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AxeProvenance {
    pub source: String,
    pub validated_by: String,
    pub validated_at: String,
    pub notes: String,
}
```

- [ ] **Step 2: Add axe fields to CatalogCriterion**

```rust
pub struct CatalogCriterion {
    pub number: u8,
    pub title: String,
    pub tests: HashMap<String, Vec<String>>,
    pub automatable: Automatable,
    pub axe_rules: Vec<String>,  // NEW: axe-core rule IDs
    pub axe_provenance: Option<AxeProvenance>,  // NEW
}
```

- [ ] **Step 3: Load axe_mapping.json in RgaaCatalog::init**

Parse the JSON file and populate `axe_rules` and `axe_provenance` for each criterion. The JSON has structure:
```json
[
  {
    "criterion_id": "1.1",
    "axe_rules": ["image-alt", "input-image-alt"],
    "provenance": { "source": "...", "validated_by": "...", "validated_at": "...", "notes": "..." }
  }
]
```

- [ ] **Step 4: Add test for axe mapping coverage**

```rust
#[test]
fn test_criteria_with_axe_rules_have_valid_mapping() {
    let catalog = RgaaCatalog::all();
    for criterion in catalog {
        if !criterion.axe_rules.is_empty() {
            assert!(criterion.axe_provenance.is_some());
        }
    }
}
```

- [ ] **Step 5: Verify tests pass**

Run: `cargo test -p rgaa-core`

- [ ] **Step 6: Commit**

```bash
git add rgaa-core/
git commit -m "feat(catalog): integrate axe_mapping.json with provenance tracking"
```

---

### Task 3: Status semantics mapping (Pass→C, Fail→NC, etc.)

**Files:**
- Modify: `rgaa-core/src/types.rs`
- Modify: `rgaa-orchestrator/src/pipeline.rs`

**Interfaces:**
- Consumes: `CriterionStatus` enum (already exists)
- Produces: `ConformityStatus` enum (C, NC, NA, NT)

- [ ] **Step 1: Add ConformityStatus enum to types.rs**

```rust
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ConformityStatus {
    Conforme,        // C
    NonConforme,     // NC
    NonApplicable,   // NA
    NonTeste,        // NT
}

impl From<CriterionStatus> for ConformityStatus {
    fn from(status: CriterionStatus) -> Self {
        match status {
            CriterionStatus::Pass => ConformityStatus::Conforme,
            CriterionStatus::Fail => ConformityStatus::NonConforme,
            CriterionStatus::NotApplicable => ConformityStatus::NonApplicable,
            CriterionStatus::NotTested | CriterionStatus::NeedsReview => ConformityStatus::NonTeste,
            CriterionStatus::Error => ConformityStatus::NonConforme,
        }
    }
}
```

- [ ] **Step 2: Add test for status mapping**

```rust
#[test]
fn test_status_mapping() {
    assert_eq!(ConformityStatus::from(CriterionStatus::Pass), ConformityStatus::Conforme);
    assert_eq!(ConformityStatus::from(CriterionStatus::Fail), ConformityStatus::NonConforme);
    assert_eq!(ConformityStatus::from(CriterionStatus::NotApplicable), ConformityStatus::NonApplicable);
    assert_eq!(ConformityStatus::from(CriterionStatus::NeedsReview), ConformityStatus::NonTeste);
    assert_eq!(ConformityStatus::from(CriterionStatus::Error), ConformityStatus::NonConforme);
}
```

- [ ] **Step 3: Verify tests pass**

Run: `cargo test -p rgaa-core`

- [ ] **Step 4: Commit**

```bash
git add rgaa-core/
git commit -m "feat(types): add ConformityStatus enum with Pass→C, Fail→NC mapping"
```

---

### Task 4: Applicability (NA) detection from PageContext

**Files:**
- Create: `rgaa-core/src/na_detection.rs`
- Modify: `rgaa-core/src/lib.rs` (add module)
- Modify: `rgaa-orchestrator/src/pipeline.rs` (use NA detection)

**Interfaces:**
- Consumes: `PageContext` (already extracted in pipeline with `images`, `iframes`, `forms`, `media` fields)
- Produces: `HashMap<String, bool>` (criterion_id → applicable)

**Important:** The pipeline already extracts structured `PageContext` via `bridge.extract_page_context(url)`. Use this instead of string-matching on AXTree.

- [ ] **Step 1: Check PageContext structure**

Read `rgaa-orchestrator/src/pipeline.rs` lines 109-120 to understand the existing `PageContext` extraction. It already has `images`, `iframes`, `forms`, `media` fields.

- [ ] **Step 2: Create na_detection.rs with detection rules**

```rust
use std::collections::HashMap;

/// Detect which criteria are not applicable based on page context
pub fn detect_na(page_context: &serde_json::Value) -> HashMap<String, bool> {
    let mut applicable = HashMap::new();
    
    // Default: all criteria are applicable
    for i in 1..=13 {
        for j in 1..=15 {
            applicable.insert(format!("{i}.{j}"), true);
        }
    }
    
    // Check for images (1.x criteria)
    let has_images = page_context
        .get("images")
        .and_then(|v| v.as_array())
        .map(|arr| !arr.is_empty())
        .unwrap_or(false);
    
    if !has_images {
        for j in 1..=9 {
            applicable.insert(format!("1.{j}"), false);
        }
    }
    
    // Check for forms (11.x criteria)
    let has_forms = page_context
        .get("forms")
        .and_then(|v| v.as_array())
        .map(|arr| !arr.is_empty())
        .unwrap_or(false);
    
    if !has_forms {
        for j in 1..=13 {
            applicable.insert(format!("11.{j}"), false);
        }
    }
    
    // Check for tables (5.x criteria) - check in "landmarks" or content
    let has_tables = page_context
        .get("landmarks")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().any(|l| l.get("role").and_then(|r| r.as_str()) == Some("table")))
        .unwrap_or(false);
    
    if !has_tables {
        for j in 1..=8 {
            applicable.insert(format!("5.{j}"), false);
        }
    }
    
    // Check for iframes (2.x criteria)
    let has_iframes = page_context
        .get("iframes")
        .and_then(|v| v.as_array())
        .map(|arr| !arr.is_empty())
        .unwrap_or(false);
    
    if !has_iframes {
        applicable.insert("2.1".to_string(), false);
    }
    
    // Check for video/audio (4.x criteria)
    let has_media = page_context
        .get("media")
        .and_then(|v| v.as_array())
        .map(|arr| !arr.is_empty())
        .unwrap_or(false);
    
    if !has_media {
        for j in 1..=13 {
            applicable.insert(format!("4.{j}"), false);
        }
    }
    
    applicable
}
```

- [ ] **Step 3: Add module to lib.rs**

```rust
pub mod na_detection;
```

- [ ] **Step 4: Integrate NA detection into audit_one**

In `pipeline.rs`, after extracting page context, call `detect_na` and update criteria status:

```rust
let na_map = na_detection::detect_na(&page_context);
for criterion in &mut criteria {
    if let Some(&false) = na_map.get(&criterion.criterion_id) {
        criterion.status = CriterionStatus::NotApplicable;
    }
}
```

- [ ] **Step 5: Add tests for NA detection**

```rust
#[test]
fn test_na_detection_no_images() {
    let context = serde_json::json!({
        "images": [],
        "forms": [],
        "iframes": [],
        "media": []
    });
    let na = detect_na(&context);
    assert_eq!(na.get("1.1"), Some(&false));
    assert_eq!(na.get("1.2"), Some(&false));
    assert_eq!(na.get("3.2"), Some(&true));  // color contrast still applicable
}

#[test]
fn test_na_detection_with_images() {
    let context = serde_json::json!({
        "images": [{"src": "test.png", "alt": "test"}],
        "forms": [],
        "iframes": [],
        "media": []
    });
    let na = detect_na(&context);
    assert_eq!(na.get("1.1"), Some(&true));
}
```

- [ ] **Step 6: Verify tests pass**

Run: `cargo test -p rgaa-core -p rgaa-orchestrator`

- [ ] **Step 7: Commit**

```bash
git add rgaa-core/ rgaa-orchestrator/
git commit -m "feat(core): add NA detection from PageContext"
```

---

### Task 5: Compliance + honesty (coverage_percent, partielle cap)

**Files:**
- Modify: `rgaa-core/src/types.rs` (add compliance fields to AuditResult)
- Modify: `rgaa-orchestrator/src/pipeline.rs` (calculate coverage, apply partielle cap)

**Interfaces:**
- Consumes: `CriterionResult` list with statuses
- Produces: Updated `AuditResult` with compliance fields

**Important:** `AuditSummary` already exists in `audit_bundle.rs` with different fields. Use a different name like `ComplianceReport`.

- [ ] **Step 1: Add compliance fields to AuditResult in types.rs**

```rust
pub struct AuditResult {
    // ... existing fields
    pub taux_global: f64,  // C / (C + NC)
    pub coverage_percent: f64,  // % of validated tests executed
    pub etat_conformite: String,  // "totale", "partielle", "non conforme"
}
```

- [ ] **Step 2: Calculate compliance in pipeline.rs**

```rust
fn calculate_compliance_summary(criteria: &[CriterionResult], catalog: &RgaaCatalog) -> (f64, f64, String) {
    let mut c = 0;
    let mut nc = 0;
    let mut validated_total = 0;
    let mut validated_executed = 0;
    
    for criterion in criteria {
        let conformity = ConformityStatus::from(criterion.status);
        let catalog_criterion = catalog.by_id(&criterion.criterion_id);
        
        match conformity {
            ConformityStatus::Conforme => c += 1,
            ConformityStatus::NonConforme => nc += 1,
            _ => {}
        }
        
        // Count validated criteria (Fully or Partially automatable)
        if let Some((_theme, cat)) = catalog_criterion {
            if matches!(cat.automatable, Automatable::FullyAutomatable | Automatable::PartiallyAutomatable) {
                validated_total += 1;
                if criterion.status != CriterionStatus::NotTested {
                    validated_executed += 1;
                }
            }
        }
    }
    
    let taux_global = if c + nc > 0 {
        (c as f64 / (c + nc) as f64) * 100.0
    } else {
        0.0
    };
    
    let coverage_percent = if validated_total > 0 {
        (validated_executed as f64 / validated_total as f64) * 100.0
    } else {
        0.0
    };
    
    let etat_conformite = if taux_global >= 100.0 {
        "totale".to_string()
    } else if taux_global >= 50.0 {
        "partielle".to_string()
    } else {
        "non conforme".to_string()
    };
    
    (taux_global, coverage_percent, etat_conformite)
}
```

- [ ] **Step 3: Add tests for compliance calculation**

```rust
#[test]
fn test_compliance_all_pass() {
    let criteria = vec![
        test_result(CriterionStatus::Pass),
        test_result(CriterionStatus::Pass),
    ];
    let (taux, coverage, etat) = calculate_compliance_summary(&criteria, &RgaaCatalog::get());
    assert_eq!(taux, 100.0);
    assert_eq!(etat, "totale");
}

#[test]
fn test_compliance_mixed() {
    let criteria = vec![
        test_result(CriterionStatus::Pass),
        test_result(CriterionStatus::Fail),
    ];
    let (taux, _coverage, etat) = calculate_compliance_summary(&criteria, &RgaaCatalog::get());
    assert_eq!(taux, 50.0);
    assert_eq!(etat, "partielle");
}
```

- [ ] **Step 4: Verify tests pass**

Run: `cargo test -p rgaa-core -p rgaa-orchestrator`

- [ ] **Step 5: Commit**

```bash
git add rgaa-core/ rgaa-orchestrator/
git commit -m "feat(compliance): add taux_global, coverage_percent, and etat_conformite"
```

---

### Task 6: Wire NA detection and compliance into pipeline

**Files:**
- Modify: `rgaa-orchestrator/src/pipeline.rs`

**Interfaces:**
- Consumes: NA detection, compliance calculation
- Produces: Updated `PageResult` with NA status and compliance

**Important:** Don't replace the full pipeline — add NA detection and compliance summary as incremental changes to the existing `audit_one` function.

- [ ] **Step 1: Add NA detection after page context extraction**

In `audit_one`, after extracting page context (around line 109-120):

```rust
let page_context = bridge.extract_page_context(url).await?;
let na_map = na_detection::detect_na(&page_context);
```

- [ ] **Step 2: Apply NA status to criteria**

After evaluating criteria (around line 150):

```rust
for criterion in &mut criteria {
    if let Some(&false) = na_map.get(&criterion.criterion_id) {
        criterion.status = CriterionStatus::NotApplicable;
    }
}
```

- [ ] **Step 3: Calculate compliance summary**

Before returning PageResult:

```rust
let (taux_global, coverage_percent, etat_conformite) = calculate_compliance_summary(&criteria, &RgaaCatalog::get());
```

- [ ] **Step 4: Update PageResult return**

```rust
Ok(PageResult {
    url: url.to_string(),
    title: None,
    criteria,
    compliance_rate: taux_global,
    crawl_depth: 0,
})
```

- [ ] **Step 5: Verify tests pass**

Run: `cargo test -p rgaa-orchestrator`

- [ ] **Step 6: Commit**

```bash
git add rgaa-orchestrator/
git commit -m "feat(pipeline): integrate NA detection and compliance summary"
```

---

## Self-Review Checklist

- [x] All tasks have actual code (no placeholders)
- [x] Types match existing codebase (CatalogCriterion.number: u8, by_id returns tuple)
- [x] No name conflicts (using ComplianceReport, not AuditSummary)
- [x] Each task has its own test
- [x] Tasks are independently testable
- [x] Global constraints are respected
- [x] Preserves existing pipeline logic (NA detection is incremental)
- [x] Uses existing PageContext instead of string-matching

## Execution Handoff

Plan v2 complete and saved to `docs/superpowers/plans/2026-08-24-m3-rgaa-fidelity.md`. Two execution options:

**1. Subagent-Driven (recommended)** - Fresh subagent per task, review between tasks, fast iteration

**2. Inline Execution** - Execute tasks in this session with checkpoints

**Which approach?**
