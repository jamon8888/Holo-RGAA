# Obscura substrate pin

Pinned browser substrate for Holo-RGAA audits: **default-render `obscura 0.2.2`**
(`obscura-x86_64-linux.tar.gz` from upstream releases; stealth/no-render variants
are out of the install path).

## Verify

```bash
./obscura --version          # want: obscura 0.2.2
```

Enforced in three places (keep the version string in sync):

- `install.sh` — `OBSCURA_VERSION` + `verify_obscura_version` (fails the install on mismatch)
- `.github/workflows/ci.yml` — e2e Prepare step greps the pin
- Rust — `ObscuraBridge::binary_version()` + per-run version log in `start_server`;
  `AnalyzePageResult.obscura_version` carries it on every analyze result

## Rollback (exercised 2026-09-06: 0.2.2 → 0.2.0 → 0.2.2, `--version` checked each step)

The previous pin is kept as `obscura-x86_64-linux-0.2.0.tar.gz`. From the repo root:

```bash
tar -xzf obscura-x86_64-linux-0.2.0.tar.gz
cp obscura-x86_64-linux-0.2.0.tar.gz obscura-x86_64-linux.tar.gz
./obscura --version          # want: obscura 0.2.0
```

Re-apply 0.2.2 by extracting the current `obscura-x86_64-linux.tar.gz` from a
checkout at or after the upgrade commit (or re-downloading
`obscura-x86_64-linux.tar.gz` from upstream `v0.2.2`), then re-check `--version`.

## Production ops

Decisions from the Scale #8 (#48, part of #42) research pass over
[docs.obscura.sh/guides/run-in-production-at-scale](https://docs.obscura.sh/guides/run-in-production-at-scale),
resolved on issue #35.

### `ObscuraBridge::start_server` env vars

| Variable | Default | What it does |
|---|---|---|
| `OBSCURA_WORKERS` | one per CPU (`std::thread::available_parallelism()`) | Passed as `serve --workers N`. Override to under/over-subscribe a shared host. |
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
