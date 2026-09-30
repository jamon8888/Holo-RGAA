---
name: audit
description: "Run RGAA accessibility audit against a URL or project"
version: 0.1.0
author: RGAA Team
requires:
  - rgaa-mcp
  - rgaa-cli
mode-default: suggest
---

# audit — RGAA Accessibility Audit

## Overview
This skill orchestrates the RGAA accessibility audit workflow. It runs automated analysis via the `rgaa-mcp` server (or `rgaa-cli` fallback), maps findings to RGAA criteria, and produces a structured audit bundle.

## Triggers
- User requests "audit this URL", "run RGAA audit", "check accessibility"
- User provides a URL or local project path

## Workflow

MCP first. The `rgaa` CLI is the fallback for when no MCP session is
available — not a parallel path to pick between. Never shell out while the
tools are reachable: the CLI re-runs the whole audit, so falling back
mid-flow silently doubles the crawl and can contradict the MCP verdict.

1. **Resolve target** — If user provides URL, use directly. If local path, resolve URL profiles from `.rgaa/config.yaml`.
2. **Resolve criteria** — Call `list_criteria` once to get the 106 criterion ids, titles, and classifications. Use it to name criteria in output instead of hardcoding a list.
3. **Run the audit** — Call `audit_url` with the URL and config. This returns a **summary only** (`taux_global`, `etat_conformite`) plus `sampled_page_urls`.
4. **Get per-criterion detail** — Call `analyze` on the URLs in `sampled_page_urls`. `audit_url` alone cannot answer "which criterion failed and why"; `analyze` carries the findings, evidence, and justification.
5. **Re-read instead of re-auditing** — If the user asks about an audit already run, call `get_audit_result` with its `audit_id` rather than auditing the URL again.
6. **Map findings** — Convert raw violations to RGAA criterion findings with fingerprints and evidence references.
7. **Output** — Present audit bundle summary: compliance rate, passed/failed/needs-review counts, per-criterion status.

### CLI fallback (only when MCP is unavailable)

```bash
rgaa audit analyze --url <URL> --format <json|markdown|html|sarif|junit>
```

## Tools used

| Step | MCP tool | CLI fallback |
|------|----------|--------------|
| Criteria catalog | `list_criteria` | — (embedded in report output) |
| Site audit | `audit_url` | `rgaa audit analyze` |
| Per-page detail | `analyze` | `rgaa audit analyze` |
| Re-read an audit | `get_audit_result` | — |

`Manuel` and `PartiellementAutomatable` criteria both surface as
`NeedsReview`: that single status is the human-review queue.

## Inputs
- `url` (string, optional) — Target URL to audit.
- `profile` (string, optional) — URL profile name from config.
- `format` (string, optional) — Output format: json, markdown, html, sarif, junit.
- `output` (path, optional) — Write output to file.
- `config` (path, optional) — Path to `.rgaa/config.yaml`.

## Outputs
- Structured `AuditBundle` (schema version "1.0") with findings, evidence, checkpoints, summary.
- Human-readable summary printed to stdout unless machine format requested.

## Constraints
- Every finding MUST have a stable fingerprint (`rgaa-fp-v1-<hex>`).
- Evidence MUST reference captured screenshots/DOM snapshots.
- Incomplete results MUST be rejected (not passed as clean).
- Compliance rate MUST be computed as `passed / (passed + failed) * 100`.

## Failure Modes
- Invalid URL → exit code 2, typed error `INVALID_INPUT`.
- Browser unavailable → exit code 3, typed error `EXECUTION_FAILED`.
- Config validation error → exit code 2.

## Example
```
> audit --url https://example.test --format markdown
```