# Holo-RGAA ↔ Obscura integration map

Resolves ticket "Research the current Holo-RGAA ↔ Obscura integration". Verified against code + binary execution, not README.

- Binary pin: repo-root `./obscura --version` → `obscura 0.2.0`. `./obscura-worker --version` prints `Headless Browser v0.2.0` banner + `CDP server: ws://127.0.0.1:9222/...`. No version assertion in Rust; no obscura crate dep (`tokio, tokio-tungstenite 0.24, reqwest 0.12, futures-util, base64, sha2, uuid`).
- Name collision confirmed: workspace `spider 2.53` (HTTP crawler, `spider::website::Website`) ≠ Obscura `spider` generated CDP client (not vendored).

| Component | File:symbol | Role | v0.2.2 touch? |
|---|---|---|---|
| Binary pin | `./obscura`, `rgaa-rs/obscura.log` | browser under test | yes — swap + re-verify `serve`, `/json/version`, `scrape`/`fetch --eval` |
| Launch | `rgaa-obscura/src/lib.rs:ObscuraBridge::start_server/stop_server/Drop/get_browser_ws_url` | spawn `serve`, poll `/json/version` 50×100ms, kill on drop | yes — worker/flag changes |
| Path resolution | `lib.rs:ObscuraBridge::new/with_binary_path/from_env` | PATH vs `RGAA_OBSCURA_BIN` | yes — `from_env` has zero call sites; orchestrator, MCP `main.rs:13`, CLI `igt.rs:48` use `new()` |
| CDP transport | `lib.rs:cdp_issue/cdp_wait_response/cdp_send/cdp_send_session/cleanup_target/wait_for_load` | hand-rolled CDP; per-call target+flatten; nanos-mod-1M ids; readyState/loadEvent wait | yes — strictest seam |
| Structured analyze | `lib.rs:ObscuraBridge::analyze/findings_from_axe/validate_axe_payload/classify_error` | single-page contract → `AnalyzePageResult` | yes — new DOM/CDP methods |
| Config | `config.rs:AnalyzeConfig/AnalyzeRequest::validate` | viewport, ≤20 pre-scan actions, timeouts, profile | yes — new knobs land here |
| Cookies | `lib.rs:apply_cookies` + `config.rs:CookieReference` | `Network.setCookie`; `None` → `RGAA_COOKIE_<NAME>` | yes — drift: README says before-navigate, code navigates first (`lib.rs:411-437`) |
| Pre-scan | `lib.rs:apply_pre_scan_actions` + `PreScanAction::Click/Fill/WaitFor` | `Runtime.evaluate` snippets; `WaitFor` to 30s | yes — `Input.insertText`/label upgrades |
| axe-core | `lib.rs:fetch_axe_source/run_axe_core*` + CDN 4.9.1 | fetch, device-metrics override, inject, `axe.run` | yes — version + selector rules |
| axe→RGAA | `rgaa-rules/src/axe_mapper.rs:AxeMapper::map` | violations → per-(node×criterion) Findings | no — unless rule-ids change |
| Gap-fix | `rgaa-rules/src/gap_fix.rs:GapFixRules` (10 keys) + `lib.rs:run_gap_fix*/build_gap_fix_script` | 10 snippets via CLI (`fetch --eval`, `scrape --format json`) | yes — CLI contract |
| Page context | `lib.rs:extract_page_context/build_page_context_script` | DOM census for Holo3 prompts + NA detection | yes — if DOM APIs replace census |
| Evidence | `lib.rs:capture_evidence` + `evidence.rs:EvidenceStore` + `results.rs` | `dom_snapshot` always + screenshot policy; SHA-256; 10-variant `ObscuraError` | yes — rendering fidelity |
| IGT keyboard | `lib.rs:run_igt_keyboard` + `results.rs:IgtResults` | ≤50 Tabs; trap after 5 repeats | yes — input semantics |
| IGT guided | `guided.rs:GuidedTest/.../ObscuraGuidedExecutor` + `run_guided_test` | AXTree `ax:<id>` refs; ≤3 retries | yes — AXTree/key changes |
| Dead knob | `config.rs:patch_attach_internals` (hardcoded false, never read) | ElementInternals override | yes — wire or delete |
| Orchestrator | `rgaa-orchestrator/src/pipeline.rs:Orchestrator::run/run_batch/audit_one` | uses legacy `run_axe`+`run_gap_fix`, not `bridge.analyze` | yes — legacy-vs-`analyze()` decision |
| Crawler | `rgaa-spider/src/spider_tool.rs:SpiderTool` | HTTP discovery, no CDP | no — keep distinct |
| Session/tools | `rgaa-browser-tools/src/session.rs` + `tools/*.rs` (9 Rig tools) | delegates to bridge | yes — CDP renames ripple |
| MCP/CLI | `rgaa-mcp/src/server.rs`, `main.rs`, `rgaa-cli/src/commands/{analyze,igt}.rs` | DTO mapping; both use `new()` | yes — `from_env` one-liner + new params |
| Install/config | `install.sh` (487 lines), `.rgaa/config.yaml` | ships binaries, MCP env, default YAML | yes — 4-variant matrix, version verify, aarch64 gap |
| Docs drift | `README.md:340-535` vs code | cookie order, YAML keys, CLI flags mismatch | docs fix with upgrade, no code |
