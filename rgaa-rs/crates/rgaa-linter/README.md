# rgaa-linter

Static accessibility linting of source files, with no browser and no JavaScript
runtime. Exposed over MCP as the `lint_static` tool.

## What it is for

The rest of this workspace audits *rendered pages*. That is the only way to be
right, and it costs seconds per page. This crate covers the other half of the
problem — defects already visible in the source a developer is typing — at a
cost low enough to sit in an edit loop (tens of microseconds per file).

It is a first pass. **It is not a conformance verdict.** Quote `analyze` for
that.

## Rules

| Rule | Fires when | RGAA 4.1.2 |
|------|-----------|------------|
| `img-alt` | `<img>` has no `alt` attribute at all (`alt=""` is correct for a decorative image) | 1.1 |
| `form-label` | an `input`/`select`/`textarea` has no `<label for>`, no enclosing `<label>`, and no `aria-label`, `aria-labelledby` or `title` | 11.1 |
| `button-name` | a `<button>` has no text content and no naming attribute | 11.1 |
| `link-name` | an `<a href>` has no text content, no image with alt text, and no naming attribute | 6.1 |

Criterion numbers come from the `rgaa-core` catalog and follow the same
`axe → RGAA` mapping the browser-based audit uses, so findings from the two
paths can be deduplicated by rule id.

## Limits, stated plainly

There is no JS/TS parser behind this — see `src/scan.rs` for why. Consequently
the linter:

- cannot follow a component boundary: `<Button />` is not `<button>`, and is skipped;
- cannot evaluate an expression: `alt={caption}`, `:alt="caption"` and `{...props}` all count as "supplied";
- cannot see a `<label>` that lives in another file, so an `id` it cannot resolve statically makes the rule stand down;
- does not compute the full accessible-name algorithm: `aria-labelledby` is honoured as *present*, not resolved.

Everywhere it cannot decide, it reports nothing. A static linter that reports a
false "missing alt" on `alt={caption}` gets switched off, and then finds nothing
at all.

## Languages

`.html`, `.htm` · `.jsx`, `.tsx`, `.js`, `.ts`, `.mjs` · `.vue`. Content of
`<script>`, `<style>` and comments is never scanned. Anything else is refused
rather than guessed at.

## Configuration: `lint-rules.toml`

Resolution order, first match wins:

1. the `config_path` argument — **must exist**;
2. `$RGAA_LINT_CONFIG` — **must exist**;
3. `$XDG_CONFIG_HOME/rgaa/lint-rules.toml`, else `$HOME/.config/rgaa/lint-rules.toml` — optional, but if it exists it must be valid;
4. built-in defaults, reported as such in `LintReport::config_source`.

A config that is named and absent, unreadable, misspelled, or written for
another schema version is an **error**. It never degrades to defaults: a user
who has written a config is owed an error message, not a silent reset.

```toml
version = 1
profile = "rgaa-4.1"   # rgaa-4.1 | wcag-2.1-aa | section-508

[rules]
img-alt     = "error"   # error | warning | off
form-label  = "error"
button-name = "error"
link-name   = "error"
```

## Profiles

All three profiles detect the same four defects — each one leaves a control or
an image with no name at all, which fails under every framework. What the
profile changes is the citation a finding carries: the RGAA criterion, the WCAG
2.1 AA success criteria, or Section 508 E205.4 plus the WCAG **2.0** subset it
incorporates (WCAG 2.1 additions are filtered out, because Section 508 does not
make those requirements).

## Performance

`tests/performance.rs` measures the median cost per file and prints it. Run:

```bash
cargo nextest run -p rgaa-linter --no-capture
```

The assertion threshold is set far above the observed cost on purpose: a tight
timing assertion on a shared CI runner is a flaky test, so the guard catches an
algorithmic regression and nothing finer.
