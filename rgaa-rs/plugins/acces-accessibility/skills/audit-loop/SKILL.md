---
name: audit-loop
description: This skill should be used when the user asks to "audit and fix accessibility", "fix all a11y issues on this page", "run the accessibility loop", "remediate accessibility until clean", "scan and fix localhost", "make this page accessible", or runs /acces-accessibility:audit-loop. It drives the rgaa-mcp analyze -> source_map -> remediate -> approve -> apply -> verify loop on a URL, pausing for human approval of every patch each round, until no findings remain, the findings stop changing, or a round cap is reached.
argument-hint: "<url> [max-rounds] (default url: detected localhost, default rounds: 5)"
allowed-tools: Read, Edit, Write, Glob, Grep, AskUserQuestion
---

# audit-loop — approval-gated remediation

Read [usage](../usage/SKILL.md) and its
[field-mapping](../usage/references/field-mapping.md) before the first call.
The field traps there are what makes this loop work or silently fail.

## The round

```
analyze → source_map → remediate     → show patches → human approves → apply → verify_fix
                     └─ or write the fix yourself (no framework detected)
```

Every round costs **zero tokens**: nothing in this loop calls an LLM. Iterate as
freely as the work needs.

### 1. `analyze`

One page. Do **not** set `config.igt_tools` unless you want guided tests — it
silently moves findings from `findings` to `data.axe`.

Keep the raw finding objects. You will need them **byte-exact** for
`verify_fix`, and for comparing rounds.

### 2. `source_map`

Mandatory. `remediate` rejects an issue whose `source_locations` is empty, so
skipping this step fails every issue.

### 3. Patch each finding — by one of two paths, and say which

- **A framework was detected** (React, Next, Vue, Angular): call `remediate`.
  Batch up to 25 issues, correlate on `issue_id`. You get a policy-checked,
  templated patch and an `approval_token`.
- **No framework** — plain HTML, PHP templates, anything else: **write the fix
  yourself.** `remediate` cannot touch it; there is no HTML framework variant.

**Always state which path each fix came from.** A policy-checked patch and your
own edit do not carry the same guarantee, and a report that blurs them
overstates its own reliability.

### 4. One approval batch per round

Present **all** of the round's patches together. The human accepts or rejects
each one individually. Then apply only the accepted set.

Show `remediate`'s `approval_token` alongside each patch. It re-derives from the
proposal's identity, so it is how you — or anyone reading the record later —
can prove the patch applied is the patch approved.

Never apply a patch that has not been accepted in this round. There is no
auto-apply mode, by design.

**A rejected patch leaves the loop.** Do not re-propose it next round, or you
will offer the human the same declined patch indefinitely. Carry it to the final
report as **refused** — which is a different fact from *undetected*, and must
not be collapsed into it.

### 5. Verify — and do the comparison yourself

**`verify_fix`'s categories are currently unreliable. Do not report from them.**
A live run established that `analyze` emits one DOM-snapshot evidence hash per
*scan*, shared by every finding, and that hash is part of the fingerprint
`compare` uses. So once you have edited the page — the only reason to verify —
every reference finding reads as *fixed* and every current finding as *new*.
Full evidence in [field-mapping](../usage/references/field-mapping.md).

**Compare rounds yourself on `rule` + `target` + `criterion_id`.** That triple
was verified unique on a live page — 24 findings gave 24 distinct keys — and
`criterion_id` is the part that earns its place: one axe violation often maps to
two RGAA criteria, so the same element appears twice with the same `rule`,
`target` *and* `html`. Dropping `criterion_id` silently collapses them.

Call `verify_fix` for its re-analysis and its RGAA citations if useful, but
derive *fixed / remaining / new* from your own match.

**But a disappeared finding is not automatically a fixed one.** Two scans of an
*unedited* page differed by 6 findings, and **every one of them sat on a
positional or runtime-generated selector**. Restricted to stable selectors the
two runs were identical — 19 findings, same set.

So treat a finding as **unstable, not fixed**, when its `target` contains:

- a positional step — `:nth-child(…)`, `:nth-of-type(…)`
- a carousel, slider or other rotating container
- an obfuscated runtime class, e.g. `.go2617933356` — CSS-module or
  build-generated names that change between builds or renders

A carousel rotates between scans, so `:nth-child(1)` points at a different
element each time; such a finding vanishes and reappears with no edit at all.
Report these separately as **unstable** and do not count them toward
convergence. On the page tested this was 4 of 24 findings — not a corner case.

**Prefer a deterministic target.** The loop is reliable against a local dev
server or a static build, where the DOM does not move between scans. Against a
live, dynamic page, expect noise and say so.

Do not use finding `id` either: it is positional per rule within a scan, so the
same element was `rgaa-9.3-listitem-9` in one run and `rgaa-9.3-listitem-3` in
another. **`target` is the stable identifier.**

If you do build a reference bundle, three rules, each of which fails **silently**
when broken — full detail in
[field-mapping](../usage/references/field-mapping.md):

- findings go in top-level `findings`, **never** `pages[].findings`
- copy them **verbatim**; the comparison fingerprints on
  `rule`/`url`/`target`/`component_path`/`evidence[]`, not on `id`
- never include `status: "pass"` findings

Remember `files[].path` is echo-only: `verify_fix` re-analyses the **page**, not
your file.

## When to stop

Stop on the first of these:

1. **No findings remain.** Success — reported with the residue below.
2. **Fixpoint: the set of `rule` + `target` + `criterion_id` keys, excluding
   unstable selectors, did not change between two rounds.** Match on that
   triple, **not** on the fingerprint — the fingerprint moves on its own
   whenever the page changes (and on a dynamic page, between any two scans), so
   a fingerprint-based fixpoint would never trigger and the loop would always
   run to the cap. Excluding unstable selectors matters for the same reason: a
   rotating carousel would otherwise keep the set changing for ever.
3. **Round cap** — default 5, override with the second argument.

**The fixpoint matters more than the cap.** If a finding's `rule` + `target` +
`criterion_id` key is **still present** after you applied a patch for it, the fix
did not land. That is a failure, not slowness: report it as one, name the keys
that did not move, and stop. Spending four more rounds on patches that provably
do nothing wastes the human's attention, which is the one resource this loop
actually consumes.

Judge this on the key, **never on the fingerprint**. The fingerprint carries a
per-scan DOM hash, so it moves whenever the page changes — and on a dynamic page
between any two scans — which means "the fingerprint changed" tells you nothing
about whether a fix worked.

The check is free, but it is only precise if you kept `target` verbatim.
Restated selectors make every round look like progress.

## A clean loop is not conformity

These tools *reach* about 66 of the 106 RGAA criteria, but reaching is not
deciding. A clean loop lets you conclude **conforming for 32 criteria**. The
other 74 break down as:

- **40 never evaluated here** — 32 need the LLM, which only `audit_url` invokes,
  and 8 need a person.
- **34 partially covered** — violations are findable, but silence proves
  **nothing**: a residual judgement remains.

So a loop ending at zero findings is evidence about 32 criteria, not a claim
about 106. **Never report it as RGAA conformity.** This is a regulatory context;
the difference is not pedantry.

Consult the generated [coverage map](../usage/references/coverage-map.md) for
which criteria you actually decided and what each one's silence proves. Do not
guess, and do not round 32 up to 66.

## The report

Five categories, never merged:

| Category | Meaning |
|---|---|
| **fixed** | gone, confirmed by `verify_fix` |
| **refused** | the human declined the patch |
| **out of reach** | the 40, plus anything the coverage map shows as undecidable here |
| **outside the referential** | axe-core findings whose `criterion_id` is `null` — real defects, but not RGAA criteria. On a live WordPress/Elementor homepage this was 7 of 22 findings, so do not treat it as a corner case. |
| **unstable** | findings on a positional or runtime-generated selector, which move between scans with no edit. 4 of 24 on the page tested. Never report one as fixed. |

Also state: how many rounds ran and why it stopped; which fixes were
`remediate` patches and which you wrote yourself; and that no tokens were spent.

## Optionally, a real compliance figure

If the user wants a verdict across all 106 criteria, offer `audit_url` as a
**final, separate step** — and price it first: 22 LLM calls per page, and on a
crawl potentially many pages. Get their agreement before running it. Never fold
it into a round.

Better still, if the `rgaa-accessibility` plugin is installed, that is where the
106-criteria audit and the compliance reports belong.
