# RGAA Accessibility Plugin for Claude Code

Production-grade accessibility audit workflow for the RGAA (Référentiel Général d'Amélioration de l'Accessibilité) standard, built on the `rgaa-mcp` server and `rgaa-cli` toolchain.

## Features

- **Automated Analysis** — Run `axe-core` via `rgaa-mcp` (or `rgaa-cli` fallback) to audit URLs or local projects.
- **RGAA Mapping** — Map violations to 106 RGAA criteria (Deterministe, Ia-Assisté, Manuel).
- **Guided Tests** — Bounded, reproducible interactive tests with PNG evidence and accessibility tree refs.
- **Remediation** — Framework-aware (React, Next, Vue, Angular) source fixes with approval gating.
- **Verification** — Objective re-audit with evidence to confirm fixes.
- **Reports** — JSON (schema "1.0"), Markdown, SARIF 2.1.0, JUnit XML for CI.
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

The plugin bundles `rgaa-mcp` (stdio transport) exposing three tools:
- `analyze(AnalyzeRequest) -> AnalyzeResponse`
- `remediate(RemediationRequest) -> RemediationResponse`
- `igt(GuidedTestRequest) -> GuidedTestResponse`

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success / Policy pass |
| 1 | Policy failure |
| 2 | Invalid input / configuration |
| 3 | Execution / infrastructure error |

## License

MIT