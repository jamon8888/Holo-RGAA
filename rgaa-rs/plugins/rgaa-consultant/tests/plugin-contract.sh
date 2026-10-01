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
for skill in audit triage remediate verify report guided-test criteria; do
  check_file "skills/$skill/SKILL.md" "Skill: $skill"
done

# Commands — only the canonical tree has them, and a missing command file is a
# slash command that silently does not exist.
for command in audit-site audit-project generate-report; do
  check_file "commands/$command.md" "Command: $command"
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
for skill in audit triage remediate verify report guided-test criteria; do
  if ! grep -q "^name:" "$PLUGIN_ROOT/skills/$skill/SKILL.md" 2>/dev/null; then
    echo "❌ MISSING FRONT MATTER 'name' in skills/$skill/SKILL.md"
    FAILURES=$((FAILURES + 1))
  fi
  if ! grep -q "^description:" "$PLUGIN_ROOT/skills/$skill/SKILL.md" 2>/dev/null; then
    echo "❌ MISSING FRONT MATTER 'description' in skills/$skill/SKILL.md"
    FAILURES=$((FAILURES + 1))
  fi
done

# Command and agent files have front matter too: a command with no
# `description` shows up unlabelled in `/plugin`, and an agent with no `name`
# cannot be addressed.
for command in audit-site audit-project generate-report; do
  if ! grep -q "^description:" "$PLUGIN_ROOT/commands/$command.md" 2>/dev/null; then
    echo "❌ MISSING FRONT MATTER 'description' in commands/$command.md"
    FAILURES=$((FAILURES + 1))
  fi
done

for agent in scanner remediation-planner verification-reviewer compliance-report-writer; do
  for field in name description; do
    if ! grep -q "^$field:" "$PLUGIN_ROOT/agents/$agent.md" 2>/dev/null; then
      echo "❌ MISSING FRONT MATTER '$field' in agents/$agent.md"
      FAILURES=$((FAILURES + 1))
    fi
  done
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
# Paths are relative to the plugin root, which is now
# rgaa-rs/plugins/rgaa-consultant/ — two levels under rgaa-rs/, three under the
# repository root.
REPO_ROOT="$(cd "$PLUGIN_ROOT/../../.." && pwd)"
SERVER_RS="$REPO_ROOT/rgaa-rs/crates/rgaa-mcp/src/server.rs"
INSTALL_DOC="$REPO_ROOT/docs/rgaa-plugin-install.md"
INTEGRATION_DOC="$REPO_ROOT/docs/plugin-integration.md"

# Print a markdown file from the heading matching $2 up to the next heading of
# the same or higher level.
#
# Every check below reads a slice, never the whole file, because each of these
# docs says true things elsewhere that a whole-file reading misreports:
# plugin-integration.md has a binaries table whose first column is `rgaa`
# (read as a phantom tool), and a paragraph recounting the drift this script
# exists to stop — "described three tools for a six-tool server" (read as a
# stale count). Both are correct text. A check that fails on correct text gets
# switched off, and then it guards nothing.
doc_section() {
  local file="$1" heading_re="$2"
  [[ -f $file ]] || return 0
  awk -v re="$heading_re" '
    !started && $0 ~ re {
      started = 1
      h = $0; sub(/[^#].*$/, "", h); lvl = length(h)
      next
    }
    started {
      if ($0 ~ /^#+[ \t]/) {
        h = $0; sub(/[^#].*$/, "", h)
        if (length(h) <= lvl) exit
      }
      print
    }
  ' "$file"
}

# Tool names from a markdown table's first column: | `name` | ... |
table_tools() {
  grep -oE '^\| *`[a-z_]+` *\|' 2>/dev/null | tr -d '|` ' | sort -u
}

# Tool names from a markdown bullet list: - `name` - description
bullet_tools() {
  grep -oE '^- *`[a-z_]+` +-' 2>/dev/null | sed -E 's/^- *`([a-z_]+)` +-/\1/' | sort -u
}

# Counts claimed in prose: "nine tools", "the same six tools". Markdown
# emphasis is stripped first, because the claim is as often written
# `**nine** tools` as plainly, and a checker that reads only one of the two
# forms is a checker that passes the drifted half.
count_words() {
  sed -E 's/[*_]//g' |
    grep -oiE '\b(one|two|three|four|five|six|seven|eight|nine|ten|eleven|twelve|[0-9]+) tools?\b' |
    awk '{print tolower($1)}' | sort -u
}

# Normalise a count token to a number, so "nine" and "9" compare equal.
# These docs spell the count out in prose far more often than they write a
# digit, so a numeric-only comparison would skip almost every claim it is
# meant to check.
word_to_num() {
  case "$1" in
    one) echo 1 ;; two) echo 2 ;; three) echo 3 ;; four) echo 4 ;;
    five) echo 5 ;; six) echo 6 ;; seven) echo 7 ;; eight) echo 8 ;;
    nine) echo 9 ;; ten) echo 10 ;; eleven) echo 11 ;; twelve) echo 12 ;;
    *) echo "$1" ;;
  esac
}

if [[ -f "$SERVER_RS" ]]; then
  registered=$(grep -oE 'name = "[a-z_]+"' "$SERVER_RS" | sed 's/name = "//; s/"//' | sort -u)
  if [[ -z "$registered" ]]; then
    echo "❌ TOOL CONTRACT: found no registered tools in server.rs (parser drifted?)"
    FAILURES=$((FAILURES + 1))
  fi

  README_DOC="$PLUGIN_ROOT/README.md"
  TOOL_SECTION_RE='^#+ .*(MCP Server|MCP tools)'

  readme_tools=$(doc_section "$README_DOC" "$TOOL_SECTION_RE" | table_tools)
  install_tools=$(doc_section "$INSTALL_DOC" "$TOOL_SECTION_RE" | bullet_tools)
  integration_tools=$(doc_section "$INTEGRATION_DOC" "$TOOL_SECTION_RE" | table_tools)

  # An empty list means the section was renamed or the list reshaped, not that
  # there is nothing to check. Reported, because silently skipping is how a
  # check stops guarding anything without anyone noticing.
  [[ -n $readme_tools ]] || {
    echo "❌ TOOL CONTRACT: no tool table found under the MCP section of README.md"
    FAILURES=$((FAILURES + 1))
  }
  [[ -n $install_tools ]] || {
    echo "❌ TOOL CONTRACT: no tool bullet list found under the MCP section of $(basename "$INSTALL_DOC")"
    FAILURES=$((FAILURES + 1))
  }
  [[ -n $integration_tools ]] || {
    echo "❌ TOOL CONTRACT: no tool table found under the MCP section of $(basename "$INTEGRATION_DOC")"
    FAILURES=$((FAILURES + 1))
  }

  undocumented=0

  # Report every registered tool absent from one doc's list ($2), named by
  # $label.
  #
  # Called for EVERY doc that enumerates the tool surface, not just the
  # README. Checking the README alone is how `lint_static` and `source_map`
  # reached master documented in one place and absent from two others: the
  # install guide listed seven tools and the integration guide six, and both
  # passed. A reader follows whichever file they opened, so a tool missing
  # from one of them is missing, full stop.
  check_documented() {
    local label="$1" documented="$2"
    [[ -z $documented ]] && return 0
    for tool in $registered; do
      if ! grep -qx "$tool" <<< "$documented"; then
        echo "❌ UNDOCUMENTED TOOL: server registers '$tool' but $label does not list it"
        FAILURES=$((FAILURES + 1))
        undocumented=$((undocumented + 1))
      fi
    done
  }
  check_documented "the README tool table" "$readme_tools"
  check_documented "$(basename "$INSTALL_DOC")" "$install_tools"
  check_documented "$(basename "$INTEGRATION_DOC")" "$integration_tools"
  if [[ $undocumented -eq 0 && -n "$registered" && -n "$readme_tools" ]]; then
    echo "✅ TOOL CONTRACT: all $(echo "$registered" | wc -w | tr -d ' ') registered tools documented in README, install and integration docs"
  fi

  # The count claimed in prose must match reality too. Every drift above was
  # accompanied by a stale number in the sentence introducing the list
  # ("seven tools", "the same six tools") that no check looked at, so the
  # docs contradicted both the server and each other while passing.
  n_registered=$(echo "$registered" | wc -w | tr -d ' ')
  miscounts=0
  for pair in \
    "README.md:$README_DOC" \
    "$(basename "$INSTALL_DOC"):$INSTALL_DOC" \
    "$(basename "$INTEGRATION_DOC"):$INTEGRATION_DOC"; do
    label=${pair%%:*}
    path=${pair#*:}
    [[ -f $path ]] || continue
    for w in $(doc_section "$path" "$TOOL_SECTION_RE" | count_words); do
      n=$(word_to_num "$w")
      if [[ $n != "$n_registered" ]]; then
        echo "❌ STALE TOOL COUNT: $label says '$w tools' but the server registers $n_registered"
        FAILURES=$((FAILURES + 1))
        miscounts=$((miscounts + 1))
      fi
    done
  done
  if [[ $miscounts -eq 0 && -n $registered ]]; then
    echo "✅ TOOL CONTRACT: prose tool counts agree with the $n_registered registered tools"
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
  # shellcheck disable=SC2086
  check_phantoms "$(basename "$INTEGRATION_DOC")" $integration_tools
  if [[ $phantoms -eq 0 ]]; then
    echo "✅ TOOL CONTRACT: no phantom tool names in README.md, $(basename "$INSTALL_DOC") or $(basename "$INTEGRATION_DOC")"
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