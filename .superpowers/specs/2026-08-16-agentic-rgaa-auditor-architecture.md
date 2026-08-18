# Agentic RGAA Auditor — Well-Engineered Architecture (rig + Obscura RIG + Holo3)

**Date:** 2026-08-16 · **Status:** Design (pending review) · **Replaces:** ad-hoc hand-rolled Holo3 client + manual action loop.

## Why the previous design was "poorly engineered"

The earlier vision relied on:
1. A hand-written `Holo3` HTTP client (`reqwest` + manual `serde_json`) instead of a typed agent runtime.
2. The model emitting **free-form JSON "actions"** that we parse and dispatch ourselves — brittle, no schema guarantee, no tool-call safety.
3. No separation between *reasoning* and *acting* — the same prompt tried to both decide and execute.
4. No grounding: a screenshot + "click the button" with no stable element identity → model guesses coordinates → drift.
5. No verification: nothing re-checks page state after an action, so a failed step silently poisons the verdict.
6. No human-in-the-loop or evidence trail → not defensible under EU enforcement (which demands 100% of applicable criteria proven).

## The better architecture (grounded in real, shipped Rust)

Use **`rig`** (`0xPlaygrounds/rig`, 8.3k★) as the agent runtime. It is exactly the missing layer:
- **OpenAI-compatible provider** (`OpenAIClient::from_url_env` / `GenericCompletionModel`) — point it at `https://api.hcompany.ai/v1/` with `HAI_API_KEY`. Holo3 is OpenAI-compatible and supports `structured_outputs` + function calling → native tool use, no manual JSON parsing.
- **Typed `Tool`s** via the `#[rig_tool]` derive → compile-time-safe functions the model can call. Replaces hand-parsed "actions".
- **`extractor`** for schema-validated structured verdicts (maps to our `CriterionStatus` + evidence).
- **`AgentHook`** for human-in-the-loop approval + `NeedsReview` escalation (ties to `CriterionStatus::NeedsReview` from the deep-integration spec).
- Multi-agent composition, OTel tracing, and **mock models** for deterministic tests.

### Browser acting — expose Obscura RIG as tools (or MCP)

Our `rgaa-obscura` already speaks CDP. The *well-engineered* pattern (seen in `remix-browser`, `gsd-browser`, `chrome-agent` — all Rust, CDP, MCP) is: a **tool surface** of granular, idempotent browser actions with stable element identity + accessibility-tree snapshots. Add to Obscura RIG (or a thin `rgaa-browser-tools`):

| Tool | Purpose | RGAA use |
|------|---------|----------|
| `navigate(url)` | load page | entry |
| `screenshot()` | CDP `Page.captureScreenshot` | visual `IA_ASSISTE` (contrast, text-in-image, layout) |
| `accessibility_tree()` | a11y-tree snapshot with stable `backendNodeId` UIDs | element identity, avoids coordinate-guessing |
| `eval_js(snippet)` | run axe/gap-fix/in-page checks | deterministic stage |
| `click(ref)` / `type(ref,text)` / `press_key(key)` | act on a stable ref | keyboard operability, focus order |
| `tab_order()` | enumerate focusable sequence | Crit. 7.3 / 12.8 (focus order) |
| `assert_state(predicate)` | verify post-action state | closes the act→verify loop |

Expose these as `rig` Tools directly, **or** as an MCP server that `rig` consumes (`rig` has first-class `rmcp` support) — giving us cross-tool interop with `remix-browser`/`gsd-browser` for free.

### Grounding + verification (from Mantis + Holo3 blog)

- **Grounded mode:** feed the model the accessibility tree / interactive-element list; the model emits actions referencing *stable element refs*, not pixels. Cuts coordinate drift.
- **Act→verify loop:** every `click`/`type` is followed by `assert_state` (or a fresh `accessibility_tree`) before the next step. A stale-screen / no-op step triggers a retry with a guard (max N) — never an infinite loop.
- **Trace per criterion:** `{screenshot, action, resulting_state, verdict, evidence}` → the legally-defensible audit record EU enforcement needs.

### Three-engine composition (ties to existing 106-criteria catalog)

- **Deterministic engine** (already built): axe-core + 10 gap-fix snippets + page context — no model.
- **Agentic engine** (rig + Holo3 + Obscura tools): drives the browser for the ~27 `IA_ASSISTE` interaction/visual criteria (focus order, keyboard operability, name/role/value, rendered contrast, dynamic-update, etc.).
- **Regulatory-intelligence engine:** `rig` RAG over the RGAA catalog + EN 301 549 v3.2.1 + WCAG 2.2 + EAA scope rules, so verdicts carry the correct *legal basis* and the tool auto-produces the accessibility statement, multi-year plan, and derogation docs. (Retrieval via a search tool where web access is available; vector store where offline.)

### Model tiering

- `holo3-1-35b-a3b` (free, 10 req/min) for tactical acting + visual reads.
- `holo3-122b-a10b` (paid) for reasoning/planning + low-confidence escalation.
- **RPM rate-limiter** (token-bucket, as in Mantis) — replaces the current fixed `HOLO3_CONCURRENCY=12` semaphore.

## Compliance posture

- Every criterion gets a verdict **with evidence** (screenshot/state trace) → satisfies the French "100% of applicable criteria" bar.
- `Na` auto-applied for non-applicable (e.g. video → audio-only criteria); `NeedsReview` for model-uncertain → human sign-off, never silent pass.
- Output artifacts: per-criterion results, accessibility statement, multi-year remediation plan, derogation register — EAA/EN 301 549 ready.

## Phased build

1. **Foundation:** add `rig` dep; OpenAI-compatible provider → Holo3; one `BrowserTools` tool (screenshot) backed by Obscura RIG. Verify with `rig`'s mock model.
2. **Acting:** full tool surface + grounding + act→verify loop on 2–3 high-value criteria (focus order, keyboard operability).
3. **Multimodal verdicts:** `holo3-1-35b` visual reads for visual `IA_ASSISTE`; structured `extractor` → `CriterionResult`.
4. **Regulatory RAG:** catalog + EN 301 549/WCAG 2.2 embeddings; statement/plan/derogation generation.
5. **HIL + traces:** `AgentHook` approval + per-criterion evidence bundle; wire into `rgaa-orchestrator`.

## Decision (2026-08-16)

- **Obscura tools are exposed as an MCP server** that `rig` consumes via its `rmcp` support (not native `#[rig_tool]`). Rationale: interop with `remix-browser`/`gsd-browser`, clean process isolation, and a reusable tool surface for any MCP-capable client.
- **No code scaffold yet** — this spec is the deliverable for now. Implementation begins after scope/model-tiering alignment.

### Resulting component layout

```
rgaa-obscura  ──►  rgaa-browser-mcp  (MCP server: navigate/screenshot/a11y-tree/click/type/assert)
                                        │  (rmcp, stdio/SSE)
                                        ▼
rgaa-agent  (rig Agent: OpenAI-compatible provider → Holo3, RAG, HIL)  ──►  rgaa-orchestrator
```
