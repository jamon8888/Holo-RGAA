#!/usr/bin/env bash
# plugin-contract.sh — Validate the acces-accessibility plugin's structure.
#
# Cheap, no network, no build. Run from anywhere:
#   ./rgaa-rs/plugins/acces-accessibility/tests/plugin-contract.sh

set -uo pipefail

# Without jq every check below fails with a misleading message, blaming the
# JSON rather than the missing tool.
command -v jq >/dev/null || { echo "❌ jq is required by this script"; exit 1; }

PLUGIN_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
REPO_ROOT="$(cd "$PLUGIN_ROOT/../../.." && pwd)"
PLUGIN_NAME="acces-accessibility"
FAILURES=0

fail() { echo "❌ $1"; FAILURES=$((FAILURES + 1)); }
ok()   { echo "✅ $1"; }

need_file() {
  [[ -f "$PLUGIN_ROOT/$1" ]] && ok "exists: $1" || fail "missing: $1"
}

need_json() {
  if jq empty "$PLUGIN_ROOT/$1" 2>/dev/null; then ok "valid JSON: $1"
  else fail "invalid JSON: $1"; fi
}

echo "— files"
need_file ".claude-plugin/plugin.json"
need_file ".mcp.json"
need_file "README.md"

echo "— JSON"
need_json ".claude-plugin/plugin.json"
need_json ".mcp.json"

echo "— manifest"
MANIFEST="$PLUGIN_ROOT/.claude-plugin/plugin.json"
for field in name version description license; do
  v=$(jq -r ".$field // empty" "$MANIFEST" 2>/dev/null)
  [[ -n "$v" ]] && ok "manifest.$field = $v" || fail "manifest.$field is missing or empty"
done
name=$(jq -r '.name // empty' "$MANIFEST" 2>/dev/null)
[[ "$name" == "$PLUGIN_NAME" ]] \
  && ok "manifest.name matches the directory name" \
  || fail "manifest.name ($name) must equal the directory name ($PLUGIN_NAME)"

echo "— MCP server"
if jq -e '.mcpServers["rgaa-mcp"].command == "rgaa-mcp"' "$PLUGIN_ROOT/.mcp.json" >/dev/null 2>&1; then
  ok "declares the rgaa-mcp stdio server"
else
  fail ".mcp.json must declare mcpServers['rgaa-mcp'] with command 'rgaa-mcp'"
fi

echo "— skills"
shopt -s nullglob
skill_dirs=("$PLUGIN_ROOT"/skills/*/)
if [[ ${#skill_dirs[@]} -eq 0 ]]; then
  fail "no skills found under skills/"
fi
for dir in "${skill_dirs[@]}"; do
  slug=$(basename "$dir")
  md="$dir/SKILL.md"
  if [[ ! -f "$md" ]]; then
    fail "skills/$slug has no SKILL.md"
    continue
  fi
  # Front matter must be the first line and must close.
  if [[ "$(head -1 "$md")" != "---" ]]; then
    fail "skills/$slug/SKILL.md does not open with front matter"
    continue
  fi
  fm=$(sed -n '2,/^---$/p' "$md" | sed '$d')
  fm_name=$(printf '%s\n' "$fm" | sed -n 's/^name:[[:space:]]*//p' | head -1)
  fm_desc=$(printf '%s\n' "$fm" | sed -n 's/^description:[[:space:]]*//p' | head -1)
  [[ -n "$fm_desc" ]] && ok "skills/$slug has a description" \
                      || fail "skills/$slug/SKILL.md has no description"
  [[ "$fm_name" == "$slug" ]] && ok "skills/$slug name matches its directory" \
    || fail "skills/$slug/SKILL.md declares name '$fm_name', expected '$slug'"
done

echo "— marketplace"
MKT="$REPO_ROOT/.claude-plugin/marketplace.json"
if [[ -f "$MKT" ]]; then
  src=$(jq -r --arg n "$PLUGIN_NAME" '.plugins[] | select(.name==$n) | .source // empty' "$MKT" 2>/dev/null)
  if [[ -z "$src" ]]; then
    fail "marketplace.json has no entry for $PLUGIN_NAME"
  else
    ok "marketplace entry present (source: $src)"
    [[ -d "$REPO_ROOT/${src#./}" ]] && ok "marketplace source path resolves" \
      || fail "marketplace source path does not exist: $src"
  fi
  mkt_ver=$(jq -r --arg n "$PLUGIN_NAME" '.plugins[] | select(.name==$n) | .version // empty' "$MKT" 2>/dev/null)
  man_ver=$(jq -r '.version // empty' "$MANIFEST" 2>/dev/null)
  [[ "$mkt_ver" == "$man_ver" ]] && ok "marketplace version matches the manifest ($man_ver)" \
    || fail "version drift: marketplace $mkt_ver vs manifest $man_ver"
else
  fail "no marketplace.json at the repository root"
fi

echo
if [[ $FAILURES -eq 0 ]]; then
  echo "contract OK"
else
  echo "$FAILURES failure(s)"
fi
exit $(( FAILURES > 0 ? 1 : 0 ))
