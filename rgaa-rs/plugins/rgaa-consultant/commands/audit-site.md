# `/audit-site` — Audit a Live URL

## Description

Run a full RGAA 4.1.2 accessibility audit against a live URL. Analyzes all 106 criteria, captures evidence (screenshots, AXTree dumps), and returns a structured audit bundle with findings, compliance rate, and severity classification.

## Prerequisites

- `rgaa-cli` installed and accessible, OR
- `rgaa-api` server running at configured URL

## Usage

```
/audit-site [url] [--viewport desktop|mobile|tablet] [--format json|markdown|sarif|html]
```

## Arguments

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `url` | string | Yes | Target URL to audit |
| `--viewport` | string | No | Device viewport (default: `desktop`) |
| `--format` | string | No | Output format (default: `markdown`) |

## Advanced Arguments

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `--viewport-width` | integer | No | Viewport width in pixels (default: 1000) |
| `--viewport-height` | integer | No | Viewport height in pixels (default: 1080) |
| `--selector` | string | No | CSS selector to scope audit to a sub-tree |
| `--timeout` | integer | No | Per-page timeout in milliseconds |
| `--retry` | integer | No | Retry limit on failure (default: 3) |
| `--advanced-rules` | string | No | Rule strictness: `thorough`, `standard`, `disabled` |
| `--igt-tools` | string | No | IGT tools to run: `keyboard` (runs keyboard navigation test) |
| `--needs-review` | string | No | NeedsReview handling: `record` (default) or `fail` |
| `--wait-for` | string | No | Pre-scan: `selector:visible|attached|hidden|detached` |
| `--click` | string | No | Pre-scan: click selector before scanning |
| `--fill` | string | No | Pre-scan: `selector:value` — fill input before scanning |
| `--cookie` | string | No | Pre-scan: `name=value@domain` — inject cookie |
| `--screenshot-format` | string | No | Screenshot format: `png` (default) or `jpeg` |
| `--screenshot-save` | string | No | Save screenshots to file |
| `--screenshot-inline` | bool | No | Return screenshots inline in response |

## Examples

```
/audit-site https://example.gouv.fr
/audit-site https://example.test --viewport mobile
/audit-site https://example.test --format sarif
/audit-site https://example.test --viewport-width 1920 --viewport-height 1080
/audit-site https://example.test --advanced-rules thorough
/audit-site https://example.test --igt-tools keyboard
/audit-site https://example.test --wait-for "#modal:visible" --click "#accept-cookies"
/audit-site https://example.test --cookie "session=abc@example.test" --cookie "analytics=no@.example.test"
/audit-site https://example.test --screenshot-format jpeg --screenshot-save /tmp/screenshots
/audit-site https://example.test --needs-review fail
```

```
/audit-site https://example.gouv.fr
/audit-site https://example.test --viewport mobile
/audit-site https://example.test --format sarif
```

## Workflow

1. **Validate URL** — Check format and reachability
2. **Launch audit** — Run `rgaa audit analyze --url <url>` via MCP or CLI
3. **Capture evidence** — Screenshots, AXTree for each finding
4. **Map to RGAA criteria** — Each violation → criterion ID
5. **Compute compliance** — Calculate Taux Global
6. **Present results** — Summary + prioritized findings

## Output Summary

```
RGAA Audit Results
═══════════════════════════════════════════
URL:        https://example.test
Date:       2026-08-30
Taux Global: 72.4%
Status:     Partiellement Conforme
──────────────────────────────────────────
Coverage:   85% (90/106 criteria tested)
Passed:     77 criteria
Failed:     18 criteria
Needs Review: 11 criteria
═══════════════════════════════════════════

Critical Findings (3)
─────────────────────
• RGAA 11.1 — Form fields missing labels
• RGAA 12.1 — Missing skip navigation link
• RGAA 1.1 — Images without alt attributes

Major Findings (15)
───────────────────
...
```

## Viewport Options

| Viewport | Dimensions | Use Case |
|----------|------------|----------|
| `desktop` | 1280×720 | Default, full site audit |
| `mobile` | 375×667 | Mobile experience audit |
| `tablet` | 768×1024 | Tablet experience audit |

## Post-Audit Actions

After audit:
- `/triage` — Prioritize findings
- `/remediate` — Generate fix proposals
- `/report` — Generate formal compliance report

## Error Codes

| Code | Meaning |
|------|---------|
| 2 | Invalid URL or input |
| 3 | Browser/execution failure |
| 4 | Network unreachable |
| 5 | Partial results (some pages failed) |
