# RGAA Plugin v2.0 — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Update the rgaa-consultant Claude Code plugin to v2.0.0 — wire in full rgaa-mcp tool surface (all 6 tools), 3 new commands, updated CLI config, refreshed docs.

**Architecture:** The plugin wraps rgaa-mcp (primary MCP server) and rgaa-cli (CLI fallback). Three new commands expose audit_url (full crawl), igt (guided test runner), and remediate (fix proposals) directly. Existing commands/skill files are updated to reference new params and tools.

**Tech Stack:** Claude Code plugin format (.claude-plugin/plugin.json), MCP .mcp.json, Markdown command files, Markdown skill files.

**Spec:** `rgaa-rs/plugins/rgaa-consultant/SPEC.md` (new — describes full plugin surface)

---

## File Map

```
rgaa-rs/plugins/rgaa-consultant/
├── .claude-plugin/plugin.json          # Modify: version 1.0.0 → 2.0.0
├── .mcp.json                           # Modify: add rgaa-mcp server + update description
├── CONNECTORS.md                        # Modify: refresh rgaa-mcp section, keep others
├── README.md                           # Modify: add 3 new commands, update skill table
├── SPEC.md                             # Create: full plugin surface reference
├── commands/
│   ├── audit-site.md                   # Modify: document all analyze params
│   ├── audit-project.md                # No change
│   ├── audit-url.md                    # Create: wraps audit_url MCP tool
│   ├── generate-report.md              # No change
│   ├── run-igt.md                     # Create: wraps igt MCP tool
│   └── remediate.md                   # Create: wraps remediate MCP tool
└── skills/
    ├── audit/SKILL.md                  # Modify: reference new analyze params + igt_tools
    ├── triage/SKILL.md                 # No change
    ├── remediate/SKILL.md              # Modify: reference new /remediate command
    ├── report/SKILL.md                 # Modify: add audit-url to workflow
    ├── guided-test/SKILL.md            # Modify: reference new /run-igt command
    └── criteria/SKILL.md               # No change
```

---

## Global Constraints

- Plugin version: `2.0.0`
- Claude Code plugin: `name = "rgaa-accessibility"`, `.claude-plugin/plugin.json` format
- MCP servers registered in `.mcp.json` under `mcpServers`
- CLI commands registered in `.mcp.json` under `cli.<name>.commands[]`
- All command files are Markdown with `# /<command>` H1 header
- All skill files are Markdown with `# <skill-name>` H1 header
- French domain terminology for RGAA concepts in all docs
- Command examples use `/command-name` slash syntax

---

### Task 1: Update `plugin.json` to v2.0.0

**Files:**
- Modify: `rgaa-rs/plugins/rgaa-consultant/.claude-plugin/plugin.json`

**Interfaces:**
- Produces: v2.0.0 plugin manifest

- [ ] **Step 1: Update version and description**

Replace the file content:

```json
{
  "name": "rgaa-accessibility",
  "version": "2.0.0",
  "description": "RGAA 4.1.2 accessibility auditing for French compliance. Run audits (single URL or full crawl), triage findings, generate remediation proposals with approval-gated patches, run keyboard IGT tests, verify fixes, and produce compliance reports. Covers all 106 RGAA criteria with deterministic, AI-assisted, and manual testing tiers."
}
```

- [ ] **Step 2: Commit**

```bash
git add rgaa-rs/plugins/rgaa-consultant/.claude-plugin/plugin.json
git commit -m "docs(plugin): bump rgaa-consultant to v2.0.0"
```

---

### Task 2: Update `.mcp.json` — add rgaa-mcp, update descriptions

**Files:**
- Modify: `rgaa-rs/plugins/rgaa-consultant/.mcp.json`

**Interfaces:**
- Produces: `.mcp.json` with 3 server entries: `rgaa-mcp` (local), `rgaa-api` (http), `rgaa-cli` (commands)

- [ ] **Step 1: Replace file content**

```json
{
  "mcpServers": {
    "rgaa-mcp": {
      "type": "local",
      "command": ["rgaa-mcp"],
      "env": {
        "RGAA_OBSCURA_BIN": "${CLAUDE_PLUGIN_ROOT}/bin/obscura"
      },
      "description": "RGAA MCP server — primary tool provider. Start with: rgaa-mcp"
    },
    "rgaa-api": {
      "type": "http",
      "url": "http://localhost:3000",
      "description": "RGAA REST API server — HTTP fallback. Start with: rgaa-api"
    }
  },
  "cli": {
    "rgaa-cli": {
      "description": "RGAA CLI for local audits. Install: cargo install --path rgaa-rs/crates/rgaa-cli",
      "commands": [
        "rgaa audit analyze",
        "rgaa audit igt",
        "rgaa audit verify",
        "rgaa audit report",
        "rgaa audit policy"
      ]
    }
  }
}
```

- [ ] **Step 2: Commit**

```bash
git add rgaa-rs/plugins/rgaa-consultant/.mcp.json
git commit -m "feat(plugin): wire rgaa-mcp as primary MCP server in .mcp.json"
```

---

### Task 3: Write `SPEC.md` — full plugin surface reference

**Files:**
- Create: `rgaa-rs/plugins/rgaa-consultant/SPEC.md`

**Interfaces:**
- Produces: Human-readable spec documenting all MCP tools, CLI commands, skill activations

- [ ] **Step 1: Write the spec**

```markdown
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
```

- [ ] **Step 2: Commit**

```bash
git add rgaa-rs/plugins/rgaa-consultant/SPEC.md
git commit -m "docs(plugin): add SPEC.md — full plugin surface reference"
```

---

### Task 4: Update `CONNECTORS.md`

**Files:**
- Modify: `rgaa-rs/plugins/rgaa-consultant/CONNECTORS.md`

**Interfaces:**
- Produces: Updated connector docs with rgaa-mcp as primary MCP

- [ ] **Step 1: Replace connector docs**

Replace the "### 3. RGAA MCP Server (For Claude Code)" section and its MCP Configuration block with:

```markdown
### 3. RGAA MCP Server (For Claude Code) — PRIMARY

The `rgaa-mcp` crate provides 6 MCP tools: `analyze`, `audit_url`, `igt`, `remediate`, `get_audit_result`, `list_criteria`. This is the recommended integration path for Claude Code.

```bash
cargo install --path rgaa-rs/crates/rgaa-mcp
```

**MCP Configuration (Claude Code):**
```json
{
  "mcpServers": {
    "rgaa-mcp": {
      "type": "local",
      "command": ["rgaa-mcp"],
      "env": {
        "RGAA_OBSCURA_BIN": "${CLAUDE_PLUGIN_ROOT}/bin/obscura"
      }
    }
  }
}
```
```

Also update the "Tools used" section under ### 1 to include the 5 CLI commands as-is.

- [ ] **Step 2: Commit**

```bash
git add rgaa-rs/plugins/rgaa-consultant/CONNECTORS.md
git commit -m "docs(plugin): refresh CONNECTORS.md — rgaa-mcp is primary MCP"
```

---

### Task 5: Create `commands/audit-url.md`

**Files:**
- Create: `rgaa-rs/plugins/rgaa-consultant/commands/audit-url.md`

**Interfaces:**
- Produces: New Claude Code slash command wrapping `audit_url` MCP tool

- [ ] **Step 1: Write the command file**

```markdown
# `/audit-url` — Full-Site Crawl Audit

## Description

Run a full-site crawl via the orchestrator pipeline. Unlike `/audit-site` which analyzes a single URL, `/audit-url` starts from a seed URL and recursively discovers and audits pages up to configured limits.

## Prerequisites

- `rgaa-mcp` MCP server connected (recommended), OR
- `rgaa-cli` installed (`cargo install --path rgaa-rs/crates/rgaa-cli`)

## Usage

```
/audit-url <url> [--max-pages <n>] [--max-depth <n>] [--respect-robots] [--sample-mode]
```

## Arguments

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `url` | string | Yes | Seed URL to start crawling |
| `--max-pages` | integer | No | Maximum pages to crawl (default: 50) |
| `--max-depth` | integer | No | Maximum crawl depth (default: 5) |
| `--respect-robots` | flag | No | Honor robots.txt directives (default: true) |
| `--sample-mode` | flag | No | Sample mode — audit a subset (default: false) |

## Examples

```
/audit-url https://example.gouv.fr
/audit-url https://example.gouv.fr --max-pages 100
/audit-url https://example.gouv.fr --max-depth 3 --sample-mode
```

## Workflow

1. **Validate URL** — Check format and seed reachability
2. **Build crawl frontier** — Discover links up to max_pages / max_depth
3. **Run audits** — Each page runs `analyze` with shared config
4. **Aggregate results** — Combine findings across all pages
5. **Compute compliance** — Taux Global across the entire site
6. **Store audit** — Returns audit_id for later retrieval

## Output

```
RGAA Full-Site Audit
══════════════════════════════════════════
URL:        https://example.gouv.fr
Pages:      47 crawled, 3 failed
Max Depth:  5
Taux Global: 74.1%
Status:     Partiellement Conforme
────────────────────────────
Passed:     78 criteria (across all pages)
Failed:     16 criteria
Needs Review: 12 criteria
Audit ID:   aud_abc123
══════════════════════════════════════════
```

## Retrieve Results

After the audit:
- `/get-audit <audit-id>` — Retrieve full audit via `get_audit_result` MCP tool
- `/generate-report --input <audit-id>` — Generate a formatted report

## Error Codes

| Code | Meaning |
|------|---------|
| 2 | Invalid URL |
| 3 | Browser/execution failure |
| 4 | Seed URL unreachable |
| 5 | Partial crawl (some pages failed) |
```

- [ ] **Step 2: Commit**

```bash
git add rgaa-rs/plugins/rgaa-consultant/commands/audit-url.md
git commit -m "feat(plugin): add /audit-url command for full-site crawl audits"
```

---

### Task 6: Create `commands/run-igt.md`

**Files:**
- Create: `rgaa-rs/plugins/rgaa-consultant/commands/run-igt.md`

**Interfaces:**
- Produces: New Claude Code slash command wrapping `igt` MCP tool

- [ ] **Step 1: Write the command file**

```markdown
# `/run-igt` — Run Guided Accessibility Test

## Description

Execute a structured guided accessibility test (IGT) for criteria requiring human judgment: keyboard navigation, focus management, color contrast, touch targets. The test runs step-by-step through the browser via CDP.

## Prerequisites

- `rgaa-mcp` MCP server connected (recommended), OR
- `rgaa-cli` installed with `rgaa audit igt` subcommand

## Usage

```
/run-igt [--test <name>] [--url <url>] [--precondition <description>]
```

## Arguments

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `--test` | string | Yes* | Named test: `keyboard-navigation`, `focus-visibility`, `color-contrast`, `touch-targets`, `reading-order`, `forms-manual` (*or use `--url` for a one-shot test) |
| `--url` | string | Yes* | Target URL for a one-shot test (*or use `--test` for a named test) |
| `--precondition` | string | No | Precondition description (e.g., "user is logged in") |

## Supported Test Types

| Test | Criteria | What It Checks |
|------|----------|----------------|
| `keyboard-navigation` | 12.1, 12.2 | Tab through page, all focusables reachable, no traps |
| `focus-visibility` | 12.1 | Focus indicators visible on all interactive elements |
| `color-contrast` | 3.2, 3.3 | Text contrast meets 4.5:1 (normal) or 3:1 (large) |
| `touch-targets` | 12.5 | Touch targets at least 44×44px |
| `reading-order` | 9.1, 10.3 | Logical reading order in AXTree |
| `forms-manual` | 11.1–11.13 | Labels, errors, autocomplete attributes |

## Examples

```
/run-igt --test keyboard-navigation --url https://example.test
/run-igt --test focus-visibility --url https://example.test --precondition "user is logged in"
/run-igt --test color-contrast --url https://example.test/contact
```

## Test Steps

Each test runs through its defined steps:
1. Navigate to URL
2. Capture initial AXTree
3. Execute action (press key, click element, fill input)
4. Assert expected state
5. Capture evidence (screenshot, AXTree) at each step

## Output

```
IGT: keyboard-navigation
URL: https://example.test
──────────────────────────────────
Status:     completed
Steps:      12 completed
Issues:     0 keyboard traps detected
──────────────────────────────────
Focus Order:
  1. [skip link] → visible ✓
  2. [navigation] → visible ✓
  3. [search input] → visible ✓
  ...
──────────────────────────────────
Evidence: 6 screenshots, 6 AXTree dumps
Manual Review Required: no
```

## Termination Reasons

| Reason | Meaning |
|--------|---------|
| `completed` | All steps executed successfully |
| `keyboard_trap` | Focus cannot escape an area |
| `assertion_failed` | Expected state did not match |
| `timeout` | Step exceeded time limit |
| `missing_reference` | Referenced element not found |
| `navigation_error` | Navigation failed |

## Evidence

Evidence is saved to the evidence directory with SHA256 fingerprints:
- Screenshots: `igt-keyboard-nav-001.png`
- AXTree dumps: `igt-keyboard-nav-axtree-001.json`
```

- [ ] **Step 2: Commit**

```bash
git add rgaa-rs/plugins/rgaa-consultant/commands/run-igt.md
git commit -m "feat(plugin): add /run-igt command for guided accessibility tests"
```

---

### Task 7: Create `commands/remediate.md`

**Files:**
- Create: `rgaa-rs/plugins/rgaa-consultant/commands/remediate.md`

**Interfaces:**
- Produces: New Claude Code slash command wrapping `remediate` MCP tool

- [ ] **Step 1: Write the command file**

```markdown
# `/remediate` — Generate Remediation Proposals

## Description

Generate approval-gated source-level patch proposals for accessibility findings. Produces framework-specific fixes (React, Vue, Angular, Next) with diffs, rationale, risk assessment, and validation commands.

**Key principle: Proposals require explicit approval before any source changes.**

## Prerequisites

- `rgaa-mcp` MCP server connected (recommended), OR
- `rgaa-cli` installed with `rgaa audit verify` subcommand

## Usage

```
/remediate [--findings <path>] [--framework <react|vue|angular|next>]
```

## Arguments

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `--findings` | string | Yes* | Path to JSON file with findings, OR inline JSON (*or pipe findings from triage) |
| `--framework` | string | No | Override framework detection: `react`, `next`, `vue`, `angular` |

## Finding Format

```json
[
  {
    "id": "fng_abc123",
    "rule": "image-alt",
    "element_html": "<img src=\"hero.png\">",
    "page_url": "https://example.test",
    "source_locations": [
      { "file": "src/components/Hero.tsx", "line": 42 }
    ],
    "summary": "Image missing alt attribute",
    "criteria": ["1.1"],
    "framework": "react"
  }
]
```

## Examples

```
/remediate --findings findings.json
/remediate --findings findings.json --framework react
/audit-site https://example.test → /triage → /remediate
```

## Workflow

1. **Load findings** — 1–25 per batch from file or prior triage
2. **Detect framework** — Auto-detect or use `--framework` override
3. **Generate proposals** — Call `remediate` MCP tool
4. **Present approval request** — Each proposal shows diff, rationale, risks
5. **Wait for approval** — User confirms before any source changes
6. **Apply on approval** — Apply only after user confirms approval token

## Proposal Output

```
PROPOSAL (prop-abc123)
══════════════════════════════════════
Finding:   RGAA 1.1 — Image missing alt
File:      src/components/Hero.tsx:42
──────────────────────────────────────
Diff:
- <img src="hero.png">
+ <img src="hero.png" alt="Hero image showing team collaboration">

Rationale: Alt text provides textual alternative for screen readers
Risk:      None — purely additive change
Validation: npm run a11y:test
──────────────────────────────────────
Approval required — token: rgaa-approval-v1-sha256:abc123

Approve? (yes/no/count)
```

## Batch Processing

Process 1–25 findings per batch. For larger sets:
1. Claude presents proposals in groups
2. You approve each group before the next
3. Track proposal status across batches

## Approval States

| State | Meaning | Action |
|-------|---------|--------|
| `required` | Human review needed | Present diff, wait for approval |
| `not_required` | Safe, low-risk change | Offer one-click apply |
| `approved` | Previously approved | Apply immediately |

## CI/CD Usage

```bash
# Generate proposals from audit
rgaa audit analyze --url https://example.test --output audit.json

# Apply approved fixes
rgaa audit verify --issues fixes.json
```

## Error Handling

| Error | Meaning | Response |
|-------|---------|----------|
| `INSUFFICIENT_CONTEXT` | Cannot locate source element | Suggest manual fix |
| `UNSUPPORTED_FRAMEWORK` | Framework not detected | Return NeedsReview |
| `POLICY_DENIED` | Approval required but not granted | Block application |
```

- [ ] **Step 2: Commit**

```bash
git add rgaa-rs/plugins/rgaa-consultant/commands/remediate.md
git commit -m "feat(plugin): add /remediate command for remediation proposals"
```

---

### Task 8: Update `commands/audit-site.md` — document all new analyze params

**Files:**
- Modify: `rgaa-rs/plugins/rgaa-consultant/commands/audit-site.md:1-92`

**Interfaces:**
- Consumes: All new `analyze` MCP tool params from SPEC.md
- Produces: Updated audit-site command with full param documentation

- [ ] **Step 1: Add new arguments section**

After the existing "Arguments" table (after the `--format` row), insert:

```
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
| `--wait-for` | string | No | Pre-scan: `selector:visible\|attached\|hidden\|detached` |
| `--click` | string | No | Pre-scan: click selector before scanning |
| `--fill` | string | No | Pre-scan: `selector:value` — fill input before scanning |
| `--cookie` | string | No | Pre-scan: `name=value@domain` — inject cookie |
| `--screenshot-format` | string | No | Screenshot format: `png` (default) or `jpeg` |
| `--screenshot-save` | string | No | Save screenshots to file |
| `--screenshot-inline` | bool | No | Return screenshots inline in response |
```

Then update the "Examples" section to:

```
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

- [ ] **Step 2: Commit**

```bash
git add rgaa-rs/plugins/rgaa-consultant/commands/audit-site.md
git commit -m "feat(plugin): update /audit-site with all new analyze params"
```

---

### Task 9: Update `skills/audit/SKILL.md`

**Files:**
- Modify: `rgaa-rs/plugins/rgaa-consultant/skills/audit/SKILL.md`

**Interfaces:**
- Consumes: `analyze` MCP tool params from SPEC.md
- Produces: Updated audit skill with new param docs and igt_tools

- [ ] **Step 1: Add igt_tools to Inputs table**

Add to the Inputs table after the `pre_scan_actions` row:

```
| `igt_tools` | string[] | No | IGT tools to run, e.g. `["keyboard"]` — runs keyboard navigation test |
| `needs_review_policy` | string | No | `"record"` (default) or `"fail"` — when set to `fail`, audit returns failed status if any NeedsReview findings exist |
| `advanced_rules` | string | No | `"thorough"` / `"standard"` / `"disabled"` |
| `viewport_width` | u32 | No | Flat viewport width override |
| `viewport_height` | u32 | No | Flat viewport height override |
```

- [ ] **Step 2: Update Outputs to describe nested response**

After the existing output JSON block, add:

```
### Nested Response (when igt_tools is set)

When `igt_tools: ["keyboard"]` is passed, results include IGT keyboard test results under `data.igt.keyboard`:

```json
{
  "data": {
    "axe": { "url": "...", "findings": [...], ... },
    "igt": {
      "keyboard": {
        "status": "completed",
        "issues": [
          {
            "type": "keyboard_trap",
            "selector": "#modal",
            "description": "Focus cannot escape modal dialog"
          }
        ],
        "interactive_elements": [
          { "role": "button", "label": "Close", "focusable": true, "has_keyboard_handler": true }
        ],
        "terminated_reason": "completed",
        "completed_steps": 12
      }
    }
  }
}
```
```

- [ ] **Step 3: Commit**

```bash
git add rgaa-rs/plugins/rgaa-consultant/skills/audit/SKILL.md
git commit -m "feat(plugin): update audit skill with igt_tools and advanced_rules params"
```

---

### Task 10: Update `skills/remediate/SKILL.md` and `skills/guided-test/SKILL.md`

**Files:**
- Modify: `rgaa-rs/plugins/rgaa-consultant/skills/remediate/SKILL.md`
- Modify: `rgaa-rs/plugins/rgaa-consultant/skills/guided-test/SKILL.md`

**Interfaces:**
- Consumes: `/remediate` and `/run-igt` command files
- Produces: Updated skill references to new commands

**For remediate/SKILL.md:** In the "Workflow" section, add after step 3:
> 3. **Generate proposals** — Call `remediate` MCP tool via `/remediate` command, or `rgaa audit verify` CLI

**For guided-test/SKILL.md:** In the "Example Interactions" section, update to reference `/run-igt`:
> `/run-igt --test keyboard-navigation --url https://example.test/checkout`

- [ ] **Step 1: Commit**

```bash
git add rgaa-rs/plugins/rgaa-consultant/skills/remediate/SKILL.md rgaa-rs/plugins/rgaa-consultant/skills/guided-test/SKILL.md
git commit -m "feat(plugin): update remediate and guided-test skills to reference new commands"
```

---

### Task 11: Update `skills/report/SKILL.md`

**Files:**
- Modify: `rgaa-rs/plugins/rgaa-consultant/skills/report/SKILL.md`

**Interfaces:**
- Consumes: `get_audit_result` MCP tool
- Produces: Updated report skill referencing `/audit-url` workflow

- [ ] **Step 1: Add audit-url to workflow**

In "Example Interactions", add after the existing audit workflow:

```
User: "I need a full-site compliance report"
Claude: → Runs /audit-url on the starting URL
Claude: → Retrieves results via get_audit_result MCP tool
Claude: → Generates formatted report in requested format
```

- [ ] **Step 2: Commit**

```bash
git add rgaa-rs/plugins/rgaa-consultant/skills/report/SKILL.md
git commit -m "feat(plugin): update report skill to reference /audit-url workflow"
```

---

### Task 12: Update `README.md`

**Files:**
- Modify: `rgaa-rs/plugins/rgaa-consultant/README.md`

**Interfaces:**
- Produces: Updated README with new commands and tool table

- [ ] **Step 1: Update Commands table**

Replace the existing Commands table with:

```
## Commands

| Command | Description |
|---------|-------------|
| `/audit-site` | Audit a live URL for RGAA compliance (single page) |
| `/audit-url` | Run a full-site crawl audit (multiple pages) |
| `/audit-project` | Audit a local project (requires rgaa-cli) |
| `/run-igt` | Execute a guided accessibility test (keyboard, focus, contrast) |
| `/remediate` | Generate approval-gated source-level fix proposals |
| `/generate-report` | Produce a formatted compliance report |
```

- [ ] **Step 2: Update Skills table**

Replace the existing Skills table with:

```
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
```

- [ ] **Step 3: Update File Structure**

Replace the `commands/` section in the file structure with:

```
├── commands/
│   ├── audit-site.md
│   ├── audit-project.md
│   ├── audit-url.md
│   ├── generate-report.md
│   ├── run-igt.md
│   └── remediate.md
```

- [ ] **Step 4: Commit**

```bash
git add rgaa-rs/plugins/rgaa-consultant/README.md
git commit -m "feat(plugin): update README with 3 new commands and updated skill table"
```

---

## Spec Coverage Check

- [ ] `plugin.json` v2.0.0 ✓
- [ ] `.mcp.json` with rgaa-mcp ✓
- [ ] `SPEC.md` full surface reference ✓
- [ ] `CONNECTORS.md` refreshed ✓
- [ ] `/audit-url` command ✓
- [ ] `/run-igt` command ✓
- [ ] `/remediate` command ✓
- [ ] `/audit-site` updated with all new params ✓
- [ ] `audit` skill updated with igt_tools + needs_review_policy ✓
- [ ] `remediate` skill references new command ✓
- [ ] `guided-test` skill references new command ✓
- [ ] `report` skill references audit-url ✓
- [ ] `README.md` updated ✓

## Placeholder Scan

All steps contain actual content. No "TBD", "TODO", or placeholder code.

## Type Consistency

- All MCP tool names match `rgaa-rs/crates/rgaa-mcp/src/server.rs`: `analyze`, `audit_url`, `igt`, `remediate`, `get_audit_result`, `list_criteria`
- All command names use slash syntax: `/audit-site`, `/audit-url`, `/run-igt`, `/remediate`
- All param names match `AnalyzeConfigInput` fields: `viewport_width`, `viewport_height`, `advanced_rules`, `igt_tools`, `needs_review_policy`, `pre_scan_actions`, `cookies`, `screenshot`
