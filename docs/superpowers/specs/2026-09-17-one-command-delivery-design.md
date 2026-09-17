# One-Command Delivery (CI + Installers) — Design Spec

**Date:** 2026-09-17
**Status:** Draft, pending user review
**Approach:** Deduplicate + harden (Approach A)
**Scope:** full stack (rgaa TUI + rgaa-cli + rgaa-api + rgaa-mcp, pinned obscura 0.2.2, MCP config, both Claude plugins, default config) on the current matrix (linux-x64, mac x64+arm64, win-x64 prebuilt; linux-arm64 source-only). Complements `2026-08-24-rgaa-distribution-design.md` (product/licensing shape); this spec covers only the delivery machinery.

## Decisions (locked in Q&A)

1. Full-stack scope, not binaries-only.
2. Keep the current platform matrix; no linux-arm64 prebuilt.
3. Collapse `release.yml` + `release-latest.yml` into one workflow; delete unused `cargo-dist.toml`.
4. Pipe-install (`curl | bash`) fetches the plugin remotely so it still delivers the full stack.
5. Drop the `crates.io` publish job until versioning + publish ordering are designed.

## 1. Architecture — one workflow, four jobs

Single `.github/workflows/release.yml`, triggered by tag push (`v*`, `latest`) and `workflow_dispatch(tag)`:

```
build (4-target matrix, unchanged tar/zip steps)
  → smoke (ALL 4 artifacts, incl. aarch64-apple-darwin — currently untested)
  → installer-e2e (new: ubuntu + macos run install.sh, windows runs install.ps1,
                   against staged artifacts via RGAA_RELEASE_URL_BASE override)
  → publish (stable GitHub release, or prerelease when tag == latest)
```

`ci.yml` keeps fmt/clippy/audit and gains a `shellcheck` step on `install.sh`.

Deletions: `release-latest.yml`, `cargo-dist.toml`, the `crates.io` publish job, and `rgaa-rs/install.sh` (replaced by a stub that resolves the repo-root `install.sh` relative to its own location and execs it with a deprecation warning — old bookmarks fail loud instead of fetching the wrong asset name).

## 2. Components

**`install.sh` (root, the real one):**
- Pipe-install plugin fetch: download `codeload.github.com/jamon8888/Holo-RGAA/tar.gz/<tag>`, extract `claude-plugin/` and `rgaa-rs/plugins/rgaa-consultant/` into the plugin dir. Plugin fetch failure warns and continues (offline installs must still work).
- `build_from_source` builds only the 4 shipped bins (`-p rgaa-tui -p rgaa-cli -p rgaa-api -p rgaa-mcp`), then downloads obscura per-platform via `install_obscura` instead of copying repo-root leftovers.
- `verify_install` adds `--version` gates on all 4 rgaa bins plus a `rgaa-mcp` stdio start probe.
- New test-only seam: `RGAA_RELEASE_URL_BASE` overrides the GitHub releases base URL in `get_release_url`. Emits a `WARNING: test override active` guard line whenever set; production path unchanged.

**`install.ps1`:** same plugin fetch (`Invoke-WebRequest` + `Expand-Archive`), same 4-bin `--version` gates, plus default `.rgaa/config.yaml` creation for Unix parity. Same `RGAA_RELEASE_URL_BASE` override with guard warning.

**Code (smallest part):** route the three hot entry points (`orchestrator`, CLI `igt`, `rgaa-mcp/main.rs`) through `ObscuraBridge::from_env_async` so the `RGAA_OBSCURA_BIN` the installer writes is actually honored; log `obscura --version` per audit run. Plugin `.mcp.json` files stop hardcoding `${CLAUDE_PLUGIN_ROOT}/bin/obscura` and inherit `RGAA_OBSCURA_BIN` from the environment.

**Artifacts:** each target additionally publishes `SHA256SUMS` and `manifest.json` (`{tag, obscura_version: "0.2.2", bins: ["rgaa","rgaa-cli","rgaa-api","rgaa-mcp"]}`). Smoke verifies checksum + `--version` on all 4 bins.

## 3. Data flow

Tag push → `build` → `smoke` (checksum + version gates) → `installer-e2e` (temp `RGAA_INSTALL_DIR`/`HOME`, `RGAA_RELEASE_URL_BASE=file://<staged artifacts>`; asserts bins, obscura version gate, MCP merge, plugin fetch, default config; then `rgaa-cli analyze --url file://<local fixture>` as a no-network functional probe) → `publish`.

## 4. Error handling

Fail closed: obscura version mismatch → `die`; any rgaa bin `--version` mismatch → fail; installer-e2e failure blocks publish. Warn-and-continue only for plugin fetch (offline) and missing `jq` (fresh-write MCP fallback already exists). No external network in E2E besides GitHub artifact download; HOLO3-gated tests stay skipped.

## 5. Testing

- `shellcheck install.sh` on every PR.
- `installer-e2e` matrix (ubuntu/macos/windows) runs on every PR against PR-built staged artifacts — pre-publish signal, no seam in the user path.
- Existing smoke gates extended to all 4 targets.

## 6. Out of scope

linux-arm64 prebuilts, SBOM/attestation, auto-update, `cargo-dist` adoption, crates.io publishing, licensing/SaaS (see distribution spec).

## 7. Acceptance criteria

- [ ] `curl .../install.sh | bash` on a clean linux-x64/mac/win-x64 machine yields working `rgaa`, `rgaa-cli`, `rgaa-api`, `rgaa-mcp`, pinned obscura 0.2.2, MCP entry, both plugins, default config.
- [ ] Same via `install.ps1` on Windows.
- [ ] No duplicated release workflow; mac-arm64 artifact smoke-tested; checksums + manifest published.
- [ ] `RGAA_OBSCURA_BIN` written by the installer is honored at runtime (entry points via `from_env`).
- [ ] `rgaa-rs/install.sh` stub redirects; `cargo-dist.toml` and crates.io job gone.
