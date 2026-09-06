# Upgrade path and risk for v0.2.2

- Label: `wayfinder:task` (HITL checklist + AFK verification where possible)
- Status: closed · Assignee: opencode (claimed)
- Blocks: Adopt-now vs defer scope
- Blocked by: Research the Obscura v0.2.0 → v0.2.2 delta relevant to RGAA · Research the current Holo-RGAA ↔ Obscura integration

## Question

What is the minimum-risk upgrade path from the pinned 0.2.0 binary to v0.2.2, and what breaks if we just swap the binary? No code changes in this ticket — decide the path so execution needs no further decisions.

## Context

- Verify: 4-variant matrix (render × stealth) — which variant Holo-RGAA pins today vs should pin; version self-report (`--version` / MCP `serverInfo`) as CI/install.sh verification hook; platform matrix (x86_64-linux tarball in repo root vs aarch64/macos builds).
- Behavior changes to rule on: `DOM.setFileInputFiles` now requires `--allow-file-access` (v0.2.1 #579 — Puppeteer/Playwright uploads fail without it); SSRF deny-set expansion + `--allow-private-network` semantics for intranet audit targets; stealth transport now applies SSRF DNS guard (no longer trades protection away).
- Check `install.sh`, `docker-compose.yml`, `.rgaa/config.yaml`, `RGAA_OBSCURA_BIN` handling, and any version-string assertions in tests (`rgaa-obscura/tests/`, `rgaa-browser-tools/tests/`).
- Hand the human a precise checklist only for steps the agent cannot verify alone (e.g. private-network audit targets, file-upload audit flows).

## Answer (to record on resolution)

Post as resolution comment + map pointer: upgrade recipe (variant · verification command · config flag changes · rollback), risk table (change · blast radius · mitigation), and explicit non-goals. Link checklist results.

## Resolution (closed; verified against repo, no code changed)

### Upgrade recipe

1. **Variant:** default render `obscura-x86_64-linux` v0.2.2 (same variant as today's pin — matches evidence/screenshot needs; `-stealth`/`-no-render` stay unpinned).
2. **Swap:** replace repo-root `./obscura` + `./obscura-worker` with the v0.2.2 default-render assets (keep the 0.2.0 tarball for rollback).
3. **Verify:** `obscura --version` → `0.2.2` + MCP `serverInfo` parity; add the same check to `install.sh` (which today copies whatever is in repo-root/PATH with no version check and no variant handling) and CI, and log the version per audit run.
4. **Code prerequisite (one-liner):** route all entry points through `from_env` (or make `new()` honor `RGAA_OBSCURA_BIN`) — verified today every entry point (`rgaa-mcp/src/main.rs:13`, CLI `igt`, orchestrator, all bridge/browser-tools tests) uses `new()` (PATH lookup) while `from_env` has zero call sites, so the `RGAA_OBSCURA_BIN` env `install.sh` writes is dead.
5. **Flags:** `--allow-file-access` only where upload audits exist (default off, fail clearly); `--allow-private-network` only for intranet targets (default deny per expanded SSRF deny-set); note upstream now enforces `--obey-robots` for crawls. No such flags exist anywhere in the repo today (verified by grep).
6. **Docs with upgrade:** fix cookie-before-navigate claim (code navigates first), config keys, and CLI flags drift.
7. **Rollback:** swap back 0.2.0 binaries + re-run the version check.

### Risk table

| Change | Blast radius | Mitigation |
|---|---|---|
| Blind binary swap | Most paths transparently improve, but geometry handling breaks (`getBoxModel` placeholder quad → real protocol error) | Update error branch before/with swap; cover via orchestrator seam |
| Upload flows without flag | File inputs fail (upstream #579 behavior change) | Gate flag per audit config; clear denial error, never silent |
| Private/intranet targets | Newly denied by expanded SSRF set | Explicit opt-in flag only for listed intranet targets |
| Unverified version | Silent substrate drift (no assertions in tests today) | install.sh + CI version check + per-run log |
| Custom binary path ignored | `RGAA_OBSCURA_BIN` dead → custom installs broken | `from_env` routing prerequisite |
| Gap-fix CLI contract | `fetch --eval` / `scrape --format json` shape may shift | Re-verify both commands post-swap |
| Screenshot pixel diffs | Evidence re-baselining needed (painted inputs, control boxes, containing-block fix) | Human re-baseline approval (checklist) |

Non-goals: stealth/no-render variants, generated clients, binary bodies, screencast/PDF, structured-path migration.

### HITL checklist (human-only steps)

- [ ] Confirm intranet audit targets (if any) for `--allow-private-network` opt-in — default is deny.
- [ ] Confirm file-upload audit flows (if any) for `--allow-file-access` — default is off.
- [ ] Approve evidence screenshot re-baseline after swap (pixel diffs expected).
- [ ] Approve downloading v0.2.2 `obscura-x86_64-linux` default-render assets (or authorize automation to fetch).
