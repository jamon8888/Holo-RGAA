# Wayfinder Map — Obscura v0.2.2 integration for Holo-RGAA

> Label: `wayfinder:map` · Tracker: local-markdown (no repo issue tracker configured — see map footer)
> Source request: investigate https://github.com/h4ckf0r0day/obscura/releases#release-v0.2.2 — new features, integration, what it offers all `rgaa-*` crates.

## Destination

A decision-ready integration plan for Obscura **v0.2.2** across Holo-RGAA: what to upgrade from the pinned **v0.2.0** binary, which new CDP / DOM / rendering / security capabilities to adopt in `rgaa-obscura`, `rgaa-browser-tools`, `rgaa-orchestrator`/`rgaa-spider`, and what to explicitly defer. Done = route is clear, nothing left to decide before someone executes the upgrade — **decisions, not deliverables** (no binary swap or code change in this map).

## Notes

- Domain: RGAA 4.1.2 audit pipeline (axe-core + gap-fix + Holo3 LLM + IGT keyboard + evidence). Obscura is the CDP browser substrate.
- Current pin: repo-root `./obscura` reports **0.2.0**; `rgaa-obscura` is a hand-rolled CDP client (tokio-tungstenite) spawning the binary; `rgaa-browser-tools` wraps sessions/tools; `rgaa-spider` uses `spider 2.53` (not the Obscura `spider` CDP client — name collision, keep distinct).
- Skills every session should consult: `grilling` + `domain-modeling` by default; `research` for doc/local-source fact-finding; `prototype` only if a ticket asks for a cheap artifact.
- Key upstream source: Obscura releases page (v0.2.2: 126 commits since v0.2.1, 5 Sep; v0.2.1: 122 commits since v0.2.0, 23 Aug). v0.2.0 added native rendering/screenshots/PDF.
- Ponytail lens: smallest upgrade that holds; deletion over addition; mark deliberate ceilings with `ponytail:` comments.

## Decisions so far

<!-- one line per closed ticket: gist + link. Empty — map just charted. -->

- [Research the Obscura v0.2.0 → v0.2.2 delta relevant to RGAA](tickets/01-research-obscura-delta-v020-v022.md): adopt-now slice is frames/MCP-drain/label-fill/SPA-nav/exceptionDetails/session-contexts/boxModel/input-painting/SSRF-config + two flag changes; defer binary bodies/generated clients; full table in `research/obscura-delta-v020-v022.md`.
- [Research the current Holo-RGAA ↔ Obscura integration](tickets/02-research-current-rgaa-obscura-integration.md): hand-rolled CDP + pinned 0.2.0, `from_env` unused, orchestrator on legacy path, cookie-order drift + dead knob noted; full map in `research/rgaa-obscura-integration.md`.
- [Which v0.2.2 capabilities map to RGAA gaps?](tickets/03-grilling-which-v022-capabilities-map-to-rgaa-gaps.md): adopt forms + navigation/frames + evidence/diagnosis clusters, defer binary bodies/generated clients/CSPRNG/screencast-PDF/stealth; carries into upgrade-path + final scope.
- [Upgrade path and risk for v0.2.2](tickets/04-task-upgrade-path-and-risk-v022.md): pin default-render v0.2.2 + version verify (install.sh/CI/per-run) + `from_env` routing prerequisite + two flags (file-access/private-network, defaults off/deny); blind-swap risks (boxModel errors, uploads, private targets, drift) mitigated; checklist left for human.
- [Adopt-now vs defer scope](tickets/05-grilling-adopt-now-vs-defer-scope.md): first slice = substrate+trust → forms → navigation/frames → evidence/diagnosis → secure-defaults+docs; deferred with reasons = binary bodies, generated clients, CSPRNG, screencast/PDF, stealth, analyze() migration, aarch64. Handoff: `docs/specs/obscura-v0.2.2-integration.md`.

## Map status: CLOSED — route is clear, nothing left to decide before execution.

## Not yet specified

<!-- in-scope fog: suspected questions too coarse to ticket yet; graduates as frontier advances -->

- Evidence/screenshot fidelity: does v0.2.2's input-value painting, textarea control box, SVG/emoji rendering change what RGAA evidence capture (SHA-256 screenshots, AXTree snapshots) should assert?
- Pre-scan action reliability: do `Input.insertText`, labelable-element/`getByLabel`, `grantPermissions`, `FormData(form)` and MCP navigation-drain fix known flaky form/cookie-consent flows in `AnalyzeConfig::pre_scan_actions`?
- Robustness budget: do the v0.2.2 worker-survival fixes (iterative style cascade, timer clamping, fetched_urls/body caps, watchdog cancel-on-drop, bad-script isolation) remove current timeouts/deep-page crashes, or just move the ceiling?
- CDP strictness: does adopting generated-client fixes (`Page.enable` load sequence, schema-complete frame, `DOM.getBoxModel` integer quads, `Runtime.evaluate` exceptionDetails, session-owned execution contexts) require changes to `rgaa-obscura`'s CDP deserialization?
- Security posture: SSRF deny-set expansion (IPv6 embedded-IPv4/6to4/NAT64, CGNAT, IANA ranges + stealth DNS guard), platform/UA JSON-literal embedding, `createObjectURL` CSPRNG blob URLs, PBKDF2 caps — what must the audit pipeline (which fetches attacker-influenced pages) enforce, incl. `--allow-private-network` / `--allow-file-access` behavior changes?
- Packaging: 4-variant matrix (render × stealth), self-reported version (`0.1.0-dev+` local vs tag at release), MCP `serverInfo` parity — which variant does Holo-RGAA pin and how is it verified in CI/install.sh?

## Out of scope

<!-- ruled beyond this effort's destination; closed, never graduates -->

(none yet)

---

*Local-markdown tracker: this file is the map. Child tickets live in `tickets/` next to it. Blocking is recorded in each ticket's `Blocks` / `Blocked by` frontmatter (native dependency UI N/A locally). Frontier = open + unblocked + unclaimed tickets. Claim = set `Assignee` first, before any work.*
