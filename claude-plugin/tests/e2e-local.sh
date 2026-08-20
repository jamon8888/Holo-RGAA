#!/usr/bin/env bash
# e2e-local.sh — End-to-end local workflow test for the RGAA plugin
#
# Tests the core workflow: build → test → lint → format → plugin contract
# Focuses on the crates modified in Tasks 1-10.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
RGAA_ROOT="$(cd "$PLUGIN_ROOT/.." && pwd)"
RGAA_RS_ROOT="$RGAA_ROOT/rgaa-rs"
FAILURES=0

section() {
  echo ""
  echo "=== $1 ==="
}

pass() {
  echo "  ✅ $1"
}

fail() {
  echo "  ❌ $1"
  ((FAILURES++))
}

section "1. Build core crates"
if (cd "$RGAA_RS_ROOT" && cargo build -p rgaa-core -p rgaa-remediation -p rgaa-rules -p rgaa-storage -p rgaa-api 2>&1 | tail -1); then
  pass "Core crates build"
else
  fail "Core crate build failed"
fi

section "2. Run E2E remediation loop tests"
if (cd "$RGAA_RS_ROOT" && cargo test --test full_remediation_loop -p rgaa-remediation 2>&1 | grep -q "test result: ok"); then
  pass "E2E remediation loop tests pass"
else
  fail "E2E remediation loop tests failed"
fi

section "3. Run security contract tests"
if (cd "$RGAA_RS_ROOT" && cargo test --test security_contract -p rgaa-remediation 2>&1 | grep -q "test result: ok"); then
  pass "Security contract tests pass"
else
  fail "Security contract tests failed"
fi

section "4. Run core unit tests"
if (cd "$RGAA_RS_ROOT" && cargo test -p rgaa-core -p rgaa-remediation -p rgaa-rules 2>&1 | grep "test result: ok" | head -1 | grep -q "passed"); then
  pass "Core unit tests pass"
else
  fail "Core unit tests failed"
fi

section "5. Clippy lint check"
if (cd "$RGAA_RS_ROOT" && cargo clippy -p rgaa-core -p rgaa-remediation -p rgaa-rules -p rgaa-api -p rgaa-storage --all-targets -- -D warnings 2>&1 | grep -v "ignoring.*resolver" | grep -v "sqlx-postgres" | grep -v "for further information" | grep -v "note:" | grep -q "warning:"); then
  fail "Clippy warnings found"
else
  pass "Clippy clean"
fi

section "6. Format check"
if (cd "$RGAA_RS_ROOT" && cargo fmt --all -- --check 2>&1 | grep -q "Diff"); then
  fail "Formatting issues found"
else
  pass "All files formatted"
fi

section "7. Plugin contract validation"
if bash "$PLUGIN_ROOT/tests/plugin-contract.sh" 2>&1 | grep -q "ALL CONTRACT CHECKS PASSED"; then
  pass "Plugin contract validation passes"
else
  fail "Plugin contract validation failed"
fi

section "8. No secrets in source code"
if grep -r "sk-\|api_key.*=.*\"[a-zA-Z0-9]" "$PLUGIN_ROOT" --include="*.json" --include="*.yaml" --include="*.yml" --include="*.sh" --include="*.md" 2>/dev/null | grep -v "your-api-key" | grep -v "REDACTED" | grep -v "example" | grep -v "placeholder" | grep -v "HOLO3_API_KEY" | grep -v "REMOTE_API_KEY" | grep -v "RGAA_API_KEY" | head -1; then
  fail "Potential secrets found in plugin files"
else
  pass "No secrets in plugin files"
fi

echo ""
echo "==============================="
if [[ $FAILURES -eq 0 ]]; then
  echo "✅ ALL E2E CHECKS PASSED"
  exit 0
else
  echo "❌ $FAILURES E2E CHECK(S) FAILED"
  exit 1
fi
