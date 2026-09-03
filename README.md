# Holo-RGAA

**AI-powered RGAA 4.1.2 accessibility auditing engine in Rust.**

Holo-RGAA replaces Asqatasun (the legacy Java RGAA auditor) with a high-performance Rust workspace. It combines deterministic automated checks (axe-core + gap-fix heuristics) with Holo3 LLM-assisted evaluation for criteria that require human judgment — all in a single unified pipeline.

---

## What is RGAA?

The **Referentiel General d'Amelioration de l'Accessibilite (RGAA)** is France's accessibility standard, mandatory for:
- Public sector websites and applications
- Private sector services with public access
- E-commerce platforms operating in France

RGAA 4.1.2 defines **106 criteria** across 13 topics (images, tables, links, scripts, HTML, colors, forms, navigation, content, media, etc.). Compliance status is determined by the **Taux Global** (global rate):

```
Taux Global = Conforme / (Conforme + Non Conforme) × 100
```

| Status | Threshold |
|--------|-----------|
| **Conforme** | Taux Global = 100% |
| **Partiellement Conforme** | Taux Global >= 50% |
| **Non Conforme** | Taux Global < 50% |

---

## Why Holo-RGAA?

### The Automation Problem

Most accessibility criteria fall into three categories:

| Category | Count | Can Automated Tools Detect? |
|----------|-------|---------------------------|
| **Deterministic** | ~77 | Yes — axe-core + gap-fix |
| **LLM-Assisted** | ~22+ | Partially — requires AI judgment |
| **Manual Testing** | ~7 | No — human tester required |

The "LLM-Assisted" criteria are where traditional scanners fail. Criteria like:
- Is the alternative for a complex image "detailed enough"?
- Does the page language change break screen reader pronunciation?
- Is the reading order logical for assistive technology?

These require contextual understanding that only an LLM can provide at scale.

### How Holo-RGAA Solves This

Holo-RGAA routes each criterion to the right evaluation method:

```
┌─────────────────────────────────────────────────────────┐
│                    Page Under Audit                      │
└───────────────────────┬─────────────────────────────────┘
                        │
            ┌───────────┼───────────┐
            ▼           ▼           ▼
      ┌──────────┐ ┌────────┐ ┌──────────┐
      │ axe-core │ │ gap-fix│ │  Holo3   │
      │  (WAI-ARIA│ │ (custom│ │   LLM    │
      │  rules)  │ │   JS)  │ │ judgment │
      └────┬─────┘ └───┬────┘ └────┬─────┘
           │           │           │
           └───────────┼───────────┘
                       ▼
              ┌─────────────────┐
              │  RGAA Results   │
              │  (unified view) │
              └─────────────────┘
```

1. **axe-core** handles the WAI-ARIA and WCAG mappings it knows
2. **gap-fix snippets** patch false negatives where axe-core misses RGAA-specific patterns
3. **Holo3 LLM** evaluates judgment-required criteria using structured prompts with:
   - Criterion definition and WCAG references
   - Page context (DOM snapshot, screenshots)
   - Confidence-based escalation (low confidence → `NeedsReview`)

---

## Features

### Unified Audit Pipeline
- Runs deterministic checks, gap-fix heuristics, and LLM evaluation in a single pass
- Spider crawl with configurable depth and page limits
- Evidence capture at every step (DOM snapshots, AXTree, screenshots with SHA-256 hashes)

### Multi-Interface
- **CLI** — `rgaa-cli` for terminal-based audits and CI integration
- **MCP Server** — `rgaa-mcp` for AI assistant (Claude Code) workflows
- **REST API** — `rgaa-api` for HTTP-based integration
- **Rust Library** — Direct integration into Rust projects via `rgaa-core`

### Browser Automation (Obscura)
- Headless Chromium via CDP (Chrome DevTools Protocol)
- Pre-scan actions: clicks, form fills, wait-for-element states
- Cookie injection for authenticated pages
- Keyboard IGT (guided accessibility testing)
- Screenshot capture (PNG, JPEG) with configurable policy

### Cookie Management
- Full cookie attributes: name, value, domain, path, sameSite, secure, httpOnly, expires
- Environment variable backstop for sensitive values (`RGAA_COOKIE_SESSION`)
- Cookie injection happens **before** navigation (not after) to catch auth-redirect issues

### Interactive Guided Tests (IGT)
Structured manual testing protocols for criteria that require human observation:
- Keyboard navigation tracking with focus element identity (stable DOM paths, not fragile selectors)
- Keyboard trap detection (5 consecutive tabs on same element = trap)
- CDP input failure detection (reports `incomplete` status with `ExecutionError` termination reason)

### Remediation Workflow
- Generates patch proposals with diffs for each failing criterion
- Framework-aware suggestions (React, Vue, Angular, vanilla HTML)
- Approval state tracking (required / auto-approved / rejected)
- Batch processing (1-25 issues per request)

### Report Formats
| Format | Use Case |
|--------|----------|
| **JSON** | Machine consumption, CI pipelines, custom tooling |
| **Markdown** | Human-readable reports, GitHub issues |
| **SARIF 2.1.0** | GitHub Code Scanning, security dashboards |
| **JUnit XML** | CI test results, Jenkins, CircleCI |
| **HTML** | Stakeholder reports, archival PDF generation |

### Policy Gates
- Configurable compliance thresholds per client/audit
- `NeedsReviewPolicy::Fail` — deny analysis if any finding requires manual review
- Pass/fail gating for CI/CD pipelines

---

## Installation

### TUI Install Wizard (Recommended)

```bash
rgaa install
```

Launches an interactive terminal installer that:
- Detects your platform automatically (Linux/macOS, x86_64/arm64)
- Shows a progress bar during download
- Installs to `~/.local/bin/rgaa`
- Works entirely offline once downloaded

### One-Command Install (No TUI)

```bash
curl -sSL https://raw.githubusercontent.com/jamon8888/Holo-RGAA/main/install.sh | bash
```

Headless install — downloads binaries and configures MCP without interactive prompts.

### Build from Source

```bash
curl -sSL https://raw.githubusercontent.com/jamon8888/Holo-RGAA/main/install.sh | bash -s -- --build
```

Requires Rust 1.80+. Build takes ~5-10 minutes.

### What the Installer Does

| Step | Action |
|------|--------|
| 1 | Detect platform (Linux x86-64, macOS arm64/x86-64) |
| 2 | Download `rgaa-mcp`, `rgaa-cli`, `obscura` binaries |
| 3 | Install to `~/.local/bin/` |
| 4 | Symlink Claude Code plugin |
| 5 | Write MCP config to `~/.claude/mcp.json` |
| 6 | Create `.rgaa/config.yaml` if missing |
| 7 | Verify installation |

### After Install

```bash
# Ensure ~/.local/bin is in your PATH
export PATH="$HOME/.local/bin:$PATH"

# Launch the interactive TUI (recommended)
rgaa

# Or run a headless audit
rgaa audit https://example.test

# Set your Holo3 API key for AI-assisted evaluation
rgaa config set api-key "your-key"

# Or set via environment variable
export HOLO3_API_KEY="your-key"
```

The TUI also supports keyboard shortcuts: `a` for Audit, `h` for History, `s` for Settings, `q` to Quit.

### Uninstall

```bash
curl -sSL https://raw.githubusercontent.com/jamon8888/Holo-RGAA/main/install.sh | bash -s -- --uninstall
```

---

## Quick Start

### Interactive TUI (Recommended)

Launch the interactive TUI for a guided experience:

```bash
rgaa
```

The TUI opens a full-screen terminal interface with:

```
┌──────────────────────────────────────┐
│      rgaa — RGAA Accessibility       │
│              Auditor                 │
│                                      │
│    [A]udit URL                      │
│      Run a new accessibility audit   │
│                                      │
│    [H]istory                        │
│      View past audit results         │
│                                      │
│    [S]ettings                       │
│      Configure API key and prefs     │
│                                      │
│    [Q]uit                          │
│      Exit rgaa                      │
└──────────────────────────────────────┘
```

**Audit Wizard** — Enter a URL and watch the audit run live. Results show:
- Color-coded score (green/yellow/red)
- PASS/FAIL/REVIEW/ERROR per criterion in a scrollable table
- Press Enter to drill into any criterion and see violations, justification, and confidence

**Install Wizard** — Detects your platform (Linux/macOS x86_64/arm64), shows download progress, and installs to `~/.local/bin/`

**Settings Wizard** — Configure your Holo3 API key and base URL interactively

### CLI

```bash
# Install wizard (interactive TUI)
rgaa install

# Audit wizard (interactive TUI)
rgaa audit

# Direct audit from command line
rgaa audit https://example.test --export results.json

# View audit history
rgaa history

# Show current config
rgaa config show

# Set API key
rgaa config set api-key "your-key"

# Set base URL
rgaa config set base-url "https://api.example.com"
```

### Claude Code (MCP)

Once installed and configured, use natural language in Claude Code:

```
Audit https://example.test for RGAA compliance and report the global rate.
```

Claude Code will call the `analyze` tool, retrieve findings, and explain the results with remediation guidance.

### MCP Tool Reference

| Tool | Purpose |
|------|---------|
| `analyze` | Analyze a URL for accessibility findings |
| `audit_url` | Run full RGAA audit (crawl + analyze + report) |
| `remediate` | Generate fix proposals for failing criteria |
| `igt` | Run guided accessibility test (keyboard, focus) |
| `get_audit_result` | Retrieve a stored audit by ID |
| `list_criteria` | List all 106 RGAA criteria |

### Python (API)

```python
import requests

# Run an audit
resp = requests.post("http://localhost:3000/audit", json={
    "url": "https://example.test",
    "config": {"max_pages": 10, "max_depth": 3}
})
audit_id = resp.json()["audit_id"]

# Poll for results
result = requests.get(f"http://localhost:3000/audit/{audit_id}").json()
print(f"Taux Global: {result['taux_global']}%")
print(f"Status: {result['etat_conformite']}")
```

---

## Architecture

```
┌──────────────────────────────────────────────────────┐
│                   rgaa-orchestrator                   │
│         (unified pipeline: axe + LLM + IGT)          │
└─────────────┬──────────────────┬─────────────────────┘
              │                  │
    ┌─────────▼──────┐  ┌──────▼──────┐  ┌────────────▼────────┐
    │   rgaa-rules   │  │  rgaa-holo  │  │   rgaa-obscura      │
    │  axe-core 4.x   │  │   Holo3     │  │  CDP browser        │
    │  + gap-fix JS   │  │   LLM       │  │  automation         │
    └─────────┬──────┘  └──────┬──────┘  └────────────┬────────┘
              │                 │                      │
              └────────────────┬┴─────────────────────┘
                               │
                    ┌──────────▼──────────┐
                    │      rgaa-core       │
                    │  106 criteria domain │
                    └──────────┬───────────┘
                               │
              ┌───────────────┼───────────────────────┐
              │               │                       │
       ┌──────▼─────┐  ┌─────▼─────┐  ┌────────────▼─────────┐
       │  rgaa-tui  │  │  rgaa-mcp │  │     rgaa-api         │
       │  Ratatui   │  │   MCP 3.x │  │     HTTP REST        │
       │  TUI app   │  └───────────┘  └──────────────────────┘
       └────────────┘
```

### Core Crates

| Crate | Responsibility |
|-------|---------------|
| `rgaa-core` | Domain types, 106-criteria catalog, findings model |
| `rgaa-rules` | axe-core integration + gap-fix JavaScript snippets |
| `rgaa-holo` | Holo3 LLM client, prompt construction, response parsing |
| `rgaa-obscura` | CDP browser automation, cookie injection, IGT execution |
| `rgaa-agent` | Rig-based agentic evaluator, dual-model routing, rate limiting |
| `rgaa-orchestrator` | Pipeline orchestration, result aggregation |
| `rgaa-mcp` | MCP server implementation (3 tools: analyze, remediate, igt) |
| `rgaa-cli` | CLI application (analyze, report, policy, igt) |
| `rgaa-api` | Axum HTTP API server |
| `rgaa-storage` | PostgreSQL persistence layer |
| `rgaa-remediation` | Fix proposal generation, approval workflow |

### How Evaluation Works

1. **CDP Launch** — Obscura spawns a headless Chromium via Chrome DevTools Protocol
2. **Navigation** — Page loads with cookies injected first (before navigation, not after)
3. **Pre-scan Actions** — Click, fill, wait-for-element interactions run before scan
4. **axe-core Run** — Executes in page context, returns violation nodes
5. **gap-fix Snippets** — 10 custom JS patches catch axe-core false negatives
6. **Holo3 Evaluation** — 22+ criteria routed to LLM with structured prompts
7. **IGT Keyboard Test** — Tab navigation captures focus elements, detects traps
8. **Result Aggregation** — Findings merged, evidence attached, compliance calculated

---

## MCP Server Deep Dive

The MCP server exposes the `analyze` tool with full configuration support:

### `analyze` Parameters

```json
{
  "url": "https://example.test",
  "config": {
    "profile": "default",
    "viewport_width": 1280,
    "viewport_height": 720,
    "selector": null,
    "pre_scan_actions": [
      { "action": "click", "selector": "#cookie-consent" },
      { "action": "waitFor", "selector": "main", "state": "visible" }
    ],
    "cookies": [
      { "name": "session", "value": "secret", "domain": "example.test" }
    ],
    "screenshot": { "format": "png", "save": true },
    "advanced_rules": null,
    "igt_tools": ["keyboard"],
    "timeout_ms": 30000,
    "retry_limit": 0
  }
}
```

### Pre-Scan Actions

| Action | Description |
|--------|-------------|
| `click` | Click element before scan |
| `fill` | Fill input (value redacted in logs) |
| `waitFor` | Wait for element state: `visible`, `attached`, `hidden`, `detached` |

**Note:** `waitFor` uses async polling with `awaitPromise: true` — it does not block page timers or rendering.

### Cookie Injection

Cookies are installed via `Network.setCookie` **before** `Page.navigate` to ensure they're present for the initial request and any authentication redirects. Sensitive values can be backed by environment variables:

```json
{
  "name": "auth",
  "value": null,
  "domain": "example.test"
}
```

This reads `RGAA_COOKIE_AUTH` from the environment at runtime.

### IGT Keyboard Test

The keyboard IGT:
- Dispatches Tab key events via CDP `Input.dispatchKeyEvent`
- Captures focused element identity using **stable DOM paths** (not fragile `tag+role` concatenation)
- Uses `id` attribute when available, falls back to full DOM path
- Reports `keyboard-trap` issue after 5 consecutive tabs on the same element
- Sets `status: "incomplete"` and `terminated_reason: "ExecutionError"` on CDP failures

---

## CLI Reference

The `rgaa` binary provides both interactive TUI and headless CLI commands.

### TUI Commands

```bash
rgaa              # Launch interactive TUI (main menu)
rgaa install       # Launch install wizard
rgaa audit         # Launch audit wizard
rgaa history       # View audit history
rgaa config show   # Show current configuration
rgaa config set api-key "key"    # Set Holo3 API key
rgaa config set base-url "url"   # Set API base URL
```

### Headless CLI

```bash
rgaa audit <URL> [options]
```

| Option | Description |
|--------|-------------|
| `--url` | Target URL (required in headless mode) |
| `--format` | Output format: `json`, `markdown`, `sarif`, `junit` |
| `--output` | Write output to file |
| `--export` | Export results to file (format detected from extension) |
| `--max-pages` | Maximum pages to crawl |
| `--max-depth` | Maximum crawl depth |
| `--profile` | Test profile: `default`, `mobile` |
| `--llm` | Enable Holo3 LLM evaluation |
| `--igt` | Run IGT keyboard test |
| `--screenshot` | Screenshot policy: `always`, `on-failure`, `none` |
| `--timeout` | Per-step timeout in milliseconds |
| `--verbose` | Verbose output |

### Headless Report and Policy

```bash
# Generate a compliance report
rgaa audit --url https://example.test --export results.json

# Policy gate — block CI if non-compliant
rgaa audit --url https://example.test --export audit-bundle.json
rgaa-cli policy --input audit-bundle.json --threshold 85
```

---

## RGAA Criteria Coverage

### Topic Breakdown

| Topic | Criteria | Deterministic | LLM-Assisted | Manual |
|-------|----------|--------------|--------------|--------|
| Images | 1.1–1.9 | 6 | 3 | 0 |
| Tables | 5.1–5.8 | 5 | 2 | 1 |
| Links | 6.1–6.3 | 2 | 1 | 0 |
| Scripts | 7.1–7.5 | 3 | 1 | 1 |
| HTML | 8.1–8.10 | 8 | 1 | 1 |
| Colors | 10.1–10.14 | 10 | 3 | 1 |
| Forms | 11.1–11.13 | 10 | 2 | 1 |
| Navigation | 12.1–12.14 | 11 | 2 | 1 |
| Content | 4.1–4.13 | 8 | 4 | 1 |
| Media | 13.1–13.13 | 4 | 5 | 4 |

### Classification Definitions

| Classification | Description |
|---------------|-------------|
| `Deterministe` | Fully automated via axe-core or gap-fix |
| `IaAssistee` | Requires Holo3 LLM evaluation |
| `Manuel` | Human tester observation required |
| `PartiellementAutomatable` | Automated check + human verification |

---

## Configuration

### Environment Variables

| Variable | Description | Required |
|----------|-------------|----------|
| `HOLO3_API_KEY` | Holo3 API key for LLM evaluation | Yes (for AI-assisted audits) |
| `RGAA_OBSCURA_BIN` | Path to Obscura browser binary | Yes |
| `DATABASE_URL` | PostgreSQL connection string | No (for storage) |
| `RUST_LOG` | Logging level (`info`, `debug`, `trace`) | No |

### Cookie Environment Variables

Cookie values can be injected from environment variables using the naming convention `RGAA_COOKIE_<NAME>`:

| Cookie `name` | Environment Variable |
|---------------|---------------------|
| `session` | `RGAA_COOKIE_SESSION` |
| `auth_token` | `RGAA_COOKIE_AUTH_TOKEN` |

### Policy Configuration

```yaml
# .rgaa/config.yaml
policy:
  threshold: 85
  fail_on_needs_review: true
  allowed_domains:
    - "*.example.test"
browser:
  timeout_ms: 30000
  viewport_width: 1280
  viewport_height: 720
llm:
  enabled: true
  model: "holo3-tactical"
  timeout_ms: 60000
```

---

## Tech Stack

| Layer | Technology |
|-------|------------|
| Language | Rust 1.85+ (2024 edition) |
| Async runtime | Tokio |
| Browser automation | Obscura (custom CDP client) |
| LLM client | Holo3 (H Company) |
| MCP server | rmcp 3.1.3 |
| CLI | Clap 4.0 |
| HTTP API | Axum |
| Database | PostgreSQL 16 (optional) |
| Build | Cargo, cargo-dist |

---

## Project Structure

```
rgaa-rs/
  Cargo.toml              # Workspace root (11 crates)
  crates/
    rgaa-core/           # Domain types, 106-criteria catalog
    rgaa-rules/           # axe-core integration, gap-fix snippets
    rgaa-holo/           # Holo3 LLM client
    rgaa-browser-tools/  # Browser automation via CDP
    rgaa-obscura/        # CDP browser automation (Rust-native)
    rgaa-agent/          # Rig-based agentic evaluator
    rgaa-orchestrator/   # Pipeline orchestration
    rgaa-tui/            # Interactive TUI (Ratatui)
    rgaa-api/            # Axum HTTP API
    rgaa-mcp/             # MCP server
    rgaa-cli/             # CLI interface
    rgaa-storage/         # PostgreSQL storage
    rgaa-remediation/     # Fix proposal generation
```

### rgaa-tui

The interactive TUI is a Ratatui-based terminal application with:

| Screen | Description |
|--------|-------------|
| **Main Menu** | Arrow-key navigation between Audit/History/Settings/Quit |
| **Audit Wizard** | URL input → live progress → color-coded results table → criterion drill-down |
| **Install Wizard** | Platform detection, progress bar, install confirmation |
| **Setup Wizard** | API key and base URL configuration |
| **History Viewer** | Browse past audit results |

---

## Contributing

See [AGENTS.md](./AGENTS.md) for development guidelines.

---

## License

MIT
