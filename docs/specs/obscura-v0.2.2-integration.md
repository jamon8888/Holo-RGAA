# Spec — Obscura v0.2.2 integration for Holo-RGAA

> Status: draft for review · Proposed triage label: `ready-for-agent` (tracker not configured — run `/setup-matt-pocock-skills` to publish; this file is the local fallback).
> Source: wayfinder map `docs/wayfinder/map-obscura-v0.2.2.md` + research `obscura-delta-v020-v022.md` + `rgaa-obscura-integration.md`. Synthesized without interviews, per to-spec.

## Problem Statement

Holo-RGAA audits run against a pinned Obscura 0.2.0 browser substrate while upstream is at v0.2.2 (126 commits since v0.2.1, 248 since the pin). Form-heavy pages, single-page-app routes, iframe content, and hostile/deep pages exercise gaps the newer release closes (label-aware filling, navigation draining, real frame trees, worker survival, screenshot fidelity, SSRF hardening). Operators cannot tell which new behaviors are safe to adopt, which config flags changed meaning, or which binary variant is pinned and verified — so audits stay on stale rendering and flaky pre-scan flows.

## Solution

Upgrade the pinned browser substrate to v0.2.2 (default render variant) and adopt the capability slice with direct RGAA payoff — reliable form filling and label association, drained navigations, frame-aware and SPA-aware audits, faithful evidence screenshots, and explicit private-network/file-upload configuration — behind version verification, with everything else deferred. After this work, an audit run logs the exact browser version, behaves deterministically on forms/SPAs/iframes, and fails clearly (not silently) on file-upload and private-network targets.

Proposed test seam (confirm this matches expectations): the orchestrator batch-run entry point as the single primary seam — one end-to-end audit over representative pages covers binary, transport, checks, and evidence together. Bridge-level live tests are secondary, used only where new error modes or flags need isolated coverage.

## User Stories

1. As an audit operator, I want the audit run to log the exact browser version, so that results are attributable to a known substrate.
2. As an audit operator, I want the installer to pin one browser variant, so that installs are reproducible across machines.
3. As an audit operator, I want CI to verify the installed browser version, so that drift is caught before audits run.
4. As an accessibility auditor, I want form labels resolved per specification, so that label-association findings match what assistive technology sees.
5. As an accessibility auditor, I want text insertion to preserve newlines and quotes, so that fills exercise realistic content.
6. As an accessibility auditor, I want permission grants accepted during context setup, so that permission-gated flows can be audited.
7. As an accessibility auditor, I want cookie-banner and login pre-scan actions to settle reliably, so that audits start from the intended state.
8. As an accessibility auditor, I want submit-button clicks to issue their navigation before the tool replies, so that post-submit pages are actually audited.
9. As an accessibility auditor, I want single-page-app route changes to surface after clicks, so that client-side routers do not freeze the audit on the entry route.
10. As an accessibility auditor, I want iframe content traversed through the real frame tree, so that framed criteria are evaluated rather than skipped.
11. As an accessibility auditor, I want cross-frame message scoping honored, so that script-behavior findings reflect the hardened boundary.
12. As an accessibility auditor, I want one throwing third-party script to be logged without aborting the page, so that a single bad script does not void the whole audit.
13. As an accessibility auditor, I want deeply nested and hostile pages to complete or degrade gracefully, so that crawls survive real-world sites.
14. As an accessibility auditor, I want filled input values visible in screenshots, so that evidence shows what was actually tested.
15. As an accessibility auditor, I want text controls laid out as real control boxes, so that visibility and geometry checks are meaningful.
16. As an accessibility auditor, I want fixed headers and modals rendered against the correct containing block, so that layout findings match user experience.
17. As an accessibility auditor, I want geometry queries to return standard integer coordinates or a real error, so that target-size and overlap checks are trustworthy.
18. As an accessibility auditor, I want page errors reported through the standard exception channel, so that script failures are diagnosable.
19. As an accessibility auditor, I want evaluation state to survive tab switches, so that multi-page flows keep their context.
20. As an accessibility auditor, I want keyboard traversal to use injection-safe key simulation, so that focus-order tests are not corrupted by quoting bugs.
21. As an accessibility auditor, I want form-data construction from a form element to work, so that form-semantics checks reflect the platform.
22. As an accessibility auditor, I want locale-sensitive formatting pinned to the declared language, so that language findings are consistent.
23. As a security reviewer, I want private and special-purpose network ranges denied by default with an explicit opt-in, so that intranet targets are audited deliberately.
24. As a security reviewer, I want file-upload automation to require an explicit access flag, so that uploads fail clearly instead of silently.
25. As a security reviewer, I want profile and platform overrides embedded safely, so that configuration cannot inject scripts into the page.
26. As a crawl operator, I want cookie state cleared and canonicalized between pages, so that audits do not leak state across targets.
27. As a crawl operator, I want robots handling enforced, so that crawls respect site policy.
28. As a pipeline maintainer, I want the environment-variable binary override actually honored by every entry point, so that custom installs work uniformly.
29. As a pipeline maintainer, I want the legacy-vs-structured analyze path decision recorded, so that future work converges on one pipeline.
30. As a documentation reader, I want cookie ordering, configuration keys, and CLI flags documented as implemented, so that setup instructions work as written.

## Implementation Decisions

- Adopt the default render-variant browser binary at v0.2.2 as the single pinned substrate; stealth and no-render variants stay unpinned and out of the install path.
- Record the browser version (binary self-report, mirrored in the automation server identity) on every audit run as evidence metadata.
- Honor the binary-path environment override uniformly across all launch entry points (orchestrator, automation server, CLI test flows); today some paths ignore it.
- Route label-aware filling (spec-compliant label association plus robust text insertion) through the pre-scan fill action; no new action type.
- Treat submit-triggered navigation as settled before a tool response returns (drain synthesized navigations; pump queued tasks between tool calls) so pre-scan and guided flows observe post-navigation state.
- Evaluate framed content through the real frame tree with per-frame script realms; same-origin frame access and scoped messaging are assumed platform behavior, not reimplemented.
- Keep the single-page-application route reporting provided by the substrate; no router-specific adapters.
- Consume standard page-error reporting (exception details channel) and session-owned execution contexts; multi-tab flows retain their evaluation state.
- Consume standard geometry responses (integer coordinates; resolution failure is a real error, not a placeholder) and update failure handling to branch on it.
- Consume faithful rendering for evidence (painted input values, real control boxes, correct fixed/absolute placement); evidence capture logic itself is unchanged.
- Enforce secure defaults: deny private and special-purpose network ranges unless an explicit private-network flag is set; require an explicit file-access flag for upload automation; surface both flags as validated configuration with clear denial errors.
- Resolve the dead element-internals knob: either wire it into the automation configuration or delete it — no third state.
- Resolve the legacy-vs-structured single-page analysis path: keep the legacy composed path for this spec; record the convergence decision for later work rather than migrating now.
- Keep the HTTP crawler module and the browser substrate's generated client of the same name explicitly distinct; no dependency or naming change in this spec.
- Correct user-facing documentation (cookie injection order, configuration keys, CLI flags) to match implemented behavior as part of the upgrade.

## Testing Decisions

- A good test exercises externally observable audit behavior (findings, evidence, errors, version metadata) through the highest seam, not transport internals, message IDs, or polling loops.
- Primary seam: the orchestrator batch-run entry point over a small representative page set (form with labels, SPA route change, framed content, submit navigation, hostile/deep page) asserting findings, evidence presence, and version attribution.
- Secondary seams (only where behavior differs in isolation): bridge-level live tests for the two explicit flags (private-network denial vs opt-in; upload denial vs flag), geometry-failure error codes, and version reporting; contract tests for configuration validation and tool schemas.
- Prior art: orchestrator end-to-end tests gated on an explicit opt-in environment flag; bridge integration tests that start a live server and assert structured results; automation-server contract tests asserting exact tool names and typed schemas; CLI contract tests for user-facing commands.

## Out of Scope

- Migrating to generated typed CDP clients; binary download-body auditing; blob-URL fingerprint behavior; live session watcher tooling.
- Stealth-variant rollout, GPU-rendering claims, screencast and PDF-export paths.
- Convergence onto the structured single-page analysis path (decided later, not executed here).
- aarch64 packaging gap beyond documenting it; multi-platform matrix support.
- New RGAA criteria, scoring changes, or remediation workflow changes.

## Further Notes

- Upstream refs: v0.2.2 release notes (generated clients, form filling, bad-script isolation, worker survival, binary bodies, exception details, session contexts, box-model quads, SSRF deny-set, navigation drain, version self-report); v0.2.1 notes (frame realms, task-queue pumping, SPA nav, insertText literal, cookie clearing, promise-awaiting eval, binding validation, file-access gate, PBKDF2/DOM bounds, textarea interface, robots enforcement).
- Breaking changes handled by this spec: upload automation now requires the file-access flag; private targets require the private-network flag — both must fail with actionable errors, never silently.
- Wayfinder residue: closing the mapping ticket (Which v0.2.2 capabilities map to RGAA gaps?), the upgrade-path ticket, and the final adopt-vs-defer ticket is superseded by this spec if accepted; the two research tickets remain the evidence base.
