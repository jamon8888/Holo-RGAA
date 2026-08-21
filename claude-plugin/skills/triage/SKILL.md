---
name: triage
description: "Triage and prioritize accessibility findings"
version: 0.1.0
author: RGAA Team
requires:
  - audit
mode-default: suggest
---

# triage — Finding Triage & Prioritization

## Overview
This skill filters, prioritizes, and categorizes accessibility findings from the audit phase. It applies RGAA classification (Deterministe/IaAssiste/Manuel) and severity, and produces a triage report for remediation planning.

## Inputs
- `audit_bundle` (AuditBundle, required) — Output from `audit` skill.
- `focus` (string, optional) — Filter: "critical", "deterministic", "ia_assiste", "manual".
- `max_findings` (integer, optional) — Limit number of findings to process.

## Workflow
1. **Load audit bundle** — Validate schema version, fingerprints, evidence completeness.
2. **Classify findings** — Map each finding to RGAA classification using `RgaaCriteria::deterministic()` / `ia_assiste()` / `criterion.classification`.
3. **Apply severity** — Critical (Fail + blocking), Major (Fail + non-blocking), Minor (NeedsReview), Info (NotApplicable).
4. **Deduplicate** — Merge findings with identical fingerprints across pages.
5. **Produce triage report** — Group by criterion, classification, severity.

## Outputs
- `TriageReport` with:
  - Findings grouped by classification (Deterministe, IaAssiste, Manuel).
  - Counts per severity.
  - Recommended remediation order (deterministic → ia_assiste → manual).
  - Evidence references preserved.

## Constraints
- Deterministic criteria MUST be addressed first (objective pass/fail).
- Ia-Assisté criteria require human review flag.
- Manuel criteria are documented but not auto-remediated.

## Failure Modes
- Malformed audit bundle → exit code 2.
- Missing evidence for passing deterministic criteria → flagged in report.