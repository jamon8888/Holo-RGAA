# Holo-RGAA

**Production-ready RGAA 4.1.2 accessibility auditing in Rust — from scan to legally opposable documents, across 12 EU countries.**

Holo-RGAA audits a website against all **106 RGAA criteria** in one unified pipeline: deterministic checks (axe-core + RGAA-specific heuristics), vision-LLM evaluation for judgment-required criteria, and guided manual tests — then generates the **official deliverables**: technical audit report, publishable accessibility declaration, and validated JSON payload.

---

## Why Holo-RGAA?

Existing scanners — axe-core, WAVE, Asqatasun — are **DOM-only**: they parse HTML attributes and never *look* at the page. That caps them at roughly 50 verifiable criteria and leaves the rest as "manual testing required", which in practice means never tested.

Holo-RGAA covers the full 106-criterion path:

| Category | Count | Method |
|----------|-------|--------|
| **Deterministic** | 73 | axe-core + gap-fix heuristics, fully automated |
| **LLM-assisted** | 32 | Holo3 vision model sees screenshot + DOM/AXTree (alt-text relevance, focus visibility, reading order, link purpose, media alternatives); low confidence escalates to `NeedsReview`, never a fake PASS |
| **Manual (IGT)** | 1 | Guided keyboard test with trap detection (criterion 7.5) |

Every finding ships with DOM node, AXTree path, screenshot hash, and justification. `rgaa-remediation` turns verdicts into framework-aware patches (React/Vue/Angular/vanilla) that the agent re-verifies against a fresh screenshot.

**Speed**: ~20–40× faster than manual audits (≈1–2h of consultant review for a 15-page site instead of 45–75h), with up to 8 concurrent audits and tiered LLM routing.

---

## Official Documents (12 EU Countries)

An audit is only half the job — Holo-RGAA produces the documents administrations and courts expect:

- **Déclaration d'accessibilité** — publishable HTML per country: FR 7-section DINUM model, UE 2018/1523 core with EN/DE templates, per-country enforcement blocks. Semantic markup, no intrusive CSS.
- **Audit report PDF** — tagged PDF via headless Chromium (PDF/A-1a profile for France).
- **JSON pivot** — schema-validated payload with per-country `extensions` and registry adapters (Ara, AgID open data, NL register, WAS-Tool DK, PT observatory).
- **Guardrails** — export is blocked on a short sample, missing feedback contact, unproven non-conformity, incomplete derogation, or a legal pack unreviewed for 12+ months (unless a named, timestamped override is recorded).

Covered: FR, DE, ES, IT, BE, NL, LU, PT, AT, IE, SE, DK. One audit, one set of figures everywhere — a single `rgaa-report` engine parameterised by national `Referentiel`, which the orchestrator, CLI and TUI all share.

---

## What is RGAA?

The **Référentiel Général d'Amélioration de l'Accessibilité (RGAA)** is France's accessibility standard (law 2005-102, art. 47), mandatory for public-sector sites and many private services. Version 4.1.2 defines **106 criteria** across 13 topics. Compliance follows the **Taux Global**:

```
Taux Global = Conforme / (Conforme + Non Conforme) × 100   (NA/NT excluded)
```

| Status | Threshold |
|--------|-----------|
| **Conforme** | Taux Global = 100% |
| **Partiellement Conforme** | Taux Global ≥ 50% |
| **Non Conforme** | Taux Global < 50% (or incomplete audit) |

Across the EU, directive 2016/2102 and EN 301 549 require the same kind of public accessibility statement with national transpositions — hence the 12-country packs above.

---

## Installation

```bash
curl -sSL https://raw.githubusercontent.com/jamon8888/Holo-RGAA/master/install.sh | bash
```

That's it. Launch the TUI with `rgaa`. Windows (PowerShell):

```powershell
irm https://raw.githubusercontent.com/jamon8888/Holo-RGAA/master/rgaa-rs/install.ps1 | iex
```

<details>
<summary>Details (optional)</summary>

The script detects your platform (Linux/macOS/Windows, x86_64/aarch64), downloads
`rgaa`, `rgaa-cli`, `rgaa-api`, `rgaa-mcp` from the latest GitHub release,
installs to `~/.local/bin/`, configures the Claude Code plugin, and verifies everything.

- Bleeding edge (rebuilt on every push to `master`):
  ```bash
  RGAA_VERSION=latest curl -sSL https://raw.githubusercontent.com/jamon8888/Holo-RGAA/master/install.sh | bash
  ```
- Interactive wizard (once `rgaa` is installed): `rgaa install`
- Build from source (pinned Rust 1.98.1, ~20-40 min first build):
  ```bash
  curl -sSL https://raw.githubusercontent.com/jamon8888/Holo-RGAA/master/install.sh | bash -s -- --build
  ```
- Uninstall:
  ```bash
  curl -sSL https://raw.githubusercontent.com/jamon8888/Holo-RGAA/master/install.sh | bash -s -- --uninstall
  ```

| Step | Action |
|------|--------|
| 1 | Detect platform (Linux/macOS/Windows, x86_64/aarch64) |
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
rgaa audit https://example.com      # headless audit
export HOLO3_API_KEY="your-key"     # AI-assisted evaluation
```

TUI shortcuts: `a` audit, `h` history, `s` settings, `q` quit.

---

## Quick Start

### Interactive TUI (Recommended)

```bash
rgaa
```

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
│      Exit rgaa                       │
└──────────────────────────────────────┘
```

The **Audit Wizard** runs the audit live with color-coded scores, per-criterion drill-down (violations, justification, confidence), install wizard, and settings wizard for the Holo3 key.

### CLI

```bash
rgaa install                          # install wizard (interactive TUI)
rgaa audit                            # audit wizard (interactive TUI)
rgaa audit https://example.com --export results.json
rgaa history                          # past audits
rgaa config show                      # current configuration
rgaa config set api-key "your-key"    # Holo3 API key
rgaa config set base-url "https://api.example.com"

# Policy gate — block CI if non-compliant
rgaa audit --url https://example.com --export audit-bundle.json
rgaa-cli policy --input audit-bundle.json --threshold 85
```

### Claude Code (MCP)

```bash
Audit https://example.com for RGAA compliance and report the global rate.
```

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

resp = requests.post("http://localhost:3000/audit", json={
    "url": "https://example.com",
    "config": {"max_pages": 10, "max_depth": 3}
})
audit_id = resp.json()["audit_id"]

result = requests.get(f"http://localhost:3000/audit/{audit_id}").json()
print(f"Taux Global: {result['taux_global']}%")
print(f"Status: {result['etat_conformite']}")
```

---

## Features

### Unified Audit Pipeline
- Deterministic checks, gap-fix heuristics, and LLM evaluation in a single pass
- Spider crawl with configurable depth and page limits
- Evidence capture at every step (DOM snapshots, AXTree, screenshots with SHA-256 hashes)

### Interfaces
- **TUI** — `rgaa` Ratatui app: audit wizard, history, install/setup wizards
- **CLI** — `rgaa-cli` for terminal audits and CI integration
- **MCP Server** — `rgaa-mcp` for AI assistant (Claude Code) workflows
- **REST API** — `rgaa-api` for HTTP integration
- **Rust Library** — direct integration via `rgaa-core`

### Browser Automation
- Headless Chromium via CDP; clicks, form fills, wait-for states before scan
- Cookie injection **before** navigation (catches auth redirects); secrets via `RGAA_COOKIE_<NAME>`
- Keyboard IGT with stable DOM-path focus identity and trap detection
- Screenshots (PNG/JPEG) with configurable policy

### Remediation Workflow
- Patch proposals with diffs per failing criterion (React, Vue, Angular, vanilla)
- Approval states (required / auto-approved / rejected), batching (1–25 issues)
- Visual re-verification of fixes against fresh screenshots

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
              ┌────────────────┼────────────────────────┐
              │                │                        │
       ┌──────▼─────┐   ┌─────▼──────┐  ┌───────▼───────┐  ┌────────▼────────┐
       │  rgaa-tui  │   │  rgaa-mcp  │  │   rgaa-api    │  │  rgaa-report   │
       │  Ratatui   │   │   MCP 3.x  │  │  HTTP REST    │  │ compliance math │
       │  TUI app   │   └────────────┘  └───────────────┘  │ + documents     │
       └────────────┘                                     └─────────────────┘
```

### Core Crates

| Crate | Responsibility |
|-------|---------------|
| `rgaa-core` | Domain types, 106-criteria catalog, findings model |
| `rgaa-rules` | axe-core integration + gap-fix JavaScript snippets |
| `rgaa-holo` | Holo3 LLM client, prompt construction, response parsing |
| `rgaa-obscura` | CDP browser automation, cookie injection, IGT execution |
| `rgaa-agent` | Agentic evaluator + RAG stack (router, verifier, citations) |
| `rgaa-orchestrator` | Pipeline orchestration, result aggregation |
| `rgaa-report` | Compliance engine + official documents (12 countries) |
| `rgaa-mcp` | MCP server (analyze, audit_url, remediate, igt, get_audit_result, list_criteria) |
| `rgaa-cli` | CLI application (analyze, report, policy, igt) |
| `rgaa-api` | Axum HTTP API server |
| `rgaa-storage` | PostgreSQL persistence layer |
| `rgaa-remediation` | Fix proposal generation, approval workflow |
| `rgaa-spider` | Polite streaming crawler |
| `rgaa-data` | Shared static data (catalog, rule maps) |
| `rgaa-test-corpus` | Fixtures for regression tests |

### How Evaluation Works

1. **CDP Launch** — headless Chromium via Chrome DevTools Protocol
2. **Navigation** — cookies injected first, then page load
3. **Pre-scan Actions** — clicks, fills, wait-for states
4. **axe-core Run** — in-page violations + gap-fix patches for RGAA false negatives
5. **Holo3 Evaluation** — judgment-required criteria with DOM + screenshot context
6. **IGT Keyboard Test** — tab path capture, trap detection
7. **Aggregation** — findings merged with evidence, compliance computed once in `rgaa-report`

---

## Performance & Scalability

Production scale is enforced in code, not documentation: static data built once (`OnceLock`), up to 8 concurrent audits, LLM tiered routing with capped 8k-char page contexts and circuit breaker, polite streaming crawler, metadata-only history reads, per-request API limits with load shedding.

---

## RGAA Criteria Coverage

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

---

## Configuration

### Environment Variables

| Variable | Description | Required |
|----------|-------------|----------|
| `HOLO3_API_KEY` | Holo3 API key for LLM evaluation | Yes (for AI-assisted audits) |
| `RGAA_OBSCURA_BIN` | Path to Obscura browser binary | Yes |
| `DATABASE_URL` | PostgreSQL connection string | No (for storage) |
| `RUST_LOG` | Logging level (`info`, `debug`, `trace`) | No |

Cookie values can be injected from `RGAA_COOKIE_<NAME>` (e.g. `session` → `RGAA_COOKIE_SESSION`).

### Policy Configuration

```yaml
# .rgaa/config.yaml
policy:
  threshold: 85
  fail_on_needs_review: true
  allowed_domains:
    - "*.example.com"
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
| Language | Rust (pinned 1.98.1 via rust-toolchain.toml) |
| Async runtime | Tokio |
| Browser automation | Obscura (custom CDP client) |
| LLM client | Holo3 |
| MCP server | rmcp 3.1.3 |
| CLI | Clap 4.0 |
| HTTP API | Axum |
| Database | PostgreSQL 16 (optional) |
| Releases | GitHub Releases (Linux/macOS/Windows) |

---

## Project Structure

```
rgaa-rs/
  Cargo.toml              # Workspace root (17 crates)
  crates/
    rgaa-core/           # Domain types, 106-criteria catalog
    rgaa-rules/           # axe-core integration, gap-fix snippets
    rgaa-holo/           # Holo3 LLM client
    rgaa-browser-tools/  # Browser automation via CDP
    rgaa-obscura/        # CDP browser automation (Rust-native)
    rgaa-agent/          # Agentic evaluator + RAG stack
    rgaa-orchestrator/   # Pipeline orchestration
    rgaa-tui/            # Interactive TUI (Ratatui)
    rgaa-api/            # Axum HTTP API
    rgaa-mcp/             # MCP server
    rgaa-cli/            # CLI interface
    rgaa-storage/        # PostgreSQL storage
    rgaa-remediation/    # Fix proposal generation
    rgaa-report/         # Compliance engine + official documents
    rgaa-spider/         # Streaming crawler
    rgaa-data/           # Shared static data
    rgaa-test-corpus/    # Test fixtures
```

---

## Contributing

See [AGENTS.md](./AGENTS.md) for development guidelines.

---

## License

MIT
