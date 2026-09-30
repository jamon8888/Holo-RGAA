# Spec — nine deterministic mechanisms axe-core cannot supply

> Status: draft for review · Issue: #202 · Depends on #199 (landed as #204) and #201.
> Scope of this pass: mechanisms 4, 5 and 6 are implemented; 1, 2, 3, 7, 8 and 9 are
> specified here and not built. Each is independently landable, as #202 states.

## Problem statement

After #199 removed the verdicts that could not fail and #201 harvested the 49 RGAAv4
axe rules that were being computed and discarded, a residue remains: RGAA criteria whose
check **is** deterministic but which axe-core does not implement. Detecting the
candidates is deterministic — no judgement is required to establish applicability — and
for most of these mechanisms that is the whole check. Mechanisms 7, 8 and 9 still spend
one judging completion, but only on the candidates detection leaves standing. Either way
the audit has no mechanism for them today, so they reach the report as `NeedsReview` or
`NotTested` on every page.

Nine such mechanisms are worth building. Six need no completion at all. The other three
— mechanisms 7, 8 and 9 — make **candidate detection** deterministic and leave a judgement
only for the candidates that survive it, which removes completions rather than adding
them. All nine are cheap, and each closes a gap a client can name.

## The rule every mechanism obeys

This is the part that makes #202 a spec rather than a list. Three constraints, all
inherited from #199 and #201, and the reason the implemented mechanisms look the way
they do.

1. **It must run on the audit path.** `audit_one` in
   `crates/rgaa-orchestrator/src/pipeline.rs` never calls `analyze`. A mechanism wired
   only into `analyze` does not execute during an audit. For a DOM mechanism the audit
   path is `GapFixRules::snippets()`, which the orchestrator batches through the browser
   bridge — being in that map *is* being on the audit path, which is what
   `the_new_mechanisms_are_on_the_audit_path` asserts.
2. **A violation is always evidence; silence usually is not.** A mechanism covering 2 of
   a criterion's 4 tests may fail the criterion on what it found, but its finding
   nothing says nothing about the other 2 tests. So it must not emit a criterion-level
   `Pass`. `GapFixRules::parse_results` enforces this through
   `covers_whole_criterion`: a partial mechanism reporting no violation produces **no
   result at all** and the criterion falls to the pipeline's declared fallback. The
   default for an undeclared criterion is partial, so forgetting to declare coverage
   fails closed.
3. **An absent tool is `NotTested`, never `Pass`.** For mechanism 1 specifically, but
   the principle is general: "we could not check" and "we checked and it was fine" are
   different report entries.

## The nine mechanisms

### 1. 8.2 — source validity via the W3C Nu validator — *not implemented*

Test 8.2.1 is literally *le code source généré est-il valide*. It is the criterion's
core test and entirely unimplemented: 8.2 maps to `html-has-lang`, `html-lang-valid`
and (since #201) `duplicate-id-aria`, none of which tests validity.

The validator must run on the **generated DOM** (`document.documentElement.outerHTML`
after scripts settle), not the served source, or a single-page app validates as an empty
shell.

**AC5 decision — `vnu` is an optional feature, absent means `NotTested`.** Rejected
alternatives: a hard dependency would make a Java runtime a prerequisite for every
audit, including CI's default test job, for one criterion; a vendored WASM build is
plausible but is a supply-chain and build-time cost to be taken deliberately, not
incidentally. So:

- a Cargo feature `vnu` and a `RGAA_VNU_URL` / `RGAA_VNU_BIN` configuration;
- when neither is configured, 8.2 emits `NotTested` with the justification naming the
  missing tool — it must never read as conformance;
- when configured, errors from the validator map to `Fail` with the messages as
  evidence, and a validator that is configured but unreachable is `Error`, not `Pass`;
- coverage stays **partial**: 8.2 has 6 tests and validity is one of them.

### 2. 10.7 — visible focus, per focusable element — *not implemented*

This is W3C ACT rule [`oj04fd`](https://www.w3.org/WAI/standards-guidelines/act/rules/oj04fd/),
*Element in sequential focus order has visible focus*. Mechanism: for each element in
the sequential focus order, diff the computed style between the default and `:focus`
states across `outline`, `box-shadow`, `border` and `background`.

ACT permits `cantTell`, and that matters here: an element whose focus indicator is drawn
by a mechanism the diff cannot see (a background image, a pseudo-element, a canvas) is
**`NeedsReview`, not `Pass`**. Reporting it as conformant would be exactly the #199
defect with more machinery.

### 3. 10.4 / 10.11 — real reflow measurement — *partly done: the unsound check is gone*

The gap-fix snippet compared `scrollWidth` to a hardcoded `320` **at the unchanged
viewport**, so on any desktop viewport it reported overflow for nearly every page. That
is a false-failure machine, not a reflow check, and this pass **removes it**
(`the_unsound_reflow_snippet_is_not_on_the_audit_path`).

Until the real mechanism lands, 10.4 and 10.11 are carried by the `meta-viewport` axe
rule #201 mapped, at partial coverage — it can fail them, never pass them. The real
mechanism is `Emulation.setDeviceMetricsOverride` to 320 × 256 with 200 % text, then
re-measure; it needs the browser bridge and is #194's mode C.

### 4. 10.1 — presentational markup — **implemented**

Formatting carried in the markup rather than in CSS: `<font>`, `<center>`, `<basefont>`,
`<big>`, `<strike>`, `<tt>`, `<marquee>`; the presentational attributes `align`,
`bgcolor`, `cellpadding`, `cellspacing`, `valign`, `hspace`, `vspace`, `background`,
`bordercolor`; and 1-pixel spacer images with an empty or absent `alt`. 5 of the
criterion's 6 tests are automatable, so coverage is **partial**.

### 5. 11.5 — field groups enclosed — **implemented**

Radio or checkbox inputs sharing a `name` form one group and must be enclosed in a
`<fieldset>` or an ARIA `group` / `radiogroup`. A single control with a given name is
not a group and needs no fieldset — the mechanism requires two or more before it
reports. 2 of 4 tests automatable, coverage **partial**.

### 6. 1.9 — legend associated with its image — **implemented**

A `<figure>` fails when it has no `<figcaption>` with text, or when it wraps media whose
caption is tied to it by neither `aria-describedby` nor a non-empty alternative. A
`<figure>` that wraps no media (a captioned code sample) is legitimate and is not
reported. 12 of 25 tests automatable, coverage **partial**.

### 7. 8.4 / 8.8 — declared language vs detected language — *not implemented*

Statistical detection (`whatlang`) over text runs, compared with the declared `lang`.
Tag *validity* is deterministic and `valid-lang` / `html-xml-lang-mismatch` (mapped in
#201) cover part of it. *Pertinence* becomes one judging completion **only when
detection disagrees with the declaration**, so a correctly tagged page costs nothing.

### 8. 13.3 — office documents and their accessible versions — *not implemented*

Download links by extension (`.pdf`, `.docx`, `.xlsx`, `.odt`, …) establish
applicability deterministically, then presence of an accessible sibling version is
judged. The applicability half is most of the value: 13.3 and 13.4 currently go to
review on every page whether or not the page links a single document.

### 9. 13.5 — cryptic content — *not implemented*

ASCII art, emoticons and cryptic syntax detected by regex establish applicability;
alternative pertinence is one judgement on surviving candidates only. Also fixes the
mislabelling at `crates/rgaa-agent/src/criteria_defs.rs:694`, which comments 13.6 as
"CAPTCHA alternative relevance" — CAPTCHA alternative is **1.5**; 13.5 and 13.6 are
cryptic content.

## The pattern items 7–9 introduce

**Make applicability deterministic, and let the model judge only the surviving
candidates.** Applied across the 32 `IaAssiste` criteria this *removes* completions
rather than answering them, which is the lever on #180's cap of 10 per page. The
generalisation needs test-level result granularity first (#203) and is not in scope
here; items 7–9 are the concrete instances worth building now.

## Ceiling — what this pass does not do

- Mechanisms 1, 2, 7, 8 and 9 are not built. Mechanism 3 is only de-risked: the unsound
  check is removed, the real measurement is not written.
- The thirteen pre-#202 gap-fix snippets keep their current `complete` coverage, and so
  keep asserting the criterion-level `Pass`es they assert today. Several plainly do not
  decide their whole criterion — 11.1 has thirteen tests — but re-assessing them moves
  verdicts in the published report, which is #203's per-test question, not this one's.
  What #202 fixes is the direction of travel: a new mechanism cannot widen an
  unconditional `Pass`.
- No per-test result shape. Coverage is declared per criterion, as a single
  complete/partial flag. That is the interim representation #203 replaces.
