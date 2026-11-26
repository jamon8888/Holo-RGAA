# Obscura substrate pin

Pinned browser substrate for Holo-RGAA audits: **default-render `obscura 0.2.4`**
(`obscura-x86_64-linux.tar.gz` from upstream releases; stealth/no-render variants
are out of the install path).

## Verify

```bash
./obscura --version          # want: obscura 0.2.4
```

Enforced in four places (keep the version string in sync):

- `install.sh` / `rgaa-rs/install.ps1` — `OBSCURA_VERSION` / `$ObscuraVersion` +
  `verify_obscura_version` (fails the install on mismatch), **and** the pinned
  per-asset SHA-256 in `obscura_sha256_for` / `$ObscuraSha256`. The download is
  verified before extraction; an asset with no pinned digest is refused rather
  than installed. Regenerate the digests on every version bump:
  ```bash
  gh api repos/h4ckf0r0day/obscura/releases/tags/v0.2.4 \
    --jq '.assets[] | "\(.name)  \(.digest)"'
  ```
- `.github/workflows/ci.yml` and `.github/workflows/rgaa-audit.yml` — the Prepare
  Obscura step greps the pin; `release.yml` stamps it into the release manifest
- Rust — `OBSCURA_PINNED_VERSION` in `rgaa-obscura/src/lib.rs`, checked by
  `ObscuraBridge::start_server` (exact `name`/`version` token compare, not a
  prefix match) and logged per run; `AnalyzePageResult.obscura_version` carries
  it on every analyze result
- `rgaa-rs/crates/rgaa-obscura/Cargo.toml` — the `obscura` git dependency is
  pinned to the tag commit of the same release (`v0.2.4` =
  `1fccab2c1ecd0f7eadc16d7a520ba227f9e7f782`), so the library and the binary
  cannot drift apart

## Rollback

No tarball is vendored in the repo; roll back by re-downloading the previous
release and reverting the pins. From the repo root:

```bash
curl -fSL -o obscura.tar.gz \
  https://github.com/h4ckf0r0day/obscura/releases/download/v0.2.2/obscura-x86_64-linux.tar.gz
tar -xzf obscura.tar.gz
./obscura --version          # want: obscura 0.2.2
```

Then set the pin sites above back to `0.2.2` (and the Cargo `rev` to
`a1e09de68c7617b8079fbb1661b0548c501971c1`, the `v0.2.2` tag commit), or revert
the upgrade commit. Upgrade history: `0.2.0` → `0.2.2` (2026-09-06, rollback
exercised in both directions) → `0.2.4` (2026-10-12).

## Agent-browser study workflow

[`vercel-labs/agent-browser`](https://github.com/vercel-labs/agent-browser)
can drive the same Obscura binary for interactive page studies. Install its CLI
separately (`npm install -g agent-browser`), then launch a study with the
repository-pinned binary:

```bash
agent-browser --engine obscura --executable-path "$PWD/obscura" open https://example.com
agent-browser snapshot -i
agent-browser a11y
agent-browser screenshot
agent-browser close
```

Use this for exploratory browsing, snapshots, and manual follow-up on candidate
issues. The RGAA audit pipeline still uses `rgaa-obscura` and
`ObscuraBridge`: agent-browser's Obscura adapter is experimental and does not
provide the structured DOM, AX tree, and evidence contract consumed by the
analysis plugins. It also documents accessibility naming, hidden-element,
iframe, and screenshot fidelity gaps, so agent-browser observations are not
audit evidence and do not establish Chrome parity. Keep public URLs as the
default; local development pages require `OBSCURA_ALLOW_PRIVATE_NETWORK=1` in
the environment before starting the session.

### Repository-integrated study tool (dev only)

Two opt-in entry points drive the same pinned binary through a shared hint
contract. Both are compiled out of default builds:

```bash
# Hidden CLI subcommand, feature `dev-explore`
cargo run -p rgaa-cli --features dev-explore -- \
  explore https://example.com --executable-path ./obscura --json --keep-open
```

`rgaa-mcp --features dev-explore` adds the `explore` MCP tool (13 tools instead
of the usual 12). Either way the output is `kind: "exploration_hints"` with
`evidence: false`, `not_audit_evidence: true` and `audit_verdict: null`; there
is no `status`, `verdict` or `finding` field. Every candidate carries
`review_required: true` and must be reproduced through `ObscuraBridge` before
it can affect a result.

Pins are enforced before launch: `obscura --version` must be exactly
`obscura 0.2.4` (a prefix match would accept `0.2.40`) and agent-browser must be
`0.45.0`. The Obscura adapter rejects proxy-bypass settings, so `NO_PROXY` and
`AGENT_BROWSER_PROXY_BYPASS` are removed from the browser child process. Run
state, npm cache and sockets live in a per-run directory under `$TMPDIR`;
`--artifact-dir` persists screenshots when the run dir is cleaned up. Public
URLs are the default; localhost needs the explicit private-network opt-in.

### Parity study (2026-11-26)

The experimental adapter was compared with `ObscuraBridge` on 16 pages
(`example.com` plus fixtures for images, hidden elements, iframes, focus order,
naming, forms and language), both at 1920x1080:

| Measure | Result |
|---|---|
| Role-qualified name Jaccard (mean) | 0.614 |
| Role-agnostic, deduplicated name Jaccard | 1.000 (16/16 pages) |
| Expected-name coverage | 0.917 |
| Screenshot dimensions | 16/16 equal |
| Mean absolute RGB difference | 1.38 / 255 |

Every role-qualified difference was the same structural pattern: the bridge
emits a redundant `StaticText` child next to the named node, which
agent-browser omits. The go/no-go gate passed, so the MCP tool was added behind
its feature. This measures name and screenshot parity only — the adapter is
still experimental, the AX tree shapes differ, and none of it constitutes
audit evidence or Chrome parity.

## Production ops

Decisions from the Scale #8 (#48, part of #42) research pass over
[docs.obscura.sh/guides/run-in-production-at-scale](https://docs.obscura.sh/guides/run-in-production-at-scale),
resolved on issue #35.

### `ObscuraBridge::start_server` env vars

| Variable | Default | What it does |
|---|---|---|
| `OBSCURA_WORKERS` | one per CPU (`std::thread::available_parallelism()`) | Worker budget for the whole process, passed as `serve --workers N`. Each bridge starts its own server on its own ephemeral port; a server started while others run in the same process gets the remaining workers. Startup returns a capacity error if all workers are leased. Override to under/over-subscribe a shared host. |
| `OBSCURA_VERBOSE` | unset (silent) | `1` drops the always-on `--quiet` flag and stops nulling the child's stdout/stderr, so `RUST_LOG`/obscura's own logs are actually visible — previously `--quiet` + null stdio was unconditional, which defeated any `RUST_LOG` setting regardless of what an operator configured. |
| `OBSCURA_V8_FLAGS` | unset (obscura's own default heap) | Opt-in passthrough as `serve --v8-flags "<value>"`, e.g. `--max-old-space-size=4096`. Left unset by default rather than guessing a heap size for every deploy's memory budget. |

`scrape --concurrency` is already wired (`run_axe_batch`/`run_gap_fix_batch`/
`extract_page_context_batch`), capped at `MAX_CONCURRENCY = 32` in
`rgaa-obscura/src/config.rs`.

### Timeout ceilings

These are **ops config for the `obscura` process itself**, not something
`ObscuraBridge` sets — they're environment variables read by `obscura serve`
directly (see the systemd template below). The one thing that *is* bridge
code: `AnalyzeConfig::timeout_ms` (default `30_000`ms, the request-level
timeout `ObscuraBridge` enforces on its own analyze/fetch/scrape calls) must
stay at or below `OBSCURA_NAV_TIMEOUT_MS` — a bridge timeout longer than the
server's own navigation ceiling can never actually fire; the server gives up
first.

| Variable | Recommended | Relationship |
|---|---|---|
| `OBSCURA_NAV_TIMEOUT_MS` | `30000` | Bridge's `timeout_ms` default matches this exactly — keep them equal or bridge below. |
| `SCRIPT_DEADLINE_MS` | `30000` | Per-script execution budget inside a page. |
| `MODULE_BUDGET_MS` | `3000` | Per-module load budget — much shorter than NAV; a slow module shouldn't consume the whole navigation budget. |
| `CDP_COMMAND_TIMEOUT_MS` | `60000` | Kept *above* NAV — one CDP command can legitimately span more than one navigation-level operation. |
| `FETCH_TIMEOUT_MS` | `30000` | Matches NAV. |

### Resource caps and restart policy

Ship as deploy templates, not code: see
[`deploy/obscura.service`](../deploy/obscura.service) for a systemd unit with
`MemoryHigh`/`MemoryMax`/`LimitNOFILE`/`Restart=always` and the timeout
env vars above pre-filled with their recommended defaults.

### Bind posture (no change — keep as-is)

`obscura serve` binds `127.0.0.1` by default; `ObscuraBridge` never passes a
`--host`/`--bind` flag. Never expose it on `0.0.0.0` without a reverse proxy
that adds authentication in front — there is no auth on the CDP server
itself, by design (loopback-only is the security boundary). Reverse-proxy
WebSocket upgrade + long read timeouts and MCP-over-HTTP hardening are
explicitly deferred (#42) since nothing in this repo exposes either past
loopback today.
