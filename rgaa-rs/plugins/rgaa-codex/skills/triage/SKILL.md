---
name: triage
description: Group and prioritize RGAA findings by criterion, severity, repeated root cause, affected pages, and likely remediation effort.
---

# Triage RGAA findings

Use the findings already present in the conversation or returned by the audit
tools. Preserve each finding's identifier, criterion, URL, status, and evidence.

## Workflow

1. Separate confirmed failures from `NeedsReview`, `NotTested`, and errors.
2. Group repeated instances by rule, criterion, component, or likely shared
   template. Do not merge findings when their evidence indicates distinct
   causes.
3. Rank confirmed issues by user impact, number of affected pages, confidence,
   and whether one shared correction can resolve several instances.
4. Mark effort as a qualitative estimate and state assumptions; do not present
   estimated effort as measured data.
5. Recommend whether each group needs source remediation, a new audit, or a
   human guided check.

## Output

Return a concise priority list with the affected findings, criteria, evidence,
reason for priority, and next action. Keep uncertain conclusions labelled for
review.
