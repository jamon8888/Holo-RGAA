# RGAA Accessibility Auditor Plugin

An AI-powered accessibility auditing plugin for Claude Cowork and Claude Code. Run full RGAA 4.1.2 compliance audits, triage findings, generate remediation proposals with source-level fixes, verify corrections, and produce defensible compliance reports.

## Who This Is For

- **Accessibility consultants** performing RGAA audits for clients
- **Developers** building French government or e-commerce sites requiring RGAA compliance
- **Product teams** needing continuous accessibility monitoring in CI/CD
- **Accessibility specialists** who need guided manual testing support

## What It Does

- **Audit** any URL or project against all 106 RGAA criteria
- **Triage** findings by severity, framework, and fix complexity
- **Remediate** with approval-gated source-level patch proposals
- **Verify** fixes with objective re-testing and evidence
- **Report** in JSON, Markdown, SARIF, JUnit, or HTML formats
- **Guided test** keyboard navigation, focus management, and visual checks

## Quick Start

### 1. Install the Plugin

This directory — `rgaa-rs/plugins/rgaa-consultant/` — is the canonical plugin
tree. There is no other: the former top-level `claude-plugin/` is a deprecated
pointer to this one.

```bash
# From a clone of the repository
ln -s "$(pwd)/rgaa-rs/plugins/rgaa-consultant" ~/.claude/plugins/rgaa-accessibility

# Or let install.sh place it for you
./install.sh
```

Then `/plugin` in Claude Code to confirm `rgaa-accessibility` is loaded.

### 2. Connect Your Tools

**Option A — CLI (recommended for local development)**
```bash
cargo install --path rgaa-rs/crates/rgaa-cli
```

**Option B — API Server (recommended for teams)**
```bash
cargo install --path rgaa-rs/crates/rgaa-api
# Start server
DATABASE_URL=postgres://localhost/rgaa rgaa-api
```

Configure in `.mcp.json`:
```json
{
  "mcpServers": {
    "rgaa-api": {
      "type": "http",
      "url": "http://localhost:3000"
    }
  }
}
```

### 3. Run Your First Audit

```
/audit-site
```

Claude will ask for a URL, run the full RGAA audit, and present findings with severity and compliance rate.

## Commands

| Command | Description |
|---------|-------------|
| `/audit-site` | Audit a live URL for RGAA compliance |
| `/audit-project` | Audit a local project (requires rgaa-cli) |
| `/generate-report` | Produce a formatted compliance report |

## Skills

Skills activate automatically when relevant — no need to invoke them directly.

| Skill | When It Fires |
|-------|---------------|
| `audit` | URL or project accessibility check requested |
| `triage` | Findings need prioritization and categorization |
| `remediate` | Fix proposals needed for accessibility violations |
| `verify` | Post-fix validation or re-audit requested |
| `report` | Compliance documentation or export needed |
| `guided-test` | Manual accessibility testing (keyboard, focus, contrast) |
| `criteria` | A specific RGAA criterion needs looking up or explaining |

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

## Agents

Subagents the skills delegate to for bounded, single-purpose work.

| Agent | Used by | Purpose |
|-------|---------|---------|
| `scanner` | `audit` | Run the accessibility scan and collect evidence |
| `remediation-planner` | `remediate` | Draft approval-gated source patches |
| `verification-reviewer` | `verify` | Re-audit and confirm fixes against the baseline |
| `compliance-report-writer` | `report` | Assemble the compliance report |

## Hooks

`hooks/hooks.json` registers `scripts/check-runtime.sh` on two events:

- **SessionStart** — detect the project framework (Next, React, Vue, Angular) and
  write `RGAA_FRAMEWORK` to `.rgaa/env` for the skills to read.
- **PostToolUse** on `Edit|Write|MultiEdit` — mark audit state stale for the
  edited file, so a later verify knows the baseline no longer matches the source.

## RGAA Compliance Tiers

Not all criteria are automated. The plugin reports which tier each finding belongs to:

| Tier | Count | How It's Tested |
|------|-------|-----------------|
| **Deterministe** | 77 | axe-core + gap-fix rules (fully automated) |
| **IA-Assistee** | 22+ | Holo3 LLM visual evaluation (AI-assisted) |
| **Manuel** | Remaining | Guided testing protocol (human judgment) |

## Compliance Calculation

```
Taux Global = Conforme / (Conforme + Non Conforme) × 100
```

**Conformity Status:**
- **Conforme** — 100% criteria pass
- **Partiellement Conforme** — ≥50% pass
- **Non Conforme** — <50% pass

## Example Workflows

### Audit a Client Site

```
/audit-site
→ "https://client.gouv.fr"
→ Full 106-criteria audit runs
→ Taux Global: 72%
→ 77 pass, 18 fail, 11 needs review
→ Detailed findings with evidence
```

### Remediate Findings

```
Claude: "I found 5 missing alt attributes on the hero images"
Claude: → Generates diff for each
Claude: → Presents approval token for each patch
→ You approve
→ Patches applied with validation commands
```

### CI/CD Integration

```yaml
# GitHub Actions
- name: RGAA Audit
  run: |
    rgaa audit analyze --url ${{ env.AUDIT_URL }} --output audit.json
    rgaa audit policy --input audit.json
```

## Supported Standards

- **RGAA 4.1.2** — French accessibility standard (primary)
- **WCAG 2.2** — Cross-referenced
- **EN 301 549** — European accessibility standard cross-referenced

## File Structure

```
rgaa-consultant/
├── .claude-plugin/plugin.json
├── .mcp.json
├── README.md
├── CONNECTORS.md
├── SPEC.md
├── agents/
│   ├── scanner.md
│   ├── remediation-planner.md
│   ├── verification-reviewer.md
│   └── compliance-report-writer.md
├── commands/
│   ├── audit-site.md
│   ├── audit-project.md
│   └── generate-report.md
├── hooks/hooks.json
├── scripts/check-runtime.sh
├── tests/
│   ├── plugin-contract.sh
│   └── e2e-local.sh
└── skills/
    ├── audit/
    ├── triage/
    ├── remediate/
    ├── verify/
    ├── report/
    ├── guided-test/
    └── criteria/
```

## Getting Help

- Full documentation: `rgaa-rs/docs/`
- CLI reference: `rgaa-rs/docs/cli/README.md`
- API reference: `rgaa-rs/docs/api/README.md`
- Runbooks: `rgaa-rs/docs/runbooks/`

## Notes

- Automated testing covers ~77 criteria. Remaining criteria require guided manual testing.
- AI-assisted evaluation (Holo3) provides additional coverage but requires LLM API configuration.
- All findings include stable fingerprints for deduplication across re-audits.
- Remediation proposals require explicit approval before any source changes.
