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

```bash
# Copy the plugin to Claude Code's plugin directory
cp -r claude-plugin ~/.claude/plugins/rgaa-accessibility

# Or symlink for development
ln -s $(pwd)/claude-plugin ~/.claude/plugins/rgaa-accessibility
```

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

The MCP server provides three tools for Claude Code:

- `rgaa_analyze` - Analyze a page for accessibility issues
- `rgaa_remediate` - Generate remediation proposals
- `rgaa_igt` - Run guided tests

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
