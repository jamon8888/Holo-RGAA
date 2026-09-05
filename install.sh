#!/usr/bin/env bash
# install.sh — One-command installer for rgaa-rs
#
# Usage:
#   curl -sSL https://raw.githubusercontent.com/jamon8888/Holo-RGAA/main/install.sh | bash
#   curl -sSL .../install.sh | bash -s -- --build    # build from source
#   curl -sSL .../install.sh | bash -s -- --卸载      # uninstall
#
# Installs: rgaa-mcp, rgaa-cli, obscura browser binary, Claude Code plugin

set -euo pipefail

# ── Configuration ──────────────────────────────────────────────────────────────

REPO="jamon8888/Holo-RGAA"
RELEASE_TAG="${RGAA_VERSION:-latest}"
INSTALL_DIR="${RGAA_INSTALL_DIR:-$HOME/.local/bin}"
PLUGIN_DIR="${HOME}/.claude/plugins/rgaa-audit"
CONFIG_DIR=".rgaa"
MCP_CONFIG="${HOME}/.claude/mcp.json"

# ── Colors ─────────────────────────────────────────────────────────────────────

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
BOLD='\033[1m'
NC='\033[0m'

info()  { echo -e "${BLUE}[rgaa]${NC} $*"; }
ok()    { echo -e "${GREEN}[rgaa]${NC} $*"; }
warn()  { echo -e "${YELLOW}[rgaa]${NC} $*"; }
err()   { echo -e "${RED}[rgaa]${NC} $*" >&2; }
die()   { err "$*"; exit 1; }

# ── Platform detection ─────────────────────────────────────────────────────────

detect_platform() {
    local os arch

    case "$(uname -s)" in
        Linux*)  os="linux" ;;
        Darwin*) os="darwin" ;;
        *)       die "Unsupported OS: $(uname -s). Use --build to compile from source." ;;
    esac

    case "$(uname -m)" in
        x86_64|amd64)   arch="x86_64" ;;
        arm64|aarch64)   arch="aarch64" ;;
        *)               die "Unsupported arch: $(uname -m). Use --build to compile from source." ;;
    esac

    echo "${os}-${arch}"
}

# ── Dependency checks ─────────────────────────────────────────────────────────

check_dep() {
    if ! command -v "$1" &>/dev/null; then
        return 1
    fi
    return 0
}

ensure_dep() {
    local cmd="$1"
    local install_hint="$2"

    if ! check_dep "$cmd"; then
        err "Missing required dependency: ${cmd}"
        err "  Install: ${install_hint}"
        exit 1
    fi
}

# ── GitHub release download ───────────────────────────────────────────────────

# Maps install.sh platform names (os-arch) to Rust target triples used in release assets.
# NOTE: linux-aarch64 has no prebuilt binary (openssl-sys can't cross-compile).
# ARM Linux users must build from source: install.sh --build
platform_to_target() {
    case "$1" in
        linux-x86_64)    echo "x86_64-unknown-linux-gnu" ;;
        linux-aarch64)   die "No prebuilt binary for linux-aarch64. Use: install.sh --build" ;;
        darwin-x86_64)   echo "x86_64-apple-darwin" ;;
        darwin-aarch64)  echo "aarch64-apple-darwin" ;;
        *)               die "Unsupported platform: $1" ;;
    esac
}

get_release_url() {
    local platform="$1"
    local tag="$2"
    local target
    target=$(platform_to_target "$platform")

    if [[ "$tag" == "latest" ]]; then
        # 'latest' is a prerelease tag, so address it explicitly (releases/latest skips prereleases)
        echo "https://github.com/${REPO}/releases/download/latest/rgaa-rs-latest-${target}.tar.gz"
    else
        echo "https://github.com/${REPO}/releases/download/${tag}/rgaa-rs-${tag}-${target}.tar.gz"
    fi
}

download_and_install() {
    local platform="$1"
    local tmpdir

    ensure_dep "curl" "brew install curl (macOS) or apt install curl (Linux)"

    tmpdir=$(mktemp -d)
    trap "rm -rf '$tmpdir'" EXIT

    local url
    url=$(get_release_url "$platform" "$RELEASE_TAG")

    info "Downloading rgaa for ${platform}..."
    info "  URL: ${url}"

    if ! curl -fSL --progress-bar -o "${tmpdir}/rgaa.tar.gz" "$url"; then
        die "Download failed. Check your network and try again.
     URL: ${url}
     If this is a fresh release, binaries may not be uploaded yet.
     Try: install.sh --build"
    fi

    info "Extracting..."
    mkdir -p "$INSTALL_DIR"
    tar -xzf "${tmpdir}/rgaa.tar.gz" -C "$INSTALL_DIR"

    # Make binaries executable
    chmod +x "${INSTALL_DIR}/rgaa" 2>/dev/null || true
    chmod +x "${INSTALL_DIR}/rgaa-mcp" 2>/dev/null || true
    chmod +x "${INSTALL_DIR}/rgaa-cli" 2>/dev/null || true
    chmod +x "${INSTALL_DIR}/rgaa-api" 2>/dev/null || true
    chmod +x "${INSTALL_DIR}/obscura" 2>/dev/null || true
    chmod +x "${INSTALL_DIR}/obscura-worker" 2>/dev/null || true

    ok "Binaries installed to ${INSTALL_DIR}"
}

# ── Build from source ─────────────────────────────────────────────────────────

build_from_source() {
    local repo_dir
    local build_mode="${1:-release}"

    ensure_dep "git" "brew install git (macOS) or apt install git (Linux)"

    # Install Rust if missing
    if ! check_dep "cargo"; then
        info "Installing Rust toolchain..."
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
        source "${HOME}/.cargo/env"
        ok "Rust installed: $(rustc --version)"
    fi

    # Clone or use existing repo
    if [[ -d "rgaa-rs" ]] && [[ -f "rgaa-rs/Cargo.toml" ]]; then
        repo_dir="."
        info "Building from local source..."
    else
        repo_dir=$(mktemp -d)
        info "Cloning repository..."
        git clone --depth 1 "https://github.com/${REPO}.git" "${repo_dir}/repo"
        repo_dir="${repo_dir}/repo"
    fi

    # Build
    info "Building rgaa-rs (this may take a few minutes)..."
    (cd "${repo_dir}/rgaa-rs" && cargo build --release --workspace)

    # Install binaries
    mkdir -p "$INSTALL_DIR"
    local binaries=("rgaa" "rgaa-mcp" "rgaa-cli" "rgaa-api")
    for bin in "${binaries[@]}"; do
        local src="${repo_dir}/rgaa-rs/target/release/${bin}"
        if [[ -f "$src" ]]; then
            cp "$src" "${INSTALL_DIR}/${bin}"
            chmod +x "${INSTALL_DIR}/${bin}"
            ok "Installed ${bin}"
        else
            warn "Binary not found: ${src}"
        fi
    done

    # Look for obscura binary in the repo or system
    if [[ -f "${repo_dir}/obscura" ]]; then
        cp "${repo_dir}/obscura" "${INSTALL_DIR}/obscura"
        chmod +x "${INSTALL_DIR}/obscura"
        ok "Installed obscura"
    elif check_dep "obscura"; then
        ok "obscura already in PATH"
    else
        warn "obscura binary not found. Browser automation will not work."
        warn "  Place obscura in ${INSTALL_DIR}/ or install separately."
    fi

    ok "Build complete. Binaries in ${INSTALL_DIR}"
}

# ── Claude Code plugin setup ──────────────────────────────────────────────────

install_plugin() {
    local script_dir
    script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

    info "Installing Claude Code plugin..."

    # Remove old plugin if exists
    if [[ -L "$PLUGIN_DIR" ]]; then
        rm "$PLUGIN_DIR"
    elif [[ -d "$PLUGIN_DIR" ]]; then
        warn "Removing existing plugin directory"
        rm -rf "$PLUGIN_DIR"
    fi

    # Find the claude-plugin directory
    local plugin_source=""
    if [[ -d "${script_dir}/claude-plugin" ]]; then
        plugin_source="${script_dir}/claude-plugin"
    elif [[ -d "claude-plugin" ]]; then
        plugin_source="$(pwd)/claude-plugin"
    elif [[ -d "${script_dir}/.claude-plugin" ]]; then
        plugin_source="${script_dir}"
    fi

    if [[ -z "$plugin_source" ]]; then
        warn "claude-plugin directory not found. Skipping plugin install."
        warn "  Manually copy claude-plugin/ to ${PLUGIN_DIR}"
        return
    fi

    # Symlink plugin
    mkdir -p "$(dirname "$PLUGIN_DIR")"
    ln -sf "$plugin_source" "$PLUGIN_DIR"
    ok "Plugin symlinked: ${PLUGIN_DIR} -> ${plugin_source}"

    # Configure MCP server in Claude Code global config
    configure_mcp
}

configure_mcp() {
    info "Configuring MCP server..."

    local mcp_dir
    mcp_dir="$(dirname "$MCP_CONFIG")"
    mkdir -p "$mcp_dir"

    # Build the MCP config JSON
    local mcp_json
    if [[ -f "$MCP_CONFIG" ]]; then
        # Merge into existing config
        if command -v jq &>/dev/null; then
            mcp_json=$(jq --arg bin "${INSTALL_DIR}/rgaa-mcp" \
                          --arg obscura "${INSTALL_DIR}/obscura" \
                          '. + {"mcpServers": (.mcpServers // {} | . + {"rgaa-mcp": {"command": $bin, "env": {"RGAA_OBSCURA_BIN": $obscura}}})}' \
                          "$MCP_CONFIG" 2>/dev/null) || true
        fi
    fi

    # Fallback: write fresh config
    if [[ -z "${mcp_json:-}" ]]; then
        mkdir -p "$mcp_dir"
        cat > "$MCP_CONFIG" <<MCP_EOF
{
  "mcpServers": {
    "rgaa-mcp": {
      "command": "${INSTALL_DIR}/rgaa-mcp",
      "env": {
        "RGAA_OBSCURA_BIN": "${INSTALL_DIR}/obscura"
      }
    }
  }
}
MCP_EOF
        ok "MCP config written to ${MCP_CONFIG}"
    else
        echo "$mcp_json" > "$MCP_CONFIG"
        ok "MCP config updated in ${MCP_CONFIG}"
    fi
}

# ── Default config ────────────────────────────────────────────────────────────

create_default_config() {
    if [[ -f "${CONFIG_DIR}/config.yaml" ]]; then
        ok "Config exists: ${CONFIG_DIR}/config.yaml"
        return
    fi

    info "Creating default config..."
    mkdir -p "$CONFIG_DIR"
    cat > "${CONFIG_DIR}/config.yaml" <<'CFG_EOF'
url_profiles:
  default:
    url: https://example.test
    viewport: desktop

viewport_profiles:
  desktop:
    width: 1000
    height: 1080
  mobile:
    width: 375
    height: 812

guided_tests: []

standards:
  - wcag
  - rgai

policy:
  min_compliance: 80.0
  required_criteria: []

evidence_dir: .rgaa/evidence
remote_endpoint: null
upload_consent: false
CFG_EOF
    ok "Default config created: ${CONFIG_DIR}/config.yaml"
}

# ── Verification ──────────────────────────────────────────────────────────────

verify_install() {
    info "Verifying installation..."
    local failures=0

    # Check binaries
    for bin in rgaa rgaa-mcp rgaa-cli rgaa-api; do
        if [[ -x "${INSTALL_DIR}/${bin}" ]] || command -v "$bin" &>/dev/null; then
            ok "  ${bin}: found"
        else
            err "  ${bin}: NOT FOUND"
            ((failures++))
        fi
    done

    # Check obscura
    if [[ -x "${INSTALL_DIR}/obscura" ]] || command -v obscura &>/dev/null; then
        ok "  obscura: found"
    else
        warn "  obscura: NOT FOUND (browser automation unavailable)"
    fi

    # Check PATH
    if echo "$PATH" | tr ':' '\n' | grep -qx "$INSTALL_DIR"; then
        ok "  PATH includes ${INSTALL_DIR}"
    else
        warn "  ${INSTALL_DIR} is NOT in your PATH"
        warn "  Add to your shell profile:"
        warn "    export PATH=\"${INSTALL_DIR}:\$PATH\""
    fi

    # Check plugin
    if [[ -L "$PLUGIN_DIR" ]] || [[ -d "$PLUGIN_DIR" ]]; then
        ok "  Claude Code plugin: installed"
    else
        warn "  Claude Code plugin: not installed"
    fi

    # Check MCP config
    if [[ -f "$MCP_CONFIG" ]]; then
        ok "  MCP config: exists"
    else
        warn "  MCP config: not created"
    fi

    if [[ $failures -gt 0 ]]; then
        err "Installation incomplete. ${failures} required component(s) missing."
        return 1
    fi

    echo ""
    echo -e "${GREEN}${BOLD}Installation complete!${NC}"
    echo ""
    echo "  Binaries:  ${INSTALL_DIR}/"
    echo "  Plugin:    ${PLUGIN_DIR}"
    echo "  MCP:       ${MCP_CONFIG}"
    echo "  Config:    ${CONFIG_DIR}/config.yaml"
    echo ""
    echo "  Next steps:"
    echo "    1. Ensure ${INSTALL_DIR} is in your PATH"
    echo "    2. Restart Claude Code to load the MCP server"
    echo "    3. Set HOLO3_API_KEY for AI-assisted evaluation"
    echo "       export HOLO3_API_KEY=\"your-key\""
    echo ""
    echo "  Quick test:"
    echo "    rgaa-cli analyze --url https://example.com"
    echo ""
}

# ── Uninstall ─────────────────────────────────────────────────────────────────

uninstall() {
    info "Uninstalling rgaa-rs..."

    rm -f "${INSTALL_DIR}/rgaa" && ok "Removed rgaa (TUI)"
    rm -f "${INSTALL_DIR}/rgaa-mcp" && ok "Removed rgaa-mcp"
    rm -f "${INSTALL_DIR}/rgaa-cli" && ok "Removed rgaa-cli"
    rm -f "${INSTALL_DIR}/rgaa-api" && ok "Removed rgaa-api"
    rm -f "${INSTALL_DIR}/obscura" && ok "Removed obscura"
    rm -f "${INSTALL_DIR}/obscura-worker" && ok "Removed obscura-worker"
    rm -f "$PLUGIN_DIR" && ok "Removed Claude Code plugin"

    # Remove MCP config entry
    if [[ -f "$MCP_CONFIG" ]] && command -v jq &>/dev/null; then
        jq 'del(.mcpServers["rgaa-mcp"])' "$MCP_CONFIG" > "${MCP_CONFIG}.tmp" \
            && mv "${MCP_CONFIG}.tmp" "$MCP_CONFIG"
        ok "Removed MCP server from ${MCP_CONFIG}"
    fi

    ok "Uninstall complete."
    echo "  Config files in ${CONFIG_DIR}/ were preserved."
    echo "  To remove: rm -rf ${CONFIG_DIR}"
}

# ── Help ──────────────────────────────────────────────────────────────────────

usage() {
    cat <<EOF
rgaa-rs installer

Usage:
  install.sh              Install pre-built binaries (fastest)
  install.sh --build      Build from source (requires Rust)
  install.sh --uninstall  Remove installed files
  install.sh --help       Show this help

Environment variables:
  RGAA_VERSION          Release tag to install (default: latest)
  RGAA_INSTALL_DIR      Install directory (default: ~/.local/bin)

Examples:
  curl -sSL https://raw.githubusercontent.com/jamon8888/Holo-RGAA/main/install.sh | bash
  curl -sSL .../install.sh | bash -s -- --build
  RGAA_VERSION=v0.1.0 curl -sSL .../install.sh | bash
EOF
}

# ── Main ──────────────────────────────────────────────────────────────────────

main() {
    local mode="download"

    case "${1:-}" in
        --build|-b)
            mode="build"
            ;;
        --uninstall|-u)
            uninstall
            exit 0
            ;;
        --help|-h)
            usage
            exit 0
            ;;
        "")
            mode="download"
            ;;
        *)
            die "Unknown option: $1. Use --help for usage."
            ;;
    esac

    echo -e "${BOLD}rgaa-rs installer${NC}"
    echo ""

    local platform
    platform=$(detect_platform)
    info "Platform: ${platform}"

    if [[ "$mode" == "download" ]]; then
        download_and_install "$platform"
    else
        build_from_source
    fi

    install_plugin
    create_default_config
    verify_install
}

main "$@"
