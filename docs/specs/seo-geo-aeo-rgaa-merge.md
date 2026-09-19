# Spec — Merge SEO/GEO/AEO audit into Holo-RGAA, backed by a local LLM

> Status: draft for review · Proposed triage label: `needs-triage` (tracker: GitHub Issues, jamon8888/Holo-RGAA — see `docs/agents/issue-tracker.md`).
> Source: conversation synthesis (no wayfinder map or interviews yet) — flags itself for a human pass on crate boundaries and model choice before `ready-for-agent`.

## Problem Statement

Holo-RGAA crawls a page once (via `rgaa-spider` / `rgaa-browser-tools`) and evaluates it only against RGAA 4.1.2. That crawl already yields the DOM, heading structure, alt text, `lang`/`<title>`, and landmark data that SEO, GMB (Google My Business) listing quality, and generative-answer-engine optimization (GEO/AEO) audits are built from — but nothing in the workspace evaluates it for those. An operator who wants a combined accessibility + technical-SEO + structured-data report has no in-repo capability and no shared remediation path: an RGAA heading-order fix and an SEO heading-order fix would today have to be two disconnected efforts even though they're the same finding.

Separately, every generative judgment in the platform — including the RGAA "needs-review" verdict path — routes through `rgaa-holo`'s `HoloClient`, which is hardwired to the remote `api.hcompany.ai` endpoint and a fixed model (`crates/rgaa-holo/src/client.rs:8-9`). There is no local/offline path, which blocks running SEO copy generation or GMB fiche drafting on a machine with no reliable outbound API budget or where the operator wants generated content to never leave the box.

## Solution

Extend the existing pipeline rather than building a second one:

1. Add a `seo` module inside `rgaa-rules` — deterministic rule-based checks in the same shape as the existing axe-core→RGAA mapping (`IndexMap<String, CriterionResult>`-style output, no LLM involved) — that evaluates the *same* crawl artifact the RGAA pass consumes: meta tags, JSON-LD/schema.org validity, heading structure, canonical/hreflang presence, and GMB NAP (Name/Address/Phone) consistency checks against a supplied business-profile fixture.
2. Feed the `seo` module's findings into the existing `rgaa-remediation` proposal/adapter/policy/lifecycle machinery (`crates/rgaa-remediation/src/{proposals,adapters,policy,lifecycle}.rs`) instead of a parallel remediation concept, so dedup, baselining, and lifecycle tracking are shared.
3. Add an SEO/GEO/AEO stage to `rgaa-orchestrator`'s existing `pipeline.rs`, run alongside (not instead of) the RGAA stage per crawl.
4. Introduce an `LlmBackend` trait in `rgaa-holo`, with the current Holo3 client becoming one implementation and a new Ollama-backed `LocalBackend` becoming a second, so the generative-judgment slice (RGAA "needs-review" verdicts, SEO meta/GMB copy drafting) can run entirely locally. Sized for the actual deployment machine: CPU-only inference (i7-8750H, 32GB RAM, MX150 2GB VRAM unusable — no working NVIDIA driver), so the default local model targets are quantized 7B-class for interactive use and 14B-class for scheduled batch, not anything requiring GPU offload.
5. Add a recurring stage inside `rgaa-orchestrator` (in-process scheduler, not an external workflow tool) for nightly SEO/GMB re-audits and batch content regeneration, queued through the same remediation lifecycle as on-demand runs.
6. Add two Claude Code skills, `seo-geo-aeo-audit` and `seo-geo-aeo-remediate`, alongside the existing `claude-plugin/skills/{audit,remediate}`, following their exact SKILL.md shape and reusing `rgaa-mcp` as the tool surface.

## User Stories

1. As an accessibility auditor, I want the SEO/GEO/AEO pass to run against the same crawl the RGAA pass already captured, so that a site is never crawled twice for one audit run.
2. As an SEO auditor, I want heading-structure, meta-tag, and canonical-tag findings reported with the same fingerprint/evidence shape as RGAA findings, so that the audit bundle format stays uniform.
3. As an SEO auditor, I want JSON-LD/schema.org blocks validated against the declared `@type` before being surfaced as passing, so that a syntactically-broken schema isn't reported as compliant.
4. As a GMB operator, I want NAP (name/address/phone) fields on the page checked for consistency against a supplied business-profile fixture, so that listing mismatches are caught before they hurt local search ranking.
5. As a remediation reviewer, I want SEO/GMB findings to flow through the same proposal/dedup/lifecycle pipeline as RGAA findings, so that I review one queue, not two.
6. As a platform operator, I want an `LlmBackend` trait so the generative-judgment call site in `rgaa-holo` doesn't change when the backend does, so that the RGAA "needs-review" flow keeps working unmodified.
7. As a platform operator, I want a local Ollama-backed `LlmBackend` implementation, so that RGAA verdicts and SEO/GMB copy can be generated without an outbound API call or API key.
8. As a platform operator, I want the local backend's model choice documented against the deployment machine's actual CPU/RAM/GPU profile, so that the default doesn't silently thrash a low-VRAM or GPU-less box.
9. As a content operator, I want batch GMB/meta-description regeneration to run as a scheduled job inside the orchestrator, so that I don't need a separate workflow engine for a recurring task the platform already knows how to sequence.
10. As a content operator, I want scheduled regeneration output to land as remediation proposals (not auto-applied changes), so that generated copy is reviewed before publishing.
11. As a security reviewer, I want the local-backend path to make no outbound network call for the content it generates, so that using it is a meaningful privacy boundary versus the remote Holo3 path.
12. As a security reviewer, I want the choice of backend (remote Holo3 vs. local) to be explicit configuration, not an implicit fallback, so that operators know which one handled a given verdict.
13. As a pipeline maintainer, I want the SEO/GEO/AEO stage to be independently disableable from the RGAA stage in `pipeline.rs`, so that an RGAA-only deployment isn't forced to carry SEO evaluation.
14. As a Claude Code user, I want a `seo-geo-aeo-audit` skill with the same frontmatter/workflow/constraints shape as the existing `audit` skill, so that triggering it feels identical to what's already in the plugin.
15. As a documentation reader, I want the RGAA/SEO criterion overlap (headings, alt text, landmarks, `lang`/title) documented explicitly, so that a shared finding isn't misread as a duplicate bug in one system or the other.

## Implementation Decisions

- SEO/GEO/AEO rules live in a `seo` module of `rgaa-rules` (`crates/rgaa-rules/src/seo/`), not a separate crate: they consume the same crawl artifact and produce the same result type as the axe-core→RGAA mapping, so sharing the crate avoids duplicating the input/output types and keeps one dependency for the orchestrator. The module has its own submodules per rule family (`meta`, `schema_org`, `headings`, `canonical`, `nap`) so the vocabulary stays separable without a crate boundary.
- The `seo` module never crawls on its own; it consumes the same DOM/crawl artifact type `rgaa-rules` already consumes from `rgaa-spider`/`rgaa-browser-tools`. No second network fetch per audited page.
- The `seo` module is behind a `seo` Cargo feature on `rgaa-rules` (enabled by default) so an RGAA-only build can drop it and its schema.org validation dependencies.
- SEO/GMB findings reuse `rgaa-remediation`'s existing `proposals.rs`/`adapters.rs`/`policy.rs`/`lifecycle.rs` types; if a finding needs a shape the current types don't support, extend those types rather than adding a parallel remediation module.
- `LlmBackend` is a trait in `rgaa-holo` with the request/response shape `HoloClient` already uses (`ChatMessage`/`ChatRequest`, verdict/confidence/justification JSON) as its contract, since that shape is already OpenAI-wire-compatible and Ollama's `/v1/chat/completions` speaks the same wire format — the existing Holo3 client becomes the first trait implementation, not a rewrite.
- `LocalBackend` targets Ollama's OpenAI-compatible endpoint (`http://localhost:11434/v1` by default, configurable) and requires no API key.
- Default local models are chosen for CPU-only inference on the reference deployment machine (12 logical cores, 32GB RAM, no working GPU acceleration): a quantized ~7B-class instruct model for interactive/on-demand calls, a quantized ~14B-class instruct model for the scheduled batch path where latency matters less than output quality. Model names are configuration, not hardcoded — the spec fixes the *tier*, not the exact model, since Ollama's catalog moves.
- Backend selection (remote Holo3 vs. local) is explicit per-call or per-deployment configuration; there is no silent fallback from one to the other.
- The scheduled regeneration stage lives in `rgaa-orchestrator` using an in-process scheduler (e.g. a cron-style crate already in the Rust ecosystem), not an external workflow tool — keeps deployment to one binary/workspace.
- Scheduled-job output is always written as remediation proposals through the existing lifecycle states; nothing the scheduler produces is auto-applied to a live GMB listing or page.
- New skills (`seo-geo-aeo-audit`, `seo-geo-aeo-remediate`) copy the frontmatter and section structure of `claude-plugin/skills/audit/SKILL.md` and `claude-plugin/skills/remediate/SKILL.md` verbatim in shape (Overview/Triggers/Workflow/Inputs/Outputs/Constraints/Failure Modes/Example), reusing `rgaa-mcp` rather than inventing a new tool surface.
- Document the RGAA/SEO criterion overlap table (headings, alt text, landmarks, `lang`/title, JSON-LD) in `docs/` so a shared underlying signal isn't triaged twice as unrelated bugs.

## Testing Decisions

- Primary seam: the orchestrator batch-run entry point, same seam the Obscura v0.2.2 spec (`docs/specs/obscura-v0.2.2-integration.md`) established — one end-to-end run over representative fixture pages (missing meta description, broken JSON-LD, correct JSON-LD, heading-order violation, NAP mismatch against a fixture business profile) asserting the combined RGAA + SEO finding set, fingerprints, and evidence.
- `rgaa_rules::seo` rule functions get direct unit tests against fixture HTML/JSON-LD snippets (valid schema, invalid `@type`, missing required schema.org fields) — no browser or LLM involved, matching how `AxeMapper::map()` gaps are noted as untested today in `CLAUDE.md`'s testing-gaps table and should not be repeated here.
- `LlmBackend` gets a contract test suite run against both implementations: a mocked HTTP server standing in for Holo3 (existing pattern — `HoloClient`'s `base_url` override already supports this) and a mocked HTTP server standing in for Ollama's `/v1/chat/completions`. Both must produce the same verdict/confidence/justification parse behavior from the same wire response.
- Live-Ollama integration tests are gated behind an explicit opt-in environment flag (mirroring the orchestrator's existing end-to-end opt-in pattern) and skipped by default in CI, since CI runners won't have a local model pulled.
- Scheduled-job tests assert that a triggered run enqueues remediation proposals in the expected lifecycle state — not that it produces any specific generated text (content quality isn't a correctness assertion here).

## Out of Scope

- Actually writing back to the Google Business Profile API (reading/pushing GMB listing changes) — this spec only produces reviewable proposals; the write-back integration is separate future work.
- Tuning or evaluating generated-content quality (prompt engineering for SEO copy/GMB text) — covered by a later spec once the backend plumbing exists.
- Migrating `rgaa-holo`'s existing RGAA verdict flow off the remote Holo3 backend by default — this spec adds the local option, it does not change the default.
- Supporting local-LLM backends other than Ollama (e.g. llama.cpp server, vLLM) — Ollama only, for this spec.
- A report/UI surface for the combined RGAA+SEO bundle — output shape only, no new rendering.
- Any change to the existing axe-core→RGAA mapping in `rgaa-rules` or to the RGAA criteria catalog in `rgaa-core`; the `seo` module is additive.
- GPU-accelerated local inference — the reference machine has no usable GPU; this spec is CPU-only by necessity, not preference.

## Further Notes

- Hardware basis for the local-model tiering: reference deployment machine profiled at i7-8750H (12 threads), 32GB RAM, NVIDIA MX150 with no loaded driver (`nvidia-smi` fails) — treated as CPU-only for planning purposes.
- `HoloClient`'s current wire format (`crates/rgaa-holo/src/client.rs`) is already OpenAI-chat-compatible, which is why `LocalBackend` is scoped as a second trait implementation rather than a parallel client design.
- Existing crate inventory this spec builds on (workspace has grown past what `CLAUDE.md` documents): `rgaa-core`, `rgaa-rules`, `rgaa-holo`, `rgaa-remediation`, `rgaa-orchestrator`, `rgaa-spider`, `rgaa-browser-tools`, `rgaa-mcp`, `rgaa-mcp-http`, `rgaa-agent`, `rgaa-api`, `rgaa-cli`, `rgaa-tui`, `rgaa-data`, `rgaa-storage`, `rgaa-obscura`, `rgaa-test-corpus`.
- Crate-boundary decision already taken: SEO rules are a module of `rgaa-rules`, not a new crate (decided 2026-09-19 during spec review).
- No wayfinder map or ticket precedes this spec (unlike the Obscura spec); recommend a human pass on the exact default model names before moving this to `ready-for-agent`.
