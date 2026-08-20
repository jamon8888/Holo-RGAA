---
name: verify
description: "Verify remediation effectiveness by re-running audit"
version: 0.1.0
author: RGAA Team
requires:
  - remediate
  - audit
mode-default: suggest
---

# verify — Remediation Verification

## Overview
This skill verifies that applied remediations actually resolve the original accessibility findings. It re-runs the audit (or targeted guided tests) and confirms findings are resolved with objective evidence.

## Inputs
- `remediation_plan` (RemediationPlan, required) — Output from `remediate` skill with applied proposals.
- `audit_bundle` (AuditBundle, required) — Original audit baseline.
- `approvals` (map<proposal_id, token>, required) — Approval tokens for applied proposals.

## Workflow
1. **Validate approvals** — For each applied proposal, call `ensure_approved(token)` to confirm valid approval.
2. **Re-run audit** — Execute targeted re-analysis:
   - For deterministic findings: full re-audit or guided test (`igt` tool).
   - For Ia-Assisté findings: human review checklist + targeted `igt`.
3. **Compare findings** — Match re-audit findings against original by fingerprint.
4. **Produce verification report** — Per finding: resolved (pass), still failing, new regression, evidence.

## Outputs
- `VerificationReport` with:
  - `resolved`: findings that now pass with evidence.
  - `unresolved`: findings still failing with updated evidence.
  - `regressions`: new findings introduced.
  - `needs_review`: Ia-Assisté findings awaiting human confirmation.
  - Compliance delta: before/after compliance rate.

## Constraints
- Verification MUST use objective re-audit, not source inspection alone.
- Every resolved finding MUST have new evidence (screenshot/DOM snapshot).
- Approval tokens MUST be validated before reporting resolved.
- Proposal hash MUST match at verification time.

## Failure Modes
- Missing approval token → exit code 2 (`POLICY_DENIED`).
- Evidence missing for resolved finding → marked `needs_review`.
- Regression detected → flagged in report.