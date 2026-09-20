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
    ((FAILURES++))
  else
    echo "✅ EXISTS: $desc"
  fi
}

check_json_valid() {
  local path="$1"
  local desc="$2"
  if ! jq empty "$PLUGIN_ROOT/$path" 2>/dev/null; then
    echo "❌ INVALID JSON: $desc ($path)"
    ((FAILURES++))
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
      ((FAILURES++))
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
  ((FAILURES++))
fi

# No API keys in tracked files
if grep -r "sk-" "$PLUGIN_ROOT" --include="*.json" --include="*.yaml" --include="*.yml" --include="*.sh" 2>/dev/null | grep -v "your-api-key" | grep -v "REDACTED" | grep -v "example" | grep -v "placeholder"; then
  echo "❌ POTENTIAL API KEY FOUND in tracked files"
  ((FAILURES++))
else
  echo "✅ NO API KEYS in tracked files"
fi

# Skill files have required front matter
for skill in audit triage remediate verify report guided-test; do
  if ! grep -q "^name:" "$PLUGIN_ROOT/skills/$skill/SKILL.md" 2>/dev/null; then
    echo "❌ MISSING FRONT MATTER 'name' in skills/$skill/SKILL.md"
    ((FAILURES++))
  fi
  if ! grep -q "^description:" "$PLUGIN_ROOT/skills/$skill/SKILL.md" 2>/dev/null; then
    echo "❌ MISSING FRONT MATTER 'description' in skills/$skill/SKILL.md"
    ((FAILURES++))
  fi
done

echo
if [[ $FAILURES -eq 0 ]]; then
  echo "✅ ALL CONTRACT CHECKS PASSED"
  exit 0
else
  echo "❌ $FAILURES CONTRACT CHECK(S) FAILED"
  exit 1
fi