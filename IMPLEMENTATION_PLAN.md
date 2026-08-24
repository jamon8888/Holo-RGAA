# Holo‑RGAA — Implementation Plan

> Master, codebase‑grounded roadmap to make Holo‑RGAA a coherent, RGAA 4.1.2‑faithful
> evaluator where the agentic Holo3 pipeline is actually reachable, the result
> models are unified, and the compliance math matches the official method.

Status legend: ✅ done · 🔲 open.

- ✅ M1 — Phase 1 (correctness) complete
- ✅ M2 — Pipeline wiring complete (M2.2 CLI, M2.4 AuditBundle, M2.5 config, M2.6 ModelRouter removed)
- ✅ M3.1 — Catalog: 106 criteria vendored, `RgaaCatalog` with `OnceLock`, 5 tests passing

---

## 1. Locked decisions

1. **M1 first** — Phase 1 (correctness) is the initial deliverable.
2. **`AuditBundle` is canonical** — `AuditResult` is deprecated; add `From<AuditResult>`
   then remove it. Storage / API / MCP / remediation all consume `AuditBundle`.
3. **Full sample‑wide `taux_global`** — a criterion is **NC if NC on *any* sample page**,
   **C only if C on *all* sample pages**. Not per‑page averaging.
4. **Explore `rgaa-remediation`** before M6 and wire it into `AuditBundle`.

## 2. Ground truth from the official RGAA 4.1.2 (DINUM)

- Source of truth: `criteres.json` (+ `glossaire.json`, `methodologie.json`) published by
  DINUM at `github.com/DISIC/accessibilite.numerique.gouv.fr` (`RGAA/` folder).
  **106 criteria**, avg ~2.5 tests each, based on EN 301 549 V2.1.2 / WCAG 2.1 A+AA.
  RGAA 4.1.2 is current (18 Apr 2023); RGAA 5 planned end‑2026.
- **Tests are per‑criterion; each is `validé` (automatable) or `non validé` (needs human /
  assistive‑tech judgement).** This is *per‑test*, not per‑criterion — the current
  `Classification` enum is a coarse, incorrect proxy.
- **Taux de conformité global (the legally required metric):**
  `C / (C + NC)` over the sample. **NA and NT are excluded** from the denominator.
  `C` = conforme on *all* sample pages; `NC` = non‑conforme on *any* sample page;
  `NA` only if non‑applicable on *all* pages; `NT` (non testé) excluded.
- **État de conformité:** Totale = all 106 respected; Partielle = ≥ 50%; Non‑conformité =
  < 50% or no audit.
- **Dégradation rule:** because some tests are `non validés`, an automated tool can never
  validate them → it must report a *taux de conformité partiel* (validated tests only) and
  **can never claim "Conformité totale"** unless a human covered the non‑validated tests.
- **Applicability (NA) — 3 official reasons:** content absent; content exempted; or
  disproportionate‑burden derogation with an accessible alternative.

## 3. Target architecture

```
CLI / MCP / API
      │
      ▼
rgaa-orchestrator::pipeline      (single entry: run / run_batch)
      ├─ rgaa-obscura    (axe-core + gap-fix via CDP)
      ├─ rgaa-browser-tools (real CDP tools; replaces NotConnected stubs)
      ├─ rgaa-agent + rgaa-holo (Holo3 for IaAssiste / Manuel)
      └─ rgaa-rules     (AxeMapper, GapFixRules, catalog-backed)
                  │
                  ▼
        rgaa-core::AuditBundle   (canonical, validated)
                  │
      ┌───────────┼────────────┐
  rgaa-storage  rgaa-api  rgaa-remediation  rgaa-mcp
```

Single model: `AuditBundle`. `AuditResult` → `From<AuditResult> for AuditBundle`
(in `audit_bundle.rs`), then delete `AuditResult` after migration.

## 4. Phase 1 — Stop the bleeding (correctness, no public API churn)

| # | File : symbol | Change | Acceptance |
|---|---|---|---|
| 1.1 | `rgaa-orchestrator/src/pipeline.rs:calculate_compliance` | `rate = pass / (pass + fail)` over **applicable** criteria; exclude `NotApplicable`, `NotTested`, `NeedsReview`, `Error`. Drop the `total` param. | `[Pass,Fail,NA,NotTested]` → 50%; `[Pass,NeedsReview]` → 100%. |
| 1.2 | `pipeline.rs:audit_one` fallback (≈152) | Unmapped, non‑`Manuel` criterion with **no executed test** → `NotTested` (not `Pass`). `Manuel` → `NeedsReview`. | No false `Pass` for uncovered criteria. |
| 1.3 | `rgaa-obscura/src/lib.rs:build_gap_fix_script` (≈1161) | Capture the returned snippet from `GapFixRules` and inject it; `run_gap_fix` (≈961) consumes the real result. | All 10 gap‑fix criteria stop reporting false `Fail`. |
| 1.4 | `rgaa-holo/src/client.rs:new` (≈66) | `Result<Self, RgaaError>` via `RgaaError::Holo3`; no `.expect()`. Update callers: `rgaa-agent/src/models.rs:new_placeholder`, `client.rs` tests. | No `panic` on TLS init failure. |
| 1.5 | `client.rs:evaluate` / `evaluate_multimodal` / `evaluate_with_messages` | Return `Result<HoloResponse, RgaaError>`; map reqwest / parse / 429 to `RgaaError::Holo3`. Update `client.rs` tests. | `Err` carries `RgaaError`, not `String`. |
| 1.6 | `rgaa-holo/src/prompts.rs:build_for_criterion` (≈293) | Parse **dotted** IDs (`split('.')`); rewrite / remove `get_base_criterion` & `get_criterion_focus`. | Prompt built correctly for `1.3`, `11.2`, … |
| 1.7 | `rgaa-rules/src/axe_mapper.rs:map` (≈10) | `Result<HashMap<…>, RgaaError>`; replace `unwrap_or_default()`; log on malformed JSON. Update callers: `pipeline.rs:101`, `obscura lib.rs:96`. | Malformed axe JSON → error, not silent empty vec. |

**Exit M1:** `cargo test -p rgaa-orchestrator -p rgaa-obscura -p rgaa-holo -p rgaa-rules`
green; no false fails; no panics.

## 5. Phase 2 — Make the agentic pipeline reachable

| # | File : symbol | Change |
|---|---|---|
| 2.1 | `rgaa-browser-tools/src/tools/*.rs` | Implement real CDP via `cdp_issue` (fix the **nanos‑mod id mismatch** in `obscura lib.rs:cdp_issue`); `A11yTreeTool`, `EvalJsTool`, `ScreenshotTool`, `NavigateTool` return real data, not `Err(NotConnected)`. |
| 2.2 | `rgaa-cli/src/commands/analyze.rs:≈34`, `rgaa-mcp/src/server.rs:analyze ≈526` | Call `rgaa_orchestrator::pipeline::run / run_batch` instead of `ObscuraBridge::analyze`. |
| 2.3 | `pipeline.rs:audit_one` | Becomes the merge layer: axe (`AxeMapper`) + gap‑fix (`GapFixRules`) + Holo3 (`RgaaAgent` / `HoloClient`) for `IaAssiste` / `Manuel`; combine into `CriterionResult`. |
| 2.4 | `rgaa-core/src/audit_bundle.rs` | `impl From<AuditResult> for AuditBundle` (+ `PageResult`, `CriterionResult` mappers); map `CriterionStatus` → `Finding.status`. |
| 2.5 | `rgaa-agent/src/agent.rs:≈42`, `ratelimit.rs` | Replace hardcoded RPM `10/20` with `RateLimitConfig` from audit spec. |
| 2.6 | `rgaa-agent/src/models.rs` | Either wire `ModelRouter` / `SelectedTier` into `evaluate_criterion` (route `VISUAL_CRITERIA` + `11.` / `12.8` to Reasoning), or delete the dead code. |

**Exit M2:** `rgaa-cli analyze <url>` runs axe + gapfix + Holo3 end‑to‑end and emits a
validated `AuditBundle` consumable by storage / api / remediation with no lossy re‑mapping.

## 6. Phase 3 — RGAA 4.1.2 fidelity

### 3.1 Catalog (vendored, embedded) ✅ designed
- Vendor `criteres.json` → `rgaa-core/data/rgaa-4.1.2/`. `catalog.rs`: `RgaaCatalog`
  parsed once via `OnceLock`; `criteria()`, `by_id()`, `count()`;
  `CatalogCriterion { id, title, tests, wcag, techniques }`. Tests: parses to 106,
  ids `^\d+\.\d+$`, unique, and cross‑check vs hand `RgaaCriteria`.
- **Re‑vendor** the file (it was reverted during planning): `cp` from DINUM `RGAA/criteres.json`;
  add `catalog.rs`; wire in `lib.rs`.

### 3.2 Automatability (`validé` / `non validé`)
- `criteres.json` has **no** automatable flag. Add `rgaa-core/data/rgaa-4.1.2/automatability.json`
  (curated `criterion_id → { validated_tests, total_tests }`), built from the RGAA
  methodology. Populate `CatalogTest::validated: Option<bool>`; test asserts **every**
  criterion is classified in production (no `None`).

### 3.3 Replace `Classification`
- `rgaa-core/src/types.rs:Classification` becomes **derived**: `fully_automatable` (all tests
  validated), `partial`, `manual`. `RgaaCriteria` becomes a shim over `RgaaCatalog`.

### 3.4 Status semantics (C / NC / NA / NT)
- `Pass`→C, `Fail`→NC, `NotApplicable`→NA, `NotTested`→NT, `NeedsReview`→NT (excluded),
  `Error`→NC + separate `error_count`.
- **Sample aggregation:** NC if NC on *any* page; C only if C on *all* pages (replaces
  per‑page averaging in `audit_one`).

### 3.5 Applicability (NA) detection
- Derive content presence from AXTree / HTML: no `<img>` / `[role=img]` → `1.x`;
  no `<form>` / `<input>` / `<select>` / `<textarea>` → `11.x`; no `<table>` → `5.x`;
  no `<iframe>` → `2.x`; no `<video>` / `<audio>` → `4.x`; no `<object>` / `<svg>` → `1.x` / `4.x`.
  Feeds the `NotTested` → `NA` promotion.

### 3.6 Compliance + honesty
- `taux_global = C / (C + NC)` (NA / NT excluded).
- `taux_partiel_sur_criteres_valides` (dégradation) = validated‑tests‑passed /
  validated‑tests‑applicable.
- Emit `état de conformité` via 50% threshold; **cap at `partielle`** unless
  `human_reviewed_non_validated`.
- Add `AuditSummary.coverage_percent` = % of *validated* tests executed; surface in
  `AuditBundle`.

**Exit M3:** single‑page and sample‑wide audits match official `C/(C+NC)`; engine never
prints "Conformité totale" from automation alone.

## 7. Phase 4 — Rust hygiene

- `rgaa-rules/axe_mapper.rs`: replace `HashMap` with `IndexMap` (reproducible output);
  fill `title` / `wcag_refs` from `RgaaCatalog::by_id`.
- Build `axe_rule_id → &[criterion_id]` index from catalog references; drop the 77‑entry
  hardcoded map.
- `rgaa-core/criteria.rs`: `OnceLock` (or delete post‑3.3).
- Borrow / `Cow` in violation matching; clone once.
- Config‑driven limits (remove magic `10/20`).

## 8. Phase 5 — Tests & CI

- **rgaa-core:** `RgaaCatalog` tests (parses 106, ids, cross‑check); `From<AuditResult>`
  round‑trip; `taux_global` math; NA detection; sample‑aggregation.
- **rgaa-rules:** `AxeMapper::map` fixture test; `GapFixRules::snippets`; no‑false‑fail
  regression.
- **rgaa-holo:** `#[tokio::test]` `evaluate` (mock HTTP); `PromptBuilder::build_for_criterion`
  end‑to‑end.
- **rgaa-orchestrator:** `calculate_compliance` table tests; `audit_one` integration with a
  fake bridge.
- **CI:** `cargo fmt --check`, `cargo clippy -p … -D warnings`, `cargo test --workspace`;
  **nightly job** diffing vendored `criteres.json` vs upstream `main` (warn on RGAA 5).
- Add `#[must_use]` on `AxeMapper::map`, `GapFixRules::snippets`, `HoloClient::evaluate`.

## 9. Phase 6 — Security & ops

- `rgaa-api/src/main.rs:≈337` CORS `permissive()` → configured origins.
- `rgaa-storage/src/repository.rs:list_findings ≈421` → `sqlx` query‑builder / bound params;
  test multi‑filter.
- `hash_api_key` documented; never log keys (`tracing` redaction).
- **Explore `rgaa-remediation`** (`proposals` / `policy` / `dedup` / `lifecycle`) and
  integrate it to consume `AuditBundle.findings`, then expose via CLI / MCP.

## 10. Milestones & order

- **M1** (this PR): Phase 1 — correct numbers, no false fails, no panics.
- **M2**: Phase 2 wiring + 3.1 catalog (re‑vendor).
- **M3**: 3.2–3.6 (classification, NA, sample‑wide taux, coverage).
- **M4**: Phase 4 hygiene.
- **M5**: Phase 5 tests + CI.
- **M6**: Phase 6 security + `rgaa-remediation` integration.

## 11. Open questions (resolve before M3.2 / M6)

1. **Automatability source:** author `automatability.json` from the RGAA methodology pages,
   or pull from Ara's open dataset if pointed to it? This blocks 3.2.
2. **`rgaa-remediation` scope:** generate fix *proposals* or only *track* them?

## 12. Definition of Done

`cargo clippy -D warnings` + `cargo test --workspace` green; CLI / MCP / API produce a
validated `AuditBundle`; `taux_global` equals official `C/(C+NC)` on a known sample; no
`panic`, no `unwrap_or_default` on untrusted input; honest `coverage_percent` and
`partielle` cap.
