---
name: audit
description: Audit a URL or local web project for RGAA accessibility, using the MCP audit tools and reporting findings with their evidence and review status.
---

# RGAA audit

Use the `rgaa-mcp` tools as the source of audit results. Do not infer a pass
from missing data, a tool error, or an incomplete response.

## Workflow

1. Resolve the target. Ask for a URL if none is available. For local source,
   use `lint_static` as an initial check and explain that it is not a rendered
   conformance audit.
2. For one page, call `analyze` with its URL and any requested scope or
   viewport settings.
3. For a site crawl, call `audit_url`. It returns a summary and
   `sampled_page_urls`, not per-criterion results. Call `analyze` for the
   sampled pages when the user needs detailed findings.
4. Use `list_criteria` when criterion titles or classifications are needed.
5. Present the conformity rate only when supplied by the tool. Summarize pass,
   fail, `NeedsReview`, `NotTested`, and error/incomplete states separately.
   Keep evidence and URLs attached to their findings.
6. Offer triage, remediation proposals, or a report when useful.

## Guardrails

- `NeedsReview` means a human must assess the criterion; do not count it as a
  pass.
- State the difference between a site-level `audit_url` summary and detailed
  results from `analyze`.
- Do not claim all 106 criteria were tested unless the result explicitly
  supports that claim.
- Do not edit project files as part of an audit.
