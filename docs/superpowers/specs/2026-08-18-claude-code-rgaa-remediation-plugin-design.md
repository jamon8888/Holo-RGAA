# Claude Code RGAA Remediation Plugin

**Date:** 2026-08-18
**Status:** Approved design, production-deepened
**Scope:** Hybrid local-first accessibility auditing, remediation, verification, CI, and optional team service

## Objective

Provide a Deque-like accessibility workflow inside Claude Code without making Claude the source of truth for audit results. Developers should be able to run an audit, understand findings, approve source patches, re-run focused checks, and export defensible evidence. The first release targets RGAA 4.1.2 with WCAG 2.2 and EN 301 549 cross-references.

## Product Principles

- The Rust audit engine is authoritative for findings and statuses.
- Claude explains, plans, and proposes changes; it cannot manufacture a passing result.
- Failed navigation, parsing, evaluation, or incomplete evidence produces an error or `NeedsReview`, never a clean audit.
- Source edits are proposal-first and require explicit approval.
- Local execution works without the remote service.
- Raw source upload and API-key logging are disabled by default.
- Every resolved finding has objective post-fix verification and an evidence trail.

## Architecture

### Local engine

Extend the existing Rust workspace rather than duplicating audit logic in the plugin:

- `rgaa-core`: criteria catalog, statuses, stable finding identity, evidence models, and versioned audit bundles.
- `rgaa-rules`: axe mapping and deterministic remediation guidance.
- `rgaa-obscura`: browser/CDP lifecycle, screenshots, accessibility tree, keyboard interactions, and guided-test execution.
- `rgaa-holo`: optional model-assisted visual and interaction evaluation with bounded retries and explicit uncertainty.
- `rgaa-orchestrator`: multi-URL execution, deterministic checks, assisted criteria, diffing, and bundle generation.
- New CLI/API contract: machine-readable, versioned JSON bundles suitable for both Claude Code and CI.

### Claude Code plugin

Use Claude Code's supported plugin components: skills, agents, hooks, and a bundled MCP server.

Suggested commands:

- `/rgaa:audit`: detect project configuration and run a local or remote audit.
- `/rgaa:triage`: deduplicate, rank, and explain findings.
- `/rgaa:remediate`: inspect source and produce an approval-gated patch proposal.
- `/rgaa:verify`: run focused tests and re-audit affected scope.
- `/rgaa:report`: generate developer, CI, and compliance outputs.
- `/rgaa:guided-test`: run a named reproducible interaction workflow.

The bundled MCP server exposes a small Deque-compatible facade. Lower-level operations remain internal to the plugin or Rust service so the agent has a predictable tool surface:

- `analyze`: scan a URL or scoped page region and return structured findings.
- `remediate`: generate independent remediation guidance for a batch of findings.
- `igt`: run a named Intelligent Guided Test and return findings plus incomplete-test information.

Internal operations such as finding lookup, approval, patch application, verification, evidence export, and policy evaluation are orchestrated by skills and service code rather than exposed as separate agent-facing tools.

Agents are separated by responsibility: scanner, remediation planner, verification reviewer, and compliance report writer. Hooks are advisory and must not silently edit source:

- `SessionStart`: detect framework and load project audit configuration.
- `PostToolUse` for `Edit|Write`: mark affected findings stale and optionally suggest verification.
- `Stop`: remind the developer about accepted but unverified findings.

### Optional remote service

The remote service stores normalized audit bundles and supports team history, ownership, trends, and CI orchestration. It should receive bundle metadata and evidence references by default; raw source upload is opt-in. The local plugin remains fully functional when the service is unavailable.

## Production MCP Contract

### Analysis configuration

Every `analyze` request resolves an explicit, persisted configuration:

- Accessibility standard/profile and WCAG level.
- axe-core and ruleset versions.
- Viewport width and height.
- Optional CSS selector scope for component or partial-page scans.
- Bounded pre-scan actions such as click and fill.
- Authenticated cookie profile with secret values redacted from logs.
- Screenshot policy.
- Advanced-rule and visual-evaluation policy.
- Needs-review and best-practice policy.
- Per-page timeout, retry, and concurrency limits.

The default scan viewport is 1000x1080. Projects may override it and define mobile, tablet, and desktop profiles. A failed page must return an explicit page error and preserve completed pages; it must never disappear from a batch result.

### Remediation contract

`remediate` accepts 1 to 25 issues per request. Each issue includes a caller-owned correlation ID, rule, element HTML, page URL, source locations when available, summary, and remediation context. Results are independent so one model failure does not discard other guidance.

Successful results contain a general explanation, ordered remediation steps, concrete code guidance, confidence, and affected criteria. Failed results contain a stable error code and safe diagnostic message such as `INSUFFICIENT_CONTEXT`, `UNSUPPORTED_FRAMEWORK`, `POLICY_DENIED`, or `MODEL_FAILURE`. The service must never return a successful empty remediation.

The plugin presents remediation as a patch proposal with a diff preview. Applying it requires explicit approval and must record the proposal hash, approver, files, and resulting commit or worktree state.

### Guided-test contract

`igt` runs a named workflow against a URL or configured component state. The response contains:

- `issues`: confirmed findings.
- `unanalyzed_elements`: targets the workflow could not assess.
- `terminated_reason`: why execution stopped early, such as `keyboard-trap`, timeout, navigation failure, or browser error.
- `completed_steps`: the reproducible action trace.
- `evidence`: screenshots, accessibility trees, and state assertions.
- `manual_review_required`: whether a human must complete the checkpoint.

An incomplete guided test is never represented as `Pass`. Multiple runs of the same guided test are retained and deduplicated only during result normalization.

## Audit and Remediation Flow

Each finding is persisted as a remediation record containing:

- Stable fingerprint derived from rule, normalized target, URL, component path, and evidence hash.
- RGAA criterion, WCAG success criterion, and EN 301 549 references.
- Severity, confidence, detection source, source locations, and affected component.
- Evidence references: screenshot, DOM/accessibility snapshot, action trace, and result payload.
- Lifecycle status.

Lifecycle:

`Open -> Triaged -> Fix Proposed -> Awaiting Approval -> Applied -> Verifying -> Resolved`

Alternative terminal or review states are `NeedsReview`, `NotApplicable`, `FalsePositive`, and `Deferred`.

Workflow:

1. Run a local or remote audit and persist an audit ID.
2. Group duplicate findings by root cause and component, then rank by impact, confidence, affected pages, and fix leverage.
3. Inspect source, tests, framework conventions, and evidence before proposing a fix.
4. Present the patch, files, rationale, risk, expected audit effect, and validation commands.
5. Apply only after explicit approval, preferably in an isolated worktree or patch transaction.
6. Run focused tests and the relevant audit scope.
7. Compare before/after fingerprints and close the finding only when the original issue is objectively gone.

Framework adapters initially cover React/Next.js, Vue, and Angular. The audit engine remains framework-neutral.

## Checkpoint and Audit-Record Model

Manual and assisted testing use an Auditor-style checkpoint record:

- Criterion and checkpoint ID.
- Testing methodology and instructions.
- Result: `Pass`, `Fail`, `NotApplicable`, `NeedsReview`, or `NotTested`.
- Impact and issue type: accessibility, best practice, user agent, functionality, or usability.
- Summary, details, remediation advice, and affected instance.
- Evidence attachments and provenance.
- Reviewer, approval, publication, and change history.

Automated and guided-test findings can be imported into a checkpoint run. Manual issues can be added without browser automation, allowing criteria that cannot be proven by code or browser instrumentation to remain part of the same system of record.

## Guided Tests

Interaction and manual-assist checks use named, reproducible workflows rather than unconstrained agent exploration:

```yaml
guided_test_id: keyboard-dialog-submit
preconditions: [route-loaded]
steps:
  - navigate: /checkout
  - accessibility_tree: true
  - press_key: Tab
  - click_ref: submit-button
  - screenshot: after-submit
  - assert_state: dialog-visible
criterion_mapping: [RGAA-7.3, WCAG-2.1.1]
evidence_requirements: [accessibility_tree, action_trace, screenshot]
```

Browser actions use stable accessibility-tree references and are followed by state verification. Stale or failed actions are retried only within a bounded policy; they never silently pass.

## CI and Policy

Supported modes:

- `audit changed`: changed routes or components.
- `audit smoke`: configured critical routes.
- `audit full`: all configured URLs and guided tests.
- `audit diff`: comparison against a baseline bundle.
- `audit policy`: evaluate thresholds and exit status.

Example policy:

```yaml
policy:
  fail_on: [new_critical, new_serious, unresolved_regression]
  allow:
    known_findings: true
    needs_review: true
  require:
    evidence_for: [pass, fail, not_applicable]
```

The same versioned bundle must work from Claude Code, local CLI, and CI. Outputs include JSON, SARIF, Markdown, and JUnit-compatible summaries.

## Reliability and Security Requirements

- Versioned schemas with compatibility negotiation.
- Idempotent audit and upload IDs.
- Cancellation and bounded per-page timeouts.
- Retry classification for transient browser, network, and rate-limit failures.
- Partial-result preservation for batch audits.
- Crash-safe evidence writes and deterministic guided-test replay.
- Structured tracing, health checks, and actionable error codes.
- Stable finding deduplication across repeated scans, pages, components, and guided-test runs.
- Baselines, suppressions with expiration and justification, ownership, reopen-on-regression, and review history.
- Local browser execution by default; explicit consent before remote remediation or advanced visual evaluation.
- Redaction of cookies, passwords, tokens, personal data, and source secrets from logs and telemetry.
- No raw source persistence or upload by default.
- API-key and OAuth authentication for optional remote features.
- Organization controls for remote AI, advanced rules, retention, and deletion.

## Reports and Compliance Artifacts

- Developer report with source locations, rationale, and remediation proposals.
- SARIF for code-scanning systems.
- JUnit-compatible CI summary.
- RGAA conformity matrix with WCAG/EN 301 549 references.
- Accessibility statement inputs.
- Multi-year remediation plan.
- Derogation register.
- Evidence archive with hashes and provenance.

## Phased Delivery

### Phase 1: Local Claude Code plugin

Deliver the plugin manifest, skills, agents, three-tool MCP facade (`analyze`, `remediate`, `igt`), local Rust invocation, framework detection, versioned bundles, approval-gated patches, focused verification, and JSON/Markdown/SARIF output.

Acceptance criteria:

- A developer can install the plugin and run an audit from Claude Code.
- Findings have stable IDs, RGAA/WCAG mappings, source context, and evidence.
- Claude proposes a patch without applying it.
- Approval applies only the proposed patch.
- Verification runs tests and a re-audit.
- A finding closes only after objective verification.
- The workflow works without the remote service.
- `analyze` supports selector scope, viewport profiles, bounded pre-scan actions, screenshots, and explicit configuration.
- `remediate` supports independent batch results with structured errors and patch approval records.
- `igt` reports unanalyzed elements and termination reasons instead of silently passing incomplete coverage.

### Phase 2: Guided tests and evidence

Add accessibility-tree grounding, keyboard/focus workflows, screenshots, action traces, `NeedsReview` escalation, and regression detection.

### Phase 3: CI and policy

Add GitHub Actions integration, changed-route/full-audit modes, baseline comparison, SARIF/JUnit artifacts, and configurable failure policy.

### Phase 4: Optional remote service

Add authenticated bundle upload, project history, deduplication, team ownership/status, trends, remote CI orchestration, and retention/deletion controls.

### Phase 5: Compliance outputs

Add the conformity matrix, statement inputs, multi-year plan, derogation register, and evidence export lifecycle.

## Testing Strategy

- Unit tests for bundle schema, fingerprints, lifecycle transitions, policy evaluation, and framework adapters.
- MCP contract tests for every tool, including approval rejection and malformed input.
- Browser integration tests for navigation failure, evaluation failure, stable references, keyboard actions, cleanup, and evidence capture.
- Remediation fixtures for React/Next.js, Vue, and Angular with expected patches and re-audit results.
- End-to-end tests covering audit, proposal, approval, verification, and report generation.
- CI tests for baseline diffing, SARIF/JUnit validity, policy exit codes, and offline local mode.
- Security tests proving secrets, source, and browser content are not logged or uploaded by default.

## Risks and Boundaries

- Automated accessibility testing cannot prove every RGAA criterion; uncertain criteria must remain `NeedsReview`.
- Holo3 output is advisory and must be validated against browser state and evidence requirements.
- Remote synchronization must be idempotent and tolerate offline operation.
- Framework adapters must produce minimal, style-preserving patches and avoid broad rewrites.
- The initial release should not include a full hosted dashboard implementation, autonomous unrestricted browsing, or automatic fixes for ambiguous findings.

## Research Basis

- Claude Code Plugins Reference: https://code.claude.com/docs/en/plugins-reference
- Claude Code Hooks Reference: https://code.claude.com/docs/en/hooks
- Deque axe DevTools: https://www.deque.com/axe/devtools/
- Deque axe MCP Server: https://www.deque.com/axe/mcp-server/
- Existing repository architecture: `.superpowers/specs/2026-08-16-agentic-rgaa-auditor-architecture.md`
