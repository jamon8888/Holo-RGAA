#!/bin/bash
# Setup sccache for faster Rust builds
# Add to ~/.bashrc or ~/.zshrc to persist

export SCCACHE_IDLE_TIMEOUT=3600
export SCCACHE_CACHE_SIZE="10G"
export RUSTC_WRAPPER=sccache
export CARGO_PROFILE_DEV_DEBUG=0
export CARGO_PROFILE_TEST_DEBUG=0

# Show sccache stats
echo "SCCache stats:"
sccache --show-stats 2>/dev/null || echo "SCCache not running - run 'sccache --start-server' first"
