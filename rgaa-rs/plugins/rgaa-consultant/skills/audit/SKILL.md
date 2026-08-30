# audit — RGAA Accessibility Audit

## Purpose

Run a full RGAA 4.1.2 accessibility audit against a URL or local project. Returns a structured audit bundle with findings mapped to the 106 RGAA criteria, evidence references, and compliance rate.

## When This Skill Activates

- User says "audit this URL", "run RGAA audit", "check accessibility", "run accessibility scan"
- User provides a URL or project path for accessibility evaluation
- User asks "what's the RGAA compliance of [URL]"

## Workflow

1. **Resolve target** — URL or detect from project config (`.rgaa/config.yaml`)
2. **Run analysis** — Call `analyze` tool via rgaa-mcp, rgaa-api, or `rgaa audit analyze` CLI
3. **Collect evidence** — Screenshots, AXTree dumps, DOM snapshots
4. **Map findings** — Convert violations to RGAA criteria with stable fingerprints
5. **Compute compliance** — `Taux Global = passed / (passed + failed) × 100`
6. **Present summary** — Compliance rate, pass/fail/needs-review counts, critical findings

## Inputs

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `url` | string | Yes* | Target URL to audit (*or `project_path`) |
| `project_path` | string | Yes* | Local project to audit (*or `url`) |
| `viewport` | string | No | Viewport profile: `desktop` (1280x720), `mobile` (375x667), `tablet` (768x1024) |
| `format` | string | No | Output: `json`, `markdown`, `sarif`, `junit`, `html` |
| `scope` | string | No | CSS selector to limit audit to specific component |
| `pre_scan_actions` | array | No | Actions before scan: `click`, `fill`, `navigate` |

## Outputs

```json
{
  "audit_id": "aud_abc123",
  "url": "https://example.test",
  "taux_global": 72.4,
  "coverage_percent": 85.0,
  "etat_conformite": "partielle",
  "passed": 77,
  "failed": 18,
  "needs_review": 11,
  "total_criteria": 106,
  "pages": [...],
  "findings": [
    {
      "id": "fng_abc123",
      "rule": "image-alt",
      "criterion_id": "1.1",
      "criterion_title": "Each image has an alternative",
      "classification": "Deterministe",
      "status": "fail",
      "description": "Image missing alt attribute",
      "html": "<img src=\"hero.png\">",
      "evidence": {
        "screenshot": "/evidence/screenshot-001.png",
        "axtree": "/evidence/axtree-001.json"
      },
      "fingerprint": "rgaa-fp-v1-sha256:abc123"
    }
  ],
  "evidence": [...]
}
```

## RGAA Criterion Classifications

| Classification | Meaning | Test Method |
|---------------|---------|-------------|
| `Deterministe` | Fully automated | axe-core + gap-fix rules |
| `IaAssiste` | AI-assisted | Holo3 LLM evaluation |
| `Manuel` | Manual testing | Guided test protocol |

## Compliance Status

| Taux Global | Status |
|-------------|--------|
| 100% | Conforme |
| ≥50% | Partiellement Conforme |
| <50% | Non Conforme |

## Critical Findings First

Present findings in this order:
1. Failed `Deterministe` criteria (immediate fixes)
2. Failed `IaAssiste` criteria (AI-assisted review needed)
3. Failed `Manuel` criteria (guided manual testing required)
4. Passed criteria (for record)

## Error Handling

| Situation | Response |
|-----------|----------|
| Invalid URL | Exit code 2, `INVALID_INPUT` |
| Browser unavailable | Exit code 3, `EXECUTION_FAILED` |
| Network timeout | Retry with exponential backoff (max 3 attempts) |
| Partial results | Return what was captured, flag incomplete |

## Example Interactions

```
User: "Run an RGAA audit on https://example.test"
Claude: → Confirms target URL
Claude: → Calls rgaa-mcp analyze tool
Claude: → Presents: "Taux Global: 72.4% — Partiellement Conforme"
Claude: → Lists critical failures by category
Claude: → Offers to triage, remediate, or generate report
```

## Related Skills

- `triage` — Prioritize and categorize findings
- `remediate` — Generate fix proposals
- `verify` — Re-test after fixes
- `report` — Generate compliance documentation
