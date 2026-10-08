---
name: report
description: Turn RGAA audit results into a clear report for developers, clients, or CI while preserving evidence and distinguishing untested criteria.
---

# Report RGAA results

Use the audit tool output as the source of truth. Do not fill gaps by guessing
or describe an audit summary as a complete criterion-by-criterion report.

## Workflow

1. Reuse the available audit results. If the user needs saved output in a
   specific format and the MCP tools do not produce it, use the installed
   `rgaa` CLI only when available; do not imply the MCP server has a report
   tool.
2. Include target URLs, audit date if present, tool-reported conformity data,
   and counts/statuses that are actually available.
3. Separate confirmed failures, `NeedsReview`, `NotTested`, errors, and pages
   that could not be analyzed.
4. Include finding IDs, RGAA criteria, source locations, and evidence
   references when supplied. Redact secrets and personal data.
5. Label recommendations as recommendations, not audit findings.

Keep the report proportionate to its audience and state the scope and limits of
the source results.
