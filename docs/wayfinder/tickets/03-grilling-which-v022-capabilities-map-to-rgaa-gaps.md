# Which v0.2.2 capabilities map to RGAA gaps?

- Label: `wayfinder:grilling` (HITL — resolve only through live exchange with the human)
- Status: closed · Assignee: opencode (claimed)
- Blocks: Adopt-now vs defer scope
- Blocked by: Research the Obscura v0.2.0 → v0.2.2 delta relevant to RGAA · Research the current Holo-RGAA ↔ Obscura integration

## Question

Given the two research answers, which v0.2.2 capabilities earn adoption because they close a real RGAA gap — and which are interesting but deferred? Candidate mappings to judge: labelable-element/`getByLabel` + `Input.insertText` + `FormData(form)` → criteria 11.x forms; input-value painting + textarea control box + SVG/emoji rendering → evidence/screenshot fidelity; `postMessage` targetOrigin + `indeterminate`/label-activation + unhandledrejection/ICU-locale + `<body onload>` → 7.x scripts / 4.x content; `Runtime.evaluate` exceptionDetails + session-owned contexts + `DOM.getBoxModel` quads → axe/gap-fix reliability; MCP navigation drain + task-queue pumping → pre-scan/submit flakiness; worker-survival + bad-script isolation → deep/SPA pages; binary bodies + `takeResponseBodyAsStream` → fetch/XHR evidence.

## Context

Needs both research tickets closed first. Call the Skill tool twice for `grilling` and `domain-modeling` when working this ticket. Sharpen terms (e.g. "fidelity" vs "correctness" vs "robustness") into CONTEXT.md only as they crystallise; offer ADRs only if hard-to-reverse + surprising + real trade-off.

## Answer (to record on resolution)

Post as resolution comment + map pointer: adopt/defer table (capability · RGAA criterion/pipeline stage · verdict · rationale). Graduates fog patches into new tickets if the mapping exposes sharp follow-ups; rules anything past the destination out of scope instead.

## Resolution (closed, human-confirmed "ok rec")

| Cluster | Capability | RGAA / pipeline target | Verdict | Rationale |
|---|---|---|---|---|
| Forms | Labelable-element resolution + `getByLabel`, `Input.insertText`, `FormData(form)`, label-activation, `indeterminate` | 11.x / 1.x correctness, pre-scan fill | Adopt | Spec-compliant label association; replaces generic evaluate snippets; no new action type |
| Navigation + frames | MCP navigation drain + task-queue pumping, SPA same-document reporting, real frame trees + per-frame realms, `postMessage` scoping | Submit/cookie-consent reliability, SPA routers, iframe criteria | Adopt | Transparent substrate behaviors; unblocks verification of submits, routes, frames |
| Evidence + diagnosis | Painted input values, real control boxes, viewport containing block, font/emoji/SVG fallbacks; `exceptionDetails`, session contexts, integer `getBoxModel` quads, ICU locale, `unhandledrejection`, `<body onload>` | Fidelity of screenshots/AXTree evidence; geometry/visibility checks; script-failure diagnosis | Adopt | Evidence logic unchanged, becomes truthful; one code touch: branch on real `getBoxModel` errors |
| Deferred | Binary XHR/fetch bodies; generated typed CDP clients; CSPRNG blob URLs; screencast/PDF; stealth rollout | None currently tied to an RGAA gap | Defer | Revisit only on concrete binary-audit or stealth-audit need |

No new tickets graduated: fog patches (fidelity, pre-scan reliability, robustness, CDP strictness, security posture, packaging) are now decided here and carry into Upgrade path and risk for v0.2.2 + Adopt-now vs defer scope. Nothing ruled out of scope beyond the defer list above.
