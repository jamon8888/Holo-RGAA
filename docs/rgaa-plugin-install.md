# RGAA Accessibility Plugin - Installation Guide

## Overview

The RGAA Accessibility Plugin provides a complete workflow for auditing, triaging, remediating, and verifying accessibility issues against the RGAA (Référentiel Général d'Amélioration de l'Accessibilité) standard.

## Prerequisites

- **Rust 1.80+** - Install from [rustup.rs](https://rustup.rs/)
- **Claude Code** - Latest version with MCP support
- **Node.js 18+** - For the plugin hooks

## Quick Start

### 1. Build from source

```bash
git clone https://github.com/jamon8888/Holo-RGAA.git
cd Holo-RGAA/rgaa-rs
cargo build --release
```

The binaries will be in `target/release/`:
- `rgaa-cli` - Local audit CLI
- `rgaa-mcp` - MCP server for Claude Code
- `rgaa-api` - Remote API server (optional)

### 2. Install the Claude Code plugin

The plugin lives at `rgaa-rs/plugins/rgaa-consultant/` — that is the only plugin
tree. The top-level `claude-plugin/` directory this guide used to install from is
a deprecated pointer: it carries no manifest, and installing it gave you the
stale `rgaa-audit` 0.1.0 fork instead of `rgaa-accessibility` 2.0.0.

The repository carries a marketplace manifest (`.claude-plugin/marketplace.json`),
so the plugin installs through the normal plugin commands:

```bash
# From GitHub
claude plugin marketplace add jamon8888/Holo-RGAA
claude plugin install rgaa-accessibility@holo-rgaa

# Or from this clone, for development
claude plugin marketplace add ./
claude plugin install rgaa-accessibility@holo-rgaa

# Or let the installer do it (also removes a stale ~/.claude/plugins/rgaa-audit)
./install.sh
```

If you installed before this change, remove the old copy — otherwise Claude Code
loads two manifests for the same tools:

```bash
rm -rf ~/.claude/plugins/rgaa-audit
```

Check it loaded with `claude plugin details rgaa-accessibility`, or `/plugin`
inside Claude Code: `rgaa-accessibility` should be listed, with the
`/audit-site`, `/audit-project` and `/generate-report` commands, four agents and
two hooks. `claude plugin validate rgaa-rs/plugins/rgaa-consultant` validates the
tree without installing it.

### 3. Configure environment

```bash
# Required for AI-assisted remediation
export HOLO3_API_KEY="your-holo3-api-key"

# Optional: Remote bundle service
export REMOTE_API_KEY="your-remote-api-key"
export REMOTE_API_URL="https://your-api.example.com"

# Optional: Database for remote storage
export DATABASE_URL="postgres://localhost/rgaa"
```

## Usage

### Local Audit (Offline)

```bash
# Run a full accessibility audit
rgaa-cli analyze --url https://example.com

# Run guided tests
rgaa-cli igt --url https://example.com --criterion 1.1

# Generate reports
rgaa-cli report --url https://example.com --format sarif --output report.sarif
rgaa-cli report --url https://example.com --format junit --output report.xml
rgaa-cli report --url https://example.com --format markdown --output report.md

# Evaluate policy
rgaa-cli policy --baseline baseline.json --current current.json
```

### MCP Server (Claude Code)

The MCP server provides nine tools for Claude Code. These are the names the
server actually registers — call them exactly as written:

- `analyze` - Per-criterion findings for one page, with evidence and justification
- `audit_url` - Full site audit through the orchestrator; returns a summary (`taux_global`, `etat_conformite`) plus `sampled_page_urls`
- `get_audit_result` - Retrieve a previously run audit by `audit_id`
- `lint_static` - Static accessibility lint of HTML/JSX/TSX/Vue source: rule, severity, line/column and a fix hint, against a selectable profile (`rgaa-4.1`, `wcag-2.1-aa`, `section-508`). No browser and no build, so it suits an edit loop — a first pass over source, not a conformance verdict
- `list_criteria` - The 106 RGAA criteria with id, title, and classification
- `remediate` - Approval-gated remediation proposals
- `source_map` - Map a browser finding back to the template that produced it: `source_location` (file, line, column, snippet) per finding. Best-effort literal match over React JSX, Vue SFC, Angular and vanilla HTML; ambiguous findings return in `unmappable` rather than guessed
- `verify_fix` - Re-verify corrected files against a reference audit (`fixed` / `remaining` / `new` / `unverified`)
- `igt` - Guided keyboard test (**deprecated**: use `analyze` with `config.igt_tools: ["keyboard"]`)

`audit_url` returns a summary only. For per-criterion detail, call `analyze`
on the URLs it reports in `sampled_page_urls`.

#### HTTP transport

`rgaa-mcp-http` exposes the same nine tools as JSON-RPC over `POST /mcp`, with
progress events on `GET /mcp/events` (SSE). Cross-origin requests are denied
unless `RGAA_CORS_ORIGINS` names the allowed origins.

### API Server (Remote)

```bash
# Start the API server
rgaa-api

# Available endpoints:
# POST /audits - Create a new audit
# GET /audits - List audits
# GET /audits/:id - Get audit details
# POST /v1/audit-bundles - Upload audit bundle (requires API key)
# GET /v1/audit-bundles - List bundles (requires API key)
# GET /v1/findings - List findings (requires API key)
# POST /v1/policy/evaluate - Evaluate policy (requires API key)
# POST /v1/batches - Start a multi-URL batch audit (requires API key)
# GET /v1/batches/:id - Batch status, per-URL progress (requires API key)
# GET /v1/batches/:id/results - Final aggregated batch results (requires API key)
#   Batches expire 24h after creation; reads return 410 afterwards.
```

## Workflow

### 1. Analyze

```bash
rgaa-cli analyze --url https://example.com
```

Produces an `AuditBundle` with findings, criteria results, and evidence.

### 2. Triage

Review findings in the generated report. Each finding has:
- Rule ID (e.g., `image-alt`)
- Criterion (e.g., `RGAA-1.1`)
- Severity (critical, serious, moderate, minor)
- Source location

### 3. Remediate

```bash
rgaa-cli remediate --finding-id f-1 --approve
```

Generates patch proposals for approved findings. Supports:
- React/Next.js
- Vue.js
- Angular

### 4. Verify

```bash
rgaa-cli verify --url https://example.com --baseline baseline.json
```

Compares current audit against baseline to verify fixes.

### 5. Report

```bash
rgaa-cli report --url https://example.com --format sarif
```

Generates compliance reports in multiple formats:
- JSON - Full audit data
- SARIF - Static analysis interchange format
- JUnit - Test results for CI
- Markdown - Human-readable report

## CI Integration

### GitHub Actions

```yaml
- name: RGAA Accessibility Audit
  run: |
    cargo build --release
    ./target/release/rgaa-cli analyze --url ${{ secrets.STAGING_URL }}
    ./target/release/rgaa-cli policy --baseline .rgaa/baseline.json --current current.json
```

### Exit Codes

- `0` - Policy passed
- `1` - Policy failed (findings found)
- `2` - Analysis error
- `3` - Configuration error

## Troubleshooting

### "Obscura binary not found"

The Obscura binary provides browser automation. If not available:
- Use the `--skip-obscura` flag for basic analysis
- Install Obscura separately for full functionality

### "Holo3 API key required"

For AI-assisted remediation:
1. Get an API key from Holo3
2. Set `HOLO3_API_KEY` environment variable
3. Use `--remote` flag when remediating

### "Database connection failed"

For remote storage:
1. Ensure PostgreSQL is running
2. Create the database: `createdb rgaa`
3. Run migrations: `cargo run -- migrate`

## Architecture

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│  rgaa-cli   │────▶│  rgaa-core  │────▶│  rgaa-mcp   │
│  (CLI)      │     │  (Domain)   │     │  (MCP)      │
└─────────────┘     └─────────────┘     └─────────────┘
                           │
                           ▼
                    ┌─────────────┐
                    │rgaa-remediate│
                    │  (Proposals) │
                    └─────────────┘
                           │
                           ▼
                    ┌─────────────┐
                    │  rgaa-api   │
                    │  (Remote)   │
                    └─────────────┘
```

## License

MIT
