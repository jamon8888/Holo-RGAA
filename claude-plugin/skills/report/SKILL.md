---
name: report
description: "Generate compliance reports in multiple formats"
version: 0.1.0
author: RGAA Team
requires:
  - audit
  - verify
mode-default: suggest
---

# report — Compliance Report Generator

## Overview
This skill generates formal compliance reports from audit, triage, remediation, and verification results. Supports JSON, Markdown, SARIF, and JUnit formats for CI integration and stakeholder delivery.

## Inputs
- `audit_bundle` (AuditBundle, required) — Baseline audit.
- `triage_report` (TriageReport, optional) — Triage context.
- `remediation_plan` (RemediationPlan, optional) — Proposals.
- `verification_report` (VerificationReport, optional) — Verification results.
- `format` (string, default: json) — Output: json, markdown, sarif, junit.
- `output` (path, optional) — Write to file.
- `audit_id` (string, optional) — Override audit ID.

## Workflow
1. **Aggregate data** — Combine audit, triage, remediation, verification into a unified report model.
2. **Compute metrics** — Overall compliance, per-criterion status, remediation coverage, verification delta.
3. **Render format**:
   - **JSON** — Full structured bundle (schema version "1.0").
   - **Markdown** — Human-readable: executive summary, findings table, remediation status, compliance delta.
   - **SARIF 2.1.0** — Rules + results for IDE/SIEM ingestion.
   - **JUnit XML** — Test cases per finding for CI dashboards.
3. **Write output** — File or stdout.

## Outputs
- Report file/stdout in requested format.
- Machine-readable formats (JSON, SARIF, JUnit) contain full structured data for automation.

## Constraints
- JSON output MUST conform to AuditBundle schema ("1.0").
- SARIF MUST include rule definitions and result locations.
- JUnit MUST map findings to test cases with pass/fail/error.
- Report MUST include fingerprint, evidence refs, approval tokens for traceability.

## Failure Modes
- Unsupported format → exit code 2.
- Output path unwritable → exit code 3.