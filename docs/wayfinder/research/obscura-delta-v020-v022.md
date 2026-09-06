# Obscura v0.2.0 → v0.2.2 delta — RGAA-relevant only

Source: https://github.com/h4ckf0r0day/obscura/releases (v0.2.2 `a1e09de` 05 Sep, 126 commits; v0.2.1 `2810cb4` 23 Aug, 122 commits; v0.2.0 `97124ed` 08 Aug native-rendering baseline). Resolves ticket "Research the Obscura v0.2.0 → v0.2.2 delta relevant to RGAA".

Baseline v0.2.0: native Rust rendering (block/inline, Flex/Grid/tables/floats/positioning), screenshots (viewport/clipped/scrolled/full-page PNG/JPEG/WebP via CDP), activity-driven screencast, raster-backed PDF, 4 artifacts (`-stealth` / `-no-render` matrix).

## v0.2.1 (middle step)

| Area | Change | Ref | RGAA relevance | Hint |
|---|---|---|---|---|
| DOM/JS | Child frames own V8 realm; same-origin frame access; postMessage page↔frames; real child frame tree; realm released before isolate | #600 | iframe criteria (titles, hidden content) need real frame tree | Adopt |
| MCP | Page task queue pumped while transport waits; timers/fetches/queued navigations complete | #618, #640 | form submits no longer strand until next tool call | Adopt now |
| DOM/JS | SPA same-document navigations surface; `location` coerced | Highlights | Next.js / React Router / vue-router audits | Adopt |
| Forms | `Input.insertText` as JSON literal (newlines/quotes preserved) | CDP | fill real-world content | Adopt |
| CDP | `Storage.clearCookies`; canonicalized domains; expired deletes | CDP | clean state between pages | Adopt |
| CDP | `--eval` awaits promises (async IIFE resolves) | #693 | async axe-style scripts | Adopt |
| CDP | `Runtime.bindingCalled` to subscribed session; objectId backslash escaping | #557 | multi-session harness | Adopt (transparent) |
| DOM/JS | fetch/XHR relative URLs via URL parser; 20 redirects per Fetch | #663 | resource discovery | Adopt (transparent) |
| Security | Stealth client applies SSRF DNS guard | CDP | safe crawl of hostile targets | Adopt (transparent) |
| DOM/JS | DOMParser `<parsererror>`; `createEvent` rejects unknown; `DOMStringMap` global; `select` parity; `timeOrigin` fix | #610, #599 | Chrome parity for axe/DOM queries | Adopt (transparent) |
| Security | `Runtime.removeBinding` validates names (injection close) | #578 | harness hardening | Adopt (transparent) |
| Security ⚠️ | `DOM.setFileInputFiles` gated behind `--allow-file-access` | #579 | **Breaking:** uploads fail without flag | Adopt flag |
| Robustness | PBKDF2 caps; bounded `children()`/`ancestors()`; module-graph/heap containment | #580, #582 | survive hostile/deep pages | Adopt (transparent) |
| Rendering | Webfont icons + color emoji; textarea intrinsic box + JS interface | Highlights | screenshots + Playwright visibility/fill | Adopt |
| Packaging | target-arch `mksnapshot`; CLI enforces `--obey-robots` | #682 | reproducible builds; robots compliance | Adopt note; defer live-view |

## v0.2.2

| Area | Change | Ref | RGAA relevance | Hint |
|---|---|---|---|---|
| CDP | Generated clients work: `Page.enable` load seq once/page, schema-complete frame | #833, #703 | unblocks chromiumoxide/spider typed clients | Adopt only if using generated clients |
| Forms | `Input.insertText` done; `Element.labels` + `label.control` per spec → `getByLabel`; `grantPermissions`; `supportedEntryTypes` | #577 | label association (1.x, 11.x) + context setup | Adopt now |
| Robustness | Throwing script/rejection logged, event loop keeps scheduling | #699 | one bad script no longer aborts audit | Adopt (transparent) |
| Robustness | Iterative style cascade; clamped deep timers; capped `fetched_urls`/bodies; watchdog cancel-on-drop | #705, #580 | worker survival on deep/hostile pages | Adopt (transparent) |
| DOM/JS | Binary-safe XHR/fetch bodies; `Response.body`/`bodyUsed` | #818 | binary resources (media, PDF) | Defer unless binary audits |
| CDP | `Runtime.evaluate`/`callFunctionOn` throw via `exceptionDetails` | #746 | standard page-error rebuild | Adopt |
| CDP | Session-owned execution contexts; handles survive tab switches | #800 | multi-page/tab flows | Adopt |
| CDP | `dispatchKeyEvent` key/code as JSON literals | #819 | injection-safe key sim | Adopt (transparent) |
| CDP | `DOM.getBoxModel` Chrome-shaped integer quads; failure = real error | #576 | geometry/visibility checks | Adopt — update error handling |
| Rendering | Initial containing block = viewport (fixed/absolute insets) | #675 | fixed headers/modals fidelity | Adopt (transparent) |
| Security | SSRF deny-set: IPv6-embedded-IPv4/6to4/NAT64, CGNAT, IANA ranges; stealth DNS guard + `--allow-private-network` | #810, #816 | intranet + public audits | Adopt config |
| Security | Platform/UA as JSON literals; `createObjectURL` CSPRNG blob URLs | #792, #820 | hardening; stable fingerprint | Adopt transparent / defer CSPRNG |
| Rendering | Inputs paint value; textarea control box; font warmup only used sources; SVG fallback | #685, #667, #698 | screenshots show filled values; faster captures | Adopt now |
| DOM/JS | `postMessage` honors `targetOrigin`; `indeterminate` + label click; `FormData(form)` iterable; `unhandledrejection` + pinned ICU locale; `<body onload>`; whitespace fix | #704, #732, #803, #734, #768, #785 | form semantics; lang audits | Adopt |
| MCP | Drain synthesized navigation before tool replies | #618 | fixes premature "no navigation" | Adopt now |
| Packaging | Self-reported version (tag; `0.1.0-dev+` local); MCP `serverInfo` = CLI | MCP | pin + verify binary in CI | Adopt (log per run) |

Adopt-now slice: #600, #618/#640 + drain, #577 + insertText, SPA nav, #746, #800, #576, #685/#667, #704/#732/#803, #810/#816 + `--allow-private-network` + `--allow-file-access` (#579), version self-report. Defer: #818, #833/#703, #820/#733.
Full changelogs: `v0.2.0...v0.2.1`, `v0.2.1...v0.2.2`.
