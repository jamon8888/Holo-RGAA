# Research the current Holo-RGAA ↔ Obscura integration

- Label: `wayfinder:research` (AFK)
- Status: closed · Assignee: opencode (claimed)
- Blocks: Which v0.2.2 capabilities map to RGAA gaps · Upgrade path and risk for v0.2.2 · Adopt-now vs defer scope
- Blocked by: (none — frontier)

## Question

How does Holo-RGAA integrate Obscura today, exactly? Map the seams a v0.2.2 upgrade would touch: binary launch/version pin, CDP client surface, axe-core + gap-fix injection, cookies/pre-scan actions, screenshots/evidence, IGT keyboard, orchestrator/spider/MCP/CLI call paths.

## Context

Start from (verify, don't assume):

- `rgaa-rs/crates/rgaa-obscura/src/` (`lib.rs` ~2144 lines hand-rolled CDP over tokio-tungstenite, `config.rs` AnalyzeConfig/PreScanAction/Cookies, `evidence.rs`, `guided.rs` IGT, `results.rs`), `Cargo.toml` (no obscura client dep — spawns binary).
- Repo-root `./obscura` + `./obscura-worker` binaries (`--version` → 0.2.0), `obscura-x86_64-linux.tar.gz`, `rgaa-rs/obscura.log`, `test_cdp.js`, `test_obscura.rs`.
- `rgaa-browser-tools` (AXTree, BrowserSession, 9 tools, MCP skeleton), `rgaa-orchestrator` pipeline, `rgaa-spider`, `rgaa-cli`/`rgaa-mcp` analyze paths, `install.sh`, `.rgaa/config.yaml` (`RGAA_OBSCURA_BIN`, viewport/timeout policy).
- README's Obscura/MCP/`analyze`/`waitFor`/cookie-before-navigate/IGT sections vs actual code (note drift).

## Answer (to record on resolution)

Post as resolution comment + map pointer: integration map (component · file:symbol · role · v0.2.2 touch-point yes/no). Note name collision: workspace `spider 2.53` crate vs Obscura's `spider` generated CDP client. Do not change code.

## Resolution (closed)

Hand-rolled CDP in `rgaa-obscura/src/lib.rs` (~2144 lines) spawning pinned 0.2.0 binary; strictest seams are `cdp_issue`/load-wait, `from_env` unused (zero call sites), orchestrator on legacy `run_axe`+`run_gap_fix` (not `bridge.analyze`), gap-fix via CLI `fetch --eval`/`scrape`, cookie-before-navigate README drift (code navigates first), dead `patch_attach_internals` knob. Full map: `../research/rgaa-obscura-integration.md`.
