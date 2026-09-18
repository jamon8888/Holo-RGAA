#!/bin/bash
# DEPRECATED: this installer is stale (wrong asset names, binaries-only).
# It forwards to the real one-command installer at the repo root.
echo "WARNING: rgaa-rs/install.sh is deprecated; using ../install.sh" >&2
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
exec bash "${SCRIPT_DIR}/../install.sh" "$@"
