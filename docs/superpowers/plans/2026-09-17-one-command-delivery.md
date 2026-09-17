# One-Command Delivery Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship the full stack (4 Rust bins, pinned obscura 0.2.2, MCP config, both plugins, default config) through one tested release workflow plus one command per OS.

**Architecture:** Collapse the two release workflows into one 4-job pipeline, harden both installers to parity with a test-only URL override, and give the dead `RGAA_OBSCURA_BIN` var a real meaning as a startup version gate in `from_env_async`.

**Tech Stack:** GitHub Actions, bash (`install.sh`), PowerShell (`install.ps1`), Rust (`rgaa-obscura`), `shellcheck`, `sha256sum`.

**Spec:** `docs/superpowers/specs/2026-09-17-one-command-delivery-design.md`

## Global Constraints

- Pinned substrate is obscura `0.2.2` — the exact string `obscura 0.2.2` in `--version` output, everywhere.
- Platform matrix: prebuilt for linux-x64, mac x64+arm64, win-x64; linux-arm64 builds from source only.
- Asset names stay as-is: `rgaa-rs-<tag>-<target>.tar.gz` / `rgaa-rs-latest-<target>.tar.gz` / `rgaa-rs-<tag>-x86_64-pc-windows-msvc.zip`.
- The 4 shipped bins are `rgaa`, `rgaa-cli`, `rgaa-api`, `rgaa-mcp` — nothing else enters artifacts.
- `RGAA_RELEASE_URL_BASE` is test-only: whenever set, installers print `WARNING: RGAA_RELEASE_URL_BASE override active (test-only)`.

---

### Task 1: Consolidate the release workflow

**Files:**
- Modify: `.github/workflows/release.yml`
- Delete: `.github/workflows/release-latest.yml`
- Delete: `rgaa-rs/cargo-dist.toml`
- Modify: `.github/workflows/ci.yml`

**Interfaces:**
- Consumes: existing `release.yml` build/package steps (4-target matrix, tar/zip commands).
- Produces: `SHA256SUMS` + `manifest.json` per artifact; `shellcheck` gate in `ci.yml`; single `release.yml` with jobs `build → smoke → installer-e2e → publish`.

- [ ] **Step 0: Unify triggers with a single `TAG` variable**

Replace the `on:` block of `release.yml` with:

```yaml
on:
  push:
    tags: ['v*', 'latest']
    branches: [master, main]
  workflow_dispatch:
    inputs:
      tag:
        description: 'Release tag (e.g. v0.1.0)'
        required: true
        type: string
```

Add a top-level `env: TAG: ${{ github.ref_type == 'tag' && github.ref_name || inputs.tag || 'latest' }}`, replace every `${{ github.ref_name }}` in the file with `${{ env.TAG }}`, change every `checkout` `ref:` to `${{ github.ref_type == 'tag' && github.ref_name || github.sha }}`, and rename artifacts to `rgaa-rs-${{ env.TAG }}-${{ matrix.target }}` (updating all `download-artifact` names, including the new installer-e2e job) so stable and `latest` runs never collide.

- [ ] **Step 1: Delete `release-latest.yml` and `cargo-dist.toml`**

Run: `git rm .github/workflows/release-latest.yml rgaa-rs/cargo-dist.toml`
Expected: both removed from the index.

- [ ] **Step 2: Add checksum + manifest to the Unix package step**

In `.github/workflows/release.yml`, extend `Package (Unix)`:

```bash
mkdir -p "$GITHUB_WORKSPACE/rgaa-rs/dist"
cd "$GITHUB_WORKSPACE/rgaa-rs/target/${{ matrix.target }}/release"
tar czf "$GITHUB_WORKSPACE/rgaa-rs/dist/${{ matrix.tarball }}" rgaa rgaa-cli rgaa-api rgaa-mcp
cd "$GITHUB_WORKSPACE/rgaa-rs/dist"
sha256sum "${{ matrix.tarball }}" > "${{ matrix.tarball }}.sha256"
base="$(basename "${{ matrix.tarball }}" .tar.gz)"
cat > "${base}.manifest.json" <<EOF
{"tag": "${{ env.TAG }}", "obscura_version": "0.2.2", "bins": ["rgaa", "rgaa-cli", "rgaa-api", "rgaa-mcp"]}
EOF
```

Mirror for the Windows step with `Get-FileHash -Algorithm SHA256` writing `<zip>.sha256`.

- [ ] **Step 3: Cover all 4 targets in smoke**

Add the missing matrix entry (`aarch64-apple-darwin`, `macos-latest`) to the `smoke` job and add a checksum verification step before extraction:

```bash
cd artifact && sha256sum -c *.sha256
```

- [ ] **Step 4: Convert `publish`/`release` job to stable-or-latest**

Gate the existing `softprops/action-gh-release` step: `prerelease: ${{ env.TAG == 'latest' }}`, `tag_name: ${{ env.TAG }}`, `name: ${{ env.TAG == 'latest' && format('Latest Build ({0})', github.sha) || env.TAG }}`. Keep `generate_release_notes: true` and `files: artifacts/*` (checksums + manifests ride along).

- [ ] **Step 5: Drop the crates.io publish job**

Delete the whole `publish:` job (matrix of 14 crates) from `release.yml`.

- [ ] **Step 6: Add shellcheck to `ci.yml`**

```yaml
  installer-lint:
    name: Installer lint
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: sudo apt-get update && sudo apt-get install -y shellcheck
      - run: shellcheck -S warning install.sh
      - run: bash -n install.sh
```

- [ ] **Step 7: Validate YAML parses and commit**

Run: `python3 -c "import yaml,sys; [yaml.safe_load(open(f)) for f in ['.github/workflows/release.yml','.github/workflows/ci.yml']]; print('YAML OK')"`
Expected: `YAML OK`.
Run: `git add -A && git commit -m "ci: consolidate release workflow, checksums, shellcheck"`
Expected: commit created.

---

### Task 2: Stub out the stale `rgaa-rs/install.sh`

**Files:**
- Modify: `rgaa-rs/install.sh` (replace 67-line body with a stub)

**Interfaces:**
- Consumes: repo-root `install.sh` (real installer).
- Produces: loud redirect — anyone running the stale path lands on the real installer.

- [ ] **Step 1: Replace with a stub**

Write `rgaa-rs/install.sh`:

```bash
#!/bin/bash
# DEPRECATED: this installer is stale (wrong asset names, binaries-only).
# It forwards to the real one-command installer at the repo root.
echo "WARNING: rgaa-rs/install.sh is deprecated; using ../../install.sh" >&2
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
exec bash "${SCRIPT_DIR}/../../install.sh" "$@"
```

- [ ] **Step 2: Verify the stub resolves and commit**

Run: `bash -n rgaa-rs/install.sh && bash rgaa-rs/install.sh --help | head -8`
Expected: exit 0, prints the real installer's help (`rgaa-rs installer` usage text).
Run: `git add rgaa-rs/install.sh && git commit -m "chore: stub stale rgaa-rs/install.sh toward root installer"`
Expected: commit created.

---

### Task 3: Harden root `install.sh`

**Files:**
- Modify: `install.sh`

**Interfaces:**
- Consumes: `RGAA_RELEASE_URL_BASE` (new, test-only), `RGAA_VERSION`, `RGAA_INSTALL_DIR`.
- Produces: full-stack install via pipe (`curl | bash`), 4-bin `--build`, `verify_install` gates.

- [ ] **Step 1: Add the test-only URL override with guard**

In `get_release_url`, prepend:

```bash
if [[ -n "${RGAA_RELEASE_URL_BASE:-}" ]]; then
    echo "WARNING: RGAA_RELEASE_URL_BASE override active (test-only)" >&2
    echo "${RGAA_RELEASE_URL_BASE}/rgaa-rs-${tag}-${target}.tar.gz"
    return
fi
```

- [ ] **Step 2: Remote plugin fetch in `install_plugin`**

When no local `plugin_source` is found, fetch instead of skipping:

```bash
if [[ -z "$plugin_source" ]]; then
    info "Fetching plugin from GitHub (${RELEASE_TAG})..."
    local plugtmp
    plugtmp=$(mktemp -d)
    curl -fSL -o "${plugtmp}/repo.tar.gz" \
      "https://codeload.github.com/${REPO}/tar.gz/${RELEASE_TAG}" \
      || { warn "plugin download failed; continuing without plugin."; return; }
    tar -xzf "${plugtmp}/repo.tar.gz" -C "$plugtmp"
    plugin_source="${plugtmp}/Holo-RGAA-${RELEASE_TAG}/claude-plugin"
    [[ -d "$plugin_source" ]] || { warn "plugin not in tarball; continuing without plugin."; return; }
fi
```

Copy (not symlink) the fetched tree to `$PLUGIN_DIR`. Also install `rgaa-rs/plugins/rgaa-consultant` from the same tarball when present.

- [ ] **Step 3: Narrow `build_from_source` to the 4 shipped bins + obscura download**

Replace `cargo build --release --workspace` with:

```bash
(cd "${repo_dir}/rgaa-rs" && cargo build --release -p rgaa-tui -p rgaa-cli -p rgaa-api -p rgaa-mcp)
```

Replace the repo-root obscura copy block with a call to `install_obscura "$(detect_platform)"`.

- [ ] **Step 4: Extend `verify_install` with version gates + MCP probe**

After the binary-exists loop, add (only `rgaa` and `rgaa-cli` implement
`--version` today — `rgaa-api`/`rgaa-mcp` are gated on `--help`, matching the
existing smoke job):

```bash
for bin in rgaa rgaa-cli; do
    "${INSTALL_DIR}/${bin}" --version >/dev/null 2>&1 \
        || { err "  ${bin}: --version failed"; ((failures++)); }
done
for bin in rgaa-api rgaa-mcp; do
    "${INSTALL_DIR}/${bin}" --help >/dev/null 2>&1 \
        || { err "  ${bin}: --help failed"; ((failures++)); }
done
# MCP stdio probe: exit 124 means `timeout` killed an idle healthy server.
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"install-verify","version":"0.0.0"}}}' \
    | timeout 15 "${INSTALL_DIR}/rgaa-mcp" >/dev/null 2>&1
if [ "$?" -ne 124 ]; then
    err "  rgaa-mcp: stdio start probe failed"
    ((failures++))
fi
```

- [ ] **Step 5: Lint and commit**

Run: `bash -n install.sh && shellcheck -S warning install.sh`
Expected: no output (clean).
Run: `git add install.sh && git commit -m "feat: full-stack pipe install, 4-bin build, installer verify gates"`
Expected: commit created.

---

### Task 4: Bring `install.ps1` to parity

**Files:**
- Modify: `rgaa-rs/install.ps1`

**Interfaces:**
- Consumes: same asset names as `release.yml`; `RGAA_RELEASE_URL_BASE` (test-only, same guard).
- Produces: Windows full-stack install matching Unix behavior.

- [ ] **Step 1: Add the test-only URL override**

After `$rgaaUrl` is computed:

```powershell
if ($env:RGAA_RELEASE_URL_BASE) {
    Write-Host "  WARNING: RGAA_RELEASE_URL_BASE override active (test-only)" -ForegroundColor Yellow
    $rgaaUrl = "$env:RGAA_RELEASE_URL_BASE/rgaa-rs-${Version}-x86_64-pc-windows-msvc.zip"
}
```

- [ ] **Step 2: Add plugin fetch + default config**

Fetch `https://codeload.github.com/${Repo}/tar.gz/${Version}`, extract `claude-plugin/` to `$env:USERPROFILE\.claude\plugins\rgaa-audit` (copy, warn-and-continue on failure). Write the same default `.rgaa/config.yaml` content as `install.sh:create_default_config` into the current directory when absent.

- [ ] **Step 3: Add 4-bin version gates to the Verifying step**

```powershell
foreach ($b in @("rgaa.exe","rgaa-cli.exe")) {
    & (Join-Path $InstallDir $b) --version
    if ($LASTEXITCODE -ne 0) { Write-Host "  ERROR: $b --version failed" -ForegroundColor Red; exit 1 }
}
foreach ($b in @("rgaa-api.exe","rgaa-mcp.exe")) {
    & (Join-Path $InstallDir $b) --help 2>&1 | Out-Null
    if ($LASTEXITCODE -ne 0) { Write-Host "  ERROR: $b --help failed" -ForegroundColor Red; exit 1 }
}
```

- [ ] **Step 4: Syntax-check and commit**

Run: `pwsh -NoProfile -Command "try { [void][System.Management.Automation.Language.Parser]::ParseFile('rgaa-rs/install.ps1',[ref]$null,[ref]$errs); if ($errs.Count -gt 0) { $errs | ForEach-Object { Write-Host $_.Message }; exit 1 } else { Write-Host 'PS SYNTAX OK' } } catch { Write-Host $_; exit 1 }"`
Expected: `PS SYNTAX OK`. (On Linux without `pwsh`, note the skip and rely on the Windows CI leg.)
Run: `git add rgaa-rs/install.ps1 && git commit -m "feat: install.ps1 parity (plugin, config, version gates, test override)"`
Expected: commit created.

---

### Task 5: Give `RGAA_OBSCURA_BIN` a real meaning + fix plugin env

**Files:**
- Modify: `rgaa-rs/crates/rgaa-obscura/src/lib.rs`
- Modify: `claude-plugin/.mcp.json`
- Modify: `rgaa-rs/plugins/rgaa-consultant/.mcp.json`
- Test: `rgaa-rs/crates/rgaa-obscura/src/lib.rs` (inline `mod tests`)

**Interfaces:**
- Consumes: `RGAA_OBSCURA_BIN` env var; `ObscuraError::ProcessStartup(String)`.
- Produces: `from_env_async()` that version-gates the pinned substrate and logs it; entry points (`orchestrator`, CLI `igt`, `rgaa-mcp`) already call it, so no caller changes. Plugin JSONs inherit the environment instead of a wrong hardcoded path.

- [ ] **Step 1: Write the failing test**

In `mod tests` of `rgaa-rs/crates/rgaa-obscura/src/lib.rs`, add:

```rust
#[tokio::test]
async fn from_env_rejects_missing_binary() {
    std::env::set_var("RGAA_OBSCURA_BIN", "/nonexistent/obscura-test-binary");
    let result = ObscuraBridge::from_env_async().await;
    std::env::remove_var("RGAA_OBSCURA_BIN");
    assert!(result.is_err(), "missing binary must fail, got ok");
}

#[tokio::test]
async fn from_env_rejects_version_drift() {
    std::env::set_var("RGAA_OBSCURA_BIN", "/bin/true");
    let result = ObscuraBridge::from_env_async().await;
    std::env::remove_var("RGAA_OBSCURA_BIN");
    assert!(result.is_err(), "wrong version must fail, got ok");
}
```

- [ ] **Step 2: Run to verify they fail**

Run: `cargo test -p rgaa-obscura from_env_`
Expected: FAIL — `from_env_async` currently ignores the env var and returns Ok.

- [ ] **Step 3: Implement the version gate**

Replace `from_env_async` (`lib.rs:47-50`) with:

```rust
/// Create a bridge, honoring `RGAA_OBSCURA_BIN` when set.
///
/// The native browser comes from the `obscura` Rust crate; the env var
/// names the pinned standalone substrate and is verified here so a
/// drifted or missing install fails fast with a clear error instead of
/// silently running against the wrong backend.
pub async fn from_env_async() -> Result<Self, ObscuraError> {
    if let Ok(bin) = std::env::var("RGAA_OBSCURA_BIN") {
        let out = std::process::Command::new(&bin)
            .arg("--version")
            .output()
            .map_err(|e| {
                ObscuraError::ProcessStartup(format!(
                    "RGAA_OBSCURA_BIN '{bin}' not executable: {e}"
                ))
            })?;
        let version = String::from_utf8_lossy(&out.stdout);
        if !version.contains("obscura 0.2.2") {
            return Err(ObscuraError::ProcessStartup(format!(
                "obscura version mismatch: got '{version}', want 'obscura 0.2.2'"
            )));
        }
        tracing::info!(version = version.trim(), "obscura substrate verified");
    }
    Self::new().await
}
```

No caller changes: `pipeline.rs:131`, `igt.rs:48`, `main.rs:14` already call `from_env_async`. The `tracing::info!` on bridge creation is the per-audit-run version log (a bridge is created per `run_batch`).

- [ ] **Step 4: Run to verify they pass**

Run: `cargo test -p rgaa-obscura from_env_`
Expected: PASS (2 tests). Then `cargo clippy -p rgaa-obscura --all-targets` clean.

- [ ] **Step 5: Fix the plugin `.mcp.json` env**

In both `claude-plugin/.mcp.json` and `rgaa-rs/plugins/rgaa-consultant/.mcp.json`, delete the `"RGAA_OBSCURA_BIN": "${CLAUDE_PLUGIN_ROOT}/bin/obscura"` line so the server inherits the environment (installer-written `~/.claude/mcp.json` or PATH) instead of a path that never exists.

- [ ] **Step 6: Commit**

Run: `git add rgaa-rs/crates/rgaa-obscura/src/lib.rs claude-plugin/.mcp.json rgaa-rs/plugins/rgaa-consultant/.mcp.json && git commit -m "feat: honor RGAA_OBSCURA_BIN as substrate version gate"`
Expected: commit created.

---

### Task 6: Add the installer-e2e job + checksum verification

**Files:**
- Modify: `.github/workflows/release.yml` (new `installer-e2e` job between `smoke` and `publish`)

**Interfaces:**
- Consumes: staged `build` artifacts, `RGAA_RELEASE_URL_BASE` seam from Tasks 3–4, local HTML fixture.
- Produces: pre-publish proof that the exact user commands work on all three OSs.

- [ ] **Step 1: Add the job**

```yaml
  installer-e2e:
    name: Installer E2E (${{ matrix.os }})
    needs: [build, smoke]
    strategy:
      fail-fast: false
      matrix:
        include:
          - os: ubuntu-latest
            target: x86_64-unknown-linux-gnu
          - os: macos-latest
            target: aarch64-apple-darwin
          - os: windows-latest
            target: x86_64-pc-windows-msvc
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - name: Download artifact
        uses: actions/download-artifact@v4
        with:
          name: rgaa-rs-${{ env.TAG }}-${{ matrix.target }}
          path: staged
      - name: Stage artifacts for installer (Unix)
        if: runner.os != 'Windows'
        run: |
          TAG="${TAG:-latest}"
          echo "RGAA_VERSION=$TAG" >> $GITHUB_ENV
          mkdir -p /tmp/stage
          cp staged/*.tar.gz "/tmp/stage/rgaa-rs-${TAG}-${{ matrix.target }}.tar.gz"
          ls /tmp/stage
          echo "RGAA_RELEASE_URL_BASE=file:///tmp/stage" >> $GITHUB_ENV
          echo "RGAA_INSTALL_DIR=/tmp/rgaa-e2e-bin" >> $GITHUB_ENV
          echo "HOME=/tmp/rgaa-e2e-home" >> $GITHUB_ENV
          mkdir -p /tmp/rgaa-e2e-home
      - name: Run install.sh (Unix)
        if: runner.os != 'Windows'
        run: bash install.sh
      - name: Wiring probe (Unix)
        if: runner.os != 'Windows'
        run: |
          export PATH="/tmp/rgaa-e2e-bin:$PATH"
          unset HOLO3_API_KEY
          out=$(rgaa-cli audit analyze --url https://example.com --format json 2>&1)
          rc=$?
          echo "$out" | grep -q "invalid agent configuration" || { echo "E2E FAIL: expected agent-config gate, got: $out"; exit 1; }
          [ "$rc" -ne 0 ] || { echo "E2E FAIL: expected non-zero exit without HOLO3_API_KEY"; exit 1; }
          echo "wiring probe OK (installer -> binary -> orchestrator -> bridge -> agent gate)"
      - name: Stage + install (Windows)
        if: runner.os == 'Windows'
        run: |
          $tag = $env:TAG
          if ([string]::IsNullOrEmpty($tag)) { $tag = "latest" }
          New-Item -ItemType Directory -Force -Path C:\stage | Out-Null
          Copy-Item "staged/*.zip" "C:\stage\rgaa-rs-${tag}-x86_64-pc-windows-msvc.zip" -Force
          $env:RGAA_RELEASE_URL_BASE = "file:///C:/stage"
          .\rgaa-rs\install.ps1 -Version $tag
        shell: pwsh
      - name: Wiring probe (Windows)
        if: runner.os == 'Windows'
        run: |
          $env:Path = "$env:LOCALAPPDATA\rgaa\bin;$env:Path"
          $env:HOLO3_API_KEY = $null
          $out = & rgaa-cli audit analyze --url https://example.com --format json 2>&1
          if (($LASTEXITCODE -eq 0) -or !($out -match "invalid agent configuration")) { Write-Host "E2E FAIL: $($out)"; exit 1 }
          Write-Host "wiring probe OK"
        shell: pwsh
```

- [ ] **Step 2: Validate YAML and commit**

Run: `python3 -c "import yaml; yaml.safe_load(open('.github/workflows/release.yml')); print('YAML OK')"`
Expected: `YAML OK`.
Run: `git add .github/workflows/release.yml && git commit -m "ci: installer-e2e job against staged artifacts"`
Expected: commit created.

---

## Out of scope (explicitly not in this plan)

linux-arm64 prebuilts, SBOM/attestation, auto-update, cargo-dist adoption, crates.io publishing, runbook command-shape fixes, licensing/SaaS.
