#!/usr/bin/env bash
# check-runtime.sh — Detect framework/config and mark findings stale after edits

set -euo pipefail

CLAUDE_PLUGIN_ROOT="${CLAUDE_PLUGIN_ROOT:-${0%/*}/..}"
CLAUDE_PROJECT_DIR="${CLAUDE_PROJECT_DIR:-.}"

log() {
  echo "[rgaa-runtime] $*" >&2
}

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

# Main: on SessionStart, detect framework and config
if [[ "${HOOK_EVENT:-}" == "SessionStart" ]]; then
  framework=$(detect_framework "$CLAUDE_PROJECT_DIR")
  log "Detected framework: $framework"
  # Export for downstream skills/agents
  echo "RGAA_FRAMEWORK=$framework" >> "${CLAUDE_PROJECT_DIR}/.rgaa/env"
  if [[ -f "${CLAUDE_PROJECT_DIR}/.rgaa/config.yaml" ]]; then
    log "Found .rgaa/config.yaml"
  else
    log "No .rgaa/config.yaml found; using defaults"
  fi
fi

# Main: on PostToolUse Edit|Write, mark affected findings stale
if [[ "${HOOK_EVENT:-}" == "PostToolUse" ]]; then
  tool_name="${TOOL_NAME:-}"
  file_path="${TOOL_FILE_PATH:-}"
  if [[ "$tool_name" =~ ^(Edit|Write)$ ]] && [[ -n "$file_path" ]]; then
    mark_stale "$file_path"
  fi
fi

exit 0