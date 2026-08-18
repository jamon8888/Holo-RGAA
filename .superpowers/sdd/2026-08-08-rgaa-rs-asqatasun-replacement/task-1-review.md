# Task 1 Review: Workspace Scaffolding + rgaa-core

## Verdict

- **Spec:** ✅ (with ⚠️ items below)
- **Quality:** ✅ (with ⚠️ items below)

---

## Spec Compliance

### ✅ Passed

| Requirement | Status | Evidence |
|---|---|---|
| Workspace root `rgaa-rs/Cargo.toml` | ✅ | resolver=2, 7 members, workspace deps |
| 7 crates created | ✅ | rgaa-core, rgaa-rules, rgaa-holo, rgaa-browser, rgaa-orchestrator, rgaa-storage, rgaa-api |
| All crates `edition = "2021"` | ✅ | Verified in each Cargo.toml |
| `rgaa-core` types defined | ✅ | Classification, CriterionStatus, CriterionResult, Violation, PageResult, AuditResult, CrawlConfig |
| `rgaa-core` error handling | ✅ | RgaaError (8 variants), Result type alias, thiserror |
| 106 criteria with contiguous IDs | ✅ | 1.1→13.12, all 106 present |
| `lib.rs` re-exports | ✅ | RgaaCriteria, Criterion, types::*, RgaaError, Result |
| `cargo check` passes | ✅ | Compiles clean (only resolver warning) |
| Placeholder crates | ✅ | Empty lib.rs in rgaa-rules, rgaa-holo, rgaa-browser, rgaa-orchestrator, rgaa-storage, rgaa-api |

### ⚠️ Needs Clarification

**1. Classification distribution mismatch**

| Source | DETERMINISTE | IA_ASSISTE | MANUEL |
|---|---|---|---|
| Global constraints (task brief) | 73 | 32 | 1 |
| Reference CSV (`grille-rgaa-106.csv`) | 79 | 26 | 1 |
| Plan criteria list | 77 | 28 | 1 |
| **Implementation** | **77** | **28** | **1** |

The implementation matches the plan exactly (verified byte-for-byte). But the plan's 77/28/1 matches neither the global constraints (73/32/1) nor the CSV reference (79/26/1). Two criteria differ between CSV and implementation:
- **1.4** (CAPTCHA alternative relevance): CSV = DETERMINISTE, impl = IaAssiste
- **3.1** (color-only information): CSV = DETERMINISTE, impl = IaAssiste

This is a **plan-level discrepancy**, not an implementer error. Recommend confirming correct classification against the official RGAA 4.1.2 reference before Task 2, since the `AxeMapper` mapping depends on it.

**2. MSRV not declared in Cargo.toml**

Global constraint specifies MSRV 1.80, but neither the workspace nor crate Cargo.toml files set `rust-version = "1.80"`. This means `cargo msrv` and CI won't enforce the MSRV. Should be added to workspace Cargo.toml:

```toml
[workspace.package]
rust-version = "1.80"
```

---

## Quality

### ✅ Passed

- Code follows idiomatic Rust conventions (snake_case, proper derives, pub visibility)
- `Criterion` uses `&'static str` for id/title/wcag_refs — efficient, no allocation
- `CrawlConfig` has sensible `Default` impl
- Types derive `Serialize`/`Deserialize` — ready for JSON storage
- Error types have clear `#[error("...")]` messages
- No dead code warnings, no unused imports

### ⚠️ Minor Quality Notes (non-blocking)

**3. `all()` allocates on every call**

`RgaaCriteria::all()` builds a new `Vec<Criterion>` each time. `find()`, `deterministic()`, `ia_assiste()`, and `count()` each call `all()` independently — the full 106-criterion list is rebuilt 4 times on a typical lookup. Acceptable for scaffolding; consider `lazy_static` or `const` array in a future iteration.

**4. `Criterion` missing `PartialEq`/`Eq`**

The `Criterion` struct derives only `Debug, Clone`. Adding `PartialEq, Eq` (and optionally `Hash`) would enable use in `HashSet`, `HashMap`, and simpler test assertions.

---

## Summary

The implementation is a faithful copy of the plan's Task 1 specification. All structural requirements are met, code compiles, and conventions are followed. The two ⚠️ items (classification distribution mismatch, MSRV declaration) are **plan-level issues** that should be resolved before proceeding to Task 2, as they affect downstream crates (`rgaa-rules` axe mapping depends on correct classifications).
