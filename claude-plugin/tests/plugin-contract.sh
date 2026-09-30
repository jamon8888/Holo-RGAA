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
SERVER_RS="$PLUGIN_ROOT/../rgaa-rs/crates/rgaa-mcp/src/server.rs"
if [[ -f "$SERVER_RS" ]]; then
  registered=$(grep -oE 'name = "[a-z_]+"' "$SERVER_RS" | sed 's/name = "//; s/"//' | sort -u)
  if [[ -z "$registered" ]]; then
    echo "❌ TOOL CONTRACT: found no registered tools in server.rs (parser drifted?)"
    FAILURES=$((FAILURES + 1))
  fi
  undocumented=0
  for tool in $registered; do
    # Word-boundary match, so `analyze` counts whether the README writes it
    # bare, as `analyze`, or as `analyze(AnalyzeRequest) -> AnalyzeResponse`.
    if ! grep -qE "\\b${tool}\\b" "$PLUGIN_ROOT/README.md"; then
      echo "❌ UNDOCUMENTED TOOL: server registers '$tool' but README.md does not mention it"
      FAILURES=$((FAILURES + 1))
      undocumented=$((undocumented + 1))
    fi
  done
  if [[ $undocumented -eq 0 ]]; then
    echo "✅ TOOL CONTRACT: README documents all $(echo "$registered" | wc -w) registered tools"
  fi

  # Phantom names: documented tools the server does not register.
  for doc in "$PLUGIN_ROOT/README.md" "$PLUGIN_ROOT/../docs/rgaa-plugin-install.md"; do
    [[ -f "$doc" ]] || continue
    for phantom in rgaa_analyze rgaa_remediate rgaa_igt rgaa_audit_url; do
      if grep -q "$phantom" "$doc"; then
        echo "❌ PHANTOM TOOL: $(basename "$doc") documents '$phantom', which no server registers"
        FAILURES=$((FAILURES + 1))
      fi
    done
  done
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