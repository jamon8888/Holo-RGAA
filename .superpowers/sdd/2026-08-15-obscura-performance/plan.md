# Obscura performance follow-on

## Objective

Make the Obscura backend correct under batch load and suitable for performance measurement without silently dropping URLs or converting browser failures into clean audits.

## Tasks

1. Harden `rgaa-obscura`: CDP lifecycle, bounded concurrency, axe evaluation validation, target cleanup, and correct CLI batch parsing for all URLs.
2. Integrate batch APIs into the orchestrator without changing unrelated audit behavior.
3. Add focused regression/performance tests and run workspace verification.

## Constraints

- Preserve the existing public `ObscuraBridge` methods unless a concrete caller update is included.
- Do not modify unrelated user changes in the parent worktree.
- Use Obscura's documented CDP and `scrape` behavior rather than assuming Chromium semantics.
- Never report a successful empty result when evaluation, parsing, or navigation failed.
