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
      "type": "stdio",
      "command": "rgaa-mcp",
      "env": {
        "RGAA_OBSCURA_BIN": "${CLAUDE_PLUGIN_ROOT}/bin/obscura",
        "RGAA_LICENSE_KEY": "<your licence key — or omit it after `rgaa licence activate`>"
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
| `MYIA_API_KEY` | rgaa-holo | LLM API key for AI-assisted evaluation |
| `RGAA_OBSCURA_BIN` | rgaa-mcp, rgaa-cli | Path to Obscura browser binary |
| `RGAA_LICENSE_KEY` | rgaa-mcp, rgaa-mcp-http, rgaa-api, rgaa-cli | Licence token; overrides `~/.config/rgaa/license.toml`. When unset, the stored licence applies if present; otherwise Free tier. A blank value is ignored (the file applies) but still shadows a value in `.env` |
| `RGAA_LICENSE_PUBLIC_KEYS` | same | `k4.public.…` keys trusted to sign licences (needed until a built-in key ships) |
| `RGAA_LICENSE_SERVER` | same | Non-empty toggle for the offline grace period; its value is not read and no revalidation request is made |
| `RGAA_LICENSE_GRACE_DAYS` | same | Offline grace period in days (default 7) |
| `RGAA_LICENSE_UPGRADE_URL` | rgaa-cli | Upgrade page quoted in licence refusal messages |
| `DATABASE_URL` | rgaa-api | PostgreSQL connection string |
| `RUST_LOG` | All | Logging level (debug, info, warn) |

## Graceful Degradation

The plugin works with partial tool availability:

| Tool Available | Behavior |
|----------------|----------|
| `rgaa-cli` | Full local functionality |
| `rgaa-api` | HTTP API (may be slower) |
| Neither | Claude explains what audit would do; no runtime verification |
