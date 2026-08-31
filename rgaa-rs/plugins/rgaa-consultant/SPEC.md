# RGAA Consultant Plugin — Specification v2.0.0

## Overview

The rgaa-consultant plugin provides RGAA 4.1.2 accessibility auditing through Claude Code / Claude Cowork. It wraps two backend providers:

- **rgaa-mcp** (primary): MCP server exposing 6 tools
- **rgaa-cli** (fallback): CLI commands for local execution
- **rgaa-api** (optional): HTTP API server for team deployments

## MCP Tools (rgaa-mcp)

### `analyze` — Single-URL Accessibility Audit

Run an axe-core + gap-fix accessibility scan on a single URL.

**Parameters:**

| Param | Type | Required | Description |
|-------|------|----------|-------------|
| `url` | string | Yes | Target URL |
| `config.profile` | string | No | Profile name (default: `"default"`) |
| `config.viewport_width` | u32 | No | Viewport width in pixels (default: `1000`) |
| `config.viewport_height` | u32 | No | Viewport height in pixels (default: `1080`) |
| `config.selector` | string | No | CSS selector to scope audit to a sub-tree |
| `config.pre_scan_actions` | PreScanAction[] | No | Actions to run before scanning |
| `config.cookies` | Cookie[] | No | Cookies to inject before scanning |
| `config.screenshot` | ScreenshotInput | No | Screenshot capture settings |
| `config.timeout_ms` | u64 | No | Per-page timeout in ms |
| `config.retry_limit` | u8 | No | Retry limit on failure |
| `config.advanced_rules` | string | No | `"thorough"` / `"standard"` / `"disabled"` |
| `config.igt_tools` | string[] | No | IGT tools to run, e.g. `["keyboard"]` |
| `config.needs_review_policy` | NeedsReviewPolicyInput | No | `"record"` (default) or `"fail"` |
| `viewport_width` | u32 | No | Flat param override for viewport_width |
| `viewport_height` | u32 | No | Flat param override for viewport_height |

**PreScanAction variants:**
- `Click { selector }` — Click an element before scanning
- `Fill { selector, value }` — Fill an input before scanning
- `WaitFor { selector, state }` — Wait for element state before scanning. States: `visible`, `attached`, `hidden`, `detached`

**Cookie fields:** `name`, `value`, `domain`, `path`, `same_site` (`strict`/`lax`/`none`), `secure` (bool), `http_only` (bool), `expires` (i64 Unix timestamp)

**ScreenshotInput fields:** `format` (`png`/`jpeg`), `save_to` (file path), `save` (bool), `inline` (bool)

**Returns:** `AnalyzeResponse` — flat or nested (`{ data: { axe, igt } }` when `igt_tools` is set)

---

### `audit_url` — Full Crawl Audit

Run a full-site crawl via the orchestrator pipeline.

**Parameters:**

| Param | Type | Required | Description |
|-------|------|----------|-------------|
| `url` | string | Yes | Starting URL |
| `config.max_pages` | usize | No | Max pages to crawl (default: `50`) |
| `config.max_depth` | u32 | No | Max crawl depth (default: `5`) |
| `config.respect_robots` | bool | No | Respect robots.txt (default: `true`) |
| `config.sample_mode` | bool | No | Sample mode (default: `false`) |

**Returns:** `{ audit_id, taux_global, etat_conformite }`

---

### `igt` — Guided Accessibility Test

Execute a structured guided accessibility test.

**Parameters:**

| Param | Type | Required | Description |
|-------|------|----------|-------------|
| `test` | GuidedTestInput | Yes | Test definition with preconditions, steps, criterion mapping |

**GuidedTestInput fields:**
- `id`, `version`, `preconditions[]`
- `steps[]`: `navigate` | `accessibility_tree` | `press_key` | `click_ref` | `fill_ref` | `screenshot` | `assert_state`
- `criterion_mapping[]`: RGAA criterion IDs
- `evidence_requirements[]`

**Returns:** `{ issues[], unanalyzed_elements[], terminated_reason, completed_steps, evidence[], manual_review_required }`

---

### `remediate` — Remediation Proposals

Generate approval-gated source-level patch proposals for findings.

**Parameters:**

| Param | Type | Required | Description |
|-------|------|----------|-------------|
| `issues` | RemediationIssueInput[] | Yes | 1–25 findings to remediate |

**RemediationIssueInput fields:** `id`, `rule`, `element_html`, `page_url`, `source_locations[]`, `summary`, `criteria[]`, `framework`

**Returns:** `outcomes[]` — each is `Ok { issue_id, explanation, steps[], confidence, criteria[], proposal { diff, files[], rationale, risks[], validation_commands[], approval_state, approval_token } }` or `Error`

---

### `get_audit_result` — Retrieve Audit by ID

**Parameters:** `audit_id: string`

**Returns:** `AuditResultDto` with `audit_id`, `url`, `taux_global`, `etat_conformite`, `passed`, `failed`, `na`, `duration_ms`

---

### `list_criteria` — List All 106 RGAA Criteria

No parameters.

**Returns:** `{ criteria[] }` where each criterion has `id`, `title`, `classification`

---

## CLI Commands (rgaa-cli)

| Command | Description |
|---------|-------------|
| `rgaa audit analyze` | Run single-URL or profile audit |
| `rgaa audit igt` | Run a named guided test |
| `rgaa audit verify` | Verify remediation proposals |
| `rgaa audit report` | Generate formatted reports |
| `rgaa audit policy` | Check compliance thresholds |

---

## Claude Code Commands

| Command | File | Wraps |
|---------|------|-------|
| `/audit-site` | `commands/audit-site.md` | `analyze` MCP tool |
| `/audit-project` | `commands/audit-project.md` | `rgaa audit analyze` CLI |
| `/audit-url` | `commands/audit-url.md` | `audit_url` MCP tool |
| `/run-igt` | `commands/run-igt.md` | `igt` MCP tool |
| `/remediate` | `commands/remediate.md` | `remediate` MCP tool |
| `/generate-report` | `commands/generate-report.md` | `rgaa audit report` CLI |

---

## Skills

| Skill | File | Activates When |
|-------|------|---------------|
| `audit` | `skills/audit/SKILL.md` | URL/project accessibility check requested |
| `triage` | `skills/triage/SKILL.md` | Findings need prioritization |
| `remediate` | `skills/remediate/SKILL.md` | Fix proposals for violations |
| `report` | `skills/report/SKILL.md` | Compliance documentation needed |
| `guided-test` | `skills/guided-test/SKILL.md` | Manual testing (keyboard, focus, contrast) |
| `criteria` | `skills/criteria/SKILL.md` | RGAA criterion knowledge lookup |

---

## Response Shapes

### AnalyzeResponse (flat — backward-compatible)

```json
{
  "url": "https://example.test",
  "findings": [...],
  "evidence": [...],
  "errors": [],
  "completed": "2026-08-31T...",
  "duration_ms": 1234
}
```

### AnalyzeResponse (nested — when igt_tools is set)

```json
{
  "data": {
    "axe": {
      "url": "https://example.test",
      "findings": [...],
      "evidence": [...],
      "errors": [],
      "completed": "...",
      "duration_ms": 1234
    },
    "igt": {
      "keyboard": {
        "status": "completed",
        "issues": [...],
        "interactive_elements": [...],
        "terminated_reason": "completed",
        "completed_steps": 12
      }
    }
  }
}
```
