# RGAA Accessibility Plugin for Claude Code

Production-grade accessibility audit workflow for the RGAA (Référentiel Général d'Amélioration de l'Accessibilité) standard, built on the `rgaa-mcp` server and `rgaa-cli` toolchain.

## Features

- **Automated Analysis** — Run `axe-core` via `rgaa-mcp` (or `rgaa-cli` fallback) to audit URLs or local projects.
- **RGAA Mapping** — Map violations to 106 RGAA criteria (Deterministe, Ia-Assisté, Manuel).
- **Guided Tests** — Bounded, reproducible interactive tests with PNG evidence and accessibility tree refs.
- **Remediation** — Framework-aware (React, Next, Vue, Angular) source fixes with approval gating.
- **Verification** — Objective re-audit with evidence to confirm fixes.
- **Reports** — JSON (schema "1.0"), Markdown, HTML, SARIF 2.1.0, JUnit XML for CI.
- **Local-First** — Works offline; remote bundle sync optional.

## Installation

```bash
# From source
cargo install --path rgaa-rs/crates/rgaa-mcp
cargo install --path rgaa-rs/crates/rgaa-cli

# Configure
cp .rgaa/config.yaml.example .rgaa/config.yaml
# Edit URL profiles, viewport profiles, policy thresholds
```

## Quick Start

```bash
# Run full audit
rgaa audit analyze --url https://example.test --format markdown

# Run guided test
rgaa audit igt --test keyboard-navigation

# Verify remediation
rgaa audit verify --issues issues.json

# Policy gate
rgaa audit policy --input audit-bundle.json
```

## Claude Code Integration

```bash
# Install plugin
cp -r claude-plugin ~/.claude/plugins/rgaa-audit

# Use in Claude Code
> audit --url https://example.test
> triage
> remediate
> verify
> report --format markdown
```

## Commands

| Command | Description |
|---------|-------------|
| `audit analyze` | Run RGAA audit against URL |
| `audit igt` | Run guided accessibility test |
| `audit verify` | Verify remediation proposals |
| `audit report` | Generate compliance report |
| `audit policy` | Check compliance against policy |

## Configuration

`.rgaa/config.yaml`:
```yaml
url_profiles:
  default:
    url: https://example.test
    viewport: desktop
viewport_profiles:
  desktop: {width: 1000, height: 1080}
  mobile: {width: 375, height: 812}
policy:
  min_compliance: 80.0
  required_criteria: []
evidence_dir: .rgaa/evidence
```

## Skills & Agents

| Skill | Agent | Purpose |
|-------|-------|---------|
| audit | scanner | Run accessibility audit |
| triage | — | Classify & prioritize findings |
| remediate | remediation-planner | Generate approval-gated fixes |
| verify | verification-reviewer | Re-audit & confirm fixes |
| report | compliance-report-writer | Generate compliance reports |
| guided-test | — | Run interactive tests |

## MCP Server

The plugin bundles `rgaa-mcp` (stdio transport) exposing **nine** tools. Every
skill reaches for these first; the `rgaa` CLI is a fallback for when no MCP
session is available, not the primary path.

| Tool | Signature | Use it for |
|------|-----------|------------|
| `analyze` | `AnalyzeRequest -> AnalyzeResponse` | Per-criterion findings for one page, with evidence and justification. |
| `audit_url` | `AuditUrlInput -> AuditUrlResult` | Whole-site audit through the orchestrator. Returns a **summary only** (`taux_global`, `etat_conformite`, `sampled_page_urls`). |
| `get_audit_result` | `GetAuditInput -> Option<AuditResultDto>` | Retrieve a previously run audit by `audit_id`. |
| `lint_static` | `LintStaticRequest -> LintStaticResponse` | Lint HTML, JSX/TSX or Vue **source** for missing `alt`, unlabelled form controls and nameless buttons/links — no browser, no build. Sub-millisecond per file, so it suits an edit loop. A first pass over source, **not** a conformance verdict: use `analyze` on a rendered page for that. |
| `list_criteria` | `() -> ListCriteriaResponse` | The 106 RGAA criteria with id, title, classification. |
| `remediate` | `RemediationRequest -> RemediationResponse` | Approval-gated fix proposals for a batch of issues. |
| `source_map` | `SourceMapRequest -> SourceMapResponse` | Relocate browser findings to the template that produced them: `source_location` (file, line, column, snippet) per finding. Best-effort literal match over React JSX, Vue SFC, Angular and vanilla HTML — the browser reports rendered DOM, the repository holds templates, so there is no exact inverse. Ambiguous findings come back in `unmappable` with a reason rather than a guessed location; check `confidence` and `matched_on` before editing. |
| `verify_fix` | `VerifyFixRequest -> VerifyFixResponse` | Re-verify corrected files against a reference audit: `fixed` / `remaining` / `new`, plus `unverified` for pages that could not be re-analysed. |
| `igt` | `GuidedTestRequest -> GuidedTestResponse` | **Deprecated** — prefer `analyze` with `config.igt_tools: ["keyboard"]`. |

### MCP-first flow

`audit_url` gives the site-level verdict but no per-criterion detail. To get
both, chain the tools rather than re-running the audit:

1. `list_criteria` — once, to resolve criterion ids and classifications.
2. `audit_url` — site-level summary plus `sampled_page_urls`.
3. `analyze` — per page from `sampled_page_urls`, for findings and evidence.
4. `get_audit_result` — to re-read a completed audit instead of auditing again.
5. `remediate` — on the findings worth fixing.

Both `Manuel` and `PartiellementAutomatable` criteria surface as the single
`NeedsReview` status: watch that one status for everything needing a human.

### HTTP transport

`rgaa-mcp-http` serves the same nine tools as JSON-RPC over `POST /mcp`, with
audit progress on `GET /mcp/events` (SSE). Cross-origin access is **denied by
default** and must be opened explicitly with `RGAA_CORS_ORIGINS`
(comma-separated origins) or `--cors-origin`.

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success / Policy pass |
| 1 | Policy failure |
| 2 | Invalid input / configuration |
| 3 | Execution / infrastructure error |

## License

MIT