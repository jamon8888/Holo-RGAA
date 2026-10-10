---
name: verify
description: Recheck corrected files against a reference RGAA audit and report findings as fixed, remaining, new, or unverified.
---

# Verify RGAA fixes

Use `verify_fix` to compare corrected files with a reference audit bundle.
Source inspection alone cannot establish that an accessibility finding is
resolved.

## Workflow

1. Obtain the original audit bundle, corrected file paths, and the URL each
   file belongs to. If the required reference bundle is unavailable, explain
   what is missing instead of claiming verification.
2. Call `verify_fix` with the reference audit and corrected files.
3. Report `fixed`, `remaining`, `new`, and `unverified` results separately.
   Preserve the tool's evidence and per-page errors.
4. Treat an unverified page or incomplete analysis as unresolved for reporting
   purposes; do not convert it into a pass.
5. If new findings appear, link them to their evidence and suggest the next
   audit or remediation step.
