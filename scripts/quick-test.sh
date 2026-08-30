#!/bin/bash
# Quick test runner - tests only the changed crates
# Fast feedback loop before running full integration tests

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE="$(dirname "$SCRIPT_DIR")/rgaa-rs"

cd "$WORKSPACE"

# Test only core crates changed in M3 plan
echo "=== Quick test: rgaa-core + rgaa-orchestrator ==="
cargo test -p rgaa-core -p rgaa-orchestrator --quiet

echo ""
echo "=== Quick test: rgaa-rules ==="
cargo test -p rgaa-rules --quiet

echo ""
echo "All quick tests passed!"
