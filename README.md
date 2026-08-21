# rgaa-rs

**RGAA 4.1.2 accessibility audit engine in Rust — deterministic checks, gap-fix heuristics, and LLM-assisted evaluation, unified in a single pipeline.**

rgaa-rs replaces Asqatasun (the legacy Java-based RGAA auditor) with a high-performance Rust workspace. It maps all 106 RGAA criteria, bridges axe-core's 77 deterministic rules with 10 custom gap-fix JavaScript snippets for false negatives, and routes the 27 judgment-required criteria to Holo3 LLM evaluation.

---

## Quick Install

```bash
curl -sSL https://raw.githubusercontent.com/jamon8888/Holo-RGAA/main/install.sh | bash
```

One command. No Rust toolchain required. Downloads pre-built binaries, installs to `~/.local/bin`, and configures the Claude Code MCP server.

### Build from Source

```bash
curl -sSL https://raw.githubusercontent.com/jamon8888/Holo-RGAA/main/install.sh | bash -s -- --build
```

Installs Rust if needed, builds the workspace (~5-10 min), and sets up everything.

### What the Installer Does

| Step | What |
|------|------|
| 1 | Detects platform (Linux x86-64, macOS arm64/x86-64) |
| 2 | Downloads pre-built `rgaa-mcp`, `rgaa-cli`, `obscura` |
| 3 | Installs to `~/.local/bin/` |
| 4 | Symlinks Claude Code plugin to `~/.claude/plugins/rgaa-audit` |
| 5 | Writes MCP config to `~/.claude/mcp.json` |
| 6 | Creates `.rgaa/config.yaml` if missing |
| 7 | Verifies installation |

### After Install

```bash
# Ensure ~/.local/bin is in your PATH
export PATH="$HOME/.local/bin:$PATH"

# Set your Holo3 API key for AI-assisted evaluation
export HOLO3_API_KEY="your-key"

# Restart Claude Code to load the MCP server
# Then use: audit, triage, remediate, verify, report, guided-test
```

### Uninstall

```bash
curl -sSL https://raw.githubusercontent.com/jamon8888/Holo-RGAA/main/install.sh | bash -s -- --卸载
```

---

## CLI Usage

```bash
# Run a full RGAA audit
rgaa-cli analyze --url https://example.test

# Run a guided accessibility test
rgaa-cli igt --test keyboard-navigation

# Generate a compliance report (JSON, Markdown, SARIF, JUnit)
rgaa-cli report --format sarif

# Policy gate — check against compliance thresholds
rgaa-cli policy --input audit-bundle.json
```

## MCP Server

The installer configures the MCP server automatically. Manual setup:

```json
{
  "mcpServers": {
    "rgaa-mcp": {
      "command": "rgaa-mcp",
      "env": { "RGAA_OBSCURA_BIN": "/path/to/obscura" }
    }
  }
}
```

The MCP server exposes 3 tools for Claude Code: `analyze`, `remediate`, `igt`.

---

## Architecture

```
┌─────────────────────────────────────────────┐
│            rgaa-orchestrator                │
│  (full audit pipeline: axe + gap-fix + LLM)│
└──────┬──────────┬──────────┬───────────────┘
       │          │          │
  ┌────▼────┐ ┌───▼───┐ ┌───▼──────────┐
  │rgaa-rules│ │rgaa-  │ │rgaa-obscura  │
  │axe-core  │ │holo   │ │CDP browser   │
  │+ gap-fix │ │Holo3  │ │automation    │
  └────┬────┘ └───┬───┘ └───┬──────────┘
       └─────┬────┘         │
       ┌─────▼──────────────▼─────┐
       │       rgaa-core          │
       │ 106 criteria · Findings  │
       └─────┬──────┬──────┬─────┘
             │      │      │
      ┌──────▼┐ ┌───▼──┐ ┌─▼──────────┐
      │rgaa-  │ │rgaa- │ │rgaa-       │
      │mcp    │ │cli   │ │remediation │
      │MCP    │ │CLI   │ │lifecycle   │
      └───────┘ └──────┘ └────────────┘
```

---

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Language | **Rust 1.80+** |
| Async runtime | **Tokio** |
| Browser automation | **Obscura** (custom CDP) |
| LLM client | **Holo3** (H Company) |
| MCP server | **rmcp 3.1.3** |
| CLI | **Clap 4.0** |
| Database | **PostgreSQL 16** (optional) |
| CI | **GitHub Actions** |

---

## Report Formats

| Format | Use Case |
|--------|----------|
| **JSON** | Machine consumption, CI pipelines |
| **Markdown** | Human-readable reports |
| **SARIF 2.1.0** | Static analysis interchange, GitHub Code Scanning |
| **JUnit XML** | CI test results |

---

## License

MIT
