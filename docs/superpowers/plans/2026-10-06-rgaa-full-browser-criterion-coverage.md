# RGAA Full Browser Criterion Coverage Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make a complete Holo-RGAA audit execute and account for every one of the 258 RGAA 4.1.2 tests on every audited page, using Obscura browser evidence, Holo-guided exploration, and deterministic or domain-specific fallback components.

**Architecture:** The local catalog remains the source of truth for expected criterion and test keys. Obscura owns a bounded reusable browser session, structured observations, safe interactions, and screenshots; Holo plans exploration and judges only with those observations; the orchestrator routes each test through registered mechanisms and rejects incomplete results. A complete audit is published only when every expected test and page has a settled, evidence-backed outcome and the requested crawl was not truncated.

**Tech Stack:** Rust workspace, Tokio, Serde/serde_json, existing CDP/Obscura, Holo multimodal client, `rgaa-core` catalog/registry/engine plan, `rgaa-rules`, `rgaa-orchestrator`, `rgaa-report`, and `rgaa-cli`. No new runtime dependency without a demonstrated need.

**Spec:** `docs/superpowers/specs/2026-10-06-rgaa-browser-holo-full-test-coverage.md`

## Global Constraints

- Every successful page result accounts for all 106 criteria and exactly the 258 catalog test keys.
- Static absence is an applicability candidate only; N/A requires an active, reproducible check and evidence.
- A complete audit has no `NeedsReview`, `NotTested`, missing/duplicate/unknown test key, unresolved mechanism error, or truncated crawl.
- Holo emits schema-validated typed actions using Obscura-issued target IDs; it cannot send JavaScript or arbitrary selectors.
- Browser actions are read-only and bounded; no form submission, destructive/business action, or off-origin navigation is activated.
- Reuse page sessions, observations, screenshots, and compatible Holo batches; only missing test IDs may be retried.
- Preserve deserialization of historical JSON reports; old records retain historical/incomplete coverage.
- Follow repository Rust instructions: use `RUSTC_WRAPPER=sccache`, scope builds to changed crates while iterating, and never run `cargo clean`.

## Review Focus

1. A hidden image/media/form exposed only after interaction must invalidate an initial N/A candidate; pin this in Task 2 and Task 5.
2. Duplicate, foreign, or missing test keys must fail completeness even when the row count is 258; pin this in Task 1.
3. Holo prompt injection, invalid action IDs, off-origin links, or a form submit request must be rejected before CDP execution; pin this in Task 3.
4. A page that fails to load or a crawl stopped at `max_pages` must make complete mode fail instead of shrinking the denominator; pin this in Task 6.
5. Screenshots/evidence write failure or missing evidence references must not leave a settled test verdict; pin this in Tasks 2 and 7.

---

## File Map

- `rgaa-rs/crates/rgaa-core/src/types.rs`, `catalog.rs`, `completion.rs`, and `audit_bundle.rs`: expected test-key accounting, completion mode and backward-compatible per-test evidence references.
- `rgaa-rs/crates/rgaa-core/src/na_detection.rs` and `registry.rs`: change static N/A into candidate generation and declare which registered mechanisms can actively settle each test.
- `rgaa-rs/crates/rgaa-obscura/src/guided.rs`, `lib.rs`, `evidence.rs`, and focused new modules under `src/`: bounded session reuse, target inventory, safe typed actions, state snapshots, screenshots, and artifact references.
- `rgaa-rs/crates/rgaa-holo/src/prompts.rs` and `client.rs` (plus the prompt/agent adapter that currently builds criterion batches): structured exploration plans and test-key verdicts from shared observations, with validation and targeted missing-key retry.
- `rgaa-rs/crates/rgaa-rules/src/` and `rgaa-rs/crates/rgaa-core/data/rgaa-4.1.2/mechanisms.toml`: map each catalog test to an active component and adapt component observations into `TestOutcome`.
- `rgaa-rs/crates/rgaa-orchestrator/src/pipeline.rs`, `merge.rs`, and `site_comparison.rs`: execute every test, remove the late static-N/A overwrite, run complete crawl/site checks, retry only unresolved IDs, and stop successful finalization on incomplete coverage.
- `rgaa-rs/crates/rgaa-cli/src/commands/analyze.rs` and `rgaa-rs/crates/rgaa-core/src/types.rs`: expose/propagate an explicit complete-audit mode while preserving the current sample mode and configured safety caps.
- `rgaa-rs/crates/rgaa-report/src/report.rs`, `report/html.rs`, `format.rs`, `pdf.rs`/`pdf_native.rs`, and `rgaa-rs/crates/rgaa-core/src/audit_bundle.rs`: render per-test status, component, observation and screenshot references in JSON/HTML/PDF, plus complete/incomplete coverage totals.
- `rgaa-rs/crates/rgaa-test-corpus/criteria/`, crate-local unit tests, and orchestrator integration tests: interaction, N/A, completeness, error and crawl fixtures.
- `rgaa-rs/docs/architecture-audit-106-criteres.md` and the 106-criterion diagram under `rgaa-rs/docs/`: update implementation ownership and coverage only after the executable registry agrees.

## Tasks

### Task 1: Make catalog test-key coverage a hard core invariant

**Files:**
- Modify: `rgaa-rs/crates/rgaa-core/src/types.rs`, `catalog.rs`, `audit_bundle.rs`
- Test: `rgaa-rs/crates/rgaa-core/src/types.rs`, `catalog.rs`, `audit_bundle.rs`

**Interfaces:**
- Consume canonical `RgaaCatalog` criterion IDs and each criterion's exact ordered test keys.
- Add a core completeness validator returning structured missing, duplicate, and unknown test-key errors for a page; do not infer completeness from counts.
- Preserve `CriterionResult.tests` and existing JSON reads through serde defaults for all new optional fields.

- [ ] **Step 1: Add failing core tests** where one valid key is missing, one key is duplicated, one key belongs to another criterion, all 258 keys are present once, and a legacy result with no `tests` still deserializes.
- [ ] **Step 2: Run `RUSTC_WRAPPER=sccache cargo nextest run -p rgaa-core`** and confirm the validator tests fail before implementation.
- [ ] **Step 3: Implement the catalog-key validator** to compare `BTreeSet`/counts per expected key, returning every discrepancy rather than a boolean or total count.
- [ ] **Step 4: Make criterion reduction require every expected key to be settled and reject invalid keys**; keep a direct Fail as Fail while keeping the audit completeness error visible.
- [ ] **Step 5: Run `RUSTC_WRAPPER=sccache cargo nextest run -p rgaa-core`** and verify old JSON fixtures still load.
- [ ] **Step 6: Commit** `feat(core): validate per-test RGAA coverage`.

### Task 2: Produce evidence-backed browser observations and active applicability checks

**Files:**
- Modify: `rgaa-rs/crates/rgaa-obscura/src/guided.rs`, `lib.rs`, `evidence.rs`
- Add: focused observation/session modules under `rgaa-rs/crates/rgaa-obscura/src/`
- Modify: `rgaa-rs/crates/rgaa-core/src/na_detection.rs`
- Test: Obscura unit/integration tests and core N/A tests

**Interfaces:**
- Add one page-scoped observation/session API that returns DOM inventories, AX tree, URL/origin, focus order, rendered dimensions, structured state, and content-addressed screenshot refs.
- Extend `GuidedAction` with only explicit safe actions needed by the spec (scroll, Tab/Shift+Tab/Escape, safe disclosure activation, same-origin link navigation, capture); every action references an Obscura-inventoried target.
- Replace `detect_na`'s final-status map with `ApplicabilityCandidate { criterion_id, test_keys, observed_absence, evidence_refs }`; only a deterministic browser-backed check can turn a candidate into N/A.
- Session, time, action-count and screenshot budgets are explicit; every mutation-like UI state change is followed by an observation and cleanup/restore.

- [ ] **Step 1: Add failing tests** proving static empty inventories yield candidates only, an opened disclosure can reveal an image, unsafe targets and cross-origin links are rejected, action budgets stop loops, and a failed screenshot write is returned as an error.
- [ ] **Step 2: Run `RUSTC_WRAPPER=sccache cargo nextest run -p rgaa-obscura -p rgaa-core`** and confirm those tests fail against current behavior.
- [ ] **Step 3: Implement the reusable page observation/session API** on the existing CDP transport; collect the initial screenshot, DOM inventory, AX tree and focus order once, and store artifacts through `EvidenceStore`.
- [ ] **Step 4: Implement typed target-bound actions and cleanup**; disallow arbitrary JavaScript/selectors, form submits, external origins, and targets not in the current inventory.
- [ ] **Step 5: Change static N/A detection to candidate-only output** and add active inventory checks for each candidate category after reachable disclosures/focus states have been explored.
- [ ] **Step 6: Run `RUSTC_WRAPPER=sccache cargo nextest run -p rgaa-obscura -p rgaa-core`** and verify screenshot refs point to persisted PNG/JSON artifacts.
- [ ] **Step 7: Commit** `feat(obscura): collect reusable browser observations and proof`.

### Task 3: Define Holo's constrained exploration and per-test response contract

**Files:**
- Modify: `rgaa-rs/crates/rgaa-holo/src/prompts.rs`, `client.rs`
- Modify: the existing Holo adapter under `rgaa-rs/crates/rgaa-agent/src/` that builds criterion batches
- Test: Holo prompt/client and agent adapter unit tests

**Interfaces:**
- Holo input is a page observation bundle, relevant screenshot refs/base64, candidate test keys and safe target IDs; it does not receive browser credentials or unrestricted tools.
- Holo returns strict JSON with `actions: Vec<ExplorationAction>` and `outcomes: Vec<TestOutcomeProposal>`; outcome entries name exactly one requested `test_key`, status `pass|fail|na|unresolved`, confidence and concise evidence rationale.
- Validate response schema, criterion ownership, requested IDs, action target IDs and allowed action variants before returning proposals to the orchestrator.

- [ ] **Step 1: Add failing response-validation tests** for malformed JSON, omitted/duplicate/foreign test keys, model-invented target IDs, unsafe action variants, prompt-injected page text, and a valid plan with no extra IDs.
- [ ] **Step 2: Run `RUSTC_WRAPPER=sccache cargo nextest run -p rgaa-holo -p rgaa-agent`** and confirm invalid replies are currently accepted or not represented.
- [ ] **Step 3: Add the strict Holo schemas and prompts**; make page text explicitly untrusted data and instruct Holo to request only listed IDs and target refs.
- [ ] **Step 4: Implement a pure validator before any browser execution**; if a response omits tests, retain valid returned items and return only missing keys for targeted re-asks.
- [ ] **Step 5: Batch compatible test keys by page and Holo context limit** and expose request/action counters so duplicate whole-page requests are detectable.
- [ ] **Step 6: Run `RUSTC_WRAPPER=sccache cargo nextest run -p rgaa-holo -p rgaa-agent`** and verify retry fixtures show only missing keys in subsequent prompts.
- [ ] **Step 7: Commit** `feat(holo): plan safe browser exploration by test key`.

### Task 4: Map all 258 tests to registered executable mechanisms

**Files:**
- Modify: `rgaa-rs/crates/rgaa-core/data/rgaa-4.1.2/mechanisms.toml`, `registry.rs`, `engine_plan.rs`
- Modify: `rgaa-rs/crates/rgaa-rules/src/` mechanism adapters and axe mapping
- Test: registry invariants and mechanism tests

**Interfaces:**
- Every catalog test key has a primary component and an ordered fallback chain: axe-core where exact rule coverage exists, deterministic/browser probes, media/domain analysis, cross-page comparison, then Holo judgment using the same observations.
- `MechanismRegistry` exposes test-level coverage and required evidence kinds; criterion-level `EnginePlan` remains the ownership summary and is validated against the per-test plan.
- A component may claim Pass only for the exact test keys it fully checks; partial axe silence never closes keys; no test may silently fall out of routing because the current criterion engine is marked Human or Deterministic.

- [ ] **Step 1: Add registry tests** requiring exact coverage of catalog test keys and one declared route per key; assert the 23 formerly untested criteria map through executable routes and the full catalog totals 258 keys.
- [ ] **Step 2: Run `RUSTC_WRAPPER=sccache cargo nextest run -p rgaa-core -p rgaa-rules`** and confirm gaps in current mechanism metadata are reported by test.
- [ ] **Step 3: Update registry data and adapters** so all criteria have primary/fallback components, including all 23 previously untested IDs and the eight formerly human-only routes via browser evidence plus Holo where needed.
- [ ] **Step 4: Add per-test outcome conversion** from axe, deterministic, media and site comparison outputs to `TestOutcome`, with component source and evidence refs.
- [ ] **Step 5: Ensure existing probes for 1.6, 3.3, 4.12–4.13, 8.7, 10.9, 10.12–10.13, 11.3, 11.8, 11.11, 12.1–12.2, 12.4–12.5, 12.8–12.11, and 13.3, 13.10–13.12** produce outcomes tied to catalog test keys rather than a criterion-only verdict.
- [ ] **Step 6: Run `RUSTC_WRAPPER=sccache cargo nextest run -p rgaa-core -p rgaa-rules -p rgaa-test-corpus`** and verify registry/fixture invariants.
- [ ] **Step 7: Commit** `feat(rules): route every RGAA test to an executable mechanism`.

### Task 5: Integrate page-level exploration, routing, retries and strict finalization

**Files:**
- Modify: `rgaa-rs/crates/rgaa-orchestrator/src/pipeline.rs`, `merge.rs`
- Modify: `rgaa-rs/crates/rgaa-core/src/types.rs`, `completion.rs`
- Test: orchestrator pipeline and integration tests

**Interfaces:**
- Per-page sequence: one Obscura initial observation → candidate test inventory → one Holo exploration plan per compatible batch → validated safe actions with observations/captures → mechanism routing by test key → targeted retries for unresolved Holo keys → exact core coverage validation.
- Eliminate the post-merge `na_map` override in `pipeline.rs`; merge preserves a reproducible N/A proof and does not let lower-quality/missing results erase active browser evidence.
- Complete-mode finalization returns a typed coverage error if any expected key or evidence requirement remains unresolved; legacy/sample runs remain explicitly marked partial and cannot be mistaken for complete.

- [ ] **Step 1: Add failing pipeline tests** for N/A override after a browser failure, an element appearing after disclosure activation, Holo partial output retrying only missing test IDs, one page failure, duplicate engine outputs, and component disagreement.
- [ ] **Step 2: Run `RUSTC_WRAPPER=sccache cargo nextest run -p rgaa-orchestrator`** and confirm current N/A overwrite/incomplete behavior is reproduced.
- [ ] **Step 3: Implement the page-scoped observation cache and execution schedule**; let mechanisms share the same session/state/evidence and use `EnginePlan`/registry instead of broad criterion batches.
- [ ] **Step 4: Implement component fallback and merge rules**; preserve direct failures, require evidence for N/A, and request Holo only for unresolved test IDs where model judgment can decide from available evidence.
- [ ] **Step 5: Add strict complete-mode validation before constructing a successful `AuditResult`**; any unresolved test, page error, missing artifact or invalid key returns a typed error with page/criterion/test identifiers.
- [ ] **Step 6: Run `RUSTC_WRAPPER=sccache cargo nextest run -p rgaa-orchestrator`** and verify successful fixture output has 106 criteria and all 258 keys once per page.
- [ ] **Step 7: Commit** `feat(orchestrator): execute complete browser guided test pipeline`.

### Task 6: Make complete crawl limits and site-wide comparisons explicit

**Files:**
- Modify: `rgaa-rs/crates/rgaa-core/src/types.rs`
- Modify: `rgaa-rs/crates/rgaa-orchestrator/src/pipeline.rs`, `site_comparison.rs`
- Modify: `rgaa-rs/crates/rgaa-cli/src/commands/analyze.rs`
- Test: crawl/site comparison and CLI contract tests

**Interfaces:**
- Add explicit complete-vs-sample audit intent without changing the existing default sampling contract; complete mode sets `sample_mode = false` and retains configured `max_pages`/`max_depth` as hard safety limits.
- Crawl discovery reports whether it exhausted the reachable in-scope queue or stopped at a safety cap; do not infer completeness merely from `discovered_count < max_pages` after truncation.
- Site criteria 12.1, 12.2, 12.4 and 12.5 consume normalized observations from every page in the same crawl and return test-key outcomes plus page/sample provenance.

- [ ] **Step 1: Add failing tests** for CLI complete-mode selection, explicit page list beyond the cap, spider output at cap, a failed page, same-crawl inter-page comparisons, and incomplete sample with no Pass.
- [ ] **Step 2: Run `RUSTC_WRAPPER=sccache cargo nextest run -p rgaa-cli -p rgaa-orchestrator`** and confirm the existing crawl cap can currently lose completeness information.
- [ ] **Step 3: Add the explicit CLI/config complete-mode path** while preserving sample mode when the flag is absent; report the effective page/depth caps in audit metadata.
- [ ] **Step 4: Preserve discovery completeness and failed URLs through `audit_discovered_urls`**; return an incomplete-crawl error in complete mode when a cap is hit or a page could not be observed.
- [ ] **Step 5: Emit site-level `TestOutcome`s** for 12.1, 12.2, 12.4, 12.5 from all collected pages; an incomplete corpus cannot yield Pass/N/A.
- [ ] **Step 6: Run `RUSTC_WRAPPER=sccache cargo nextest run -p rgaa-cli -p rgaa-orchestrator`** and verify legacy sample CLI contract remains unchanged.
- [ ] **Step 7: Commit** `feat(crawl): report complete site coverage and comparisons`.

### Task 7: Expose per-test evidence and complete coverage in JSON/HTML/PDF reports

**Files:**
- Modify: `rgaa-rs/crates/rgaa-core/src/types.rs`, `audit_bundle.rs`
- Modify: `rgaa-rs/crates/rgaa-report/src/report.rs`, `report/html.rs`, `format.rs`, and `pdf.rs`/`pdf_native.rs`
- Modify: `rgaa-rs/crates/rgaa-cli/src/commands/analyze.rs`
- Test: report rendering and legacy serialization fixtures

**Interfaces:**
- Each test row carries criterion/test ID, outcome, deciding component, concise reproducible observation, optional confidence, and artifact refs for relevant screenshots/state evidence.
- Audit summary carries expected/completed criterion and test counts per page, discovered/audited/failed/truncated page counts, Holo calls/retries and mechanism errors.
- Keep fields optional/defaulted for historical reports; never relabel an old criterion-only record as complete.

- [ ] **Step 1: Add failing report tests** for a test with screenshot evidence, an N/A proof, an incomplete audit reason, all 258 test rows, prior JSON shape deserialization, and PDF format selection/output.
- [ ] **Step 2: Run `RUSTC_WRAPPER=sccache cargo nextest run -p rgaa-report -p rgaa-cli`** and confirm the report omits per-test/evidence data or incomplete-crawl detail.
- [ ] **Step 3: Extend JSON/HTML/PDF output** to list all tests under each criterion and link or print local artifact refs; show page and site completion counts separately. Add `pdf` to `ReportFormat` and wire the existing PDF renderer through the CLI output path.
- [ ] **Step 4: Ensure CLI complete mode does not write a successful compliance report** when orchestration returns a coverage error; print the exact failed page/test IDs through the existing CLI error path.
- [ ] **Step 5: Run `RUSTC_WRAPPER=sccache cargo nextest run -p rgaa-report -p rgaa-cli`** and validate serialization remains compatible with stored examples.
- [ ] **Step 6: Commit** `feat(report): show per-test outcomes and browser evidence`.

### Task 8: Prove all routes with corpus fixtures and update the architecture map

**Files:**
- Add/modify: `rgaa-rs/crates/rgaa-test-corpus/criteria/` and corpus integration tests
- Modify: `rgaa-rs/docs/architecture-audit-106-criteres.md`, `rgaa-rs/docs/diagramme-traitement-106-criteres.md`
- Test: full changed-crate suite, then workspace verification

**Interfaces:**
- Corpus fixtures cover each new mechanism's pass/fail states, interaction-only content, semantic ambiguity, missing evidence, and N/A with active proof.
- The architecture document and diagram are checked against executable registry output; implementation status is generated or asserted rather than maintained as unverified prose.

- [ ] **Step 1: Add a failing end-to-end fixture test** that executes the 106 criteria/258 expected test keys through the same router used by the orchestrator and checks unique outcomes, evidence refs, and zero unresolved statuses.
- [ ] **Step 2: Run focused corpus/orchestrator tests** and confirm each failure names the unimplemented criterion/test key or missing proof.
- [ ] **Step 3: Add pass/fail paired fixtures for all newly executable behavior**; add N/A fixtures where the criterion/test is absent from both initial and reachable states.
- [ ] **Step 4: Derive the criterion-to-component tables and percentages from `EnginePlan` plus `MechanismRegistry`**, then update the diagram and audit document with measured completeness limits.
- [ ] **Step 5: Run `RUSTC_WRAPPER=sccache cargo fmt --check`, workspace Clippy, workspace check, and `cargo nextest run --workspace`** using the exit-code-preserving command pattern in `AGENTS.md`; fix every introduced failure.
- [ ] **Step 6: Commit** `test(rgaa): prove complete browser guided test coverage`.

## Plan Self-Review

- **Spec coverage:** test-key validation and legacy compatibility → Task 1/7; active Obscura observations and screenshots → Task 2; Holo-directed safe exploration and missing-key-only retries → Task 3/5; component routing and all 23 gaps → Task 4; full per-page exploration and no static N/A override → Task 5; complete crawl and inter-page criteria → Task 6; evidence-rich reporting → Task 7; all 106/258 checks and architecture map → Task 8.
- **Review Focus tests:** hidden interaction-only elements → Tasks 2/5; duplicate/foreign/missing IDs → Tasks 1/3/5; hostile page prompts and unsafe actions → Task 3; capped crawl/page failures → Task 6; screenshot write/reference failures → Tasks 2/7.
- **Type consistency:** all components exchange catalog test keys, `TestOutcome` proposals and `EvidenceRef`s; only the orchestrator converts validated proposals into final page outcomes; complete-crawl metadata remains attached to the audit result.
- **Scope decision:** this remains one coordinated plan because browser observations, test routing, finalization and reporting share the test-key/evidence contract and none can independently deliver the approved behavior. The prior 2026-10-05 routing plan covers an earlier stage; this plan supersedes its final-status and human-only assumptions where they conflict with the approved 2026-10-06 specification.
- **No unresolved outcome claim:** if even the Holo/deterministic fallback chain cannot settle a test, complete mode returns a coverage error; it does not publish `NeedsReview`, `NotTested`, an invented Pass/Fail, or an N/A inferred from static absence.

---

Plan complete and saved to `docs/superpowers/plans/2026-10-06-rgaa-full-browser-criterion-coverage.md`. Please review the plan. Which execution approach would you prefer?

- **Subagent-driven** — a fresh worker implements each task and a fresh reviewer checks each deliverable, followed by a full review; this is thorough, while respecting the dependency boundaries between core contracts, browser/Holo work, orchestration, and reports.
- **Native** — I implement the tasks in this session and use one independent review at the end; this is faster and avoids repeated context handoffs across the shared interfaces.

Does the plan capture what you want, and which approach should we use?
