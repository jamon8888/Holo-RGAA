# Task 4 Report: Update CONNECTORS.md

**Task:** Replace section 3 of `rgaa-rs/plugins/rgaa-consultant/CONNECTORS.md` with refreshed rgaa-mcp primary MCP content.

**Commit:** `9eeafbd` — `docs(plugin): refresh CONNECTORS.md — rgaa-mcp is primary MCP`

## Changes Made

- **Section 3 title** updated to `### 3. RGAA MCP Server (For Claude Code) — PRIMARY`
- **Tools description** updated to list all 6 MCP tools: `analyze`, `audit_url`, `igt`, `remediate`, `get_audit_result`, `list_criteria`
- **MCP Configuration** refreshed to use `"rgaa-mcp"` as the server name, `"type": "local"`, command as array `["rgaa-mcp"]`, and `${CLAUDE_PLUGIN_ROOT}/bin/obscura` for the env var
- **Install command** updated to `cargo install --path rgaa-rs/crates/rgaa-mcp`
