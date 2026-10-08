---
name: guided-test
description: Run or plan bounded keyboard accessibility checks and explain which results need human review, using the keyboard IGT support in the RGAA MCP server.
---

# Guided accessibility test

The server's standalone `igt` tool is deprecated. Prefer `analyze` with
`config.igt_tools: ["keyboard"]` when a keyboard check is needed.

## Workflow

1. Confirm the target URL and the interaction or criterion to check.
2. Call `analyze` with the target URL and keyboard IGT configuration.
3. Report the returned status, completed steps, issues, termination reason,
   and evidence. A stopped or incomplete run is not a pass.
4. Explain which observations require a human to finish the assessment,
   including real assistive-technology behavior that browser automation cannot
   establish.
5. Do not invent reproducible test steps or report visual checks as completed
   unless the tool captured them.

If the MCP tool is unavailable, provide a clearly labelled manual checklist
instead of claiming that the test ran.
