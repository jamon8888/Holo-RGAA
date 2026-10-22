# Field mapping and trap fields

The contract between the `rgaa-mcp` tools. Every rule here was established by
reading the server's source, and the bundle rules by executing against it.

**All wire fields are `snake_case`.** No request struct renames. Two exceptions
worth knowing: `classification` values are capitalised Rust `Debug` spellings
(`"Deterministe"`, `"IaAssiste"`, `"Manuel"`), and `remediate`'s error `code` is
likewise a `Debug` spelling, not a stable serde name. Do not pattern-match on
either as if it were an API constant.

A caveat on the server's own words: at least one error message is written in
camelCase (`"viewportHeight requires viewportWidth to be set"`) for fields that
are `viewport_height` / `viewport_width` on the wire. Follow the schema, not the
message. The crate's `SPEC.md` has also documented six tools for a nine-tool
server — treat the live schema as authoritative over any prose, including this
file.

---

## The edges that work

| From | To |
|---|---|
| `audit_url.sampled_page_urls[]` | `analyze.url` |
| `analyze.findings[].id` | `source_map.findings[].id` |
| `analyze.findings[].target` | `source_map.findings[].selector` |
| `analyze.findings[].html` | `source_map.findings[].html` |
| `source_map.mapped[].finding_id` | `remediate.issues[].id` |
| `source_map.mapped[].source_location` | `remediate.issues[].source_locations[]` (drop `snippet`) |
| `analyze.findings[].rule` | `remediate.issues[].rule` |
| `analyze.findings[].url` | `remediate.issues[].page_url` |
| `analyze.findings[].criterion_id` | `remediate.issues[].criteria[]` — **but see trap 11** |
| `analyze.findings[].description` | `remediate.issues[].summary` — **required** |
| `analyze.findings[].remediation` + `details` | `remediate.issues[].remediation` — **required** |

`remediate.issues[]` **requires** `id`, `rule`, `element_html`, `page_url`,
`summary` **and** `remediation`. The last two are easy to miss — they are not
copies of a single `analyze` field, and omitting either is a schema error before
any of the traps below can even apply.

## The edges that do not

**`analyze.findings[].html` → `remediate.issues[].element_html` — does not work.**
See trap 1.

**`audit_url.audit_id` → `get_audit_result.audit_id` — inert.** The server wires
a no-op storage service whose lookup returns nothing unconditionally. Nothing in
`rgaa-mcp` persists an audit, so this is always empty. Do not build a flow that
depends on retrieving an audit later; keep what `audit_url` returned.

**`source_map` maps on literals, and says so.** Each mapped finding carries
`confidence` (`high`/`medium`) and `matched_on` naming the literal it matched —
`src="…/logo.png"` scores high, a `class token "submit-btn"` only medium. Read
both before editing; the tool is explicit that there is no exact inverse of
rendering.

**Document-level findings are unmappable, by nature.** Findings on `<html>` —
`html-has-lang` (8.2, 8.3), `document-title` (8.5), `page-has-heading-one` —
come back in `unmappable` with `no_distinguishing_literal`, because `<html>`
carries no literal to search for and those defects live in the HTML shell or
layout, not in a component. On a small React fixture this was **4 of 9
findings**. They are real defects that the loop structurally cannot reach: fix
them by hand in the shell, and report them separately rather than as loop
failures.

**Nothing produces `verify_fix.reference_audit`.** Nothing needs to — **you
construct it**. See "Building a reference bundle".

---

## The thirteen traps

Each fails *silently* or from an unexpected direction. None is discoverable by
trying once.

### 1. `element_html` must be framework source, not rendered DOM

`remediate` detects the framework from the source you hand it, looking for
markers like `import React`, `className=`, `v-model`, `@Component`. A rendered
`<img src="hero.png">` matches none, so the issue comes back as needing review
with "source does not safely identify the requested framework". **Passing
`framework` explicitly does not help** — a mismatch is still rejected.

Expected shape, from the crate's own fixtures:

```
element_html: "import React from \"react\"; <img src=\"hero.png\">"
```

**Corollary: plain HTML cannot be remediated at all.** The framework set is
React, Next, Vue and Angular — there is no HTML variant, while `source_map`
*does* have an `html` flavour. The two tools disagree on scope. For plain HTML
and PHP templates, write the fix yourself and say that you did.

### 2. `analyze`'s response shape switches on a request field

The response is an untagged union of two shapes. With `config.igt_tools` unset,
findings are at top-level `findings`. **Set it, and they move to `data.axe`,
with `data.igt` added.** A client reading `response.findings` gets nothing — no
error, no warning.

Check which shape you received before reading it. This is the counterpart of
Deque's empty-`elementHtml` trap, and it is worse: silent and shape-level.

### 3. `source_locations` defaults to `[]`, and `[]` fails the issue

Omitting it is not "let the tool work it out" — it is a guaranteed per-issue
error. **This is what makes `source_map` mandatory** between `analyze` and
`remediate`.

### 4. `igt_tools` is an unvalidated string list

Only `"keyboard"` does anything. A typo is **accepted**, runs no guided test,
and returns the flat shape — a plausible-looking result with silently no
keyboard testing performed.

### 5. `advanced_rules` silently coerces unknown values to off

It is an optional string, not an enum. Only `"thorough"` and `"standard"`
enable anything; everything else disables it. So `"enabled"`, `"true"` and
`"yes"` all quietly turn it **off**.

### 6. `profile` is validated late

An unknown profile is not rejected at the boundary. It surfaces from deep in the
stack, after a browser start has been attempted, as an unsupported-configuration
error. Expect the failure to look like a browser problem.

### 7. `previously_incomplete` is dropped on the wire

The domain finding carries it to mark "the checker could not decide", but the
wire DTO does not map it. On the wire that is **indistinguishable** from a
`Manuel` criterion's needs-review. The discriminator exists server-side and is
thrown away — do not infer one case from the other.

### 8. `verify_fix` clamps instead of rejecting, and ignores `path`

`per_file_timeout_ms` is clamped to a 30-second ceiling **with no warning**, so
asking for longer silently gets you 30s. And `files[].path` is **echo-only** —
never read from disk. `verify_fix` re-analyses the *page*; it does not inspect
your file. Passing a path does not make it check that file.

### 9. The `new` category conflates two causes — unless you prevent it

Its wire key is **`new`**, not `new_findings`. It concatenates genuinely-new
findings with `Pass → Fail` regressions, indistinguishably.

**You can make it mean only "genuinely new":** regressions are only emitted when
the reference holds a `Pass` finding matching a current `Fail`. Omit `Pass`
findings from the reference — which is all `analyze` gives you anyway — and the
regression set is provably empty. See the bundle rules below.

### 10. `lint_static`'s mutual exclusion is invisible in the schema

`sources` and `source_files` are both plain optional arrays, but **exactly one**
must be non-empty. Neither, or both, is an invalid-input error. The schema
cannot tell you this.

Use `source_files` to have files read from disk and findings mapped onto real
paths; use `sources` to pass content inline.

### 11. `criterion_id` is often `null`

axe-core finds real violations that map to **no RGAA criterion**. Their `id` is
prefixed `rgaa-unmapped-`, and `criterion_id` is `null`. On a live run against a
WordPress/Elementor homepage, **7 of 22 findings were unmapped**.

They are genuine defects, but they are outside the referential: they cannot be
counted for or against conformity, and passing `null` into `remediate`'s
`criteria[]` is wrong. Report them separately as out-of-referential findings.

### 12. `igt` is deprecated

Its own description says so: use `analyze` with `config.igt_tools: ["keyboard"]`.
Remember that doing so changes the response shape — trap 2.

### 13. Two places set the viewport, with different defaults

`analyze` accepts `viewport_width` / `viewport_height` **both** at the top level
(defaulting to null) **and** inside `config` (defaulting to 1000 × 1080). Which
wins is not documented. Set one, not both.

The schema's minimum is `0`, but the runtime floor is **320**: a width below
that is rejected after the browser has started, so it reads as a browser
failure. Another constraint the schema cannot tell you.

---

## The fingerprint, and what it does and does not guarantee

**The scan-level-hash defect described here was fixed in #320** (merged
2026-10-12). It is worth knowing what it was, because the measurements below
were taken against the broken version and one of them still applies.

**What was wrong.** `analyze` emits **one `dom_snapshot` evidence hash per
scan**, shared by every finding in it, and `FindingFingerprint::from_finding`
used to hash `evidence[].hash`. So any change to the page moved every
finding's fingerprint at once, and `compare` reported every reference finding
as *fixed* and every current one as *new* — exactly when you were using it,
since a change is the only reason to re-verify. Established by a real run: two
scans at different scopes gave different hashes for an element whose `rule`,
`url`, `target`, `component_path`, evidence count, kind and location were all
identical.

**What it is now.** The fingerprint is the finding's *identity* and nothing
else: `rule` + `criterion_id` + `url` + `target` + `component_path`, under the
`rgaa-fp-v2-` prefix. Evidence is excluded deliberately — it is attached per
scan, not per finding, and its *count* moved too, because
`ScreenshotPolicy::OnFailure` only appends a screenshot while the page still
fails.

So **`verify_fix`'s categories are now usable** — with one standing caveat that
the fix does not touch:

> The fingerprint is stable *given a stable `target`*. It cannot rescue a
> finding whose selector is positional or runtime-generated; see **Selector
> stability** below, which was measured on an *unedited* page and still holds.

That is the live risk now, not the fingerprint. When a round's `target` values
are all stable shapes, read `verify_fix`'s verdict and report from it.

**When they are not, comparing yourself does not help.** Your key and the
fingerprint are the same tuple, so they fail the same way: a moved `target`
makes the old finding absent, and absence is indistinguishable from a fix by
*any* key built on `target`. There is no fallback comparison to reach for.

The only sound move is to stop classifying those findings. Screen each
round's targets for the unstable shapes listed under **Selector stability**,
and report anything matching them as **unstable** — neither fixed nor
remaining, and excluded from the convergence check. A finding on
`:nth-child(2)` inside a carousel has to be re-identified by something that
does not move — a stable ancestor, an `id`, an accessible name — or carried
forward as unresolved. Never counted as fixed.

### Why all three parts of the key are needed

This measurement is why `criterion_id` is part of the v2 fingerprint. It was
taken here first, against a build that lacked it; #320 confirmed the same
collision in Rust (2 findings, 1 fingerprint) and added the field.

Measured on a live page, twice:

| | |
|---|---|
| findings | 24 and 22 |
| distinct by `(rule, target)` | 22 and 21 — **not unique** |
| distinct by `(rule, target, html)` | 22 and 21 — **html does not help** |
| distinct by `(rule, target, criterion_id)` | 24 and 22 — **unique** |

The collisions are one element mapped to **two RGAA criteria** — e.g. the same
`<img class="emoji">` reported as `rgaa-1.1-image-alt-0` and
`rgaa-1.2-image-alt-0`, identical `rule`, `target` and `html`, differing only in
`criterion_id`. Drop `criterion_id` and you silently merge two findings.

### Selector stability, measured

Two scans of the same unedited page differed by **6 findings, all of them on a
positional or runtime-generated selector**. Restricted to stable selectors the
two runs were **identical: 19 findings, same set**.

Unstable shapes seen in the wild: `:nth-child(1)`, carousel containers
(`.tm-carousel-item`, `.tm-carousel-next`) and obfuscated runtime classes
(`.go2617933356`). A rotating carousel makes `:nth-child(1)` point somewhere
else on the next scan, so the finding disappears and returns with no edit.

Never read such a disappearance as a fix. The loop is reliable against a local
dev server or static build; against a live dynamic page, expect this noise.

Note also that finding `id` is **positional per rule within a scan**: the same
element was `rgaa-9.3-listitem-9` in one run and `rgaa-9.3-listitem-3` in
another. Never match on `id`. The v2 fingerprint is stable across runs, but
only as far as its inputs are — and `target` is the input that moves.

---

## Building a reference bundle for `verify_fix`

`reference_audit` is raw JSON, deserialised into the domain bundle **without**
validation at that boundary. You construct it from `analyze` output. Three rules,
all of which fail silently if broken.

**Rule 1 — findings go in top-level `findings`, and nowhere else.**
The comparison clears the bundle's `pages` before diffing. Findings placed in
`pages[].findings` are **dropped from the diff** while still feeding the
unverified list. Such a bundle deserialises, validates, returns success, and
reports nothing.

Populate `pages[]` only for `pages[].criteria`, which is what the citation index
reads to re-attach per-criterion RGAA citations. An empty `pages` is accepted but
yields no citations.

**Rule 2 — copy findings verbatim.** `analyze`'s finding objects deserialise
**directly** into the bundle's findings. There is no translation step: copy them.
The comparison fingerprints on `rule`, `criterion_id`, `url`, `target` and
`component_path` — **not** on `id`, **not** on `status`, and since #320 **not**
on `evidence` either. Changing `target` alone splits one finding into one
"fixed" and one "new".

Copying verbatim is still the rule, and since #320 it is also sufficient: the
fields that decide the match are the ones you copied, and nothing that moves on
its own is in the hash any more. Evidence no longer has to match — copy it for
the record, not for the diff.

**Rule 3 — never include `status: "pass"` findings.** This is what makes trap 9
harmless.

### Mandatory fields

Eight top-level keys are required: `schema_version`, `audit_id`, `url`, `config`,
`pages`, `findings`, `checkpoints`, `summary`. Only `audit_complete` is optional.

Serde specifics worth knowing:

- a missing **optional** field defaults to absent even without an explicit
  default — but a missing **array** field is a hard error. Send `[]`, not nothing.
- unknown keys are **ignored**; there is no strict-field rejection on this path.
- `schema_version` must be a **string**: `"1.0"` works, `1.0` as a number is
  rejected. Its *value* is not enforced at this boundary, but send the real one.

### A minimal bundle that works

```json
{
  "schema_version": "1.0",
  "audit_id": "round-1",
  "url": "https://example.org/",
  "config": { "max_pages": 1, "max_depth": 0, "respect_robots": true, "sample_mode": false },
  "pages": [
    { "page_id": "p1", "url": "https://example.org/", "title": null,
      "criteria": [
        { "criterion_id": "1.1", "title": "…", "classification": "Deterministe",
          "status": "fail", "violations": [], "source": "axe-core",
          "citations": [ { "kind": "referentiel", "test_id": "1.1.1", "referentiel_version": "4.1.2" } ] }
      ],
      "findings": [], "errors": [], "completed": true, "duration_ms": 0 }
  ],
  "findings": [ "… analyze's finding objects, copied verbatim …" ],
  "checkpoints": [],
  "summary": { "total_pages": 1, "completed_pages": 1, "total_findings": 1,
               "passed": 0, "failed": 1, "needs_review": 0, "errors": 0 }
}
```

---

## `remediate`'s response, and why you must read the patch before applying it

The response is `{"outcomes": [...]}`. Each outcome carries `outcome`,
`issue_id`, `explanation`, `steps`, `confidence`, `criteria` and a `proposal`:

```json
{"proposal": {
  "proposal_id": "<issue_id>-proposal",
  "finding_ids": ["<issue_id>"],
  "diff": "<the replacement source line>",
  "files": ["App.jsx"],
  "rationale": "…", "risks": ["…"], "validation_commands": ["…"],
  "expected_effect": "…",
  "proposal_hash": "rgaa-proposal-v1-<hex>",
  "approval_state": "required",
  "approval_token": "rgaa-approval-v1-<proposal_id>-<proposal_hash>"
}}
```

Two things to know about it:

- **`diff` is a replacement line, not a unified diff.** You apply it yourself.
- **`files` is a bare basename** (`App.jsx`), not the path you passed in
  `source_locations`. Resolve it against your own mapping, not against the repo.

### Check the patch before applying it

A syntax defect here was **fixed** in `rgaa-remediation`: attribute insertion
used to land after a self-closing `/`, so `<img … />` came back as
`<img … / alt="">`, which does not compile. Two of three measured cases were
broken, and `outcome` was `"ok"` with `confidence` `"high"` on all of them.

It matters for two reasons even so:

- **A binary older than that fix still emits it.** Check what you are running
  before trusting a patch; the symptom is an attribute sitting after the slash.
- The same scan also mis-read JSX **brace expressions**, so the `>` of an arrow
  function in `onClick={() => …}` ended the tag and the attribute was injected
  into the middle of the expression. Also fixed.

An unquoted attribute value ending in a slash — `<img src=/logo.png/>` — is
genuinely ambiguous and now returns `NeedsReview` rather than a guess.

### The patch can also be semantically wrong for its criterion

Criteria **1.1** and **1.2** received the *identical* `alt=""` patch. They are
opposite requirements: 1.1 asks an informative image to carry a text
alternative, 1.2 asks a decorative one to be ignored. `alt=""` is what 1.2
requires of an image that is genuinely decorative, and it defeats 1.1.

It does not make 1.2 conforming on its own. Decorativeness is an editorial
judgement that markup does not carry, so `gapfix-1-2` only ever returns `fail`
on a decidable contradiction — markup declaring the image decorative while it
still exposes a textual alternative — and `review` otherwise. It never returns
`pass`.

So read `criteria` on the outcome and judge the patch against it. Remember
`confidence` is the hardcoded string `"high"` and carries no signal.

## `remediate` batching

- **1 to 25 issues** per call, enforced twice.
- **Correlate on `issue_id`, never on array position** — and the server checks
  both the count and the order of returned ids, failing the **whole call** with
  an incomplete-result error if they disagree. Stronger than correlating by
  convention: you will know.
- A **per-issue** failure is not a call failure. A bad issue comes back with an
  error outcome beside its healthy neighbours.
- **`confidence` is the hardcoded string `"high"` on every success.** It is not a
  signal. Do not rank, filter or report on it.
- More than 25 issues: split into sequential batches of 25.
- Do not collapse issues by rule. Guidance is per issue and tailored to the
  element sent; the same rule often needs different fixes.

## Viewport

The minimum width is **320**, not 1. Setting `viewport_height` requires
`viewport_width` to be set as well.
