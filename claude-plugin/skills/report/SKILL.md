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
- `format` (string, default: json) — Output: json, markdown, html, sarif, junit.
- `output` (path, optional) — Write to file.
- `audit_id` (string, optional) — Override audit ID.

## Workflow

1. **Aggregate data** — Combine audit, triage, remediation, verification into a unified report model.
2. **Delegate the metrics — never compute them here.** `rgaa-report` is the
   single source of truth for compliance computation; every caller
   (orchestrator, CLI, TUI) goes through it so one audit always yields the
   same figures. Call `rgaa_report::compute_metrics(criteria, referentiel)`
   (or `compliance_rate`) — or let `render` produce them — and quote what it
   returns. Do **not** recount passes and failures in the skill: a second
   arithmetic path is a second answer, and the one the plugin prints would
   silently disagree with the one the engine publishes.
3. **Render format** via `rgaa_report::render(bundle, format)`:
   - **JSON** — Full structured bundle (schema version "1.0").
   - **Markdown** — Executive summary, per-criterion `Sources`, findings table, remediation status, compliance delta.
   - **HTML** — Full 106-criterion table with status, detail, and `Sources`.
   - **SARIF 2.1.0** — Rules + results for IDE/SIEM ingestion.
   - **JUnit XML** — Test cases per finding for CI dashboards.
4. **Show the evidence** — Criteria whose verdict relied on retrieval carry
   citations (`CriterionResult::citations`). The Markdown `Sources` section
   and the HTML `Sources` column render them. A criterion with no citations
   is not missing evidence: deterministic rules, manual review and "not
   tested" reach their verdict without retrieval and correctly show `—`.
5. **Clear the legal gate before exporting** — see below.
6. **Write output** — File or stdout.

## Legal guardrails (minimum)

A report that leaves the plugin can be relied on by a third party. Before
exporting, the engine's guardrails decide whether it may:

- `rgaa_report::validate_export(bundle, pack)` — refuses the export outright
  when the sample is under `ECHANTILLON_MIN` (5 pages) without a
  small-site justification, when the feedback contact has no reachable
  destination, when a non-conformity lacks intitulé / URL / DOM-capture /
  recommandation, or when a dérogation lacks motif / alternative / réexamen.
  The first failure wins, with a precise message. **Do not route around it.**
- `rgaa_report::render_declaration_fr(&DeclarationFrInput)` — FR
  accessibility declaration.
- `rgaa_report::render_declaration_ue(&DeclarationUeInput)` — UE 2018/1523
  declaration.

If `validate_export` returns `Err`, surface the message and stop. Producing
the report anyway is the failure mode these guards exist to prevent.

### Out of scope (deliberately)

Country packs, PDF rendering, and report péremption are **not** part of this
skill's path. They exist in the engine (`packs`, `pdf`, `gouvernance`) and
are driven elsewhere; invoking them from here would duplicate a decision
this skill is not the owner of.

## Outputs
- Report file/stdout in requested format.
- Machine-readable formats (JSON, SARIF, JUnit) contain full structured data for automation.

## Constraints
- JSON output MUST conform to AuditBundle schema ("1.0").
- SARIF MUST include rule definitions and result locations.
- JUnit MUST map findings to test cases with pass/fail/error.
- Report MUST include fingerprint, evidence refs, approval tokens for traceability.
- Metrics MUST come from `rgaa-report`. The skill MUST NOT compute its own.
- A sourced criterion MUST show its citations; an unsourced one MUST NOT be
  padded with a fake source.
- An export MUST NOT be produced when `validate_export` refuses it.

## Failure Modes
- Unsupported format → exit code 2.
- Output path unwritable → exit code 3.
- `validate_export` refusal → exit code 2, with the guard's own message. Do
  not retry by relaxing the pack; fix the underlying gap (more pages, a
  reachable contact, the missing evidence) or stop.
