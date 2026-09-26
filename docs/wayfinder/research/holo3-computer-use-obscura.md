# Holo3 computer-use × Obscura: act→observe loops for RGAA-specific tasks?

Research findings for [#196](https://github.com/jamon8888/Holo-RGAA/issues/196). Part of map [#183](https://github.com/jamon8888/Holo-RGAA/issues/183).

- **Audited at:** `666607c` (master) for all code citations; cross-checked against `c451397` (`feat/multi-model-llm-config`) — the only drift is line numbers in `rgaa-agent/src/agent.rs` (multi-model config), no behavioural difference to anything cited here.
- **Holo API facts:** H Company Platform docs (`/models-api/*`), read 2026-09-26 via the docs MCP server. Authoritative and current; they describe a **Holo4** generation the workspace does not know about.
- **Scope note:** this ticket asks whether the *model should decide* what to interact with. It does not re-open pixel clicking — [#194](https://github.com/jamon8888/Holo-RGAA/issues/194) closed that (mode A, not worth doing).

---

## TL;DR

**Verdict: no model-driven act→observe loop. Yes to a scripted act→observe *probe* whose trace a single completion judges.**

Three findings decide it, and none of them is about cost:

1. **The loop substrate already exists and is already grounded** — `rgaa-obscura/src/guided.rs` is a stateful, single-CDP-session act→observe executor on stable `ax:<backendNodeId>` refs, with a mutate-must-be-followed-by-observation invariant, bounded retries, typed termination reasons and evidence artifacts. It is *script*-driven, not model-driven. The missing piece was never the execution layer.
2. **The tools the agent would be handed are the wrong ones.** `rgaa-browser-tools` (the 9 tools #193 enumerated) cannot host a loop at all: every primitive calls `ObscuraBridge`, and every one of those bridge methods does `Target.createTarget` → act once → `Target.closeTarget`. `press_key("Tab")` five times is **five fresh page loads with one Tab each**, not five tab stops. Cited below.
3. **Only 1 of the 9 shortlisted criteria is even LLM-eligible today.** 12.8 is the only shortlist member in `Classification::IaAssiste`. 7.5 (status messages) is the workspace's single `Manuel` criterion and #180's settled precedence sends every `Manuel` criterion straight to `NeedsReview` with no LLM call. The rest are `Deterministe`. A loop mode would have to reclassify first — that is #180/#182's decision, not this ticket's.

Budget is the fourth finding and it is emphatic: a model-driven focus-order walk costs **~22–62 completions on one page** against #180's hard cap of **10/page, 70/audit**. The scripted-probe-plus-one-judge shape costs **1–2** and fits the ~3 completions of slack the cap already has.

---

## 1. RGAA task shortlist — verified against the catalog

The issue's shortlist carries four inherited catalog errors. Corrected against `rgaa-rs/crates/rgaa-core/data/rgaa-4.1.2/criteres.json`:

| Issue says | Catalog says | Note |
|---|---|---|
| "keyboard operability/trap (12.4/12.5-adjacent)" | **12.9** = keyboard trap. 12.4/12.5 are sitemap and search-engine *reachability* | unrelated criteria |
| "form error identification/required fields (14.x)" | **RGAA 4.1.2 has 13 topics — no 14.** Forms = topic 11: 11.10 input control, 11.11 correction suggestions, 11.12 confirm/undo | |
| "CAPTCHA alternative (13.6)" | **1.5** = CAPTCHA alternative access. 13.5/13.6 are cryptic content (ASCII art, emoticons) | also mislabelled in-code: `rgaa-agent/src/criteria_defs.rs:694` comments 13.6 as "CAPTCHA alternative relevance" inside `VISUAL_CRITERIA` |
| "zoom/reflow at 400% (10.4)" | **10.4** = text legible at 200% zoom. **10.11** = reflow, no 2-D scrolling (320px/256px, i.e. the 400% case) | two different criteria |

Per-task assessment. "Eligible" = in `Classification::IaAssiste` (`criteria.rs:17-148`), the only set the LLM is allowed to touch under #180's precedence.

| Criterion | What a loop sees that a static scan can't | Already covered? | Eligible today | Verdict |
|---|---|---|---|---|
| **12.8** tab order coherent | Actual focus sequence vs DOM/visual order — requires walking focus | `run_igt_keyboard` walks ≤50 tabs and records the sequence (`rgaa-obscura/src/lib.rs:2063-2216`), **but it is not on the audit path** — `audit_one` never calls `analyze`, and `igt_tools` defaults to empty (`config.rs:150`, `tools/analyze.rs:154`) | **yes** (`IaAssiste`) | **scripted probe + 1 judge.** The walk is deterministic; only *coherence* is a judgment. Highest-value item on the list |
| **12.9** keyboard trap | Focus failing to advance | Two implementations, both deterministic and both *already sound*: `run_igt_keyboard` trap counter ≥5 (`lib.rs:2168-2181`) and `GuidedAction::PressKey`'s before/after `activeElement` signature check (`guided.rs:544-560`) | no (`Deterministe`) | **no LLM.** Pure state check — wire the existing probe, don't add a model |
| **10.13 / 12.11** content on hover/focus | Whether revealed content is reachable, dismissible, persistent | `10.14` gap-fix snippet only pairs `:hover` CSS selectors with `:focus` (`rgaa-rules/src/gap_fix.rs:206-250`) — static, blind to JS-driven reveals | no (`Deterministe`) | **defer.** Genuine loop value, but needs reclassification first and the per-component cost multiplies (§4) |
| **11.10 / 11.11** input control, correction suggestions | Error messages that only exist *after* a bad submit | Nothing. No submit is ever performed anywhere in the pipeline | 11.10 **yes**; 11.11 no | **scripted probe + 1 judge** for 11.10 — the probe (fill invalid, submit, observe) is scriptable; the judgment (is the message identifying and suggestive?) is not |
| **7.5** status messages | Live-region announcement after an interaction | Nothing | **no — `Manuel`** | **out of scope.** #180: `Manuel` → `NeedsReview`, no LLM call. Hard-blocked by a settled constraint |
| **1.5** CAPTCHA alternative | Whether the alternative route actually works | Nothing | no (`Deterministe`) | **no.** Exercising a CAPTCHA alternative means completing a bot challenge. Out of bounds |
| **accordion/modal states** | `aria-expanded` flipping, focus moving into/out of a dialog | Nothing | not a criterion — surfaces under 7.1/7.3 | **no LLM.** Assertable state transitions, deterministic |
| **10.4 / 10.11** zoom & reflow | Layout after a real viewport/zoom change | `10.11` snippet compares `scrollWidth` against a hardcoded 320 **at the current viewport without resizing it** (`gap_fix.rs:188-202`) — the check is unsound as written | no (`Deterministe`) | **no LLM — fix the measurement.** `Emulation.setDeviceMetricsOverride` + re-measure. This is #194's mode C, already on #183's fog |

**Shortlist result:** 12.8 and 11.10 justify an act→observe *probe*. Neither justifies the *model* choosing the actions.

---

## 2. What the hosted Holo API actually exposes

From the H Platform docs. The workspace's picture of this API is ~13 months stale.

| Capability | Reality | Consequence here |
|---|---|---|
| Protocol | OpenAI-compatible `POST https://api.hcompany.ai/v1/chat/completions`; multi-turn; `tools`/`tool_calls` standard fields | rig's openai provider already speaks it — no transport work needed |
| **Function calling** | Every model **except `holo3-122b-a10b`**, which supports structured outputs *only* | **The workspace routes all `VISUAL_CRITERIA` — including 12.8 — to the reasoning tier, which by default is a tier that cannot call tools at all if set to 122b** (`agent.rs:34-40`, `criteria_defs.rs:680-695`). A tool loop on 122b is impossible; it needs the structured-output loop shape instead |
| Images | `image_url` parts, HTTPS or base64 data URI, JPEG/PNG/WebP, **≤5 per request**; docs further advise **keeping at most the last 3 screenshots**, evicting older ones to a text placeholder | Hard ceiling on observations carried per request. An N-step visual loop cannot show the model all N states |
| Action space | Holo's *trained* browser harness is `click(element, x, y)` / `type` / `scroll` / `goto` / `answer` with **coordinates normalized to [0,1000]** | The trained harness **is** the coordinate pattern #194 rejected. A custom `ax:`-ref action space is off-distribution — the grounding strength cited as the reason to pick Holo3 is specifically the thing this architecture declines to use |
| `answer` tool | "the only way the model signals it is done"; a turn with no tool call is a no-op to be re-looped, not a stop | Workspace has **no `answer` tool** and bounds the loop with `default_max_turns(3)` (`agent.rs:95`) — a hard truncation, not a completion signal |
| Reasoning | Two channels per call: `message.reasoning` + `message.content`. "Reasoning is essential in agent mode"; `reasoning_effort: "medium"`. Past reasoning is dropped by the chat template — durable facts must flow through `content` | **Collides with #186's transport patch** (`enable_thinking` off). "Thinking off" is correct for single-shot judging and wrong for a loop. If both modes ship, the flag becomes per-mode, not global |
| Sampling | `temperature: 0.8` for loops; `0.0` for single-shot | Neither current path does this: `transport.rs` sends 0.1, the rig path sends nothing (#193, 1b) |
| Grounding | `element-localization`: single call, thinking off, temp 0, coords in [0,1000] | Available, deliberately unused (#194) |
| Rate limits | Free = 10 RPM, `holo3-1-35b-a3b` only. Paid = higher, all models | Free tier caps a loop at ~10 steps/minute regardless of budget |
| **New generation** | `holo4-35b-a3b` (262k ctx, $0.30/1M in) and `holo4-27b` — both function-calling, "start here"; Holo3 stays served. `GET /v1/models` exposes `deprecation_date` and `supported_features` | The `holo3-*` IDs the workspace pins are one generation behind, and `supported_features` is machine-readable — a bake-off roster should be resolved at runtime, not hardcoded. Worth its own note on #183 |
| Retention | Zero data retention by default; prompts/responses not stored | Relevant to sending client screenshots; no blocker |

---

## 3. Execution safety — the substrate is built, the tools are the wrong ones

### 3a. `rgaa-browser-tools` cannot host a loop

Every tool delegates to `BrowserSession` → `ObscuraBridge`, and each bridge method is self-contained: connect WebSocket → `Target.createTarget({url})` → `wait_for_load` → act → `cleanup_target`.

- `click_element` `rgaa-obscura/src/lib.rs:1058-1116`
- `type_input` `:1904-1955`
- `press_key` `:1958-2008`
- `get_tab_order` `:2010-2060`
- `assert_state` `:2221-2290`

`BrowserSession` holds `current_url` and hands it to each call (`session.rs:64-120`), so state is a *URL string*, not a live page. Consequences:

- **`press_key` is unusable in sequence.** Each press lands on a freshly loaded page. Tab stop 1, five times over.
- **`assert_state` always observes a fresh load**, never the post-action state it is supposed to verify.
- **`click` is CSS-selector based**, not `ax:`-ref based — the arch spec's stable-ref requirement is only actually honoured in the *guided* path.
- **`get_tab_order` is also wrong on its own terms:** it sorts all focusables by `tabIndex` ascending (`:2049`), which places every `tabindex="0"` element before every positive `tabindex`. Real tab order is positive values first, in value order, then document order for 0. It also reads `aria-role` (not a real attribute) for `role`.

`BrowserMcpServer` is a name-listing placeholder (`rgaa-browser-tools/src/mcp.rs`), as #193 found. The orchestrator constructs a `ToolContext` (`pipeline.rs:190,264-265`) purely as a mutex around the bridge — the tools inside it are never invoked on the audit path.

### 3b. `rgaa-obscura/src/guided.rs` is the real act→verify loop

One target, one session, held across all steps (`guided.rs:322-380`). It already implements what the arch spec described:

| Spec requirement | Where |
|---|---|
| Stable refs only, hard error otherwise | `resolve_reference` accepts `ax:<backendNodeId>` or `ax-role=R;name=N`, else `MissingReference` — `guided.rs:433-458`, `:71-76` |
| Act→verify ordering invariant | a mutating step not followed by an observation/assertion → `InvalidOrdering`, remaining steps marked unanalyzed — `:152-166`, `:270-283` |
| Bounded retries | `MAX_STEP_ATTEMPTS = 3`, retryable = timeout/CDP transport only — `:9`, `:236-253` |
| Typed failure reasons | `TerminationReason` × 8 — `:78-89` |
| Evidence trail | tree + screenshot artifacts through `EvidenceStore`, `EvidenceRef` list, required-evidence gate — `:170-186`, `:225-238` |
| Trap detection inline | `PressKey` compares `activeElement` signature before/after; Tab with no change → "keyboard trap detected" — `:544-560` |
| Honest failure | `is_pass()` demands Completed + no issues + no unanalyzed + no manual-review + all required evidence — `:125-137` |

It is reachable only from the deprecated MCP `igt` tool (`rgaa-mcp/src/server.rs:731-745`) and `rgaa-cli`'s `igt` command, which passes an empty `criterion_mapping` (`commands/igt.rs:44`). **No RGAA guided-test library exists** — zero `GuidedTest` definitions ship for any criterion.

### 3c. Minimal tool set — and why `assert_state` must not be free-form

If a model were ever put in this loop, the registration is small: `navigate`, `accessibility_tree`, `press_key`, `click_ref`, `fill_ref`, `screenshot`, `assert_state`, plus Holo's mandatory **`answer`**. That is `GuidedAction` (`guided.rs:37-45`) plus `answer`. Do **not** register `eval_js`.

Two things must change before any of it is model-facing:

**Observations are evidence-shaped, not model-shaped.** `AccessibilityTree` returns a flat `Vec<String>` of `"ax:<id>"` strings with **no role, name, or order** (`guided.rs:511-538`) — opaque ids a model cannot reason over, even though the role/name index is built in the same pass and thrown away. `PressKey`, `ClickRef` and `FillRef` return `GuidedObservation::default()` — *nothing*. And the screenshot bytes go to the evidence store, never into a message (`:565-590`).

**`assert_state` exists in two incompatible forms, and the model-facing one is unsound:**

- guided form: no predicate at all. `observe_state` returns a **fixed** shape — `{url, title, active_tag, values[]}` (`guided.rs:398-414`) — and the runner subset-matches it against the step's `expected` (`state_matches`, `:104-123`). Sound, but `active_tag` alone ("INPUT") cannot express a focus-order or `aria-expanded` assertion.
- tool form: `AssertStateArgs { predicate: String }` — an **arbitrary JavaScript expression** (`tools/assert_state.rs:14-19`). A model that authors its own predicate can return `true` and manufacture its own PASS. For an evidence-grade audit that is disqualifying.

The right shape is the guided form with a widened, closed vocabulary — focused element's `ax:` ref + role + accessible name, `aria-expanded`/`aria-selected`/`aria-invalid` on named refs, live-region text, visibility of a named ref — asserted declaratively by the harness, never by model-authored JS.

---

## 4. Budget collision with #180

#180's settled numbers: **10 completions/page, 70/audit**, batch size 5 mapped by `criterion_id`, **"Evaluation agent: no tools, no crawl preamble, 1 turn"**, **"Text-only v1"**. Current consumption: 32 `IaAssiste` criteria ÷ 5 = **~7 completions/page** before NA/axe-Fail skips. Slack ≈ 3.

Holo's loop costs 1 completion per step (one tool call per step, `tool_choice: "required"`):

| Task | Model-driven loop | Scripted probe + judge |
|---|---|---|
| 12.8 focus order | walk N tab stops → **N+2**; IGT bounds N at 50 → **≤52**, typically 22–62 on a real page | probe = 0 completions; **1** to judge the trace |
| 12.9 trap | same walk → **≤52** | **0** — deterministic |
| 11.10 error identification | fill + submit + observe + judge, per form → **3–4 × forms** | probe = 0; **1** to judge |
| 10.13/12.11 hover content | focus + observe + assert per component → **3 × K**, K = 5–30 | probe = 0; **1** per page |
| 10.11 reflow | resize + observe + judge → **2–3** | **0** — measurement, not judgment |

**A single model-driven 12.8 walk exceeds the per-page cap by 2–6× and can exhaust the whole 70-completion audit budget on one page.** It also breaks the "1 turn, no tools" constraint by construction, and "text-only v1" by construction.

The scripted-probe shape costs **1–2 completions total** for 12.8 + 11.10 (+10.13 if reclassified) — batched at 5, that is **one batch**, inside the existing slack, inside the existing text-only shape (the trace is text), and it never touches the settled constraints. It is not a new mode; it is a new *evidence source* for criteria already in the flow.

If a true model-driven loop is ever wanted, it is a **separate opt-in mode with its own budget** (`--deep`, its own cap, off by default), never inside #180's flow.

---

## 5. Bake-off implication

**Neither a second race track nor a later round — not yet scoreable.**

`rgaa-test-corpus` has 37 fixtures over 26 criteria (#185) and **zero fixtures for any shortlisted interaction criterion**: nothing for 12.8, 12.9, 7.5, 10.11, 10.13, 11.10, 11.11, 12.11. The corpus cannot measure an interaction verdict at all, so #192's floor rule (per-criterion recall ≥ 70% AND precision ≥ 70%) has no denominator here. A "computer-use Holo3 vs Qwen-with-tools" track would produce numbers with nothing to compare them against.

Two further reasons it is not a #192 roster question:

- **The roster follows the set at run time** (#192, #182). 12.8 is the only shortlist member in the set; the rest need reclassification first.
- **Capability, not roster.** `holo3-122b-a10b` cannot call tools at all; Ollama-hosted Qwen tool-calling fidelity varies by build. A tool-loop track is a *harness* comparison, which is a different protocol from a judge comparison.

Sequence if this is ever pursued: fixtures for the interaction criteria → judge bake-off on the trace-judging prompt (fits inside #192 unchanged, since the trace is text) → only then a harness comparison, as its own ticket.

---

## 6. Verdict and routing

| Question | Answer |
|---|---|
| Should Holo3 computer-use drive Obscura for RGAA tasks? | **No** — not model-*decided* actions. The decisions worth making are already scriptable; the judgments are already text-shaped |
| What is worth the loop? | **The acting, scripted; the judging, one completion.** 12.8 (focus order) first, 11.10 (error identification) second |
| What stays static? | 12.9, accordion/modal states (deterministic assertions); 10.4/10.11 (fix the measurement — #194 mode C); 1.5 (out of bounds); 7.5 (`Manuel`, hard-blocked) |
| Where does the decision land? | **#180's flow spec.** Two guided probes feeding evidence into two criteria already in the flow, at +1–2 completions inside existing slack. Not #192 — no fixtures, so nothing to score. Not a new map |

### Prerequisites, in order

1. **Guided-test library for RGAA.** Define `GuidedTest`s with populated `criterion_mapping` for 12.8 and 11.10. Currently zero exist. This is the whole deliverable — the executor is done.
2. **Put the probe on the audit path.** `audit_one` never calls `analyze` and `igt_tools` defaults to empty, so even the keyboard walk that *is* built never runs during an audit.
3. **Widen `observe_state`** to carry focused `ax:` ref + role + name and the ARIA state flags a focus-order or error-identification verdict needs. Keep it a closed vocabulary.
4. **Fix `get_tab_order`'s ordering** (positive `tabindex` before 0) or delete it in favour of the guided walk.
5. **Never register `eval_js` or predicate-form `assert_state`** on a model. A model-authored predicate can fabricate its own PASS.

### Two findings for #183's fog

- **Model IDs are a generation stale.** `holo4-35b-a3b` / `holo4-27b` ship with 262k context and function calling; `GET /v1/models` exposes `supported_features` and `deprecation_date`. `holo3-122b-a10b`'s lack of function calling is a capability fact the tier router should not hardcode around.
- **`enable_thinking` is per-mode, not global.** #186's transport patch turning it off is right for single-shot judging and wrong for any loop; Holo's docs call reasoning "essential in agent mode".
