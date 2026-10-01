#!/usr/bin/env bash
# check-runtime.sh — Detect framework/config and mark findings stale after edits
#
# Claude Code hands a hook its payload as JSON on stdin (hook_event_name,
# tool_name, tool_input.file_path, cwd), not as HOOK_EVENT / TOOL_NAME
# environment variables. This script read those variables for both of its
# branches, so neither branch ever ran: every invocation fell through to
# `exit 0`. It now parses stdin, and keeps the environment variables as a
# fallback so it stays callable by hand.

set -euo pipefail

CLAUDE_PLUGIN_ROOT="${CLAUDE_PLUGIN_ROOT:-${0%/*}/..}"

log() {
  echo "[rgaa-runtime] $*" >&2
}

# Environment values first: a hand invocation can still set them, and the
# payload parsing below overwrites the same names.
ENV_HOOK_EVENT="${HOOK_EVENT:-}"
ENV_TOOL_NAME="${TOOL_NAME:-}"
ENV_FILE_PATH="${TOOL_FILE_PATH:-}"

# Read the payload once: stdin is not seekable, so each field is pulled from
# the same buffered copy rather than re-reading it.
PAYLOAD=""
if [[ ! -t 0 ]]; then
  PAYLOAD="$(cat || true)"
fi

# Pull one dotted path out of the payload, empty when absent or unparsable.
payload_field() {
  local path="$1"
  [[ -n $PAYLOAD ]] || return 0
  command -v jq >/dev/null 2>&1 || return 0
  jq -r "$path // empty" <<< "$PAYLOAD" 2>/dev/null || true
}

HOOK_EVENT="$(payload_field '.hook_event_name')"
HOOK_EVENT="${HOOK_EVENT:-$ENV_HOOK_EVENT}"
TOOL_NAME="$(payload_field '.tool_name')"
TOOL_NAME="${TOOL_NAME:-$ENV_TOOL_NAME}"
FILE_PATH="$(payload_field '.tool_input.file_path')"
FILE_PATH="${FILE_PATH:-$ENV_FILE_PATH}"
PAYLOAD_CWD="$(payload_field '.cwd')"
CLAUDE_PROJECT_DIR="${CLAUDE_PROJECT_DIR:-${PAYLOAD_CWD:-.}}"

# Detect framework from project structure
detect_framework() {
  local project_dir="$1"
  if [[ -f "$project_dir/package.json" ]]; then
    if grep -q '"next"' "$project_dir/package.json"; then
      echo "next"
      return
    fi
    if grep -q '"react"' "$project_dir/package.json"; then
      echo "react"
      return
    fi
    if grep -q '"vue"' "$project_dir/package.json"; then
      echo "vue"
      return
    fi
    if grep -q '"@angular/core"' "$project_dir/package.json"; then
      echo "angular"
      return
    fi
  fi
  echo "unknown"
}

# Mark audit state stale for files matching the edit
mark_stale() {
  local file="$1"
  local state_dir="${CLAUDE_PROJECT_DIR}/.rgaa/state"
  mkdir -p "$state_dir"
  # In a real implementation, this would update a finding-to-file mapping
  # For now, just touch a stale marker
  touch "${state_dir}/stale_$(basename "$file" | sed 's/[^a-zA-Z0-9]/_/g')_$(date +%s)"
  log "Marked audit state stale for $file"
}

# On SessionStart, detect framework and config
if [[ "$HOOK_EVENT" == "SessionStart" ]]; then
  framework=$(detect_framework "$CLAUDE_PROJECT_DIR")
  log "Detected framework: $framework"
  # Export for downstream skills/agents. The directory is created first: the
  # append used to fail outright on a project with no .rgaa/ yet, which is
  # every project on its first session.
  mkdir -p "${CLAUDE_PROJECT_DIR}/.rgaa"
  echo "RGAA_FRAMEWORK=$framework" >> "${CLAUDE_PROJECT_DIR}/.rgaa/env"
  if [[ -f "${CLAUDE_PROJECT_DIR}/.rgaa/config.yaml" ]]; then
    log "Found .rgaa/config.yaml"
  else
    log "No .rgaa/config.yaml found; using defaults"
  fi
fi

# On PostToolUse Edit|Write, mark affected findings stale
if [[ "$HOOK_EVENT" == "PostToolUse" ]]; then
  if [[ "$TOOL_NAME" =~ ^(Edit|Write|MultiEdit)$ ]] && [[ -n "$FILE_PATH" ]]; then
    mark_stale "$FILE_PATH"
  fi
fi

exit 0
