# Design: Deep Holo3 Integration into the RGAA Platform

- **Date:** 2026-08-16
- **Status:** Approved design, pending implementation plan
- **Owner:** RGAA-RS / rgaa-holo + rgaa-orchestrator + rgaa-obscura + rgaa-core

## Context & Goal

The orchestrator audits each URL by running axe-core + gap-fix (Déterministe
criteria) and a Holo3 LLM evaluation of the 27 `IA_ASSISTE` criteria. The
current Holo3 path has correctness and accuracy gaps that the user wants fixed
"to perfect":

- Output parsing relies on fragile regex / code-block extraction of a JSON blob
  the model is only *asked* (not *forced*) to emit.
- Concurrency is a static `HOLO3_CONCURRENCY = 12` semaphore, which exceeds the
  Holo3 free-tier limit of **10 requests/minute** and leans entirely on 429
  retries.
- The API key is hardcoded in `pipeline.rs`.
- Prompts tell the model to "evaluate criterion RGAA 1.1" with page context but
  **no definition of what 1.1 requires**, hurting judgment quality.
- Low-confidence LLM verdicts are treated as definitive Pass/Fail.
- Holo3 is a vision-language model, but we send text only — visual criteria
  (contrast, color, layout, images) get no screenshot.

**Goal:** a trustworthy, accurate, rate-limit-respecting, multimodal Holo3
integration, with schema-validated output and explicit human-review flags.

## Decisions (from brainstorming)

| Topic | Decision |
|-------|----------|
| Scope | Comprehensive (reliability + accuracy + multimodal) |
| Model | Both supported; default `holo3-1-35b-a3b`, opt-in `holo3-122b-a10b` |
| Screenshots | Yes — multimodal for visual criteria |
| Architecture | A — extend `HoloClient` in place + thin `ratelimit` module |
| Low-confidence | New `CriterionStatus::NeedsReview` |

## Architecture Overview

```
pipeline::run_batch
  └─ ObscuraBridge (CDP)
       ├─ run_axe / run_gap_fix        (Déterministe criteria)
       └─ screenshot(url)  [NEW]       → base64 PNG
  └─ HoloClient  [REWRITTEN]
       ├─ structured_outputs schema    (schema-validated JSON)
       ├─ RateLimiter (Arc)            (model RPM: 35b=10/min, 122b=config)
       ├─ model selection             (HOLO3_MODEL env / with_model)
       ├─ multimodal content          (text + image_url)
       └─ auth via HAI_API_KEY         (no hardcoded default)
  └─ PromptBuilder  [ENRICHED]
       └─ embeds criterion title + wcag_refs + definition
```

## Phase 1 — Reliability

### 1.1 Structured outputs
Holo3 is OpenAI-compatible and supports a top-level `structured_outputs` body
field: `{"structured_outputs": {"json": <schema>}}` (passed via the request
body; the model returns schema-validated JSON in `content`).

- Define `STRUCTURED_SCHEMA` (JSON schema) for:
  `{ "verdict": enum["pass","fail","na"], "confidence": number 0..1, "justification": string }`.
- `ChatRequest` gains `structured_outputs: Option<serde_json::Value>`; primary
  parse path deserializes `content` straight into `HoloResponse`.
- Keep `extract_json` only as a defensive fallback (unit-tested) but it is no
  longer the main path.

### 1.2 Rate limiter (`rgaa-holo/src/ratelimit.rs`, NEW)
- Token-bucket `RateLimiter` enforcing a per-model RPM budget.
- Defaults: `holo3-1-35b-a3b` → 10 RPM (free tier); `holo3-122b-a10b` →
  configurable (default 60, overridable via `HOLO3_RPM`).
- `HoloClient` holds `Arc<RateLimiter>`; `evaluate` acquires a permit (await)
  before sending. This bounds real concurrency to the API budget.
- `pipeline.rs` removes `HOLO3_CONCURRENCY` / the static `Semaphore`; the limiter
  inside the client governs pacing. The `JoinSet` of 27 evaluations stays, but
  calls now self-throttle.

### 1.3 Auth cleanup
- `HoloClient::new` reads `HAI_API_KEY`, falling back to `HOLO3_API_KEY`.
- No hardcoded default. If absent, `Orchestrator::run_batch` returns a clear
  error at startup instead of using a fake key.
- `pipeline.rs` removes the inline `env::var(...).unwrap_or_else(|_| "hk-...")`.

### 1.4 `NeedsReview` status
- Add `NeedsReview` to `CriterionStatus` in `rgaa-core/src/types.rs`.
- Verdict mapping in `pipeline.rs`:
  - `confidence >= CONFIDENCE_THRESHOLD (0.6)` → `Pass`/`Fail` per verdict.
  - `confidence < 0.6` → `NeedsReview` (justification preserved).
- **Serialization impact:** `CriterionStatus` is `serde` string-derived; any
  storage column / API serializer must accept the new variant. The plan includes
  verifying `rgaa-storage` and `rgaa-api` handle it (add a column default or
  migration if needed).

## Phase 2 — Accuracy

### 2.1 Prompt enrichment
- Add a curated `descriptions` map (crate-local in `rgaa-holo`, or
  `rgaa-core`) for the 27 `IA_ASSISTE` criteria: `id -> short RGAA definition`.
- `PromptBuilder::build(criterion, context, visual: bool)` embeds:
  - "Critère à évaluer": `id`, `title`, `wcag_refs`.
  - The curated definition.
  - The page context (unchanged), plus a note to focus on the sub-criterion when
    applicable.
- Clean up / remove the hacky `get_base_criterion` integer-match (AGENTS.md
  flagged it); replace with the dotted-id grouping already implicit in the
  catalog.

### 2.2 Model configuration
- `HoloClient::with_model(model)` and `with_rpm(u32)`; env `HOLO3_MODEL`
  (default `holo3-1-35b-a3b`), `HOLO3_RPM` (override).
- `pipeline.rs` builds the client from env/config; RPM derived from model.

## Phase 3 — Multimodal (visual criteria)

### 3.1 Screenshot capture (`rgaa-obscura`)
- Add `ObscuraBridge::screenshot(url) -> Result<String, String>` using CDP
  `Page.captureScreenshot` (reuse the existing CDP session from `run_axe`).
  Returns base64 PNG.

### 3.2 Send image
- `HoloClient::evaluate` gains `image: Option<String>` (base64). When `Some`,
  build a multimodal `content` array:
  `[{type:"text",...}, {type:"image_url", image_url:{url:"data:image/png;base64,..."}}]`.
- `pipeline.rs` captures **one** screenshot per URL (before the Holo3 loop) and
  passes it for visual criteria (contrast/color/layout/image — a curated id
  set). Non-visual criteria stay text-only to save tokens.
- Privacy: Holo3 defaults to **zero data retention**; page content + screenshot
  are not persisted by H Company.

## Data Flow

1. `run_batch` starts Obscura CDP server; builds `HoloClient` (model+RPM+key).
2. For each URL: axe + gap-fix (Déterministe); `screenshot` once.
3. For each of 27 `IA_ASSISTE` criteria (concurrent `JoinSet`):
   - `PromptBuilder::build(criterion, context, is_visual)`,
   - `holo.evaluate(&prompt, image_if_visual)` — acquires rate-limit permit,
     sends `structured_outputs` request.
   - Map verdict + confidence → `Pass`/`Fail`/`Na`/`NeedsReview`.
4. Merge axe + gap-fix + holo + (Manuel→Na) + (Déterministe unflagged→Pass);
   compute compliance over all 106 criteria.

## Error Handling
- Missing API key → startup error (no fake default).
- Rate-limit (429): limiter already paces; on residual 429 keep bounded
  exponential backoff + jitter (existing behavior).
- Structured-output parse failure → fallback `extract_json`; if still failing →
  `NeedsReview` with justification "Holo3 response unparsable", not a silent
  Pass.
- Screenshot failure → evaluation proceeds text-only (warn), not a hard failure.

## Testing
- `rgaa-holo`:
  - Mock-server test returns schema-shaped JSON; assert parse succeeds via
    `structured_outputs`.
  - Rate-limiter test: fire >RPM requests in a short window, assert actual send
    rate ≤ RPM.
  - `evaluate` multimodal: assert request body contains `image_url` when image
    provided.
  - Confidence→`NeedsReview` mapping unit test.
- `rgaa-obscura`: `screenshot` returns non-empty base64 PNG (integration).
- `rgaa-orchestrator`: existing `obscura_audit` e2e still passes; assert
  `NeedsReview` appears for a low-confidence mock verdict.

## Risks / Out of Scope
- **`NeedsReview` touches `rgaa-core` + storage/API serialization** — must verify
  and migrate.
- Adding 27 criterion definitions is content work; kept to `IA_ASSISTE` only.
- Holo3 `122b` requires paid credits; default path stays on free `35b`.
- Not changing the axe/gap-fix Déterministe path beyond the already-done
  "default to Pass" fix.

## Acceptance Criteria
- `cargo test -p rgaa-holo`, `-p rgaa-obscura`, `-p rgaa-orchestrator` all pass.
- Output parsing no longer depends on regex/code-block extraction.
- Live Holo3 calls (35b) respect ≤10 req/min by construction.
- No hardcoded API key; missing key fails fast with a clear message.
- Low-confidence verdicts surface as `NeedsReview`, not blind Pass/Fail.
- Visual `IA_ASSISTE` criteria receive a screenshot; text-only path unchanged.
- Compliance rate computed over all 106 criteria with Déterministe defaults.
