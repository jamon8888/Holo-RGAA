---
name: remediate
description: "Generate approval-gated remediation proposals for accessibility findings"
version: 0.1.0
author: RGAA Team
requires:
  - triage
  - rgaa-remediation
mode-default: suggest
---

# remediate — Remediation Proposal Generator

## Overview
This skill generates source-level remediation proposals for triaged accessibility findings. It uses the `rgaa-remediation` crate to produce framework-specific fixes with explicit approval gating.

## Inputs
- `triage_report` (TriageReport, required) — Output from `triage` skill.
- `framework` (string, optional) — Override auto-detected framework: react, next, vue, angular.
- `batch_size` (integer, optional, default: 10) — Number of findings per batch (1..25).
- `policy` (object, optional) — RemediationPolicy overrides: `require_approval`, `allow_remote_ai`, `allowed_frameworks`.

## Workflow
1. **Load triage report** — Extract findings marked for remediation.
2. **Detect framework** — Auto-detect from source code (React, Next, Vue, Angular) or use override.
3. **Batch findings** — Group 1..25 findings per remediation call.
4. **Generate proposals** — Call `rgaa-mcp` tool `remediate` (or `rgaa audit verify` CLI) per batch.
5. **Surface approval** — Each proposal includes:
   - `proposal_id`, `proposal_hash` (`rgaa-proposal-v1-<hex>`)
   - `diff`, `files`, `rationale`, `risks`, `validation_commands`
   - `approval_state` (Required/NotRequired) + `approval_token` (`rgaa-approval-v1-<id>-<hash>`)

## Outputs
- `RemediationPlan` with proposals per finding.
- Each proposal shows: issue ID, explanation, steps, confidence, criteria, diff, approval state/token.
- Agent MUST present proposal hash, files, diff, rationale, risks, validation commands before applying any edit.

## Approval Gates
- `require_approval: true` (default) → all proposals require explicit approval via `approve(token)`.
- `require_approval: false` → proposals with `NeedsReview` error still require review; safe fixes auto-approved.
- Agent CANNOT apply edits without user confirming approval token matches.

## Framework Adapters
- **React/Next** — JSX-aware, proposes `alt`, `aria-label`, semantic HTML.
- **Vue** — Template-aware, proposes `v-bind:alt`, ARIA attributes.
- **Angular** — Template + decorator aware, proposes property bindings, ARIA.
- **Ambiguous/Dynamic** → `NeedsReview` error, no unsafe fix proposed.

## Constraints
- Batch size 1..25 enforced.
- One outcome per input issue ID, correlation preserved.
- Proposals with `NeedsReview` MUST NOT be auto-applied.
- Proposal hash MUST be verified before approval.

## Failure Modes
- Batch > 25 → exit code 2 (`INVALID_INPUT`).
- Framework mismatch → `NeedsReview` error per issue.
- Missing approval → exit code 2 (`POLICY_DENIED`).