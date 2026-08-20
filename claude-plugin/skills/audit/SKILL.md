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
1. **Resolve target** — If user provides URL, use directly. If local path, resolve URL profiles from `.rgaa/config.yaml`.
2. **Run analysis** — Call `rgaa-mcp` tool `analyze` with URL and config (viewport, profile, pre-scan actions). Falls back to `rgaa audit analyze` CLI if MCP unavailable.
3. **Map findings** — Convert raw violations to RGAA criterion findings with fingerprints and evidence references.
4. **Output** — Present audit bundle summary: compliance rate, passed/failed/needs-review counts, per-criterion status.

## Inputs
- `url` (string, optional) — Target URL to audit.
- `profile` (string, optional) — URL profile name from config.
- `format` (string, optional) — Output format: json, markdown, sarif, junit.
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