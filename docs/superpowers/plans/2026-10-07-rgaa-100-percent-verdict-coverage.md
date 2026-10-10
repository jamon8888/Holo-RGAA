# RGAA 100% Automatic Verdict Coverage Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Produce a traceable automatic verdict for all 106 RGAA 4.1.2 criteria on every successfully audited page, while keeping unsupported predictions out of verified compliance metrics.

**Architecture:** Extend `CriterionResult` with an automatic prediction, its evidence basis, calibrated confidence, review flag, verified status, and review history. Turn the existing engine plan and mechanism registry into an enforced test-level routing contract, then make the orchestrator collect deterministic/browser/IA evidence and ask MyIA for a prediction for every remaining criterion, including human-classified criteria. Reports show prediction coverage, evidence execution, and verified compliance separately.

**Tech Stack:** Rust workspace (Serde, Tokio, `rgaa-core`, `rgaa-orchestrator`, `rgaa-agent`, `rgaa-report`, `rgaa-cli`), `mechanisms.toml` and JSON catalogs, Obscura/CDP, axe-core, MyIA OpenAI-compatible API, Python standalone report renderer.

**Spec:** `docs/superpowers/specs/2026-10-07-rgaa-100-percent-coverage-design.md`

## Global Constraints

- For each page, produce one automatic verdict for each of the 106 RGAA 4.1.2 criteria.
- The catalog contains 258 RGAA tests; every test must have a routed mechanism, a coverage declaration, and a fallback.
- A model prediction without sufficient evidence is not a verified conformity result.
- A partial mechanism may report an observed failure, but its silence cannot establish `pass`.
- The 100% target applies to completed audits with required services available; service failures must remain visible and prevent a complete-audit claim.
- Preserve the original machine prediction when a human review confirms or changes the verified status.
- Keep legacy JSON readable by applying Serde defaults to newly added fields.
- Before Rust commands, export `RUSTC_WRAPPER=sccache` and start sccache once in the shell; do not vary `RUSTFLAGS` between commands.
- Initialize the shared build cache once per shell with `sccache --start-server`; do not run `cargo clean`.
- The working tree already contains unrelated edits; never stage or commit their hunks while implementing these tasks. Inspect every staged diff before each commit.

## Review Focus

- Missing or malformed MyIA batch entries must leave the affected criterion without a fabricated verified status; pin with malformed and incomplete response tests in Task 4.
- MyIA outage or circuit-breaker activation must produce an incomplete audit and must not claim 100% verdict coverage; pin in Task 3.
- Conflicting model and deterministic results must preserve both sources while deterministic evidence controls `verified_status`; pin in Task 3.
- Model-only inapplicability must not establish verified `not_applicable`; pin in Task 1 and Task 3.
- Old JSON without prediction/review fields and multi-page aggregation must deserialize and produce correct counts; pin in Tasks 1 and 7.
- Legacy rows with `source = "agent"` and a model-derived `status = pass` must not count as verified compliance; pin in Task 7.

---

## File Map

- `rgaa-rs/crates/rgaa-core/src/types.rs`: add serializable automatic-verdict and review-event data to `CriterionResult`, plus a backward-compatible `AuditResult.audit_complete` flag.
- Existing `CriterionResult` struct literals in `rgaa-agent/src/rag/verifier.rs`, `rgaa-core/src/audit_bundle.rs`, `rgaa-mcp/tests/verify_fix.rs`, `rgaa-orchestrator/src/merge.rs`, `rgaa-orchestrator/tests/integration_test.rs`, `rgaa-remediation/tests/full_remediation_loop.rs`, `rgaa-report/src/{declaration.rs,lib.rs,pdf_native.rs,report.rs,report/html.rs,sources.rs}`, `rgaa-rules/src/{axe_mapper.rs,gap_fix.rs}`, `rgaa-storage/tests/union_schema.rs`, and `rgaa-tui/src/tui/history.rs`: supply defaults through a shared constructor or explicit assessment defaults.
- Existing `AuditResult` struct literals in `rgaa-api/src/{routes.rs}`, `rgaa-api/tests/{batch_endpoints.rs,batch_store_postgres.rs}`, `rgaa-core/src/audit_bundle.rs`, `rgaa-orchestrator/tests/integration_test.rs`, `rgaa-storage/tests/union_schema.rs`, `rgaa-tui/src/{storage.rs,tui/history.rs}`: set `audit_complete` explicitly.
- `rgaa-rs/crates/rgaa-core/src/engine_plan.rs`: load and validate a per-test routing plan alongside the existing 106-row criterion plan.
- `rgaa-rs/crates/rgaa-core/src/test_plan.rs`: parse the per-test route file and expose indexed lookup.
- `rgaa-rs/crates/rgaa-core/src/catalog.rs`: expose the catalog's 258 `(criterion_id, test_key)` pairs for validation.
- `rgaa-rs/crates/rgaa-core/data/rgaa-4.1.2/test_routes.json`: new explicit route for each of the catalog's 258 test keys, including the `holo_estimate` fallback.
- `rgaa-rs/crates/rgaa-core/src/registry.rs`: validate route coverage, mechanism references, outcome limits, and fixture requirements.
- `rgaa-rs/crates/rgaa-orchestrator/src/pipeline.rs`: execute plan routes, request estimates for unresolved and human criteria, and mark failed/incomplete runs.
- `rgaa-rs/crates/rgaa-agent/src/agent.rs` and `prompts.rs`: request structured predictions and preserve evidence/confidence metadata.
- `rgaa-rs/crates/rgaa-agent/src/verify.rs`: validate model verdict values and response completeness.
- `rgaa-rs/crates/rgaa-agent/tests/live_verdict_eval.rs`: opt-in evaluation against the labeled corpus, with false-pass and false-fail counts.
- `rgaa-rs/crates/rgaa-test-corpus/criteria/`: add paired fixtures for new deterministic/browser mechanisms.
- `rgaa-rs/crates/rgaa-test-corpus/tests/registry_invariants.rs`: enforce complete test-key routing and fixture invariants.
- `rgaa-rs/crates/rgaa-cli/src/commands/review.rs`, `commands/mod.rs`, `main.rs`: add a local JSON review command that records verified decisions without editing predictions.
- `rgaa-rs/crates/rgaa-report/src/lib.rs`: compute prediction, evidence, and verified-compliance metrics from unrounded counts.
- `rgaa-rs/crates/rgaa-core/src/types.rs` and `rgaa-rs/crates/rgaa-orchestrator/src/pipeline.rs`: persist the three aggregate metric values on `AuditResult`.
- `rgaa-rs/crates/rgaa-report/src/report/html.rs`: show automatic and verified results in native report output.
- `scripts/rgaa-report-html.py`: show the same fields and metrics in the standalone per-page HTML report.
- `scripts/tests/test_rgaa_report_html.py`: test the standalone report's metrics and legacy-input rendering.

## Interfaces

Task 1 defines these public types in `rgaa-core::types`:

```rust
pub enum AutomatedVerdict { Pass, Fail, NotApplicable }
pub enum VerdictBasis { Axe, Deterministic, Browser, ModelEstimate }
pub struct ReviewEvent {
    pub status: CriterionStatus,
    pub author: String,
    pub reviewed_at: String,
    pub reason: String,
}
```

`CriterionResult` gains `automated_verdict: Option<AutomatedVerdict>`, `verdict_basis: Vec<VerdictBasis>`, `evidence: Vec<EvidenceRef>`, `confidence_calibration_version: Option<String>`, `review_required: bool`, `review_reason: Option<String>`, `verified_status: Option<CriterionStatus>`, and `review_events: Vec<ReviewEvent>`. `AuditResult` gains `audit_complete: bool` with `#[serde(default)]`; old results deserialize as incomplete. `status` remains the compatibility/verified status; when an automatic prediction alone is uncertain, `status` stays `NeedsReview`.

Task 2 defines `CoverageLevel { Complete, Partial }`, `TestRoute { criterion_id: String, test_key: String, mechanisms: Vec<String>, coverage: CoverageLevel, fallback: String }`, `RgaaCatalog::all_test_keys() -> Vec<(String, String)>`, `TestRoutePlan::builtin() -> &'static TestRoutePlan`, and `TestRoutePlan::for_test(criterion_id: &str, test_key: &str) -> Option<&TestRoute>`. Each of the 258 route rows declares `coverage`; routes relying only on model fallback are `Partial`. `EnginePlan::route_test(criterion_id: &str, test_key: &str) -> Option<&'static TestRoute>` delegates to that index for the orchestrator. Test keys use the catalog's local key, such as `"1"` for criterion `1.1`; the pair is the unique identity.

Task 3 defines `CoverageError { pub missing_criterion_ids: Vec<String> }` and `validate_automatic_verdict_coverage(results: &[CriterionResult]) -> Result<(), CoverageError>`. It validates one output per catalog criterion and reports missing predictions as incomplete; it never invents a prediction after a provider failure. It derives `automated_verdict` from the per-test `TestOutcome` list only after all expected test keys have a deterministic outcome or model estimate.

## Task 1: Add the Automatic Assessment and Review Data Contract

**Files:**
- Modify: `rgaa-rs/crates/rgaa-core/src/types.rs`
- Test: unit tests in `rgaa-rs/crates/rgaa-core/src/types.rs`
- Modify: `rgaa-rs/crates/rgaa-core/src/lib.rs` to export the new public types.
- Update: all existing `CriterionResult` literals listed in the File Map and all `AuditResult` literals listed there, adding explicit defaults or the correct audit-completion value.

**Interfaces:**
- Consumes: existing `CriterionStatus`, `EvidenceRef`, `CriterionResult`.
- Produces: `AutomatedVerdict`, `VerdictBasis`, `ReviewEvent`, and optional fields on `CriterionResult` as defined above.

- [ ] **Step 1: Add failing serialization tests**

Add tests that serialize/deserialize each verdict and basis; decode pre-change `CriterionResult` and `AuditResult` JSON objects with none of the new fields; verify an old audit defaults to `audit_complete == false`; and verify a human review event round-trips with author, timestamp, status, and reason.

```rust
const LEGACY_CRITERION_JSON: &str = r#"{"criterion_id":"1.1","title":"images","classification":"Deterministe","status":"pass","violations":[],"confidence":1.0,"justification":"alt present","source":"axe-core","citations":[],"considered_sources":[],"tests":[]}"#;
const LEGACY_AUDIT_JSON: &str = r#"{"audit_id":"old","url":"https://example.test","pages":[],"total_criteria":0,"passed":0,"failed":0,"na":0,"overall_compliance":0.0,"taux_global":0.0,"coverage_percent":0.0,"etat_conformite":"non conforme","duration_ms":0}"#;
let decoded: CriterionResult = serde_json::from_str(LEGACY_CRITERION_JSON)?;
assert_eq!(decoded.automated_verdict, None);
assert!(decoded.review_events.is_empty());
let old_audit: AuditResult = serde_json::from_str(LEGACY_AUDIT_JSON)?;
assert!(!old_audit.audit_complete);
```

- [ ] **Step 2: Run the focused tests and confirm the new tests fail to compile or deserialize**

Run from `rgaa-rs/` after enabling sccache: `cargo nextest run -p rgaa-core types::tests::criterion_result_legacy_json_defaults_assessment_fields`

Expected: the test fails because the new types and fields do not exist yet.

- [ ] **Step 3: Implement backward-compatible fields**

Add `#[serde(default)]` to every new collection/boolean/optional field. Keep `automated_verdict` optional so older or incomplete runs can be represented; a later completion gate determines whether the audit can claim 100%. Set `audit_complete` only after all page/criterion outputs pass the completion gate.

- [ ] **Step 4: Run all `rgaa-core` tests**

Run: `cargo nextest run -p rgaa-core`

Expected: PASS, including legacy JSON compatibility and round-trip assertions.

- [ ] **Step 5: Commit the core data contract**

This checkout already has unrelated edits in many of these tracked files. Stage only this task’s hunks with `git add -p`; inspect the complete staged diff with `git diff --cached` and `git diff --cached --check` before committing. Do not use blanket path-based staging for already-dirty files.

```bash
git commit -m "feat(core): model automatic RGAA verdicts and reviews"
```

## Task 2: Make Every RGAA Test Key Routable

**Files:**
- Create: `rgaa-rs/crates/rgaa-core/data/rgaa-4.1.2/test_routes.json`
- Create: `rgaa-rs/crates/rgaa-core/src/test_plan.rs`
- Modify: `rgaa-rs/crates/rgaa-core/src/catalog.rs`
- Modify: `rgaa-rs/crates/rgaa-core/src/engine_plan.rs`
- Modify: `rgaa-rs/crates/rgaa-core/src/registry.rs`
- Modify: `rgaa-rs/crates/rgaa-core/data/rgaa-4.1.2/mechanisms.toml` only where a mechanism needs explicit test keys or outcomes
- Test: `rgaa-rs/crates/rgaa-test-corpus/tests/registry_invariants.rs`

**Interfaces:**
- Consumes: `RgaaCatalog::by_id`, `CatalogCriterion::tests`, current `EnginePlan`, and `MechanismRegistry`.
- Produces: `TestRoutePlan::for_test(&str, &str)` and a registry invariant that identifies every missing/duplicate/unresolvable catalog test route.

- [ ] **Step 1: Add failing route coverage tests**

Enumerate each `(criterion_id, test_key)` from the catalog and assert exactly one route exists, every named mechanism exists, every fallback equals `holo_estimate`, and the plan contains no unknown catalog keys. Assert the expected catalog total is 258.

```rust
let routes = TestRoutePlan::builtin();
let keys = RgaaCatalog::all_test_keys();
assert_eq!(keys.len(), 258);
for (criterion_id, test_key) in keys {
    let route = routes.for_test(&criterion_id, &test_key).expect("every catalog test is routed");
    assert!(matches!(route.coverage, CoverageLevel::Complete | CoverageLevel::Partial));
}
```

- [ ] **Step 2: Run the focused invariant test and confirm it fails on the absent plan**

Run from `rgaa-rs/`: `cargo nextest run -p rgaa-test-corpus --test registry_invariants every_catalog_test_has_one_valid_route`

Expected: FAIL because `test_routes.json` has not been added.

- [ ] **Step 3: Add the 258 explicit route rows**

For each catalog test key, list only existing axe, JS, browser, or media mechanisms whose test coverage is explicit, and set the route-level `coverage` to `complete` or `partial`. If no deterministic mechanism covers that test, leave `mechanisms` empty, mark coverage `partial`, and set `fallback` to `holo_estimate`. Do not claim complete coverage based only on a criterion-level string in `engine_plan.json`.

- [ ] **Step 4: Enforce route and fixture invariants**

Extend `TestRoutePlan` and `MechanismRegistry::check` to reject duplicate routes, missing routes, unknown test keys, unknown mechanism IDs, invalid or missing coverage declarations, invalid fallback names, and a `pass` outcome from a partial mechanism. Require pass/fail fixture pairs for every newly introduced mechanism; preserve the existing `legacy` exemption.

- [ ] **Step 5: Run catalog, registry, and corpus invariant tests**

Run: `cargo nextest run -p rgaa-core -p rgaa-test-corpus`

Expected: PASS; invariant output identifies the exact criterion and test key if a route is removed or duplicated.

- [ ] **Step 6: Commit the test routing plan**

Stage changed tracked hunks interactively; add the two new files by exact path. Inspect the complete staged diff and run `git diff --cached --check` before committing.

```bash
git commit -m "feat(core): route every RGAA test to an evaluator"
```

## Task 3: Execute the Plan and Preserve Verified Status Semantics

**Files:**
- Modify: `rgaa-rs/crates/rgaa-orchestrator/src/pipeline.rs`
- Modify: `rgaa-rs/crates/rgaa-orchestrator/src/merge.rs`
- Test: `rgaa-rs/crates/rgaa-orchestrator/tests/full_audit.rs`
- Test: unit tests in `rgaa-rs/crates/rgaa-orchestrator/src/pipeline.rs`

**Interfaces:**
- Consumes: `EnginePlan`, `TestRoutePlan`, `CriterionResult` assessment fields, existing axe/gap-fix/keyboard result maps, and MyIA estimates from Task 4.
- Produces: one `CriterionResult` per catalog criterion with an automatic verdict on a complete run; a run-completeness indicator and explicit error for missing predictions.
- `CoverageError` carries every criterion ID whose verdict or expected test outcome is missing; `validate_automatic_verdict_coverage(&[CriterionResult])` checks against `RgaaCriteria::all()` and each criterion's catalog test keys.

- [ ] **Step 1: Add failing routing and precedence tests**

Cover these inputs: one deterministic `Fail` plus a conflicting model `Pass`; an unresolved deterministic criterion; a `PlanEngine::Human` criterion; and a missing model prediction. Assert deterministic evidence remains the verified status, but the model prediction is retained separately.

```rust
let merged = merge_candidates(vec![deterministic_fail, model_pass]).unwrap();
assert_eq!(merged.status, CriterionStatus::Fail);
assert_eq!(merged.automated_verdict, Some(AutomatedVerdict::Pass));
assert_eq!(merged.verdict_basis.len(), 2);
```

- [ ] **Step 2: Run the focused orchestrator tests and confirm failure**

Run from `rgaa-rs/`: `cargo nextest run -p rgaa-orchestrator routing_tests`

Expected: FAIL because pipeline routing and separate assessment fields are not implemented.

- [ ] **Step 3: Route every catalog test through its planned stages**

Use `TestRoutePlan::for_test` to schedule test-level axe, deterministic, browser, and media mechanisms. Use `EnginePlan::primary` to group criteria and select prompts. Keep the existing merge rule that deterministic evidence outranks a model estimate for verified `status`; collect each source into `verdict_basis`, `evidence`, and `TestOutcome` rather than discarding losing candidates.

- [ ] **Step 4: Add an automatic-verdict completion gate**

After merging, require an entry for every `RgaaCriteria::all()` item and an outcome for every expected test key. If a prediction is missing, collect the criterion IDs and mark the run incomplete; do not fill `Pass`, `Fail`, or `NotApplicable` from a default. Provider failure may return an auditable partial `AuditResult`, but must set `audit_complete = false` and automatic-verdict coverage below 100%.

- [ ] **Step 5: Run the focused orchestrator tests**

Run: `cargo nextest run -p rgaa-orchestrator routing_tests`

Expected: PASS for precedence, human-owned routing, missing prediction, and catalog completeness.

- [ ] **Step 6: Commit the plan execution and completion gate**

Stage only this task’s hunks with `git add -p` in tracked files, then inspect the complete staged diff and run `git diff --cached --check` before committing.

```bash
git commit -m "feat(orchestrator): enforce automatic verdict coverage"
```

## Task 4: Produce Structured MyIA Estimates for Unresolved and Human Criteria

**Files:**
- Modify: `rgaa-rs/crates/rgaa-agent/src/agent.rs`
- Modify: `rgaa-rs/crates/rgaa-agent/src/prompts.rs`
- Modify: `rgaa-rs/crates/rgaa-agent/src/verify.rs`
- Test: `rgaa-rs/crates/rgaa-agent/tests/integration.rs`
- Test: unit tests in the relevant agent modules

**Interfaces:**
- Consumes: `Criterion`, `PageContext`, per-test routes from Task 2, and the current deterministic/browser evidence structures; the estimator API is implemented before orchestrator integration.
- Produces: `run_automatic_estimates(criteria, page_context, prior_results) -> HashMap<String, CriterionResult>` and `map_automatic_response(criteria, response_json) -> HashMap<String, CriterionResult>`; each successful response returns one `TestOutcome` for each fallback test key plus `automated_verdict`, `verdict_basis = [ModelEstimate]`, evidence references where available, and a separate `review_required` flag.

- [ ] **Step 1: Add failing response validation tests**

Test a valid `pass`, valid `fail`, invalid verdict string, missing criterion ID, duplicate criterion ID, missing array item, and malformed JSON. Verify the mapper never converts malformed output into a verified `Pass`.

```rust
let criteria = vec![RgaaCriteria::find("4.2").unwrap().clone()];
let response_json = r#"[{"criterion_id":"4.2","tests":[{"test_key":"1","verdict":"fail","justification":"no transcript"},{"test_key":"2","verdict":"fail","justification":"no transcript"},{"test_key":"3","verdict":"fail","justification":"no transcript"}],"verdict":"fail","justification":"no transcript","confidence":0.72,"review_required":true}]"#;
let result = map_automatic_response(&criteria, &response_json);
assert_eq!(result["4.2"].automated_verdict, Some(AutomatedVerdict::Fail));
assert_eq!(result["4.2"].status, CriterionStatus::NeedsReview);
assert!(result["4.2"].review_required);
```

- [ ] **Step 2: Run the focused agent tests and confirm failure**

Run from `rgaa-rs/`: `cargo nextest run -p rgaa-agent verify::tests`

Expected: FAIL until the response contract and validator support the automatic assessment fields.

- [ ] **Step 3: Add the structured estimate prompt and parser**

Ask MyIA for one outcome per supplied fallback test key, an aggregate prediction per criterion, a short evidence-based reason, and whether human review remains required. Parse only known criterion/test IDs and `pass`/`fail`; derive verified `not_applicable` only from deterministic evidence. Preserve model responses as estimates, not as verified status.

- [ ] **Step 4: Support estimates for unresolved and `PlanEngine::Human` criteria**

Ensure the agent API can estimate every criterion passed to it, including human-classified criteria, and that tests cover this behavior with mocked provider responses. Keep the pipeline invocation in Task 3, which consumes this API after deterministic/browser evidence is collected. When MyIA fails or returns an incomplete batch, return the IDs with no `automated_verdict`; Task 3 marks the audit incomplete. Do not silently substitute a deterministic default.

- [ ] **Step 5: Run agent unit and integration tests**

Run: `cargo nextest run -p rgaa-agent`

Expected: PASS for all response-shape cases and estimate routing with mocked provider responses.

- [ ] **Step 6: Commit automatic model estimates**

Stage only this task’s hunks with `git add -p` in tracked files, then inspect the complete staged diff and run `git diff --cached --check` before committing.

```bash
git commit -m "feat(agent): predict verdicts for every RGAA criterion"
```

## Task 5: Calibrate Confidence and Evaluate Estimates

**Files:**
- Create: `rgaa-rs/crates/rgaa-agent/data/verdict-calibration.json`
- Create: `rgaa-rs/crates/rgaa-agent/data/verdict-evaluation.json`
- Modify: `rgaa-rs/crates/rgaa-agent/src/verify.rs`
- Modify: `rgaa-rs/crates/rgaa-test-corpus/src/lib.rs` to parse test-key and evidence annotations from the versioned evaluation manifest
- Test: `rgaa-rs/crates/rgaa-agent/tests/integration.rs`
- Test: `rgaa-rs/crates/rgaa-agent/tests/live_verdict_eval.rs`
- Test: annotated fixtures under `rgaa-rs/crates/rgaa-test-corpus/criteria/`

**Interfaces:**
- Consumes: model estimates, catalog criterion/test identities, and labeled evaluation cases.
- Produces: a calibrated `confidence: Option<f64>` plus a calibration version; absent calibration data yields `None` and `review_required = true`.
- Defines `calibrate_confidence(criterion_id: &str, raw_confidence: f64, table: &CalibrationTable) -> Option<f64>`; the function returns `None` until the criterion family has held-out samples.
- Defines `CalibrationTable::from_json(raw: &str) -> Result<CalibrationTable, CalibrationError>` and validates `sample_count`, `accuracy`, raw-confidence bounds, and manifest version.

- [ ] **Step 1: Add failing calibration tests**

Verify that a sufficiently sampled bin returns its 95% Wilson lower bound, a bin with fewer than 30 cases or missing criterion returns `None`, and stale or malformed calibration data is rejected. Include separate expected counts for false-pass and false-fail results.

```rust
let table = CalibrationTable::from_json(r#"{"version":"2026-10-07","bins":[{"criterion_family":"4","min_confidence":0.7,"max_confidence":0.9,"sample_count":100,"accuracy":0.71}]}"#)?;
let confidence = calibrate_confidence("4.2", 0.8, &table).unwrap();
assert!(confidence > 0.0 && confidence < 0.71);
assert_eq!(calibrate_confidence("13.7", 0.8, &table), None);
```

- [ ] **Step 2: Run the focused calibration tests and confirm failure**

Run from `rgaa-rs/`: `cargo nextest run -p rgaa-agent calibration`

Expected: FAIL because calibration lookup and versioned data do not exist.

- [ ] **Step 3: Add a versioned calibration manifest**

Store evaluated sample counts and accuracy by criterion family and raw-confidence band. Return no confidence for bins with fewer than 30 held-out examples; for larger bins, report the conservative 95% Wilson lower bound rather than the raw accuracy. Mark unavailable bins as uncalibrated.

- [ ] **Step 4: Add held-out adversarial and standard evaluation cases**

Use existing criterion fixtures, then add labeled cases for ambiguous alt text, form labels, media equivalence, visual contrast, time limits, and scripted interaction. Record expected verdict and rationale in a machine-readable manifest.

- [ ] **Step 5: Run agent and corpus tests without live model credentials**

Run: `cargo nextest run -p rgaa-agent -p rgaa-test-corpus`

Expected: PASS using deterministic fixture responses. Run the opt-in real-model evaluation with `cargo nextest run -p rgaa-agent --test live_verdict_eval --run-ignored all -- --nocapture`; it must print false-pass and false-fail counts by criterion family.

- [ ] **Step 6: Commit confidence calibration and labeled corpus**

Stage changed tracked hunks interactively, add only the new calibration/evaluation files and fixtures for this task, then inspect the complete staged diff and run `git diff --cached --check` before committing.

```bash
git commit -m "feat(agent): calibrate RGAA verdict confidence"
```

## Task 6: Record Human Reviews Without Replacing Predictions

**Files:**
- Modify: `rgaa-rs/crates/rgaa-cli/src/commands/mod.rs`
- Create: `rgaa-rs/crates/rgaa-cli/src/commands/review.rs`
- Create: `rgaa-rs/crates/rgaa-cli/tests/review_command.rs`

**Interfaces:**
- Consumes: a saved audit JSON, `criterion_id`, page URL for multi-page audits, final status, reviewer name, and reason.
- Produces: a validated audit JSON with appended `ReviewEvent`; update `verified_status` and compatibility `status` but preserve `automated_verdict` and its source.
- Defines `apply_review(result: &mut CriterionResult, status: CriterionStatus, author: &str, reviewed_at: DateTime<Utc>, reason: &str) -> Result<(), ReviewError>`; only `Pass`, `Fail`, and `NotApplicable` are valid final review statuses.

- [ ] **Step 1: Add failing CLI tests**

Test applying a review to an existing criterion; reject an unknown criterion, missing author, empty reason, unsupported status, and ambiguous multi-page criterion without a page URL; assert the prediction remains unchanged.

```rust
let before = result.automated_verdict;
apply_review(&mut result, CriterionStatus::Fail, "reviewer", Utc::now(), "piste absente")?;
assert_eq!(result.automated_verdict, before);
assert_eq!(result.verified_status, Some(CriterionStatus::Fail));
assert_eq!(result.review_events.len(), 1);
```

- [ ] **Step 2: Run the focused CLI tests and confirm failure**

Run from `rgaa-rs/`: `cargo nextest run -p rgaa-cli review`

Expected: FAIL because the `audit review` command does not exist.

- [ ] **Step 3: Implement `rgaa audit review`**

Add required `--input`, `--criterion`, `--status`, `--author`, and `--reason` arguments, with optional `--url` and `--output`. Accept only `pass`, `fail`, or `not_applicable` as reviewed statuses. Validate the target criterion/page and append an RFC 3339 review event; write to `--output` or atomically replace the input file.

- [ ] **Step 4: Run CLI tests**

Run: `cargo nextest run -p rgaa-cli`

Expected: PASS for review validation, multi-page targeting, history, and prediction preservation.

- [ ] **Step 5: Commit the review workflow**

Add the new files by exact path and stage only this task’s hunks in `commands/mod.rs`. Inspect the complete staged diff and run `git diff --cached --check` before committing.

```bash
git commit -m "feat(cli): record verified RGAA reviews"
```

## Task 7: Report Verdict Coverage, Evidence Coverage, and Verified Compliance Separately

**Files:**
- Modify: `rgaa-rs/crates/rgaa-report/src/lib.rs`
- Modify: `rgaa-rs/crates/rgaa-report/src/report/html.rs`
- Modify: `rgaa-rs/crates/rgaa-core/src/types.rs`
- Modify: `rgaa-rs/crates/rgaa-orchestrator/src/pipeline.rs`
- Modify: `scripts/rgaa-report-html.py`
- Test: unit tests in `rgaa-rs/crates/rgaa-report/src/lib.rs` and `report/html.rs`
- Create and test: `scripts/tests/test_rgaa_report_html.py`

**Interfaces:**
- Consumes: per-criterion automatic assessments, verified statuses, test outcomes, review events, and audit completion state.
- Produces: aggregate counts for automatic verdict coverage, evidence execution coverage, and verified compliance; both HTML renderers display predictions and verified statuses separately.
- `AuditResult` carries `automatic_verdict_coverage_percent`, `test_evidence_coverage_percent`, and `verified_compliance_percent`; `SiteMetrics` exposes the same values derived from integer counters.
- Defines `AuditMetrics { expected_criterion_pages, automatic_verdicts, automatic_verdict_coverage_percent, expected_tests, tests_with_non_model_evidence, test_evidence_coverage_percent, verified_compliance_percent }` and `compute_audit_metrics(pages: &[PageResult]) -> AuditMetrics`.

- [ ] **Step 1: Add failing metric tests**

Use a four-page result with 106 criteria per page: assert 424 expected automatic verdicts; one missing prediction lowers automatic coverage to `423 / 424`; uncalibrated estimates do not change verified compliance; and evidence coverage uses test counts rather than page percentage averages.

```rust
let metrics = compute_audit_metrics(&pages);
assert_eq!(metrics.expected_criterion_pages, 424);
assert_eq!(metrics.automatic_verdict_coverage_percent, 100.0 * 423.0 / 424.0);
assert_eq!(metrics.verified_compliance_percent, 0.0); // fixture statuses are all NeedsReview
```

- [ ] **Step 2: Run focused report tests and confirm failure**

Run from `rgaa-rs/`: `cargo nextest run -p rgaa-report`

Expected: FAIL because the three new metrics are not present.

- [ ] **Step 3: Compute metrics from integer counts**

Add explicit `SiteMetrics` and serialized `AuditResult` fields named `automatic_verdict_coverage_percent`, `test_evidence_coverage_percent`, and `verified_compliance_percent`. Compute official `taux_global`/`overall_compliance` from verified statuses only; a legacy `status = pass` from `source = "agent"` without a verified status is not a verified pass. Count a test as having evidence only when a non-model mechanism ran and supplied evidence; count model-only estimates in the automatic-verdict metric, never in the evidence metric. Preserve the old `coverage_percent` value semantics as a deprecated compatibility field for one transition period; do not repurpose it as any of the three new measures. Never derive a multi-page total by averaging already-rounded percentages.

- [ ] **Step 4: Render the separate results in both HTML reports**

Show the automatic verdict and evidence basis on every criterion row; show review flags, calibrated confidence, and human review events; label predictions without evidence as estimates. Replace the single generic `Couverture moteur` card with the three named measures.

- [ ] **Step 5: Verify old report JSON and the four-page 424-row fixture**

Run: `cargo nextest run -p rgaa-report`

Run Python tests: `python3 -m unittest discover -s scripts/tests -p 'test_rgaa_report_html.py'`

Expected: old JSON renders with absent prediction fields; new JSON renders exactly 106 rows per page and counts 424 automatic verdict slots without counting estimates as verified compliance.

- [ ] **Step 6: Commit reporting and metric changes**

This checkout already has unrelated edits in many of these tracked files. Stage only this task’s hunks with `git add -p`; inspect the complete staged diff with `git diff --cached` and `git diff --cached --check` before committing. Do not use blanket path-based staging for already-dirty files.

```bash
git commit -m "feat(report): separate verdict and evidence coverage"
```

## Task 8: End-to-End Completion Gate

**Files:**
- Modify: `rgaa-rs/crates/rgaa-orchestrator/tests/full_audit.rs`
- Modify: `rgaa-rs/crates/rgaa-cli/tests/` or add `rgaa-rs/crates/rgaa-cli/tests/automatic_verdict_report.rs`

**Interfaces:**
- Consumes: completed pipeline, versioned schema, review command, and report metrics from Tasks 1–7.
- Produces: an integration test proving the user-visible 100% automatic-verdict goal without inflating verified compliance.

- [ ] **Step 1: Add a failing mocked end-to-end audit test**

Run four mocked pages through the orchestrator with deterministic findings, model estimates for manual criteria, one low-confidence estimate, and one review event. Assert 424 criteria-page rows, 424 automatic verdicts, the expected lower verified-status count, and correct report labels.

```rust
assert_eq!(result.pages.iter().map(|p| p.criteria.len()).sum::<usize>(), 424);
assert!(result.audit_complete);
assert_eq!(result.automatic_verdict_coverage_percent, 100.0);
assert!(result.verified_compliance_percent < 100.0);
```

- [ ] **Step 2: Add an LLM-outage end-to-end case**

Mock MyIA failure for one human-owned criterion. Assert the audit is incomplete, automatic coverage is below 100%, the error is attached to that criterion, and no report labels the run complete.

- [ ] **Step 3: Run all affected crates**

Run from `rgaa-rs/`: `cargo nextest run -p rgaa-core -p rgaa-agent -p rgaa-orchestrator -p rgaa-report -p rgaa-cli -p rgaa-test-corpus`

Expected: PASS for the integrated path and backward-compatible data decoding.

- [ ] **Step 4: Run formatting and workspace checks**

Run: `cargo fmt --check`

Then run the repository-prescribed `cargo clippy --workspace --all-targets` and `cargo check --workspace --all-targets`, checking each filtered pipeline's `PIPESTATUS[0]` as documented in `AGENTS.md`.

Expected: no formatting, compilation, or clippy errors.

- [ ] **Step 5: Commit the end-to-end guarantee**

Stage only the end-to-end test file(s) changed by this task, not the whole CLI test directory. Inspect the complete staged diff and run `git diff --cached --check` before committing.

```bash
git commit -m "test: enforce complete RGAA verdict reporting"
```

## Delivery Order and Dependencies

Tasks are sequential because each establishes a contract consumed by the next: core result types → test routes → structured model-estimate API → orchestrator integration → calibration → review history → reports → end-to-end gate. Task 4 builds and tests the estimator before Task 3 wires it into the pipeline, avoiding overlapping ownership of orchestration files. The first shippable milestone is Tasks 1–4: automatic predictions for all criteria with honest incomplete-run handling. Tasks 5–8 complete calibration, review, metrics, and acceptance coverage.

**Do not claim RGAA 100% conformity when automatic verdict coverage reaches 100%.** The former requires verified statuses; the latter only means every criterion received a machine prediction in a completed audit.
