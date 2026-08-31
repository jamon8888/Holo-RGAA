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

```
claude plugins add rgaa-accessibility
```

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
| `/audit-site` | Audit a live URL for RGAA compliance (single page) |
| `/audit-url` | Run a full-site crawl audit (multiple pages) |
| `/audit-project` | Audit a local project (requires rgaa-cli) |
| `/run-igt` | Execute a guided accessibility test (keyboard, focus, contrast) |
| `/remediate` | Generate approval-gated source-level fix proposals |
| `/generate-report` | Produce a formatted compliance report |

## Skills

Skills activate automatically when relevant — no need to invoke them directly.

| Skill | When It Fires |
|-------|---------------|
| `audit` | URL or project accessibility check requested |
| `triage` | Findings need prioritization and categorization |
| `remediate` | Fix proposals needed for accessibility violations |
| `report` | Compliance documentation or export needed |
| `guided-test` | Manual accessibility testing (keyboard, focus, contrast) |
| `criteria` | RGAA criterion knowledge lookup |

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
├── commands/
│   ├── audit-site.md
│   ├── audit-project.md
│   ├── audit-url.md
│   ├── generate-report.md
│   ├── run-igt.md
│   └── remediate.md
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
