---
name: remediate
description: Prepare approval-gated source remediation proposals for RGAA findings, with evidence, source locations, rationale, risks, and validation guidance.
---

# Propose RGAA remediations

The `remediate` MCP tool produces guidance and patch proposals; it does not
apply changes to the repository. Keep the user's source untouched until the
user explicitly approves a specific change.

## Workflow

1. Select findings the user asked to address. The tool accepts 1–25 findings
   per call; split larger sets into batches.
2. If source locations are missing and a project root is available, call
   `source_map`. Treat its matches as best-effort: check `confidence`,
   `matched_on`, and any `unmappable` result before proposing an edit.
3. Call `remediate` with each issue's ID, rule, page URL, element HTML,
   summary, remediation context, criteria, and any known source locations and
   framework. The server accepts React, Next, Vue, and Angular framework names.
4. Present each proposal's diff, rationale, risks, confidence, and validation
   commands. If the tool returns an error or insufficient context, report it
   rather than inventing a patch.
5. Ask for explicit approval before applying any proposal. Approval of one
   proposal does not authorize unrelated changes.
6. After an approved change, run the relevant project validation and use
   `verify_fix` with the reference audit and corrected files when those inputs
   are available.

Never describe a proposal as applied or verified unless that action actually
occurred and its result was observed.
