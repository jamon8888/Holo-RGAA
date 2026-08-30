# RGAA Plugin Connectors

This plugin connects to accessibility auditing tools via MCP (Model Context Protocol) or direct CLI invocation.

## Connection Options

### 1. RGAA CLI (Local)

The `rgaa-cli` crate provides the full auditing engine locally.

```bash
cargo install --path rgaa-rs/crates/rgaa-cli
```

**Tools used:**
- `rgaa audit analyze` — Run full audit
- `rgaa audit igt` — Guided manual tests
- `rgaa audit verify` — Verify remediation proposals
- `rgaa audit report` — Generate formatted reports
- `rgaa audit policy` — Check compliance thresholds

### 2. RGAA API Server (HTTP)

The `rgaa-api` crate exposes the auditing engine as a REST API.

```bash
cargo install --path rgaa-rs/crates/rgaa-api
DATABASE_URL=postgres://localhost/rgaa rgaa-api
```

**Endpoints:**
- `POST /audit` — Run new audit
- `GET /audit/{id}` — Retrieve audit
- `GET /criteria` — List all 106 criteria
- `GET /health` — Health check

**MCP Configuration:**
```json
{
  "mcpServers": {
    "rgaa-api": {
      "type": "http",
      "url": "http://localhost:3000"
    }
  }
}
```

### 3. RGAA MCP Server (For Claude Code)

The `rgaa-mcp` crate provides MCP tools for Claude Code integration.

```bash
cargo install --path rgaa-rs/crates/rgaa-mcp
```

**MCP Configuration (Claude Code):**
```json
{
  "mcpServers": {
    "rgaa": {
      "command": "rgaa-mcp",
      "env": {
        "RGAA_OBSCURA_BIN": "/path/to/obscura"
      }
    }
  }
}
```

### 4. Browser Automation (Obscura)

For CDP-based browser automation (screenshots, AXTree, keyboard interactions):

```bash
# Install Obscura
curl -L -o obscura https://github.com/example/obscura/releases/latest
chmod +x obscura
export RGAA_OBSCURA_BIN=/path/to/obscura
```

## Environment Variables

| Variable | Used By | Description |
|----------|---------|-------------|
| `HOLO3_API_KEY` | rgaa-holo | LLM API key for AI-assisted evaluation |
| `RGAA_OBSCURA_BIN` | rgaa-mcp, rgaa-cli | Path to Obscura browser binary |
| `DATABASE_URL` | rgaa-api | PostgreSQL connection string |
| `RUST_LOG` | All | Logging level (debug, info, warn) |

## Graceful Degradation

The plugin works with partial tool availability:

| Tool Available | Behavior |
|----------------|----------|
| `rgaa-cli` | Full local functionality |
| `rgaa-api` | HTTP API (may be slower) |
| Neither | Claude explains what audit would do; no runtime verification |
