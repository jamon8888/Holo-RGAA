---
name: seo-geo-aeo
description: "Combined SEO/GEO/AEO + RGAA audit with criticality-ordered remediation; RGAA pass is mandatory"
version: 0.1.0
author: RGAA Team
requires:
  - audit
  - triage
  - remediate
  - rgaa-mcp
mode-default: suggest
---

# seo-geo-aeo — Combined SEO/GEO/AEO + RGAA Audit & Remediation

## Overview
This skill runs a single-crawl audit that evaluates a page against RGAA 4.1.2 **and** technical SEO, Generative Engine Optimization (GEO), and Answer Engine Optimization (AEO) rules, then produces a unified remediation plan ordered by criticality. The RGAA pass is mandatory: the skill never emits an SEO-only result. Both passes consume the same crawl artifact — no second fetch per page.

SEO/GEO/AEO rules come from the `seo` module of `rgaa-rules` (see `docs/specs/seo-geo-aeo-rgaa-merge.md`): meta tags, JSON-LD/schema.org validity, heading structure, canonical/hreflang, and GMB NAP (Name/Address/Phone) consistency.

## Triggers
- User requests "audit SEO and accessibility", "SEO/GEO/AEO audit", "site health audit", "check schema and RGAA"
- User provides a URL, a URL profile, or a local project path plus (optionally) a business profile for NAP checks

## Inputs
- `url` (string, optional) — Target URL. Falls back to `profile`.
- `profile` (string, optional) — URL profile name from `.rgaa/config.yaml`.
- `business_profile` (path, optional) — JSON/YAML fixture with `name`, `address`, `phone` for GMB NAP consistency. NAP rules are skipped (reported as `na`) when absent.
- `min_criticality` (string, optional, default: `P2`) — Lowest criticality included in the remediation plan: `P0`, `P1`, `P2`, `P3`.
- `framework` (string, optional) — Passed through to `remediate`.
- `format` (string, optional) — `json`, `markdown`, `sarif`.
- `output` (path, optional) — Write bundle to file.

## Workflow
1. **Resolve target** — Same resolution as `audit` (URL or profile from `.rgaa/config.yaml`).
2. **Mandatory RGAA pass** — Invoke `audit`. If the resulting `AuditBundle` is incomplete (missing evidence, rejected run, browser failure), **stop**: exit code 3, typed error `EXECUTION_FAILED`. Do not proceed to SEO with a missing or partial RGAA bundle.
3. **SEO/GEO/AEO pass** — Call `rgaa-mcp` tool `analyze` with `rules: seo` against the crawl artifact already captured in step 2. Rule families: `meta`, `schema_org`, `headings`, `canonical`, `nap`.
4. **Merge & dedupe** — Findings from both passes share the fingerprint scheme (`rgaa-fp-v1-<hex>`). A finding present in both (e.g. heading order, missing alt text, missing `lang`/`<title>`) is emitted once, tagged with both origins (`rgaa`, `seo`), and takes the **highest** criticality of the two.
5. **Assign criticality** — Apply the ladder below to every finding.
6. **Triage** — Invoke `triage` on the merged bundle. RGAA classification (Deterministe / IaAssiste / Manuel) is preserved and drives ordering within a criticality tier.
7. **Remediate by criticality** — Invoke `remediate` tier by tier, P0 first, stopping at `min_criticality`. Within a tier: deterministic → ia_assiste → manual. Never start a lower tier while a higher tier still has unreviewed proposals.
8. **Output** — Unified `SiteHealthBundle`: RGAA compliance rate, SEO/GEO/AEO score, findings grouped by criticality then origin, and the ordered `RemediationPlan`.

## Criticality Ladder

| Tier | RGAA (mandatory, never downgraded) | SEO/GEO/AEO |
|------|-----------------------------------|-------------|
| **P0 — Bloquant** | Critical: `Fail` on a blocking criterion (keyboard trap, missing form labels, missing page language, non-accessible CAPTCHA…) | Indexation-breaking: unintended `noindex`, canonical pointing to another origin or a 4xx/5xx, JSON-LD with invalid syntax that disables all rich results |
| **P1 — Majeur** | Major: `Fail` on a non-blocking criterion | Missing `<title>`, missing meta description, missing or multiple `<h1>`, schema.org block missing required properties for its `@type`, NAP mismatch against `business_profile` |
| **P2 — Mineur** | Minor: `NeedsReview` | Heading-level skips, meta title/description outside recommended length, missing `hreflang` on multilingual pages, `Organization`/`LocalBusiness` schema present but incomplete optional fields |
| **P3 — Info** | Info: `NotApplicable` | GEO/AEO opportunities: no `FAQPage`/`HowTo`/`Article` schema where content qualifies, no short answer-block near the `<h1>`, no `speakable` markup |

Rules:
- An RGAA `Fail` is **never** downgraded below P1, regardless of SEO impact — accessibility is a legal obligation, SEO is not.
- A shared finding (both origins) takes the higher tier.
- `business_profile` absent → NAP rules report `na`, not `fail`.

## Outputs
- `SiteHealthBundle` (schema version "1.0"):
  - `rgaa`: full `AuditBundle` from the mandatory pass (unchanged).
  - `seo`: SEO/GEO/AEO findings with evidence references.
  - `merged_findings`: deduplicated list with `criticality`, `origins`, `classification`, fingerprint.
  - `summary`: RGAA compliance rate (`passed / (passed + failed) * 100`), counts per criticality tier, counts per origin.
  - `remediation_plan`: `RemediationPlan` from `remediate`, ordered P0 → P3.
- Human-readable summary on stdout unless a machine format is requested.

## Constraints
- RGAA pass MUST run and MUST be complete before any SEO/GEO/AEO finding is reported.
- Both passes MUST consume the same crawl artifact; a second crawl of the same URL in one run is a bug.
- Every finding MUST have a stable fingerprint (`rgaa-fp-v1-<hex>`) and at least one evidence reference.
- Remediation proposals inherit all `remediate` approval gates; nothing is auto-applied.
- Generated content (meta descriptions, GMB copy) MUST be emitted as remediation proposals, never written to a page or listing directly.
- Backend for generative steps (local Ollama vs. remote Holo3) is explicit configuration; the skill MUST surface which one produced each proposal.

## Failure Modes
- Invalid URL / profile → exit code 2, `INVALID_INPUT`.
- RGAA bundle incomplete or rejected → exit code 3, `EXECUTION_FAILED`, no SEO output.
- `business_profile` unreadable → exit code 2, `INVALID_INPUT`.
- `min_criticality` not in `P0..P3` → exit code 2, `INVALID_INPUT`.
- Remediation approval missing → exit code 2, `POLICY_DENIED` (from `remediate`).

## Example
```
> seo-geo-aeo --url https://example.test --business-profile .rgaa/business.yaml --min-criticality P1 --format markdown
```
