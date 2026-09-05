#!/bin/bash
set -e

REPO="jamon8888/Holo-RGAA"
INSTALL_DIR="${HOME}/.local/bin"
TMPDIR=$(mktemp -d)

cleanup() { rm -rf "$TMPDIR"; }
trap cleanup EXIT

detect_arch() {
    case "$(uname -m)" in
        x86_64)  echo "x86_64" ;;
        aarch64|arm64) echo "aarch64" ;;
        *)        echo "unsupported" ;;
    esac
}

detect_os() {
    case "$(uname -s)" in
        Linux)  echo "linux" ;;
        Darwin) echo "darwin" ;;
        *)      echo "unsupported" ;;
    esac
}

say() { echo "  $1"; }
step() { echo ""; echo "==> $1"; }

if [ "$(uname -s)" = "Darwin" ] && [ "$(uname -m)" = "arm64" ]; then
    PLATFORM="darwin-aarch64"
elif [ "$(uname -s)" = "Darwin" ]; then
    PLATFORM="darwin-x86_64"
elif [ "$(uname -s)" = "Linux" ] && [ "$(uname -m)" = "aarch64" ]; then
    PLATFORM="linux-aarch64"
else
    PLATFORM="linux-x86_64"
fi

ARCH=$(detect_arch)
OS=$(detect_os)

if [ "$ARCH" = "unsupported" ]; then
    echo "Error: unsupported architecture $(uname -m)" >&2
    exit 1
fi

LATEST=$(curl -s "https://api.github.com/repos/${REPO}/releases/latest" | grep '"tag_name"' | sed 's/.*"v\?\([^"]*\)".*/\1/')
say "Latest version: v${LATEST}"
say "Installing rgaa-cli v${LATEST} for ${PLATFORM}..."

ASSET="rgaa-cli-${LATEST}-${PLATFORM}.tar.gz"
URL="https://github.com/${REPO}/releases/download/v${LATEST}/${ASSET}"

step "Downloading ${ASSET}"
curl -fsSL "$URL" -o "$TMPDIR/${ASSET}"

step "Extracting"
tar -xzf "$TMPDIR/${ASSET}" -C "$TMPDIR"

step "Installing to ${INSTALL_DIR}"
mkdir -p "${INSTALL_DIR}"
cp "$TMPDIR/rgaa" "${INSTALL_DIR}/"
chmod +x "${INSTALL_DIR}/rgaa"

say "Installed! Add ${INSTALL_DIR} to your PATH if needed."
say "Run: rgaa tui"
