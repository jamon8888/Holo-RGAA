# Claude Code RGAA Remediation Plugin Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a production-grade, local-first Claude Code accessibility workflow that analyzes pages, runs guided tests, proposes approved source remediations, verifies fixes, exports CI/compliance artifacts, and optionally synchronizes bundles with a remote service.

**Architecture:** Keep the Rust workspace authoritative for audit schemas, browser execution, finding lifecycle, evidence, policy, and verification. Add a focused remediation crate, a three-tool `rmcp` facade (`analyze`, `remediate`, `igt`), a local CLI, and a Claude Code plugin that orchestrates the workflow without duplicating audit logic. Add remote persistence only after the local and CI contracts are stable.

**Tech Stack:** Rust 1.80+, Tokio, Serde/JSON, Axum, SQLx/PostgreSQL, Obscura/CDP, axe-core, `rmcp` stdio transport, Claude Code plugin skills/agents/hooks, GitHub Actions, SARIF, JUnit XML.

## Global Constraints

- The Rust audit engine is authoritative for findings and statuses.
- Claude explains, plans, and proposes changes; it cannot manufacture a passing result.
- Failed navigation, parsing, evaluation, or incomplete evidence produces an error or `NeedsReview`, never a clean audit.
- Source edits are proposal-first and require explicit approval.
- Local execution works without the remote service.
- Raw source upload and API-key logging are disabled by default.
- Every resolved finding has objective post-fix verification and an evidence trail.
- The first release targets RGAA 4.1.2 with WCAG 2.2 and EN 301 549 cross-references.
- Initial framework adapters cover React/Next.js, Vue, and Angular.
- `analyze` exposes viewport, selector, bounded pre-scan actions, screenshots, and explicit ruleset configuration.
- `remediate` accepts 1 to 25 issues and returns independent per-issue success or structured error results.
- `igt` reports issues, unanalyzed elements, termination reason, completed steps, evidence, and manual-review state.
- No unrestricted autonomous browsing or automatic fixes for ambiguous findings.
- Every task must preserve existing single-URL behavior unless the task explicitly changes the public contract and updates callers.

---

## Repository Map Before Implementation

Existing code to preserve and extend:

- `rgaa-rs/crates/rgaa-core/src/types.rs`: current `CriterionStatus`, `CriterionResult`, `PageResult`, and `AuditResult` models.
- `rgaa-rs/crates/rgaa-obscura/src/lib.rs`: Obscura server lifecycle, CDP targets, axe evaluation, batch execution, page context, and gap-fix execution.
- `rgaa-rs/crates/rgaa-orchestrator/src/pipeline.rs`: current single/batch pipeline and Holo3 evaluation fan-out.
- `rgaa-rs/crates/rgaa-rules/src/axe_mapper.rs`: axe violation mapping.
- `rgaa-rs/crates/rgaa-rules/src/gap_fix.rs`: deterministic gap-fix snippets and parsing.
- `rgaa-rs/crates/rgaa-api/src/main.rs`: current minimal Axum audit CRUD API.
- `rgaa-rs/crates/rgaa-storage`: current PostgreSQL repository and migration boundary.
- `docs/superpowers/specs/2026-08-18-claude-code-rgaa-remediation-plugin-design.md`: approved product and production requirements.

New boundaries:

- `rgaa-core`: durable audit/finding/evidence/checkpoint/policy contracts.
- `rgaa-obscura`: browser configuration, evidence capture, and guided-test execution.
- `rgaa-remediation`: finding fingerprints, remediation proposals, lifecycle, and framework adapter interfaces.
- `rgaa-mcp`: three agent-facing MCP tools backed by existing orchestrator/remediation services.
- `rgaa-cli`: local command-line entry point for Claude Code and CI.
- `claude-plugin/`: Claude Code manifest, skills, agents, hooks, MCP configuration, and project instructions.
- `rgaa-storage` and `rgaa-api`: optional remote bundle/history/policy service.

## Task 1: Define Versioned Audit and Finding Contracts

**Files:**
- Modify: `rgaa-rs/crates/rgaa-core/src/types.rs`
- Create: `rgaa-rs/crates/rgaa-core/src/audit_bundle.rs`
- Create: `rgaa-rs/crates/rgaa-core/src/findings.rs`
- Create: `rgaa-rs/crates/rgaa-core/src/evidence.rs`
- Create: `rgaa-rs/crates/rgaa-core/src/checkpoints.rs`
- Modify: `rgaa-rs/crates/rgaa-core/src/lib.rs`
- Test: `rgaa-rs/crates/rgaa-core/src/types.rs` and new module test blocks

**Interfaces:**
- Produces `CriterionStatus::{Pass, Fail, NotApplicable, Error, NeedsReview, NotTested}` with stable serde names.
- Produces `AuditBundle`, `AuditConfig`, `PageAudit`, `PageError`, `Finding`, `FindingFingerprint`, `EvidenceRef`, `CheckpointResult`, and `AuditSummary`.
- `AuditBundle::schema_version` is a string initialized to `"1.0"`.
- `FindingFingerprint::from_finding(&Finding) -> String` is deterministic for identical rule, URL, target, component path, and evidence inputs.
- `AuditBundle::validate(&self) -> Result<(), RgaaError>` rejects missing IDs, invalid statuses, duplicate finding IDs, and a `Pass` checkpoint with incomplete evidence.

- [ ] **Step 1: Write failing contract tests**

  Add tests for JSON round-trip, all six criterion statuses, deterministic fingerprints, duplicate-ID rejection, explicit page errors, and rejection of a successful result with missing required evidence.

- [ ] **Step 2: Run the focused tests and confirm they fail**

  Run: `cargo test -p rgaa-core`

  Expected: compilation failures for the new models and methods.

- [ ] **Step 3: Implement the models and validation**

  Keep existing `CriterionResult`, `PageResult`, and `AuditResult` fields source-compatible where possible. Add explicit serde names rather than relying on Rust variant spelling. Use `Option` for unavailable evidence and a typed error for invalid bundles; do not silently default malformed fields.

- [ ] **Step 4: Run the focused tests**

  Run: `cargo test -p rgaa-core`

  Expected: all contract tests pass.

- [ ] **Step 5: Commit**

  ```bash
  git add rgaa-rs/crates/rgaa-core
  git commit -m "feat: add versioned audit and finding contracts"
  ```

## Task 2: Add Production Analysis Configuration and Structured Obscura Results

**Files:**
- Create: `rgaa-rs/crates/rgaa-obscura/src/config.rs`
- Create: `rgaa-rs/crates/rgaa-obscura/src/results.rs`
- Modify: `rgaa-rs/crates/rgaa-obscura/src/lib.rs`
- Modify: `rgaa-rs/crates/rgaa-obscura/Cargo.toml`
- Test: `rgaa-rs/crates/rgaa-obscura/src/config.rs`
- Test: `rgaa-rs/crates/rgaa-obscura/src/results.rs`

**Interfaces:**
- `AnalyzeConfig` contains profile, viewport, optional selector, pre-scan actions, cookie references, screenshot policy, advanced-rule policy, needs-review policy, timeout, retry limit, and concurrency.
- `AnalyzeRequest { url: String, config: AnalyzeConfig }` validates URL, viewport range, selector length, action count, and timeout bounds.
- `AnalyzePageResult { url, findings, evidence, errors, completed, duration_ms }` preserves errors instead of dropping failed pages.
- `ObscuraError` is the typed `thiserror` error enum for process startup, CDP transport, navigation, evaluation, validation, timeout, and evidence failures; it replaces new `String` error boundaries while preserving existing methods until migration.
- `ObscuraBridge::analyze(&self, request: &AnalyzeRequest) -> Result<AnalyzePageResult, ObscuraError>` is the structured replacement for callers that currently consume raw `String` results.
- Existing `run_axe`, `run_axe_batch`, `extract_page_context`, and `run_gap_fix` remain available until all callers migrate.

- [ ] **Step 1: Write failing configuration and result tests**

  Cover default viewport `1000x1080`, mobile override, selector validation, bounded click/fill actions, secret-safe cookie representation, screenshot opt-in, and a failed URL represented as a page error.

- [ ] **Step 2: Run tests to verify the new contract is absent**

  Run: `cargo test -p rgaa-obscura`

  Expected: compilation failures for the new types and `analyze` method.

- [ ] **Step 3: Implement request validation and structured result conversion**

  Route the existing axe, gap-fix, and page-context paths through the request. Use `Result` for setup/navigation/evaluation failures. Preserve per-page errors in batch responses and make the `analyze` response serializable without exposing cookie values.

- [ ] **Step 4: Add lifecycle and failure regression tests**

  Assert that navigation failure, axe exception, invalid JSON, timeout, and target cleanup are represented as errors or incomplete results, never as empty/pass output.

- [ ] **Step 5: Run tests and commit**

  Run: `cargo test -p rgaa-obscura`

  ```bash
  git add rgaa-rs/crates/rgaa-obscura
  git commit -m "feat: add structured Obscura analysis contract"
  ```

## Task 3: Implement Reproducible Guided Tests and Evidence Capture

**Files:**
- Create: `rgaa-rs/crates/rgaa-obscura/src/guided.rs`
- Create: `rgaa-rs/crates/rgaa-obscura/src/evidence.rs`
- Modify: `rgaa-rs/crates/rgaa-obscura/src/lib.rs`
- Test: `rgaa-rs/crates/rgaa-obscura/tests/guided_tests.rs`

**Interfaces:**
- `GuidedTest { id, version, preconditions, steps, criterion_mapping, evidence_requirements }` is serde-compatible with the design YAML/JSON shape.
- `GuidedStep` supports `Navigate`, `AccessibilityTree`, `PressKey`, `ClickRef`, `FillRef`, `Screenshot`, and `AssertState`.
- `GuidedRunResult { issues, unanalyzed_elements, terminated_reason, completed_steps, evidence, manual_review_required }` always distinguishes incomplete coverage from pass.
- `ObscuraBridge::run_guided_test(&self, test: &GuidedTest) -> Result<GuidedRunResult, ObscuraError>` performs bounded action/verify loops.
- `EvidenceStore::write(&self, evidence: EvidenceArtifact) -> Result<EvidenceRef, ObscuraError>` writes crash-safe local evidence with content hashes.

- [ ] **Step 1: Write failing tests for action ordering and incomplete results**

  Use a fake CDP/action executor. Test successful keyboard flow, missing element reference, assertion failure, keyboard trap termination, timeout termination, and screenshot/evidence hash creation.

- [ ] **Step 2: Implement the typed workflow and fake executor seam**

  Keep browser operations behind a trait so tests do not require a live browser. Validate that every mutating action is followed by a state observation or assertion within the workflow.

- [ ] **Step 3: Implement Obscura-backed execution**

  Resolve stable accessibility-tree references, execute actions with bounded retries, stop on keyboard traps and navigation errors, and populate `unanalyzed_elements` for targets that could not be assessed.

- [ ] **Step 4: Add integration tests**

  Extend the existing Obscura integration setup with a keyboard/focus fixture and assert that the output contains action trace, screenshot or tree evidence, termination state, and criterion mapping.

- [ ] **Step 5: Run tests and commit**

  Run: `cargo test -p rgaa-obscura`

  ```bash
  git add rgaa-rs/crates/rgaa-obscura
  git commit -m "feat: add reproducible guided tests and evidence"
  ```

## Task 4: Create the Remediation and Finding-Lifecycle Crate

**Files:**
- Modify: `rgaa-rs/Cargo.toml`
- Create: `rgaa-rs/crates/rgaa-remediation/Cargo.toml`
- Create: `rgaa-rs/crates/rgaa-remediation/src/lib.rs`
- Create: `rgaa-rs/crates/rgaa-remediation/src/lifecycle.rs`
- Create: `rgaa-rs/crates/rgaa-remediation/src/proposals.rs`
- Create: `rgaa-rs/crates/rgaa-remediation/src/adapters.rs`
- Create: `rgaa-rs/crates/rgaa-remediation/src/policy.rs`
- Test: `rgaa-rs/crates/rgaa-remediation/src/*.rs`

**Interfaces:**
- `FindingState::{Open, Triaged, FixProposed, AwaitingApproval, Applied, Verifying, Resolved, NeedsReview, NotApplicable, FalsePositive, Deferred}`.
- `FindingLifecycle::transition(&mut self, next: FindingState, actor: &str, reason: &str) -> Result<(), RemediationError>` rejects invalid transitions and records history.
- `RemediationIssue { id, rule, element_html, page_url, source_locations, summary, remediation, criteria }` is the 1-to-25 MCP input model.
- `RemediationOutcome::{Ok(RemediationGuidance), Error(RemediationErrorInfo)}` is independent per issue.
- `PatchProposal { proposal_id, finding_ids, diff, files, rationale, risks, validation_commands, expected_effect, proposal_hash }` is never directly applied by this crate.
- `FrameworkAdapter` exposes `detect`, `locate`, and `propose` for React/Next.js, Vue, and Angular adapters.
- `RemediationPolicy` controls remote AI, allowed frameworks, maximum batch size, and approval requirements.

- [ ] **Step 1: Write lifecycle, fingerprint, batch, and policy tests**

  Test valid/invalid transitions, independent success/error outcomes, batch limits 1 and 25, rejection of 0 and 26 issues, proposal hash stability, and policy denial of remote remediation.

- [ ] **Step 2: Implement pure contracts and transitions**

  Keep patch application out of the crate. Return typed errors with safe messages and preserve all input correlation IDs.

- [ ] **Step 3: Implement framework adapter detection and fixture proposals**

  Add fixture projects under `rgaa-rs/crates/rgaa-remediation/tests/fixtures/{react,next,vue,angular}`. Start with high-confidence patterns: missing image alternative text, unlabeled form controls, and missing button names. Ambiguous cases must return `NeedsReview`.

- [ ] **Step 4: Run tests and commit**

  Run: `cargo test -p rgaa-remediation`

  ```bash
  git add rgaa-rs/Cargo.toml rgaa-rs/crates/rgaa-remediation
  git commit -m "feat: add remediation lifecycle and framework adapters"
  ```

## Task 5: Add the Three-Tool `rgaa-mcp` Server

**Files:**
- Modify: `rgaa-rs/Cargo.toml`
- Create: `rgaa-rs/crates/rgaa-mcp/Cargo.toml`
- Create: `rgaa-rs/crates/rgaa-mcp/src/lib.rs`
- Create: `rgaa-rs/crates/rgaa-mcp/src/server.rs`
- Create: `rgaa-rs/crates/rgaa-mcp/src/tools/analyze.rs`
- Create: `rgaa-rs/crates/rgaa-mcp/src/tools/remediate.rs`
- Create: `rgaa-rs/crates/rgaa-mcp/src/tools/igt.rs`
- Create: `rgaa-rs/crates/rgaa-mcp/src/main.rs`
- Test: `rgaa-rs/crates/rgaa-mcp/tests/contract.rs`

**Interfaces:**
- Use `rmcp` tool routing with `schemars::JsonSchema` input/output types and Tokio stdio transport.
- `analyze(AnalyzeRequest) -> AnalyzeResponse` delegates to the orchestrator/Obscura structured analysis path.
- `remediate(RemediationRequest) -> RemediationResponse` accepts 1 to 25 issues and returns one outcome per input ID.
- `igt(GuidedTestRequest) -> GuidedTestResponse` delegates to `ObscuraBridge::run_guided_test`.
- MCP errors must use stable codes and never expose secrets, raw cookie values, or full source files unless explicitly included in a remediation request.

- [ ] **Step 1: Write MCP contract tests without a live browser**

  Assert tool names and JSON schemas, required fields, batch limits, independent per-issue errors, malformed input handling, and secret redaction.

- [ ] **Step 2: Implement the `rmcp` server router**

  Use the current macro-based tool router or trait-based async tools. Keep each tool in its own module and inject an application service so the transport layer does not own browser or storage state.

- [ ] **Step 3: Add local stdio executable behavior**

  Read configuration from environment and project config, write protocol messages only to stdout, and send logs to stderr through `tracing`.

- [ ] **Step 4: Run contract tests and commit**

  Run: `cargo test -p rgaa-mcp`

  ```bash
  git add rgaa-rs/Cargo.toml rgaa-rs/crates/rgaa-mcp
  git commit -m "feat: add Deque-compatible accessibility MCP server"
  ```

## Task 6: Add a Stable Local CLI for Claude Code and CI

**Files:**
- Modify: `rgaa-rs/Cargo.toml`
- Create: `rgaa-rs/crates/rgaa-cli/Cargo.toml`
- Create: `rgaa-rs/crates/rgaa-cli/src/main.rs`
- Create: `rgaa-rs/crates/rgaa-cli/src/config.rs`
- Create: `rgaa-rs/crates/rgaa-cli/src/commands/analyze.rs`
- Create: `rgaa-rs/crates/rgaa-cli/src/commands/igt.rs`
- Create: `rgaa-rs/crates/rgaa-cli/src/commands/verify.rs`
- Create: `rgaa-rs/crates/rgaa-cli/src/commands/report.rs`
- Create: `rgaa-rs/crates/rgaa-cli/src/commands/policy.rs`
- Create: `.rgaa/config.yaml`
- Test: `rgaa-rs/crates/rgaa-cli/tests/cli_contract.rs`

**Interfaces:**
- Commands: `rgaa audit analyze`, `rgaa audit igt`, `rgaa audit verify`, `rgaa audit report`, and `rgaa audit policy`.
- Every command accepts `--config`, `--output`, `--format`, and `--audit-id` where applicable.
- Formats: `json`, `markdown`, `sarif`, and `junit`.
- Exit codes: `0` policy pass, `1` policy failure, `2` invalid configuration/input, `3` execution/infrastructure error.
- `.rgaa/config.yaml` configures URL profiles, viewport profiles, guided tests, standards, policy, evidence directory, remote endpoint, and upload consent.

- [ ] **Step 1: Write CLI parsing and exit-code tests**

  Test default config resolution, explicit config path, format selection, invalid YAML, missing URL, policy pass, policy failure, and infrastructure failure.

- [ ] **Step 2: Implement CLI configuration and command dispatch**

  Use a typed configuration model and return errors instead of falling back to a clean empty bundle. Print human-readable summaries to stderr/stdout only outside machine formats.

- [ ] **Step 3: Implement report serializers**

  Add JSON bundle passthrough, SARIF rule/result locations, JUnit test cases for findings/checkpoints, and Markdown grouped by severity/component.

- [ ] **Step 4: Run tests and commit**

  Run: `cargo test -p rgaa-cli`

  ```bash
  git add rgaa-rs/Cargo.toml rgaa-rs/crates/rgaa-cli .rgaa/config.yaml
  git commit -m "feat: add local audit CLI and report formats"
  ```

## Task 7: Package the Claude Code Plugin

**Files:**
- Create: `claude-plugin/.claude-plugin/plugin.json`
- Create: `claude-plugin/.mcp.json`
- Create: `claude-plugin/skills/audit/SKILL.md`
- Create: `claude-plugin/skills/triage/SKILL.md`
- Create: `claude-plugin/skills/remediate/SKILL.md`
- Create: `claude-plugin/skills/verify/SKILL.md`
- Create: `claude-plugin/skills/report/SKILL.md`
- Create: `claude-plugin/skills/guided-test/SKILL.md`
- Create: `claude-plugin/agents/scanner.md`
- Create: `claude-plugin/agents/remediation-planner.md`
- Create: `claude-plugin/agents/verification-reviewer.md`
- Create: `claude-plugin/agents/compliance-report-writer.md`
- Create: `claude-plugin/hooks/hooks.json`
- Create: `claude-plugin/scripts/check-runtime.sh`
- Create: `claude-plugin/README.md`
- Test: `claude-plugin/tests/plugin-contract.sh`

**Interfaces:**
- The plugin bundles or resolves the `rgaa-mcp` executable through `${CLAUDE_PLUGIN_ROOT}`.
- Skills call only `analyze`, `remediate`, and `igt` for audit operations, then use Claude Code's normal read/edit/test tools under approval rules.
- `remediate` must display proposal hash, files, diff, rationale, risk, and validation commands before applying any edit.
- `verify` must call the CLI or MCP verification path and cannot mark a finding resolved from source inspection alone.
- Hooks mark audit state stale after `Edit|Write` and never mutate files.

- [ ] **Step 1: Write plugin contract tests**

  Verify manifest fields, skill/agent files, `.mcp.json` executable path, hook matchers, no API keys in tracked files, and valid JSON/YAML front matter.

- [ ] **Step 2: Implement plugin skills and agent prompts**

  Include explicit failure semantics, approval gates, evidence requirements, and commands for local/offline mode. Keep prompts concise enough that the structured MCP responses remain the source of truth.

- [ ] **Step 3: Implement hooks and runtime checks**

  Use `SessionStart` to detect framework/configuration and `PostToolUse` for `Edit|Write` to mark affected findings stale. Use `${CLAUDE_PROJECT_DIR}` and `${CLAUDE_PLUGIN_ROOT}` paths; never use hardcoded machine paths.

- [ ] **Step 4: Run plugin contract tests and commit**

  Run: `bash claude-plugin/tests/plugin-contract.sh`

  ```bash
  git add claude-plugin
  git commit -m "feat: package Claude Code accessibility plugin"
  ```

## Task 8: Implement Policy, Baselines, Deduplication, and CI Integration

**Files:**
- Modify: `rgaa-rs/crates/rgaa-remediation/src/policy.rs`
- Create: `rgaa-rs/crates/rgaa-remediation/src/dedup.rs`
- Create: `rgaa-rs/crates/rgaa-remediation/src/baseline.rs`
- Create: `.github/workflows/rgaa-audit.yml`
- Create: `.github/actions/rgaa-audit/action.yml`
- Create: `docs/rgaa-ci.md`
- Test: `rgaa-rs/crates/rgaa-remediation/src/*.rs`
- Test: `.github/tests/policy-fixtures.sh`

**Interfaces:**
- `Policy::evaluate(&self, current: &AuditBundle, baseline: Option<&AuditBundle>) -> PolicyResult`.
- `PolicyResult { passed, failures, warnings, counts }` maps to the CLI exit codes.
- `Deduplicator::normalize(&[Finding]) -> Vec<FindingGroup>` groups repeated rule/target/component findings without merging distinct evidence.
- `Baseline::compare(previous, current) -> BaselineDiff` identifies new, resolved, unchanged, reopened, suppressed, and expired-suppression findings.
- Suppressions require finding fingerprint, reason, actor, created time, and expiration time.

- [ ] **Step 1: Write failing policy/baseline/dedup tests**

  Cover new critical failure, known finding allowance, needs-review allowance, expired suppression, reopen-on-regression, duplicate component findings, and no-baseline behavior.

- [ ] **Step 2: Implement pure evaluation and comparison**

  Keep policy deterministic and independent of Claude/Holo3. A missing bundle or infrastructure error must fail closed rather than produce a pass.

- [ ] **Step 3: Add GitHub Actions modes**

  Configure changed, smoke, full, diff, and policy modes. Upload JSON/SARIF/JUnit/Markdown artifacts and make the configured policy exit code the job result.

- [ ] **Step 4: Run tests and commit**

  Run: `cargo test -p rgaa-remediation` and `bash .github/tests/policy-fixtures.sh`

  ```bash
  git add rgaa-rs/crates/rgaa-remediation .github docs/rgaa-ci.md
  git commit -m "feat: add audit policy baselines and CI integration"
  ```

## Task 9: Add Optional Remote Bundle and Team History Support

**Files:**
- Modify: `rgaa-rs/crates/rgaa-storage/migrations/001_initial_schema.sql`
- Create: `rgaa-rs/crates/rgaa-storage/migrations/002_audit_bundles_findings.sql`
- Modify: `rgaa-rs/crates/rgaa-storage/src/repository.rs`
- Create: `rgaa-rs/crates/rgaa-storage/src/bundles.rs`
- Modify: `rgaa-rs/crates/rgaa-api/src/main.rs`
- Create: `rgaa-rs/crates/rgaa-api/src/bundles.rs`
- Create: `rgaa-rs/crates/rgaa-api/src/auth.rs`
- Test: `rgaa-rs/crates/rgaa-storage/tests/bundles.rs`
- Test: `rgaa-rs/crates/rgaa-api/tests/bundles.rs`

**Interfaces:**
- `Repository::put_bundle(&self, bundle: &AuditBundle) -> Result<(), StorageError>` is idempotent on `audit_id` and schema version.
- `Repository::get_bundle(&self, audit_id: Uuid) -> Result<Option<AuditBundle>, StorageError>`.
- `Repository::list_findings(&self, project_id, filters) -> Result<Vec<FindingSummary>, StorageError>`.
- API endpoints: `POST /v1/audit-bundles`, `GET /v1/audit-bundles/{id}`, `GET /v1/findings`, and `POST /v1/policy/evaluate`.
- Authentication is required for remote endpoints; unauthenticated local CLI mode remains supported.

- [ ] **Step 1: Write migration/repository tests**

  Test idempotent upload, duplicate finding preservation, bundle retrieval, filtering by status/severity/owner, and retention deletion.

- [ ] **Step 2: Add schema and repository methods**

  Store normalized metadata, compressed evidence references, finding lifecycle history, ownership, suppression reason/expiry, and bundle hashes. Do not store raw source or secret cookie values.

- [ ] **Step 3: Add authenticated Axum endpoints**

  Validate bundle schema before persistence, enforce request size limits, return stable error codes, and avoid `CorsLayer::allow_origin(Any)` for authenticated production routes.

- [ ] **Step 4: Run storage/API tests and commit**

  Run: `cargo test -p rgaa-storage -p rgaa-api`

  ```bash
  git add rgaa-rs/crates/rgaa-storage rgaa-rs/crates/rgaa-api
  git commit -m "feat: add optional remote audit bundle service"
  ```

## Task 10: End-to-End Verification, Security, and Release Packaging

**Files:**
- Create: `rgaa-rs/tests/full_remediation_loop.rs`
- Create: `rgaa-rs/tests/security_contract.rs`
- Create: `claude-plugin/tests/e2e-local.sh`
- Create: `docs/rgaa-plugin-install.md`
- Modify: `README.md`
- Modify: `rgaa-rs/README.md`
- Modify: `rgaa-rs/Cargo.toml`

**Interfaces:**
- The end-to-end loop is `analyze -> triage -> remediate -> approve -> apply -> test -> verify -> report`.
- Test fixtures include one React/Next.js, one Vue, and one Angular application with deterministic findings and expected patches.
- Security assertions cover no secrets in logs, no cookie values in serialized requests, remote upload opt-in, request-size limits, and safe error messages.

- [ ] **Step 1: Write the failing end-to-end and security tests**

  Assert that an unresolved finding cannot be marked resolved, an incomplete IGT cannot pass, a failed page remains in the bundle, an approved proposal applies only its recorded files, and a re-audit is required before resolution.

- [ ] **Step 2: Implement only the integration fixes exposed by those tests**

  Do not weaken the contract to make a test pass. Fix the owning crate and update its focused test when a boundary defect is found.

- [ ] **Step 3: Run the full verification suite**

  ```bash
  cargo fmt --all -- --check
  cargo test --workspace
  cargo clippy --workspace --all-targets -- -D warnings
  bash claude-plugin/tests/plugin-contract.sh
  bash claude-plugin/tests/e2e-local.sh
  ```

  Expected: all commands exit 0. Live Holo3 and remote-service tests must be explicitly marked integration tests and skipped unless credentials/configuration are present; they must never be treated as passing offline tests.

- [ ] **Step 4: Document install, local/offline mode, remote consent, and CI setup**

  Include exact commands for installing the plugin, locating the Obscura binary/worker, configuring the existing `HOLO3_API_KEY` and remote credentials through environment variables, running a first local audit, approving a remediation, and exporting SARIF/JUnit.

- [ ] **Step 5: Commit the release readiness changes**

  ```bash
  git add rgaa-rs/tests claude-plugin/tests docs README.md rgaa-rs/README.md rgaa-rs/Cargo.toml
  git commit -m "test: verify end-to-end RGAA remediation workflow"
  ```

## Execution Order and Review Gates

Implement tasks in order. Each task must pass its focused tests before the next task starts. Task 9 may be deferred after Task 8 if the local plugin and CI release is the immediate goal, but its bundle contract must remain compatible with the remote schema.

After Task 5, manually exercise the MCP server over stdio with one successful `analyze`, one failed URL, one 25-issue `remediate`, one invalid 26-issue request, and one incomplete `igt`. After Task 7, install the plugin into a clean Claude Code session and confirm the three MCP tools are visible. After Task 10, run the full verification suite on a clean checkout.

## Spec Coverage Checklist

- Audit/finding/evidence schemas: Task 1.
- Viewports, selectors, actions, screenshots, errors, and partial results: Task 2.
- Guided tests, stable refs, incomplete coverage, and evidence: Task 3.
- Finding lifecycle, batch remediation, framework adapters, and approval proposals: Task 4.
- Deque-compatible `analyze`, `remediate`, and `igt` MCP facade: Task 5.
- Local/offline CLI and JSON/SARIF/Markdown/JUnit: Task 6.
- Claude Code skills, agents, hooks, and plugin packaging: Task 7.
- Baselines, deduplication, suppressions, ownership, and CI policy: Task 8.
- Optional remote history, authentication, retention, and team API: Task 9.
- End-to-end loop, security, documentation, and release verification: Task 10.
