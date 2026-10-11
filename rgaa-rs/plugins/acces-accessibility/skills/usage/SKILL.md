---
name: usage
description: This skill should be used whenever the rgaa-mcp tools (analyze, remediate, source_map, verify_fix, igt, lint_static, audit_url, get_audit_result, list_criteria) are about to be called and the task involves accessibility, RGAA, a11y, WCAG conformance, keyboard navigation, accessible names and roles, or fixing accessibility issues in web UI. It teaches the tool chain, the field mappings between the tools, the trap fields that fail silently, and which calls cost tokens. Trigger phrases include "check accessibility", "scan for a11y issues", "fix accessibility violations", "make this accessible", "RGAA audit", "test keyboard accessibility", "lint this template for a11y". It does NOT answer what an RGAA criterion requires — that is the rgaa-accessibility plugin's criteria skill.
---

# usage — the rgaa-mcp tool contract

## When this applies

Read this before the **first** call to any `rgaa-mcp` tool. Most of the rules
below cannot be discovered by trying: the failures are silent, returning a
plausible wrong answer rather than an error.

This skill is about the **instruments**. If the question is what a criterion
requires, how it is tested, or its WCAG 2.2 / EN 301 549 cross-references, that
is domain knowledge and belongs to the `rgaa-accessibility` plugin's `criteria`
skill, if installed. The boundary:

> The domain layer says what a criterion requires. The tooling layer says
> whether our instruments can decide it, and what their silence proves.

## The nine tools

| Tool | Browser | LLM | What it is for |
|---|---|---|---|
| `list_criteria` | no | no | the 106-criteria catalogue; takes **no parameters** |
| `lint_static` | no | no | source linting, sub-millisecond per file; a first pass, **not** a verdict |
| `analyze` | **yes** | no | one page, per-criterion findings with evidence |
| `source_map` | no | no | map findings onto source locations |
| `remediate` | no | no | approval-gated patch proposals, 1–25 issues per call |
| `verify_fix` | **yes** | no | re-analyse corrected pages and categorise the diff |
| `igt` | **yes** | no | guided tests |
| `audit_url` | **yes** | **yes** | crawl + full 106-criteria audit |
| `get_audit_result` | no | no | projection of a stored audit — **inert**, see below |

**Only `audit_url` spends tokens.** Everything else is free. See
[cost-control.md](references/cost-control.md).

All wire fields are `snake_case`. The one exception: `classification` values are
capitalised — `"Deterministe"`, `"IaAssiste"`, `"Manuel"`. Do not lowercase them.

## The chain

```
audit_url ──(sampled_page_urls)──▶ analyze ──▶ source_map ──▶ remediate ──▶ verify_fix
                 (discovery)        (depth)     (mandatory)    (patches)    (confirm)
```

`analyze` and `audit_url` are **different scopes, not alternatives**:
`audit_url` is discovery across a crawl and returns summary scalars;
`analyze` is depth on one page and returns findings you can act on.

**`source_map` is not optional.** Going straight from `analyze` to `remediate`
documents a path that does not work — see trap 1 in
[field-mapping.md](references/field-mapping.md).

## The five rules that matter most

Full detail, with every field, in [field-mapping.md](references/field-mapping.md).
These are the ones that bite first:

1. **`remediate`'s `element_html` wants framework *source*, not rendered DOM.**
   Passing `analyze`'s `html` fails every time. Plain HTML and PHP templates
   cannot be remediated at all — there is no `Html` framework variant. Fix those
   by hand, and **say which path you took**: a policy-checked patch and your own
   edit do not carry the same guarantee.
2. **`analyze`'s response shape switches on `config.igt_tools`.** Set it, and
   findings move from top-level `findings` to `data.axe`. Read `findings` then
   and you get nothing, with no error.
3. **Match findings across rounds on `rule` + `target`, not on the fingerprint
   and not on `id`.** `id` is positional per rule within a scan, and the
   fingerprint includes a scan-level DOM hash that moves whenever the page
   changes — so `verify_fix`'s own categories are unreliable after an edit.
   `target` is the stable identifier. See the fingerprint defect in
   [field-mapping](references/field-mapping.md).
4. **`remediate` correlates on `issue_id`, never on array position** — and the
   server *verifies* it, failing the whole call on mismatch. Invent stable ids.
5. **`criterion_id` is often `null`.** axe-core finds real violations mapped to
   no RGAA criterion (`rgaa-unmapped-*` ids) — 8 of 20 on a live
   WordPress/Elementor homepage. Report them separately; they cannot count for
   or against conformity, and `null` must not go into `remediate`'s `criteria[]`.
6. **A clean result is not RGAA conformity.** 40 of the 106 criteria are
   structurally beyond these tools: 32 need the LLM (`audit_url` only) and 8
   need a person. Report that residue every time.

## What silence proves

This is the question the tooling layer exists to answer, and it differs per
criterion. By primary engine, across the 106:

| Engine | Criteria | A missing finding means |
|---|---|---|
| `AxeCore` | 13 | depends on rule coverage — check the coverage map |
| `Deterministic` | 53 | depends on rule coverage — check the coverage map |
| `Holo` | 32 | **nothing** — these tools never evaluated it |
| `Human` | 8 | **nothing** — needs a person |

Only 42 of the 106 carry even one axe rule, and the usable figure is sharper
still: a clean result lets you conclude **conforming for 32 criteria**. Another
**34** are partially covered — these tools find violations there, but silence
proves **nothing**, because a residual judgement remains. With the 40 above,
that is **74 of 106 that cannot be called conforming** on a clean run.

Never flatten "we found nothing" into "it conforms". Per-criterion detail is in
the generated [coverage map](references/coverage-map.md) — consult it rather
than guessing, and never assert conformity for a criterion you have not checked
there.

## Before you report

Three categories, never merged:

- **fixed** — a finding that is gone, confirmed by `verify_fix`
- **refused** — a patch the human declined. Not the same as undetected, and it
  must not be silently dropped or re-proposed.
- **out of reach** — the residue above

And if you ran anything that cost tokens, say what it cost.

## References

- [field-mapping.md](references/field-mapping.md) — every field, every trap
- [cost-control.md](references/cost-control.md) — what spends, and what is only slow
