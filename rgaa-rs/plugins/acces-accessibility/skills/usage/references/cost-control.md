# Cost control

Our cost model is not Deque's, and transposing their advice gives wrong answers.
Deque prices **remediation**: one AI credit per issue, 25 issues costing 25× one.
We price **pages**, and remediation is free.

## Only `audit_url` spends tokens

`analyze`, `remediate`, `source_map`, `verify_fix`, `igt`, `lint_static`,
`list_criteria` and `get_audit_result` make **no LLM call at all**. The browser
path (`analyze`, `igt`, `verify_fix`) goes to the browser bridge; the rest is
local computation.

Two consequences worth acting on:

- **The remediation loop is free.** Iterate it as many rounds as the work needs.
  Only a fresh `audit_url` re-spends.
- **A 25-issue `remediate` batch costs the same as a 1-issue batch.** Under
  Deque that is 25 credits against 1. There is no reason to be sparing.

## What `audit_url` costs

**22 LLM calls per page.** The 106 criteria are batched 5 at a time, and the
pipeline makes one pass. Not one call per criterion, and not one per finding.

Per call, the page context is capped at **8,000 characters** (~2–2.5k input
tokens) and output at 4,096 tokens. The one variable input is prior axe findings
for the batch's criteria, so a page with many violations costs more than a clean
one — but only for the criteria those violations touch.

**Multi-page.** `audit_url` crawls. With the default `sample_mode`, the page set
is the RGAA sample probed by URL pattern — typically **6 pages or fewer, so ≤132
calls**. With `sample_mode` off it is a spider crawl capped at 50 pages, i.e.
**up to 1,100 calls**. Check which mode you are in before launching, and say the
expected call count to the user first.

One agent, and therefore one rate-limit budget, is shared across every page of a
site audit. The budget is global, not per page.

## A dead provider is cheap

There is **no retry** on the audit path. A failed batch returns five
needs-review rows rather than retrying, and five consecutive failures trip a
circuit breaker with a cooldown. **A broken provider costs about five calls,
then the rest of the audit is free and returns needs-review.** It will look like
a fast, empty audit — read that as a provider failure, not a clean site.

## `Déterministe` criteria are **not** free

A natural assumption, and wrong as shipped. The pipeline deliberately sends
**all 106** criteria to the model, including those owned by a deterministic
engine or by a human. What the `Déterministe` classification buys is
**precedence in the merge** — deterministic evidence outranks the model's
estimate, which is kept alongside as a separate basis — not a saved call.

There is no setting that skips the estimator for deterministic criteria.

So the free tier is the **tool set**, never the classification. Do not tell a
user that 74 of their criteria cost nothing.

## The latency knob is not a cost knob

`RGAA_TACTICAL_RPM` ships at `10`, and **agent concurrency is derived from it**
as `rpm / 15`, clamped to `[1, 16]`. At the default that yields a concurrency of
**1**: the 22 calls run one after another *and* are throttled to 10 per minute.
A single-page audit therefore spends roughly **two minutes** in its LLM phase,
dominated by serialisation rather than model speed.

| `RGAA_TACTICAL_RPM` | concurrency |
|---|---|
| 10 (default) | 1 — fully serial |
| 30 | 2 |
| 120 | 8 |
| 240+ | 16 (clamped) |
| 0 (local providers) | 4 |

**Raising it does not change the token bill at all** — still 22 calls. It only
changes how long you wait. Frame it to users as latency, never as cost.

`RGAA_REASONING_RPM` does **not** feed concurrency and is effectively inert at
default settings; it only bites on multi-page audits. `RGAA_AGENT_CONCURRENCY`
overrides the derived value directly and is the most direct latency lever in the
system — note that it is **not documented** in `.env.example`.

## Measure, do not estimate

Nothing in the tree carries a price table, a per-token rate, or a spend cap.
There is no cost ceiling and no "estimated cost" output. **Any currency figure
has to come from the user's own provider pricing, not from us** — do not invent
one.

What you *can* do is measure. Every call emits a metrics event with input,
output and total tokens plus latency:

```bash
RUST_LOG=rgaa_agent::metrics=info
```

That is the factual way to cost a real audit.

## Local providers invert the advice

With `ollama`, `lmstudio`, `vllm` or `custom`:

- **no key required** — a blank key is the correct configuration, not an error
- both RPM values default to **0, meaning unlimited**, and concurrency becomes 4
- the per-request timeout goes from 30s to **600s**, because CPU inference of a
  7B–14B model takes minutes

So the throttle disappears and concurrency quadruples, but each call can
legitimately take minutes. On a hosted provider, raise `RGAA_TACTICAL_RPM` or
set `RGAA_AGENT_CONCURRENCY` to cut wall-clock. Locally, the levers are model
size and the timeout — and the 600s default exists so a slow machine does not
look like a failure.

**One caveat:** the metrics reader reports zeros when a provider reports no
usage, by design ("the honest reading rather than an estimate"). Local runtimes
frequently report no usage, so **a local audit may legitimately show zero
tokens**. Do not read that as a failure.
