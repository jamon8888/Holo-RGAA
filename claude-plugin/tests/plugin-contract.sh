#!/usr/bin/env bash
# plugin-contract.sh — Validate Claude Code plugin structure and contracts

set -euo pipefail

PLUGIN_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
FAILURES=0

check_file() {
  local path="$1"
  local desc="$2"
  if [[ ! -f "$PLUGIN_ROOT/$path" ]]; then
    echo "❌ MISSING: $desc ($path)"
    FAILURES=$((FAILURES + 1))
  else
    echo "✅ EXISTS: $desc"
  fi
}

check_json_valid() {
  local path="$1"
  local desc="$2"
  if ! jq empty "$PLUGIN_ROOT/$path" 2>/dev/null; then
    echo "❌ INVALID JSON: $desc ($path)"
    FAILURES=$((FAILURES + 1))
  else
    echo "✅ VALID JSON: $desc"
  fi
}

check_manifest_fields() {
  local path="$1"
  local fields=("$@")
  for field in "${fields[@]:1}"; do
    if ! jq -e ".$field" "$PLUGIN_ROOT/$path" >/dev/null 2>&1; then
      echo "❌ MISSING FIELD: $field in $path"
      FAILURES=$((FAILURES + 1))
    fi
  done
}

echo "=== Plugin Contract Validation ==="

# Required files
check_file ".claude-plugin/plugin.json" "Plugin manifest"
check_file ".mcp.json" "MCP config"
check_file "README.md" "Documentation"
check_file "scripts/check-runtime.sh" "Runtime check script"
check_file "hooks/hooks.json" "Hooks config"

# Skills
for skill in audit triage remediate verify report guided-test; do
  check_file "skills/$skill/SKILL.md" "Skill: $skill"
done

# Agents
for agent in scanner remediation-planner verification-reviewer compliance-report-writer; do
  check_file "agents/$agent.md" "Agent: $agent"
done

# JSON validity
check_json_valid ".claude-plugin/plugin.json" "Plugin manifest"
check_json_valid ".mcp.json" "MCP config"
check_json_valid "hooks/hooks.json" "Hooks config"

# Manifest required fields
check_manifest_fields ".claude-plugin/plugin.json" name version description author license

# Scripts executable
if [[ -x "$PLUGIN_ROOT/scripts/check-runtime.sh" ]]; then
  echo "✅ EXECUTABLE: scripts/check-runtime.sh"
else
  echo "❌ NOT EXECUTABLE: scripts/check-runtime.sh"
  FAILURES=$((FAILURES + 1))
fi

# No API keys in tracked files
if grep -r "sk-" "$PLUGIN_ROOT" --include="*.json" --include="*.yaml" --include="*.yml" --include="*.sh" 2>/dev/null | grep -v "your-api-key" | grep -v "REDACTED" | grep -v "example" | grep -v "placeholder"; then
  echo "❌ POTENTIAL API KEY FOUND in tracked files"
  FAILURES=$((FAILURES + 1))
else
  echo "✅ NO API KEYS in tracked files"
fi

# Skill files have required front matter
for skill in audit triage remediate verify report guided-test; do
  if ! grep -q "^name:" "$PLUGIN_ROOT/skills/$skill/SKILL.md" 2>/dev/null; then
    echo "❌ MISSING FRONT MATTER 'name' in skills/$skill/SKILL.md"
    FAILURES=$((FAILURES + 1))
  fi
  if ! grep -q "^description:" "$PLUGIN_ROOT/skills/$skill/SKILL.md" 2>/dev/null; then
    echo "❌ MISSING FRONT MATTER 'description' in skills/$skill/SKILL.md"
    FAILURES=$((FAILURES + 1))
  fi
done

# Documented MCP tools must match the ones the server actually registers.
#
# This is the check that was missing when the README said "three tools" for a
# six-tool server, and when docs/rgaa-plugin-install.md advertised
# `rgaa_analyze` / `rgaa_remediate` / `rgaa_igt` — three names no server has
# ever registered. Both read as correct to anyone not holding server.rs open,
# so the drift is invisible without comparing the two.
#
# Both directions are derived, never hardcoded. A hardcoded phantom list only
# catches the names already known to be wrong, so the next invented tool name
# would pass; and a free-text search for a registered name matches incidental
# prose, so a tool "documented" only by being mentioned in a sentence would
# pass too. Both sides therefore read the structured lists:
#
#   README.md                  | `tool` | ... |   (the tool table)
#   docs/rgaa-plugin-install.md - `tool` - ...    (the tool bullet list)
SERVER_RS="$PLUGIN_ROOT/../rgaa-rs/crates/rgaa-mcp/src/server.rs"
INSTALL_DOC="$PLUGIN_ROOT/../docs/rgaa-plugin-install.md"

# Tool names from a markdown table's first column: | `name` | ... |
table_tools() {
  grep -oE '^\| *`[a-z_]+` *\|' "$1" 2>/dev/null | tr -d '|` ' | sort -u
}

# Tool names from a markdown bullet list: - `name` - description
bullet_tools() {
  grep -oE '^- *`[a-z_]+` +-' "$1" 2>/dev/null | sed -E 's/^- *`([a-z_]+)` +-/\1/' | sort -u
}

if [[ -f "$SERVER_RS" ]]; then
  registered=$(grep -oE 'name = "[a-z_]+"' "$SERVER_RS" | sed 's/name = "//; s/"//' | sort -u)
  if [[ -z "$registered" ]]; then
    echo "❌ TOOL CONTRACT: found no registered tools in server.rs (parser drifted?)"
    FAILURES=$((FAILURES + 1))
  fi

  readme_tools=$(table_tools "$PLUGIN_ROOT/README.md")
  if [[ -z "$readme_tools" ]]; then
    echo "❌ TOOL CONTRACT: README.md has no tool table (expected rows like '| \`analyze\` | ... |')"
    FAILURES=$((FAILURES + 1))
  fi
  install_tools=$(bullet_tools "$INSTALL_DOC")

  # Forward: every registered tool must appear in the README tool table.
  undocumented=0
  for tool in $registered; do
    if ! grep -qx "$tool" <<< "$readme_tools"; then
      echo "❌ UNDOCUMENTED TOOL: server registers '$tool' but the README tool table does not list it"
      FAILURES=$((FAILURES + 1))
      undocumented=$((undocumented + 1))
    fi
  done
  if [[ $undocumented -eq 0 && -n "$registered" && -n "$readme_tools" ]]; then
    echo "✅ TOOL CONTRACT: README table documents all $(echo "$registered" | wc -w) registered tools"
  fi

  # Reverse: every documented name must be a registered tool. Derived from
  # the same structured lists, so an invented name fails whatever it is
  # called — not only the three that were wrong when this check was written.
  phantoms=0
  check_phantoms() {
    local label="$1"
    shift
    for name in "$@"; do
      if ! grep -qx "$name" <<< "$registered"; then
        echo "❌ PHANTOM TOOL: $label documents '$name', which no server registers"
        FAILURES=$((FAILURES + 1))
        phantoms=$((phantoms + 1))
      fi
    done
  }
  # shellcheck disable=SC2086
  check_phantoms "README.md" $readme_tools
  # shellcheck disable=SC2086
  check_phantoms "$(basename "$INSTALL_DOC")" $install_tools
  if [[ $phantoms -eq 0 ]]; then
    echo "✅ TOOL CONTRACT: no phantom tool names in README.md or $(basename "$INSTALL_DOC")"
  fi
else
  echo "⚠️  SKIP TOOL CONTRACT: $SERVER_RS not found (plugin checked out standalone)"
fi

echo
if [[ $FAILURES -eq 0 ]]; then
  echo "✅ ALL CONTRACT CHECKS PASSED"
  exit 0
else
  echo "❌ $FAILURES CONTRACT CHECK(S) FAILED"
  exit 1
fi