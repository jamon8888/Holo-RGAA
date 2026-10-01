#!/usr/bin/env bash
#
# Release-time version parity check (issue #170).
#
# WHY THIS EXISTS
# ---------------
# rgaa-cli and rgaa-mcp-http are separate crates with separate `version =`
# fields in their own Cargo.toml. Nothing forces them to move together, so a
# release can ship a CLI at 0.2.0 next to an MCP server still reporting 0.1.0.
# Clients that negotiate on the server's advertised version then believe they
# are talking to an older build than the one they installed. Cheap to catch
# here, confusing to debug in the field.
#
# The check runs against the BINARIES, not the Cargo.toml files, because what
# ships is what the binary prints — a stale build in the release assets is
# exactly one of the failures this is meant to catch.
#
# The tag comparison is a WARNING, not a failure, on purpose: this repo tags
# releases (v0.2.x, and a rolling `latest`) independently of the crate
# versions, so making it fatal would red every release run without telling
# anyone anything they did not already know. Aligning tags with crate
# versions is a separate decision, not something to smuggle in here.
#
# Usage:
#   scripts/check-version-parity.sh --cli <path> --server <path> [--tag vX.Y.Z]

set -uo pipefail

CLI=""
SERVER=""
TAG=""

while [ $# -gt 0 ]; do
    case "$1" in
        --cli) CLI="$2"; shift 2 ;;
        --server) SERVER="$2"; shift 2 ;;
        --tag) TAG="$2"; shift 2 ;;
        -h|--help) sed -n '2,30p' "$0"; exit 0 ;;
        *) echo "unknown argument: $1" >&2; exit 2 ;;
    esac
done

[ -n "$CLI" ] && [ -n "$SERVER" ] || { echo "FATAL: --cli and --server are required" >&2; exit 2; }
[ -x "$CLI" ] || { echo "FATAL: $CLI is not executable" >&2; exit 2; }
[ -x "$SERVER" ] || { echo "FATAL: $SERVER is not executable" >&2; exit 2; }

# Both binaries print "<name> <semver>"; take the first semver-shaped token so
# a future clap-generated banner with extra words still parses.
extract() {
    "$1" --version 2>&1 | tr ' ' '\n' | grep -Eo '^[0-9]+\.[0-9]+\.[0-9]+([-+][0-9A-Za-z.-]+)?$' | head -1
}

cli_version="$(extract "$CLI")"
server_version="$(extract "$SERVER")"

echo "rgaa-cli       : ${cli_version:-<unparseable>}  ($("$CLI" --version 2>&1 | head -1))"
echo "rgaa-mcp-http  : ${server_version:-<unparseable>}  ($("$SERVER" --version 2>&1 | head -1))"

if [ -z "$cli_version" ] || [ -z "$server_version" ]; then
    echo "FAIL: could not parse a version out of one of the binaries" >&2
    exit 1
fi

if [ "$cli_version" != "$server_version" ]; then
    echo "FAIL: MCP server version ${server_version} != CLI version ${cli_version}." >&2
    echo "      Bump both crates together in rgaa-rs/crates/*/Cargo.toml." >&2
    exit 1
fi

if [ -n "$TAG" ]; then
    tag_version="${TAG#v}"
    if [ "$tag_version" != "$cli_version" ] && [ "$TAG" != "latest" ]; then
        echo "::warning::release tag ${TAG} does not match crate version ${cli_version} (not fatal; tags and crate versions are managed separately in this repo)"
    fi
fi

echo "OK: server and CLI both report ${cli_version}"
