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

**Nothing produces `verify_fix.reference_audit`.** Nothing needs to — **you
construct it**. See "Building a reference bundle".

---

## The ten traps

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
WordPress/Elementor homepage, **8 of 20 findings were unmapped**.

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

## The fingerprint defect — read before trusting `verify_fix`

**Established by a real run, not by reading code.** `analyze` emits **one
`dom_snapshot` evidence hash per scan**, shared by every finding in it. Two
scans of the same page at different scopes produced different hashes for an
element whose `rule`, `url`, `target`, `component_path`, evidence count, kind and
location were all identical.

`FindingFingerprint::from_finding` includes `evidence[].hash`. So **any change to
the page changes every finding's fingerprint**, and `compare` then reports every
reference finding as *fixed* and every current finding as *new* — exactly when
you are using it, because a change is the only reason to re-verify.

**Until this is fixed, do not trust `verify_fix`'s categories.** Compare rounds
yourself on `rule` + `target` + `criterion_id`, which were empirically stable
across runs. Call `verify_fix` for its re-analysis and its citations if you like,
but derive *fixed / remaining / new* from your own comparison.

Note also that finding `id` is **positional per rule within a scan**: the same
element was `rgaa-9.3-listitem-9` in one run and `rgaa-9.3-listitem-3` in
another. Neither `id` nor the fingerprint is a stable identifier. `target` is.

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
The comparison fingerprints on `rule`, `url`, `target`, `component_path` and
`evidence[].{kind,hash,location}` — **not** on `id`, **not** on `status`.
Changing `target` alone splits one finding into one "fixed" and one "new".

Copying verbatim is still right, but it is **not sufficient**: the evidence hash
moves on its own when the page changes. See the fingerprint defect above.

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
