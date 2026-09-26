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

# Pinned browser substrate: default-render variant. Keep in sync with the
# version gate in .github/workflows/ci.yml (e2e Prepare step).
OBSCURA_VERSION="0.2.2"
OBSCURA_REPO="h4ckf0r0day/obscura"

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

# Fail the install when the installed obscura is not the pinned substrate.
verify_obscura_version() {
    local bin=""
    if [[ -x "${INSTALL_DIR}/obscura" ]]; then
        bin="${INSTALL_DIR}/obscura"
    elif command -v obscura &>/dev/null; then
        bin="obscura"
    else
        warn "obscura binary not found; skipping version check."
        return 0
    fi
    local version
    version="$("$bin" --version 2>/dev/null)" || die "Failed to query obscura version."
    if [[ "$version" != *"obscura ${OBSCURA_VERSION}"* ]]; then
        die "obscura version mismatch: got '${version}', want 'obscura ${OBSCURA_VERSION}'. Re-run install or update OBSCURA_VERSION."
    fi
    ok "obscura version: ${version}"
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

    if [[ -n "${RGAA_RELEASE_URL_BASE:-}" ]]; then
        echo "WARNING: RGAA_RELEASE_URL_BASE override active (test-only)" >&2
        echo "${RGAA_RELEASE_URL_BASE}/rgaa-rs-${tag}-${target}.tar.gz"
        return
    fi

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
    trap 'if [ -n "${tmpdir:-}" ]; then rm -rf "$tmpdir"; fi' EXIT

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

# ── Obscura browser substrate (upstream, per-platform) ──────────────────────
# Upstream ships default-render + stealth + no-render variants for
# linux/macOS (x86_64/aarch64) and Windows. We pin the default-render variant.
# Maps install.sh platform names to upstream asset names.
obscura_asset_for() {
    case "$1" in
        linux-x86_64)   echo "obscura-x86_64-linux.tar.gz" ;;
        linux-aarch64)  echo "obscura-aarch64-linux.tar.gz" ;;
        darwin-x86_64)  echo "obscura-x86_64-macos.tar.gz" ;;
        darwin-aarch64) echo "obscura-aarch64-macos.tar.gz" ;;
        *)              return 1 ;;
    esac
}

install_obscura() {
    local platform="$1"

    local asset
    if ! asset=$(obscura_asset_for "$platform"); then
        warn "obscura has no prebuilt binary for ${platform}; browser automation unavailable."
        return
    fi

    # Skip if pinned version already installed
    if [[ -x "${INSTALL_DIR}/obscura" ]] \
        && "${INSTALL_DIR}/obscura" --version 2>/dev/null | grep -q "obscura ${OBSCURA_VERSION}"; then
        ok "obscura ${OBSCURA_VERSION} already installed"
        return
    fi

    local url="https://github.com/${OBSCURA_REPO}/releases/download/v${OBSCURA_VERSION}/${asset}"
    local tmpdir
    tmpdir=$(mktemp -d)
    trap 'if [ -n "${tmpdir:-}" ]; then rm -rf "$tmpdir"; fi' EXIT

    info "Downloading obscura ${OBSCURA_VERSION}..."
    if ! curl -fSL --progress-bar -o "${tmpdir}/obscura.tar.gz" "$url"; then
        warn "obscura download failed (${url}); browser automation unavailable."
        return
    fi

    tar -xzf "${tmpdir}/obscura.tar.gz" -C "$INSTALL_DIR"
    chmod +x "${INSTALL_DIR}/obscura" 2>/dev/null || true
    chmod +x "${INSTALL_DIR}/obscura-worker" 2>/dev/null || true
    ok "obscura installed to ${INSTALL_DIR}"
}

# ── Build from source ─────────────────────────────────────────────────────────

build_from_source() {
    local repo_dir

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

    # Build only the 4 shipped bins (workspace also carries dev-only crates).
    info "Building rgaa-rs (this may take a few minutes)..."
    (cd "${repo_dir}/rgaa-rs" && cargo build --release -p rgaa-tui -p rgaa-cli -p rgaa-api -p rgaa-mcp)

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

    install_obscura "$(detect_platform)"
    # install_obscura only warns on failure (download error, or no prebuilt
    # asset for this platform), so verify_obscura_version's own missing-binary
    # case would also just warn and let this report success. The obscura
    # substrate is required for browser automation, and start_server now
    # rejects a missing/drifted binary at runtime, so treat it as required
    # here too instead of shipping a build that can't run.
    if [[ -x "${INSTALL_DIR}/obscura" ]]; then
        verify_obscura_version
    else
        die "obscura substrate could not be installed; browser automation would be unavailable.
     Re-run 'install.sh --build' once the download succeeds, or install obscura manually to ${INSTALL_DIR}/obscura."
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

    local fetched_root=""
    if [[ -z "$plugin_source" ]]; then
        info "Fetching plugin from GitHub (${RELEASE_TAG})..."
        local plugtmp
        plugtmp=$(mktemp -d)
        if ! curl -fSL -o "${plugtmp}/repo.tar.gz" \
            "https://codeload.github.com/${REPO}/tar.gz/${RELEASE_TAG}"; then
            warn "plugin download failed; continuing without plugin."
            return
        fi
        tar -xzf "${plugtmp}/repo.tar.gz" -C "$plugtmp"
        fetched_root="$(find "$plugtmp" -maxdepth 1 -mindepth 1 -type d -name "Holo-RGAA-*" | head -1)"
        if [[ -z "$fetched_root" ]] || [[ ! -d "${fetched_root}/claude-plugin" ]]; then
            warn "plugin not in tarball; continuing without plugin."
            return
        fi
        plugin_source="${fetched_root}/claude-plugin"
    fi

    # Copy (not symlink): a fetched tree lives in a tmpdir that gets removed.
    mkdir -p "$(dirname "$PLUGIN_DIR")"
    if [[ -n "$fetched_root" ]]; then
        cp -R "$plugin_source" "$PLUGIN_DIR"
        ok "Plugin installed: ${PLUGIN_DIR}"
        if [[ -d "${fetched_root}/rgaa-rs/plugins/rgaa-consultant" ]]; then
            local consultant_dir="${HOME}/.claude/plugins/rgaa-consultant"
            rm -rf "$consultant_dir"
            cp -R "${fetched_root}/rgaa-rs/plugins/rgaa-consultant" "$consultant_dir"
            ok "Plugin installed: ${consultant_dir}"
        fi
    else
        ln -sf "$plugin_source" "$PLUGIN_DIR"
        ok "Plugin symlinked: ${PLUGIN_DIR} -> ${plugin_source}"
    fi

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

# Portable timeout: python3 first (deterministic stdin forwarding, no bash
# backgrounding quirks), then GNU timeout / Homebrew gtimeout, then a bash
# fallback loop. Contract: child's exit code, or 124 when it had to be
# killed. python3 ships with macOS runners and is present on Linux runners.
probe_with_timeout() {
    local secs="$1"; shift
    if command -v python3 &>/dev/null; then
        PY_SECS="$secs" python3 -c '
import os, subprocess, sys
secs = int(os.environ["PY_SECS"])
data = sys.stdin.buffer.read()
p = subprocess.Popen(sys.argv[1:], stdin=subprocess.PIPE, stdout=subprocess.DEVNULL)
try:
    p.communicate(data, timeout=secs)
    sys.exit(p.returncode if p.returncode is not None else 1)
except subprocess.TimeoutExpired:
    p.kill()
    p.wait()
    sys.exit(124)
' "$@"
        return $?
    fi
    if command -v timeout &>/dev/null; then
        timeout "$secs" "$@"
        return $?
    fi
    if command -v gtimeout &>/dev/null; then
        gtimeout "$secs" "$@"
        return $?
    fi
    "$@" & local pid=$!
    local waited=0
    while kill -0 "$pid" 2>/dev/null && [ "$waited" -lt "$secs" ]; do
        sleep 1
        waited=$((waited + 1))
    done
    if kill -0 "$pid" 2>/dev/null; then
        kill "$pid" 2>/dev/null
        wait "$pid" 2>/dev/null
        return 124
    fi
    wait "$pid" 2>/dev/null
    return $?
}

verify_install() {
    info "Verifying installation..."
    local failures=0

    # Check binaries
    for bin in rgaa rgaa-mcp rgaa-cli rgaa-api; do
        if [[ -x "${INSTALL_DIR}/${bin}" ]] || command -v "$bin" &>/dev/null; then
            ok "  ${bin}: found"
        else
            err "  ${bin}: NOT FOUND"
            failures=$((failures + 1))
        fi
    done

    # Version/help gates: only rgaa and rgaa-cli implement --version today.
    for bin in rgaa rgaa-cli; do
        "${INSTALL_DIR}/${bin}" --version >/dev/null 2>&1 \
            || { err "  ${bin}: --version failed"; failures=$((failures + 1)); }
    done
    for bin in rgaa-api rgaa-mcp; do
        "${INSTALL_DIR}/${bin}" --help >/dev/null 2>&1 \
            || { err "  ${bin}: --help failed"; failures=$((failures + 1)); }
    done
    # MCP stdio probe: exit 124 means `timeout` killed an idle healthy server,
    # exit 0 means it answered and shut down cleanly on EOF. Both are healthy.
    # set +e around this: under `set -e` above, a non-zero (expected: 124)
    # from the pipeline would abort the script before we get to check it.
    local mcp_probe_status
    local mcp_probe_log
    mcp_probe_log="$(mktemp)"
    set +e
    # Complete minimal handshake: initialize + initialized notification,
    # then EOF. A clean shutdown (0) or an idle kill (124) both mean healthy.
    printf '%s\n' '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"install-verify","version":"0.0.0"}}}' '{"jsonrpc":"2.0","method":"notifications/initialized"}' \
        | probe_with_timeout 15 "${INSTALL_DIR}/rgaa-mcp" >/dev/null 2>"$mcp_probe_log"
    mcp_probe_status=$?
    set -e
    if [[ $mcp_probe_status -ne 124 && $mcp_probe_status -ne 0 ]]; then
        err "  rgaa-mcp: stdio start probe failed (status=$mcp_probe_status)"
        err "  server stderr (tail):"
        tail -n 5 "$mcp_probe_log" | sed 's/^/    /' || true
        failures=$((failures + 1));
    fi
    rm -f "$mcp_probe_log"

    # Check obscura
    if [[ -x "${INSTALL_DIR}/obscura" ]] || command -v obscura &>/dev/null; then
        ok "  obscura: found"
        verify_obscura_version
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
    echo "    3. Configure the LLM for AI-assisted evaluation"
    # Fetched from the default branch rather than ${RELEASE_TAG}: the installer
    # is normally run straight off a URL with no repository checkout, so there
    # is no local .env.example to copy.
    echo "       curl -fsSL https://raw.githubusercontent.com/${REPO}/master/.env.example -o .env"
    echo "       then set RGAA_LLM_PROVIDER / RGAA_LLM_MODEL / RGAA_LLM_API_KEY in .env"
    echo "       providers: holo3, openai, openrouter, groq, mistral, ollama, custom, ..."
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
        install_obscura "$platform"
    else
        build_from_source
    fi

    install_plugin
    create_default_config
    verify_install
}

main "$@"
