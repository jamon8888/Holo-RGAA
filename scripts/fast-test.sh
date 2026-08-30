#!/bin/bash
# Fast test runner for rgaa-rs workspace
# Uses sccache + parallel jobs + optimized test profile

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE="$SCRIPT_DIR/rgaa-rs"

# Ensure sccache is running
if ! pgrep -x sccache > /dev/null 2>&1; then
    echo "Starting sccache server..."
    export SCCACHE_IDLE_TIMEOUT=3600
    export SCCACHE_CACHE_SIZE="10G"
    sccache --start-server || true
fi

# Configure for fast test builds
export RUSTFLAGS="-C codegen-units=16 -C link-arg=-Wl,--compress-debug-sections=zlib"
export CARGO_INCREMENTAL=1
export CARGO_PROFILE_DEV_DEBUG=0  # No debug info for faster builds
export CARGO_PROFILE_TEST_DEBUG=0

# Number of parallel jobs (match CPU cores, max 8 for memory)
JOBS=$(nproc 2>/dev/null || sysctl -n hw.ncpu 2>/dev/null || echo 4)
[ "$JOBS" -gt 8 ] && JOBS=8

echo "=== Fast test run with $JOBS parallel jobs ==="

# Run tests with optimizations
cd "$WORKSPACE"
cargo test \
    --jobs "$JOBS" \
    --all \
    --quiet \
    -- --test-threads="$JOBS" \
    "$@"
