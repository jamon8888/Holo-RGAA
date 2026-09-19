# Holo-RGAA

**AI-powered RGAA 4.1.2 accessibility auditing engine in Rust.**

Holo-RGAA replaces Asqatasun (the legacy Java RGAA auditor) with a high-performance Rust workspace. It combines deterministic automated checks (axe-core + gap-fix heuristics) with Holo3 LLM-assisted evaluation for criteria that require human judgment — all in a single unified pipeline.

---

## Why This Is a Breakthrough

Every existing accessibility scanner — axe-core, WAVE, Asqatasun — is **DOM-only**:
it parses HTML attributes and never *looks* at the page. That caps them at roughly
**50 criteria** they can actually verify, leaving more than half of RGAA 4.1.2's
**106 criteria** as "manual testing required" — which, in practice, means never tested.

Holo-RGAA is the first auditor built to cover the **full 106-criterion path** with its internal pipeline:

- **73 deterministic criteria** via axe-core + RGAA-specific gap-fix heuristics;
- **32 LLM-assisted criteria** via the **Holo3 vision-language model**, which receives
  the rendered screenshot alongside the DOM/AXTree and judges what DOM-only tools
  cannot see — pertinent image alternatives, visible focus indicators, reading order,
  link purpose from context, captions checked against rendered media;
- **1 guided manual test (IGT)** with keyboard-trap detection for the remainder (criterion 7.5).

No other product sees the page. Holo-RGAA does — and that single difference is what
turns a 50-criterion scan into a complete RGAA audit pipeline, from detection to
visually verified patch.

**Compared to manual audits (Koena references: 8 pages/5 days, 19 pages/9 days ≈ 3–5h/page), Holo-RGAA reduces active consultant time from ~45–75h to ~1–2h for a 15-page site — a 20–40× speedup. With PR #85 merged (8 concurrent audits, tiered LLM, circuit breaker), 15 pages process in ~2–4 min wall-clock (parallel); consultant only reviews ~20 NeedsReview findings.**

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

Most accessibility criteria fall into three categories in **Holo-RGAA's pipeline**:

| Category | Count | How Holo-RGAA Handles It |
|----------|-------|-------------------------|
| **Deterministic** | 73 | axe-core + gap-fix JS (fully automated) |
| **LLM-Assisted** | 32 | Holo3 vision model evaluates; human reviews decision |
| **Manual (IGT)** | 1 | Keyboard navigation test (criterion 7.5) |

Traditional scanners (axe-core, WAVE, Asqatasun) are **DOM-only** and cap at ~50 criteria. Holo3 vision unlocks the 32 `IaAssiste` criteria by evaluating visual context (screenshots + DOM). Only criterion 7.5 (status messages via AT) remains truly manual.

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
3. **Holo3 vision LLM** evaluates judgment-required criteria using structured prompts with:
   - Criterion definition and WCAG references
   - Page context (DOM snapshot, AXTree, **screenshots seen by the model**)
   - Confidence-based escalation (low confidence → `NeedsReview`)

---

## Why Holo3 Vision Changes the Game

Traditional scanners (axe-core, WAVE, Asqatasun) are **DOM-only**: they parse HTML
attributes and never *look* at the page. That caps them at ~77 fully deterministic
criteria and leaves the rest as "manual testing required" — which in practice means
*never tested*.

Holo3 is a **vision-language model**: it receives the rendered screenshot alongside
the DOM/AXTree. That single difference unlocks the ~22+ `IaAssistee` criteria that
decide real RGAA compliance:

| What DOM-only tools cannot do | What Holo3 vision does |
|---|---|
| Tell if an image `alt` is *pertinent* (crit. 1.x) vs. placeholder text | Reads the image **and** its alt, judges relevance |
| Detect a focus indicator that exists in CSS but is invisible (crit. 10.7) | **Sees** contrast, outline, and position on the screenshot |
| Judge reading order, visual hierarchy, link purpose from context (crit. 10.x, 12.x) | Reasons over layout, proximity, and visual grouping |
| Verify captions, audio description, media alternatives (crit. 4.x) | Cross-checks declared alternatives against rendered media |
| Catch keyboard traps, hidden content, off-screen text | Correlates focus path (IGT) with what is actually visible |

### The remediation payoff

Detection without a fix is a PDF nobody reads. Holo3 closes the loop:

1. **Evidence-grounded verdicts** — every finding ships with DOM node, AXTree path,
   screenshot hash, and model justification, so a developer can reproduce it in seconds.
2. **Framework-aware patches** — `rgaa-remediation` turns a vision verdict
   ("contrast 2.1:1 on hero CTA") into a concrete diff for React/Vue/Angular/vanilla,
   with approval states (required / auto-approved / rejected) and batching (1–25 issues).
3. **No false confidence** — low-confidence vision judgments escalate to `NeedsReview`
   instead of fake PASS, and the engine **never claims "Conformité totale" from
   automation alone** (dégradation rule: non-validated tests need human review).
4. **Agentic loop** — `rgaa-agent` (Rig-based, dual-model routing: fast tier for
   text criteria, reasoning tier for visual criteria 11.x/12.8+) re-checks the fix
   against a fresh screenshot, so remediation is verified visually, not assumed.

Result: the audit goes from *"here are 40 violations, good luck"* to
*"here is the failing criterion, what the model saw, the patch, and the re-test"* —
the difference between a compliance chore and a fix pipeline.

---

## Performance & Scalability

Holo-RGAA audits at production scale — many pages, many audits, bounded resources.
Every layer below is enforced in code, not in documentation:

| Layer | Mechanism |
|-------|-----------|
| Static data | Criteria catalog, axe map (`IndexMap`, deterministic order), gap-fix snippets built **once** (`OnceLock`) and shared as `&'static` — never rebuilt per audit |
| Batch orchestration | Up to **8 audits concurrently** (`tokio::Semaphore` + `buffer_unordered(8)`), incremental persistence, one browser session per audit |
| Single audits | Routed through the same batch entry points — one code path, no duplicate logic |
| LLM lane | Tiered routing (fast tier for text criteria, reasoning tier for visual 11.x/12.8+), page context rendered **once per URL** and capped at **8 000 chars**, shared circuit breaker (fails loud on Holo3 outage) + RPM rate limiting |
| HTTP API | Per-request timeouts, `GlobalConcurrencyLimitLayer` (one shared semaphore via `RGAA_API_MAX_CONCURRENT_AUDITS`), load shedding on audit endpoints |
| Crawler | Streaming polite spider: configurable concurrency, delay, per-request/crawl timeouts, retry budget, URL blacklist |
| Storage | `list_audits` reads metadata only (full blobs skipped), dead N+1 write path removed |
| Browser substrate | Obscura `serve --workers N` (one per CPU by default, `OBSCURA_WORKERS` override), V8 heap tuning, `systemd` template, validated input bounds (selectors, viewports, timeouts, retries) |
| Browser core | `BrowserWorker` on a dedicated thread (tokio-incompatible internals isolated) + `Send`-safe `BrowserHandle` over channels; browser auto-starts in background, deny-by-default network policy |
| Build | `mold` linker (OOM-safe fat-LTO links), `sccache`, `cargo-nextest`, `Makefile`, pinned toolchain (1.98.1) |

### Why this matters for RGAA

A full RGAA audit is 106 criteria × N pages. The expensive multiplications are
killed at the source: static data is allocated once per process, page context is
rendered once per URL (not once per criterion), LLM calls are tiered + capped +
circuit-broken, and concurrent audits are bounded so the 9th audit waits instead
of OOM-killing the first 8.

---

## Project Status — What Is Done

### Shipped (merged to `main`)

- **Unified pipeline** (`rgaa-orchestrator`): axe-core + gap-fix + Holo3 + IGT merged
  into one run, single `AuditBundle` model, official `taux_global = C / (C + NC)`
  math (NA/NT excluded), sample-wide aggregation (NC on any page → NC).
- **Obscura browser substrate** (`rgaa-obscura`): Rust-native CDP automation, pinned
  v0.2.2 binary with version gates, vendored axe-core 4.13 (`elementRef`,
  `incomplete`, `patch_attach_internals`), label-aware pre-scan fill, cookie
  injection **before** navigation, `RGAA_OBSCURA_BIN` honored everywhere.
- **axe-core parity + IGT** (`rgaa-mcp`, `rgaa-browser-tools`): `waitFor`, cookies,
  screenshots, keyboard IGT with stable DOM-path focus identity and trap detection
  (5× same element = trap), CDP failure → `incomplete` + `ExecutionError`.
- **Interfaces**: unified `rgaa` TUI (Ratatui: audit wizard with live progress,
  history viewer, install/setup wizards) + headless CLI + MCP server
  (`analyze`, `audit_url`, `remediate`, `igt`, `get_audit_result`, `list_criteria`)
  + Axum HTTP API + spider crawler + remediation plugin (Consultant v2.0.0).
- **One-command delivery**: `install.sh` / `install.ps1` (per-platform obscura assets,
  Claude Code plugin symlink, MCP config), `cargo-dist` releases, CI hardened
  (protoc everywhere, OOM-guarded Linux link, rust-cache workspaces fix).

### Channel-based browser core (PR #65, incl. #75 review fixes)

Channel-based browser core: `BrowserWorker` on a dedicated thread (tokio-incompatible
internals isolated) + `Send`-safe `BrowserHandle` over channels. All methods wired
(navigate, eval_js, click, screenshot, a11y_tree, type_input, press_key, tab_order,
assert_state). Security deny-by-default (private-network + `file://` blocked unless
opted in). MCP server reports name/version on initialize. Orchestrator/MCP/CLI all
migrated to `BrowserHandle`; browser auto-starts in background. Includes TUI real
progress (pipeline phases + `RgaaError`), audit wizard running the real orchestrator,
history via storage, and all CodeRabbit findings addressed — including typed
`RgaaError → McpFailure` mapping (`invalid` / `unsupported` / `incomplete` /
`execution`) instead of blanket execution errors.

### Production-scale reliability — Scale #1–8 (PR #74, closes #43–#50)

- **#43** build-once static data (criteria catalog, axe map, gap-fix snippets via `OnceLock`).
- **#44** streaming polite crawler (configurable concurrency/delay/timeout/retry/blacklist).
- **#45** single audits routed through batch entry points (no duplicate paths).
- **#46** LLM lane: visual-tier routing wired, page context rendered once and capped at 8 000 chars,
  shared circuit breaker.
- **#47** API timeouts, shared concurrency limit (`GlobalConcurrencyLimitLayer`),
  load shedding.
- **#48** Obscura workers/V8 heap config, systemd template, fixed silenced logging.
- **#49** bounded batch orchestration (max 8 in flight), incremental persistence,
  per-audit browser session.
- **#50** storage: `list_audits` skips blobs, dead N+1 write path removed.
- Plus: pinned toolchain, Makefile, mold linking, sccache, nextest, expanded CI. Two real bugs
  fixed (streaming-crawler deadlock, tower semaphore non-sharing).

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

```bash
curl -sSL https://raw.githubusercontent.com/jamon8888/Holo-RGAA/main/install.sh | bash
```

That's it. Launch the TUI with `rgaa`. Windows (PowerShell):

```powershell
irm https://raw.githubusercontent.com/jamon8888/Holo-RGAA/main/rgaa-rs/install.ps1 | iex
```

<details>
<summary>Details (optional)</summary>

The script detects your platform (Linux/macOS, x86_64/aarch64), downloads
`rgaa`, `rgaa-cli`, `rgaa-api`, `rgaa-mcp` from the latest GitHub release,
installs to `~/.local/bin/`, configures the Claude Code plugin, and verifies everything.

- Bleeding edge (rebuilt on every push to `main`):
  ```bash
  RGAA_VERSION=latest curl -sSL https://raw.githubusercontent.com/jamon8888/Holo-RGAA/main/install.sh | bash
  ```
- Interactive wizard (once `rgaa` is installed): `rgaa install`
- Build from source (Rust 1.80+, ~20-40 min first build):
  ```bash
  curl -sSL https://raw.githubusercontent.com/jamon8888/Holo-RGAA/main/install.sh | bash -s -- --build
  ```
- Uninstall:
  ```bash
  curl -sSL https://raw.githubusercontent.com/jamon8888/Holo-RGAA/main/install.sh | bash -s -- --uninstall
  ```

| Step | Action |
|------|--------|
| 1 | Detect platform (Linux/macOS, x86_64/aarch64) |
| 2 | Download `rgaa`, `rgaa-cli`, `rgaa-api`, `rgaa-mcp` |
| 3 | Install to `~/.local/bin/` |
| 4 | Symlink Claude Code plugin |
| 5 | Write MCP config to `~/.claude/mcp.json` |
| 6 | Create `.rgaa/config.yaml` if missing |
| 7 | Verify installation |

</details>

### After Install

```bash
export PATH="$HOME/.local/bin:$PATH"
rgaa                                # interactive TUI
rgaa audit https://example.test  # headless audit
export HOLO3_API_KEY="your-key"     # AI-assisted evaluation
```

TUI shortcuts: `a` audit, `h` history, `s` settings, `q` quit.

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
| `rgaa-spider` | Polite streaming crawler (PR #74) |
| `rgaa-data` | Shared static data (catalog, rule maps) |
| `rgaa-test-corpus` | Fixtures for regression tests |

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

### Topic Breakdown (Holo-RGAA pipeline classification)

| Topic | Criteria | Deterministe | IaAssiste | Manuel |
|-------|----------|--------------|-----------|--------|
| Images | 1.1–1.9 | 5 | 4 | 0 |
| Tables | 5.1–5.8 | 5 | 2 | 1 |
| Links | 6.1–6.3 | 2 | 1 | 0 |
| Scripts | 7.1–7.5 | 3 | 1 | 1 |
| HTML | 8.1–8.10 | 7 | 2 | 1 |
| Colors | 10.1–10.14 | 11 | 3 | 0 |
| Forms | 11.1–11.13 | 10 | 3 | 0 |
| Navigation | 12.1–12.14 | 11 | 3 | 0 |
| Content | 4.1–4.13 | 8 | 5 | 0 |
| Media | 13.1–13.13 | 11 | 2 | 0 |

**Total: 73 Deterministe, 32 IaAssiste, 1 Manuel** (sums to 106)

### Pipeline Classification

| Classification | Description | Count |
|---------------|-------------|-------|
| `Deterministe` | Fully automated via axe-core or gap-fix | 73 |
| `IaAssiste` | Holo3 vision LLM evaluates; human reviews decision | 32 |
| `Manuel` | Human tester observation required (IGT) | 1 (7.5) |

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
  Cargo.toml              # Workspace root (16 crates)
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
    rgaa-spider/          # Streaming crawler
    rgaa-data/            # Shared static data
    rgaa-test-corpus/     # Test fixtures
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
