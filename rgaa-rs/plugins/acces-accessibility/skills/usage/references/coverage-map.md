<!-- GENERATED FILE — do not edit.
     Source: rgaa-rs/crates/rgaa-core/data/rgaa-4.1.2/engine_plan.json
     Regenerate: ./scripts/generate-coverage-map.py -->

# Coverage map

For each RGAA 4.1.2 criterion: which instrument decides it, and **what a
missing finding proves**. This is the tooling layer's own question — what a
criterion *requires* is domain knowledge and lives elsewhere.

Generated from the engine plan, so it cannot drift from the engines.

## The one number that matters

Of 106 criteria, a clean result lets you conclude **conforming**
for **32**.

- **32** fully decided here — silence means conforming.
- **34** partially covered — these tools can find violations,
  but silence proves **nothing**: a residual judgement remains.
- **40** never evaluated by this loop — `Holo` needs the LLM
  (only `audit_url` calls it), `Human` needs a person.

So 74 of 106 criteria cannot be
called conforming on the strength of a clean loop. Report them as residue.

## Legend

| Column | Meaning |
|---|---|
| Engine | the instrument that primarily decides this criterion |
| Axe rules | axe-core rules mapped to it, if any |
| Silence proves | what a *missing* finding lets you conclude |

## Criteria

| Criterion | Engine | Axe rules | Silence proves |
|---|---|---|---|
| 1.1 | `Deterministic` | `image-alt`, `input-image-alt`, `area-alt`, `role-img-alt`, `svg-img-alt`, `object-alt` | **nothing conclusive** — residual judgement |
| 1.2 | `Holo` | `image-redundant-alt`, `presentation-role-conflict` | **nothing** — not evaluated here |
| 1.3 | `Holo` | `image-alt` | **nothing** — not evaluated here |
| 1.4 | `Holo` | — | **nothing** — not evaluated here |
| 1.5 | `Holo` | — | **nothing** — not evaluated here |
| 1.6 | `Deterministic` | — | **nothing conclusive** — residual judgement |
| 1.7 | `Holo` | — | **nothing** — not evaluated here |
| 1.8 | `Holo` | — | **nothing** — not evaluated here |
| 1.9 | `Deterministic` | — | **nothing conclusive** — residual judgement |
| 2.1 | `AxeCore` | `frame-title`, `frame-title-unique` | conforming |
| 2.2 | `Holo` | — | **nothing** — not evaluated here |
| 3.1 | `Holo` | `link-in-text-block` | **nothing** — not evaluated here |
| 3.2 | `AxeCore` | `color-contrast` | **nothing conclusive** — residual judgement |
| 3.3 | `Deterministic` | — | **nothing conclusive** — residual judgement |
| 4.1 | `Deterministic` | — | **nothing conclusive** — residual judgement |
| 4.2 | `Human` | — | **nothing** — not evaluated here |
| 4.3 | `AxeCore` | `video-caption`, `audio-caption` | **nothing conclusive** — residual judgement |
| 4.4 | `Human` | — | **nothing** — not evaluated here |
| 4.5 | `Deterministic` | — | **nothing conclusive** — residual judgement |
| 4.6 | `Human` | — | **nothing** — not evaluated here |
| 4.7 | `Deterministic` | — | **nothing conclusive** — residual judgement |
| 4.8 | `Deterministic` | — | **nothing conclusive** — residual judgement |
| 4.9 | `Holo` | — | **nothing** — not evaluated here |
| 4.10 | `Deterministic` | `no-autoplay-audio` | conforming |
| 4.11 | `Deterministic` | — | conforming |
| 4.12 | `Deterministic` | — | conforming |
| 4.13 | `Deterministic` | — | **nothing conclusive** — residual judgement |
| 5.1 | `Deterministic` | — | **nothing conclusive** — residual judgement |
| 5.2 | `Holo` | — | **nothing** — not evaluated here |
| 5.3 | `Holo` | — | **nothing** — not evaluated here |
| 5.4 | `Deterministic` | `table-fake-caption` | conforming |
| 5.5 | `Holo` | — | **nothing** — not evaluated here |
| 5.6 | `Holo` | `th-has-data-cells`, `scope-attr-valid`, `td-has-header` | **nothing** — not evaluated here |
| 5.7 | `AxeCore` | `td-headers-attr`, `th-has-data-cells`, `scope-attr-valid`, `td-has-header` | conforming |
| 5.8 | `Deterministic` | — | conforming |
| 6.1 | `Holo` | `identical-links-same-purpose` | **nothing** — not evaluated here |
| 6.2 | `AxeCore` | `link-name` | conforming |
| 7.1 | `Holo` | `aria-allowed-attr`, `aria-valid-attr`, `aria-valid-attr-value`, `aria-required-attr`, `aria-roles`, `aria-hidden-focus`, `nested-interactive` | **nothing** — not evaluated here |
| 7.2 | `Holo` | — | **nothing** — not evaluated here |
| 7.3 | `Deterministic` | `scrollable-region-focusable`, `tabindex`, `nested-interactive`, `frame-focusable-content` | **nothing conclusive** — residual judgement |
| 7.4 | `Deterministic` | — | **nothing conclusive** — residual judgement |
| 7.5 | `Human` | `aria-allowed-role` | **nothing** — not evaluated here |
| 8.1 | `Deterministic` | — | conforming |
| 8.2 | `Deterministic` | `duplicate-id-aria` | conforming |
| 8.3 | `AxeCore` | `html-has-lang` | conforming |
| 8.4 | `Holo` | `html-lang-valid`, `html-xml-lang-mismatch` | **nothing** — not evaluated here |
| 8.5 | `AxeCore` | `document-title` | conforming |
| 8.6 | `Holo` | — | **nothing** — not evaluated here |
| 8.7 | `Deterministic` | — | **nothing conclusive** — residual judgement |
| 8.8 | `Holo` | `valid-lang` | **nothing** — not evaluated here |
| 8.9 | `Deterministic` | — | conforming |
| 8.10 | `Deterministic` | — | **nothing conclusive** — residual judgement |
| 9.1 | `Holo` | `heading-order`, `empty-heading`, `page-has-heading-one` | **nothing** — not evaluated here |
| 9.2 | `Holo` | `landmark-one-main`, `region`, `landmark-no-duplicate-banner` | **nothing** — not evaluated here |
| 9.3 | `Deterministic` | `list`, `listitem`, `definition-list`, `dlitem` | **nothing conclusive** — residual judgement |
| 9.4 | `Deterministic` | — | **nothing conclusive** — residual judgement |
| 10.1 | `Deterministic` | — | conforming |
| 10.2 | `Deterministic` | — | **nothing conclusive** — residual judgement |
| 10.3 | `Holo` | — | **nothing** — not evaluated here |
| 10.4 | `Deterministic` | `meta-viewport` | **nothing conclusive** — residual judgement |
| 10.5 | `Deterministic` | — | conforming |
| 10.6 | `AxeCore` | `link-in-text-block` | **nothing conclusive** — residual judgement |
| 10.7 | `Deterministic` | `focus-order-semantics` | **nothing conclusive** — residual judgement |
| 10.8 | `AxeCore` | `aria-hidden-focus`, `hidden-content` | **nothing conclusive** — residual judgement |
| 10.9 | `Holo` | — | **nothing** — not evaluated here |
| 10.10 | `Holo` | — | **nothing** — not evaluated here |
| 10.11 | `Deterministic` | `meta-viewport`, `meta-viewport-large` | **nothing conclusive** — residual judgement |
| 10.12 | `Deterministic` | `avoid-inline-spacing` | conforming |
| 10.13 | `Deterministic` | — | **nothing conclusive** — residual judgement |
| 10.14 | `Deterministic` | — | conforming |
| 11.1 | `AxeCore` | `label`, `select-name`, `input-button-name`, `form-field-multiple-labels`, `aria-input-field-name` | conforming |
| 11.2 | `Holo` | `label-content-name-mismatch`, `label-title-only` | **nothing** — not evaluated here |
| 11.3 | `Deterministic` | — | **nothing conclusive** — residual judgement |
| 11.4 | `Deterministic` | — | conforming |
| 11.5 | `Deterministic` | — | **nothing conclusive** — residual judgement |
| 11.6 | `Deterministic` | — | conforming |
| 11.7 | `Holo` | — | **nothing** — not evaluated here |
| 11.8 | `Deterministic` | — | **nothing conclusive** — residual judgement |
| 11.9 | `Holo` | `button-name`, `input-button-name` | **nothing** — not evaluated here |
| 11.10 | `Holo` | — | **nothing** — not evaluated here |
| 11.11 | `Holo` | — | **nothing** — not evaluated here |
| 11.12 | `Human` | — | **nothing** — not evaluated here |
| 11.13 | `Holo` | `autocomplete-valid` | **nothing** — not evaluated here |
| 12.1 | `Deterministic` | — | conforming |
| 12.2 | `Deterministic` | — | conforming |
| 12.3 | `Holo` | — | **nothing** — not evaluated here |
| 12.4 | `Deterministic` | — | conforming |
| 12.5 | `Deterministic` | — | conforming |
| 12.6 | `Deterministic` | `landmark-one-main`, `region`, `landmark-unique`, `landmark-banner-is-top-level` | conforming |
| 12.7 | `Deterministic` | `bypass`, `skip-link` | conforming |
| 12.8 | `Deterministic` | `focus-order-semantics` | **nothing conclusive** — residual judgement |
| 12.9 | `Deterministic` | — | conforming |
| 12.10 | `Deterministic` | `accesskeys` | conforming |
| 12.11 | `Deterministic` | — | conforming |
| 13.1 | `Human` | `meta-refresh`, `meta-refresh-no-exceptions` | **nothing** — not evaluated here |
| 13.2 | `Deterministic` | — | conforming |
| 13.3 | `Deterministic` | — | **nothing conclusive** — residual judgement |
| 13.4 | `Human` | — | **nothing** — not evaluated here |
| 13.5 | `Deterministic` | — | **nothing conclusive** — residual judgement |
| 13.6 | `Holo` | — | **nothing** — not evaluated here |
| 13.7 | `Human` | — | **nothing** — not evaluated here |
| 13.8 | `Deterministic` | `blink`, `marquee` | **nothing conclusive** — residual judgement |
| 13.9 | `Deterministic` | `css-orientation-lock` | conforming |
| 13.10 | `Deterministic` | — | **nothing conclusive** — residual judgement |
| 13.11 | `Deterministic` | `target-size` | conforming |
| 13.12 | `Deterministic` | — | **nothing conclusive** — residual judgement |
