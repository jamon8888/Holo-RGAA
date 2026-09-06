# Research the Obscura v0.2.0 → v0.2.2 delta relevant to RGAA

- Label: `wayfinder:research` (AFK)
- Status: closed · Assignee: opencode (claimed)
- Blocks: Which v0.2.2 capabilities map to RGAA gaps · Upgrade path and risk for v0.2.2 · Adopt-now vs defer scope
- Blocked by: (none — frontier)

## Question

What exactly changed between the pinned **v0.2.0** binary and **v0.2.2** that matters to an RGAA audit engine? Produce a filtered delta (not a full 248-commit dump): CDP/protocol, DOM/JS, rendering/screenshots, form filling, MCP behavior, security/SSRF, robustness, packaging — with issue links and one-line RGAA relevance each.

## Context

- Upstream: https://github.com/h4ckf0r0day/obscura/releases (v0.2.2 § Highlights/CDP/Security/Rendering/DOM/MCP; v0.2.1 Highlights/CDP/DOM/Security/Tooling as the middle step; v0.2.0 native rendering baseline).
- Known v0.2.2 highlights to verify: generated CDP clients work (#833, #703); Playwright form filling + context setup (#577); bad-script isolation (#699); worker survival on deep/hostile pages (#705, #580); binary XHR/fetch bodies (#818); `Runtime.evaluate` exceptionDetails (#746); session-owned execution contexts (#800); `DOM.getBoxModel` integer quads (#576); SSRF deny-set + stealth DNS guard (#810, #816); `createObjectURL` CSPRNG (#820); MCP navigation drain (#618); version self-report; corrected stealth download note (#733).
- Known v0.2.1 middle-step to cover: iframe V8 realms/postMessage (#600); MCP task-queue pumping (#618, #640); SPA same-document navigation reporting; `Input.insertText` JSON literal; `Storage.clearCookies`; `--eval` promise awaiting (#693); `Runtime.removeBinding` validation (#578); `DOM.setFileInputFiles` gated behind `--allow-file-access` (#579, behavior change); PBKDF2 caps (#580); cyclic-DOM bounds (#582).

## Answer (to record on resolution)

Post as resolution comment + append context pointer to map's Decisions-so-far: versioned delta table (area · change · upstream ref · RGAA relevance · adopt/defer hint). Link any scratch branch `research/obscura-delta-v022` if created. Do not change code.

## Resolution (closed)

Adopt-now slice: iframe realms (#600), MCP pumping + navigation drain (#618/#640), label/`insertText` form filling (#577), SPA nav reporting, `exceptionDetails` (#746), session contexts (#800), `getBoxModel` quads (#576), input painting + screenshot warmup (#685/#667), `postMessage`/`FormData`/label semantics (#704/#732/#803), SSRF config (#810/#816) + `--allow-private-network` / `--allow-file-access` (#579), version self-report. Defer: binary bodies (#818), generated clients (#833/#703), CSPRNG/docs (#820/#733). Full table: `../research/obscura-delta-v020-v022.md`.
