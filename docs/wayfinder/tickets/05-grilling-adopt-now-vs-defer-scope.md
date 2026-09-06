# Adopt-now vs defer scope

- Label: `wayfinder:grilling` (HITL — resolve only through live exchange with the human)
- Status: closed · Assignee: opencode (claimed)
- Blocks: (none — terminal decision)
- Blocked by: Which v0.2.2 capabilities map to RGAA gaps? · Upgrade path and risk for v0.2.2

## Question

What is the final v0.2.2 adoption scope — the ordered, minimal slice to execute first — and what is explicitly deferred? This ticket closes the map: after it, nothing is left to decide before execution.

## Context

Needs both predecessor answers. Default bias per effort Notes: smallest upgrade that holds (binary + verification + one or two capability adoptions with direct RGAA payoff); everything else deferred with a named ceiling. Redraw the destination if the human wants execution carried into the map (per-map Notes override); otherwise produce the decision and hand off to planning/build.

## Answer (to record on resolution)

Post as resolution comment + map pointer: ordered adopt list (first slice vs later), deferred list with why, and the handoff (spec / plan / execution entry point). Close the map when this closes.

## Resolution (closed)

### Adopt now — first slice, in order

1. **Substrate + trust:** swap to default-render v0.2.2, version verify (`--version`/`serverInfo` in install.sh + CI + per-run log), `from_env` routing prerequisite, keep 0.2.0 tarball for rollback.
2. **Forms:** label-aware filling through existing pre-scan fill (`Element.labels`/`control`, `getByLabel`, `Input.insertText`, `FormData(form)`, label-activation) — the 11.x/1.x correctness fix.
3. **Navigation + frames:** MCP navigation drain + task-queue pumping, SPA same-document reporting, real frame trees with per-frame realms and scoped `postMessage`.
4. **Evidence + diagnosis:** painted input values, real control boxes, viewport containing block, font/emoji/SVG fallbacks; `exceptionDetails`, session-owned contexts, `getBoxModel` integer quads (with real-error branch update), pinned ICU locale, `unhandledrejection`, `<body onload>`.
5. **Secure defaults + docs:** private-network deny / file-access off with explicit opt-in flags, `--obey-robots` note, cookie-order/YAML/CLI docs corrected.

Riding along transparently (no work): permission grants, cookie canonicalization, binding validation, PBKDF2/DOM bounds, timer clamping, style-cascade/watchdog survival, UA/platform literal embedding.

### Deferred — and why

| Deferred | Why | Revisit when |
|---|---|---|
| Binary XHR/fetch bodies + response streaming | No RGAA gap — pipeline doesn't audit binary assets | Media/PDF download audits appear |
| Generated typed CDP clients (chromiumoxide/spider) | Hand-rolled transport stays; no rewrite in this effort | Transport rewrite proposed |
| CSPRNG blob-URL shape | Fingerprint-only, zero audit verdict impact | Blob-URL assertions needed |
| Screencast / PDF-export paths | Unused by the audit pipeline | Stakeholder-report rendering needs them |
| Stealth-variant rollout | No stealth-audit requirement; default render pinned | Evasion-resistant audits required |
| Structured `analyze()` migration | Convergence decided later, not executed here | Pipeline unification planned |
| aarch64 packaging matrix | x86_64-linux is the pinned path; gap only documented | Multi-arch CI required |

### Handoff

Execution entry: `docs/specs/obscura-v0.2.2-integration.md` (primary seam: orchestrator batch-run; secondary: bridge live tests + contract tests). Map closed with this ticket — nothing left to decide before execution.
