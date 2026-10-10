#!/usr/bin/env bash
# Verify that the Codex package exposes the expected skills and MCP server.

set -euo pipefail

PLUGIN_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
REPO_ROOT="$(cd "$PLUGIN_ROOT/../../.." && pwd)"
FAILURES=0

fail() {
  echo "FAIL: $1" >&2
  FAILURES=$((FAILURES + 1))
}

require_file() {
  [[ -f "$PLUGIN_ROOT/$1" ]] || fail "missing $1"
}

for file in .codex-plugin/plugin.json .mcp.json README.md; do
  require_file "$file"
done

for skill in audit triage remediate verify report guided-test criteria; do
  skill_file="$PLUGIN_ROOT/skills/$skill/SKILL.md"
  require_file "skills/$skill/SKILL.md"
  if [[ -f "$skill_file" ]]; then
    grep -q "^name: $skill$" "$skill_file" || fail "invalid name front matter in skills/$skill/SKILL.md"
    grep -q '^description:' "$skill_file" || fail "missing description in skills/$skill/SKILL.md"
  fi
done

if command -v jq >/dev/null 2>&1; then
  jq -e '.name == "rgaa-accessibility-codex" and .version and .skills == "./skills/" and .mcpServers == "./.mcp.json"' \
    "$PLUGIN_ROOT/.codex-plugin/plugin.json" >/dev/null 2>&1 || fail "invalid Codex plugin manifest contract"
  jq -e '.mcpServers."rgaa-mcp".type == "stdio" and .mcpServers."rgaa-mcp".command == "rgaa-mcp"' \
    "$PLUGIN_ROOT/.mcp.json" >/dev/null 2>&1 || fail "missing local rgaa-mcp server configuration"
  jq empty "$REPO_ROOT/.agents/plugins/marketplace.json" >/dev/null 2>&1 || fail "invalid repo plugin marketplace JSON"
  jq -e '.plugins[] | select(.name == "rgaa-accessibility-codex" and .source.path == "./rgaa-rs/plugins/rgaa-codex")' \
    "$REPO_ROOT/.agents/plugins/marketplace.json" >/dev/null 2>&1 || fail "Codex plugin missing from repo marketplace"

  registered_tools=$(grep -oE 'name = "[a-z_]+"' "$REPO_ROOT/rgaa-rs/crates/rgaa-mcp/src/server.rs" | sed 's/name = "//; s/"//' | sort -u)
  documented_tools=$(grep -oE '^\| *`[a-z_]+` *\|' "$PLUGIN_ROOT/README.md" | tr -d '|` ' | sort -u)
  if [[ "$registered_tools" != "$documented_tools" ]]; then
    fail "README MCP tool table differs from tools registered in rgaa-mcp"
  fi
else
  fail "jq is required to validate plugin JSON"
fi

if (( FAILURES > 0 )); then
  echo "Codex plugin contract failed ($FAILURES issue(s))." >&2
  exit 1
fi

echo "Codex plugin contract passed."
