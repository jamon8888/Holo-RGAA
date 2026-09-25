# Could pixel grounding / coordinates enhance the audit?

Wayfinder research ticket: jamon8888/Holo-RGAA#194 (map #183).
Base: `666607c` (master). Hybrid: local code/spec evidence + literature review.

Three modes kept separate: **acting** by coordinates, **seeing** pixels (model input), **measuring** pixels (deterministic).

## Mode A — Acting by coordinates (OSWorld-style GUI agent)

**Verdict: not worth doing; the spec's rejection stands.**

- The architecture spec (`.superpowers/specs/2026-08-16-agentic-rgaa-auditor-architecture.md:11`) rejected it for a structural reason, not a hardware one: *"No grounding: a screenshot + 'click the button' with no stable element identity → model guesses coordinates → drift."* Its **grounded mode** deliberately uses the accessibility tree with stable `backendNodeId` refs instead (line 42, line 32 table). Drift is not a benchmark gap — coordinate clicking is *less* reliable than refs, and Holo3's OSWorld strengths optimize exactly the rejected pattern.
- Current surface matches the spec: 9 browser primitives, all selector/key/JS-predicate; Obscura guided clicks require `ax:` refs (#193). Zero coordinate params exist anywhere.
- Which criteria would coordinate-acting uniquely cover? Candidates: canvas/WebGL widgets, elements absent from AXTree, drag-and-drop, hover-only menus. But RGAA's interaction criteria are state-based, not pixel-based: 2.1.1/2.1.2 (keyboard), 4.1.2 (role/name/value), 12.8 (focus order) are testable via keyboard walk + AXTree — and the IGT/keyboard path already exists in the pipeline. Hover-only menus are a CSS/DOM query (':hover' state, computed style), not a click-at-pixel problem.
- The spec's phased plan ("Acting: act→verify loop on focus order, keyboard operability") was never wired (#193) — but its design still says actions reference *element refs*, not pixels. Re-opening pixel-acting would reverse a deliberate, still-valid decision.
- Residual niche: canvas-heavy apps where the AX tree is empty. Even then the defensible move is *evidence capture* (screenshot of the canvas state for the human validator), not model-driven pixel clicking.

## Mode B — Seeing pixels (model receives screenshots)

**Verdict: the largest genuine fidelity lever — already parked as #180's multimodal fog.**

- Today the judge is **text-only**: screenshot plumbing exists (`evaluate_multimodal`, `build_with_image`, ScreenshotTool) with zero production callers (#193).
- `VISUAL_CRITERIA` (`rgaa-agent/src/criteria_defs.rs:680-695`) names **14 criteria explicitly annotated "must SEE"**: 1.3, 1.7, 3.1, 5.3, 10.3, 10.10, 11.2, 11.7, 11.8, 11.9, 11.10, 12.3, 12.8, 13.6 — alt-text relevance, color-only info, reading order, label relevance, focus order. A text-only judge is handicapped *by design* on these; any bake-off (#192) measures that handicap equally for all models.
- Literature (web, 2025-26):
  - *Benchmarking PDF Accessibility Evaluation* (arXiv 2509.18965): Claude-3.5 scored **82% structural vs 63% visual** criteria — vision closes exactly this gap; GPT-4o-Vision reached perfect accuracy on the visual color-contrast criterion. But vision also *over-flags* (citation links marked failures). Vision helps visual criteria; it is not uniformly better.
  - *Turning manual web accessibility success criteria into automatic* (Springer, Universal Access 2025): text-only LLMs already hit **87% detection** on manual semantic criteria (1.1.1, 2.4.4, 3.1.2) where classic tools scored 0% on intentional failures — semantic judging doesn't need pixels; don't oversell the visual lever for text-judge criteria.
  - *VIABLE* (arXiv 2605.31351): best VLM-as-judge only **52.6%** single-failure diagnostic accuracy, open-source 24.6% — vision judges are a fidelity lever with a low ceiling today; per-criterion measurement (#192 floor rule) remains mandatory.
- Where it belongs: **map #180's deferred multimodal lever** (fog already names it; #180 comment from this research thread posted the wiring facts). Not #183: capability choice, not trust/measurement decision.

## Mode C — Measuring pixels (deterministic, no LLM)

**Verdict: enhances cheaply; evidence already half-built; belongs in #183's fog next to rule expansion.**

- Evidence capture is already PNG + SHA-256 (`rgaa-obscura/src/evidence.rs:46-56`) — the *plumbing* to sample rendered pixels exists; what's missing is any consumer that reads pixels as data.
- Contrast today: axe `color-contrast` is mapped to criteria 3.2/3.3/10.5 (`axe_mapper.rs:80-81,174,186`) but computes from **styles, not rendered pixels** — gradients, opacity, background images and layered backgrounds are the classic blind spots of style-based contrast (the reason WCAG technique-compliance vs rendered-reality diverge). Pixel sampling of the already-captured screenshot computes the true WCAG ratio on rendered output.
- Other deterministic pixel wins: reflow/zoom 400% (10.4/10.10 state capture at zoom levels), hover/focus state screenshots as evidence for 4.1.3/12.7-adjacent discussions, OCR for text-in-images (feeds 1.3 verdicts — complements Mode B without an LLM in the loop).
- Ecosystem precedent: axe/Lighthouse are style-based; PAC 2024 (PDF) analyzes rendered documents; Accessibility Insights ships visual/tab-stop passes. Nobody's differentiator is LLM pixels — deterministic pixel checks are table stakes, cheap, and *more* defensible than model judgment (reproducible, hashable, no confidence threshold).
- Cost: local compute on screenshots Obscura already takes; no model tokens. Fits the trust story: deterministic, evidence-grade, replayable.

## Synthesis

| Mode | Criteria covered (incremental) | Fidelity gain | Cost/complexity | Ecosystem precedent | Belongs |
|---|---|---|---|---|---|
| A: act by pixels | ~0 beyond keyboard/AX (canvas niche only) | Negative vs stable refs (drift) | High (agent loop, act→verify still unwired) | OSWorld = desktop GUI, not audit | **Not worth doing** — spec rejection stands |
| B: see pixels | 14 `VISUAL_CRITERIA` + alt-text quality | High on visual criteria; best VLM judge ~53% ceiling; text criteria already ~87% text-only | Medium (wiring exists; cost per call) | PDF benchmark: structural 82% vs visual 63% | **#180 multimodal lever** (parked, ready to graduate) |
| C: measure pixels | 3.2/3.3/10.5 true contrast, reflow/zoom state, OCR evidence | Medium but deterministic & evidence-grade | Low (screenshots + hashing exist) | PAC 2024, Accessibility Insights, axe blind spots | **#183 fog** — next to rule-expansion, not instead of it |

**One-line answer:** coordinates don't enhance this audit (stable refs win, spec still right); *showing* the judge the page would — and is already parked with map #180; *measuring* the page's pixels is the cheap, deterministic, evidence-grade win this map can pick up in its fog.
