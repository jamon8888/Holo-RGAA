# Plus de critères automatiques ou semi-automatiques — où ils sont réellement

Findings for the question "find more automatic or semi-automatic criteria". Part of map [#183](https://github.com/jamon8888/Holo-RGAA/issues/183).

- **Audited at:** `338c179` (master).
- **Evidence:** the 106-criterion catalog (`criteres.json`), `automatable_criteres.json`, `axe_mapping.json`, `axe_rules.json`, `crates/rgaa-rules/src/axe_mapper.rs`, and a **real 5-page audit** (`rgaa_parisprivate_final_report.json`, 530 criterion-slots, 4 527 s).
- **External:** W3C ACT Rules index, axe-core 4.9.1 rule metadata (which carries its own `RGAAv4` tags), read 2026-09-26.

---

## TL;DR

**The answer is not that more criteria could be automated. It is that most of the automation the workspace already claims does not exist, and the automation it already runs is thrown away.**

Three tiers, in the order they pay off:

| Tier | What | Criteria affected | Cost |
|---|---|---|---|
| **0 — repair** | **40 of the 63 "axe rules" in `axe_mapper.rs` are not axe-core rules.** 48 criteria are mapped *exclusively* to invented names, so they are initialised `Pass` and no violation can ever match them — **structurally unfalsifiable**. In the real audit this produced **125 unfalsifiable PASSes = 66 % of all 190 reported passes** | **48** | rename/remap, no new tech |
| **1 — harvest** | **49 axe-core rules that axe itself tags `RGAAv4` are absent from the mapping.** `axe.run()` is called with no `runOnly`, so all 105 rules already execute every page and these violations are computed then **silently discarded** | **21** (14 with zero working rule today) | mapping only |
| **2 — new mechanisms** | Nine cheap deterministic checks outside axe's scope (HTML validation, language detection, focus-visible computed style, CSS-off, real reflow, field grouping, figure/figcaption, download detection, cryptic-content regex) | **~12** | small, bounded |
| **3 — semi-automatic** | Deterministic *applicability* pre-filter in front of the 32 `IaAssiste` pertinence criteria, so the LLM only judges surviving candidates | **32** | reduces [#180](https://github.com/jamon8888/Holo-RGAA/issues/180)'s budget draw |

Plus two blockers that must be fixed first or none of the above reaches the report (§5).

---

## 1. Baseline: what the workspace thinks vs what runs

| Source | Picture |
|---|---|
| `criteria.rs` classification | **73 `Deterministe`**, 32 `IaAssiste`, 1 `Manuel` (7.5) |
| `automatable_criteres.json` | 39 `FullyAutomatable`, 45 `PartiallyAutomatable`, 22 `NotAutomatable` |
| Actual mechanisms | **two**: `AxeMapper` (77 mapping entries) and 13 `gap_fix` snippets (1.1, 1.2, 2.1, 3.2, 6.1, 8.3, 8.5, 10.2, 10.11, 10.14, 11.1, 11.4, 12.7) |
| Real 5-page audit | 190 `pass`, **201 `needs_review`**, 121 `not_applicable`, 18 `fail`. Sources: `agent` 303, `axe-core` 180, `gap-fix` 25, **`agent-error` 17**, `manual` 5 |

**58 of 106 criteria never reach a verdict on any of the 5 pages** — 34 of them classified `Deterministe`.

## 2. Tier 0 — the 48 unfalsifiable criteria

`AxeMapper::map` initialises every mapped criterion to `Pass`, then downgrades on a matching violation id (`axe_mapper.rs:19-34`). So a mapping entry whose rule names never appear in axe output yields **`Pass` unconditionally**.

Of the 63 rule names referenced by `axe_mapper.rs`, **only 23 are real axe-core rule ids**. The other 40 are WCAG success-criterion or technique slugs that axe-core never emits:

```
audio-control, audio-description, autocomplete, blockquote, character-key-shortcuts,
consistent-navigation, deprecated-element, doctype, error-prevention, error-suggestion,
fieldset, figure-caption, focus-order, focus-visible, iframe-title, image-text, keyboard,
keyboard-trap, lang, layout-table, link-purpose-in-context, longdesc, meaningful-sequence,
motion-actuation, non-text-content, on-focus, on-input, orientation, page-title,
pause-stop-hide, pdf, pointer-cancellation, pointer-gestures, reflow, resize-text,
table-header, text-spacing, three-flashes, timing-adjustable, video-description
```

Several are near-misses for a real rule — `autocomplete` vs **`autocomplete-valid`**, `iframe-title` vs **`frame-title`**, `lang` vs **`valid-lang`**, `layout-table` vs **`table-fake-caption`**, `table-header` vs **`td-has-header`** — which is what makes this look like a working mapping.

**48 criteria are mapped exclusively to these names:**

`1.8, 1.9, 2.1, 4.1, 4.5, 4.7, 4.8, 4.10, 4.11, 4.12, 4.13, 5.1, 5.4, 5.8, 7.1, 7.3, 7.4, 8.1, 8.5, 8.7, 8.9, 8.10, 9.4, 10.1, 10.4, 10.7, 10.11, 10.12, 10.13, 10.14, 11.5, 11.6, 11.11, 11.12, 11.13, 12.2, 12.5, 12.9, 12.10, 12.11, 13.1, 13.2, 13.7, 13.8, 13.9, 13.10, 13.11, 13.12`

Measured consequence in the real audit: **125 PASSes across 29 of these criteria that no page content could ever have turned into a Fail — 66 % of the 190 reported passes.** Among them 12.9 (keyboard trap), 12.11, 10.4 (200 % zoom), 13.1, 13.8, 13.9, 13.10–13.12, 8.1, 8.9, 9.4, 11.6, 11.11, 12.2, 12.5 — five pages each, all `Pass`, all unfalsifiable.

This is the same family as the concern on `research/false-pass-head-state`, but located and quantified: it is not a head-state edge case, it is the mapping table's vocabulary.

> **Tier 0 is therefore a precondition, not an improvement.** Until it is fixed, adding mechanisms only adds more entries to a table whose PASSes carry no information, and `taux_global` (81 % here) is not defensible.

## 3. Tier 1 — 49 real axe rules already computed and discarded

`axe.run()` is invoked with no `runOnly` filter (`cdp_pool.rs:339`, `native.rs:337`), so **all 105 rules execute on every page**. axe-core's own metadata tags **67** of them `RGAAv4`; the mapping uses **23**. The remaining **49** are free: computed, returned, discarded.

Proposed assignment. Every target number verified against `criteres.json` (topic maxima: 1→9, 2→2, 3→3, 4→13, 5→8, 6→2, 7→5, 8→10, 9→4, 10→14, 11→13, 12→11, 13→12).

| RGAA | Criterion | axe rules to map | Today |
|---|---|---|---|
| **2.1** | Chaque cadre a-t-il un titre de cadre ? | `frame-title`, `frame-title-unique`, `frame-focusable-content` | **no working rule**; and labelled `NotAutomatable` + `IaAssiste` — demonstrably wrong, axe decides this exactly |
| **11.13** | Finalité d'un champ de saisie (autocomplete) | `autocomplete-valid` | no working rule (`autocomplete` is fictional) |
| **13.8** | Contenu en mouvement ou clignotant contrôlable | `blink`, `marquee` | no working rule |
| **13.1** | Contrôle de chaque limite de temps | `meta-refresh` | no working rule |
| **13.9** | Consultable en portrait et paysage | `css-orientation-lock` | no working rule |
| **10.4** | Texte lisible à 200 % | `meta-viewport` | no working rule |
| **10.11** | Contenus sans défilement bidirectionnel | `meta-viewport` | gap-fix only, and unsound (§4) |
| **4.10** | Son déclenché automatiquement contrôlable | `no-autoplay-audio` | no working rule |
| **7.3** | Script contrôlable clavier et pointage | `scrollable-region-focusable`, `server-side-image-map`, `focus-order-semantics` | no working rule |
| **7.1** | Script compatible avec les technologies d'assistance | `aria-roles`, `aria-valid-attr`, `aria-valid-attr-value`, `aria-allowed-attr`, `aria-required-attr`, `aria-required-children`, `aria-required-parent`, `aria-prohibited-attr`, `aria-conditional-attr`, `aria-deprecated-role`, `aria-hidden-body`, `nested-interactive`, `aria-command-name`, `aria-meter-name`, `aria-progressbar-name`, `aria-tab-name` | no working rule — **16 rules for the single most under-served criterion** |
| **11.1** | Chaque champ a-t-il une étiquette ? | + `button-name`, `input-button-name`, `select-name`, `aria-input-field-name`, `aria-toggle-field-name`, `form-field-multiple-labels` | `label` only (1/13 tests) |
| **11.2** | Étiquette pertinente | `label-content-name-mismatch` | none |
| **8.8** | Code de langue des changements valide | `valid-lang` | none |
| **8.4** | Code de langue par défaut pertinent | `html-xml-lang-mismatch` | none |
| **8.2** | Code source valide | + `duplicate-id-aria` | `html-has-lang`, `html-lang-valid` — neither tests validity |
| **9.1** | Structuration par titres | `p-as-heading` | `heading-order` |
| **9.3** | Listes correctement structurées | + `definition-list`, `dlitem` | `list`, `listitem` |
| **5.4** | Titre de tableau correctement associé | `table-fake-caption`, `table-duplicate-name` | no working rule |
| **5.7** | Technique d'association cellules/en-têtes | + `td-has-header` | `td-headers-attr`, `th-has-data-cells` |
| **1.1 / 1.2** | Alternatives textuelles | + `area-alt`, `object-alt`, `role-img-alt`, `svg-img-alt` | `image-alt`, `input-image-alt` |

**21 criteria gain real deterministic evidence, 14 of them from zero.** No new dependency, no new call, no new browser work — a table edit plus tests.

## 4. Tier 2 — nine mechanisms axe cannot supply

| RGAA | Mechanism | Nature |
|---|---|---|
| **8.2** (test 8.2.1) | W3C Nu validator (`vnu`) on the *generated* DOM, not the served source | deterministic; the criterion's core test, currently unimplemented |
| **8.4 / 8.8** | statistical language detection (`whatlang`) on text runs vs the declared `lang` | semi: valid-tag part deterministic, pertinence = 1 judge on mismatch only |
| **10.7** | computed-style diff between default and `:focus` per focusable (outline / box-shadow / border / background) — W3C ACT rule **`oj04fd`** | deterministic |
| **10.1** | presentational HTML detection (`<font>`, `<center>`, `align`, `bgcolor`, spacer images) | deterministic |
| **10.4 / 10.11** | real `Emulation.setDeviceMetricsOverride` then re-measure, instead of comparing `scrollWidth` to a hardcoded 320 at the current viewport (`gap_fix.rs:188-202`, unsound as written) | deterministic — this is [#194](https://github.com/jamon8888/Holo-RGAA/issues/194) mode C |
| **11.5** | radio/checkbox sharing a `name` but outside a `fieldset`/`role=group` | deterministic |
| **1.9** | `<figure>`/`<figcaption>` association, `aria-describedby` to a visible caption | deterministic |
| **13.3** | download links by extension (`.pdf`, `.docx`, `.xlsx`…) → applicability, then presence of an accessible sibling version | semi: applicability deterministic |
| **13.5** | ASCII-art / emoticon / cryptic-syntax regex → applicability | semi: applicability deterministic, alternative pertinence = 1 judge |

## 5. Tier 3 — semi-automatic: make applicability deterministic, judge only survivors

The 32 `IaAssiste` criteria are almost all *pertinence* questions (1.3, 1.7, 4.2, 5.5, 8.4, 11.2, 11.9, 13.6 …). They share a shape: **a deterministic part that finds candidates, and a judgment on each candidate.** Today the whole criterion goes to the model.

Two consequences of splitting them:

1. **Most pages need no call at all.** "Is each alt text relevant?" is `not_applicable` when the page has no informative image, and that is a DOM query. In the audited site, 121 of 530 slots were already NA — the same filter applied before the LLM removes calls rather than answers them.
2. **The call that remains is cheaper and better grounded**: a short list of candidate elements with their computed accessible names, not a page.

This is the lever on [#180](https://github.com/jamon8888/Holo-RGAA/issues/180)'s cap of 10 completions/page: a deterministic applicability gate reduces the 32-criterion draw without touching the batch size.

## 6. Two blockers that must be fixed or none of this reaches the report

**(a) The merge order discards deterministic results.** `all_results.extend(axe_results)` → `gap_results` → **`holo_results`** (`pipeline.rs:757-760`). Later entries overwrite earlier ones, so an agent verdict replaces axe evidence for the same criterion. Measured: **30 axe-mapped criteria were overwritten to `needs_review`** in the audited run — and **7 of them by `agent-error`** (1.2, 6.1, 8.10, 9.3, 11.1, 12.6, 13.4). *A failed LLM call currently erases a deterministic result.* An error must never overwrite evidence; that part is a straight bug with no policy question attached.

**(b) `PartiallyAutomatable` → blanket `NeedsReview`** (`pipeline.rs:787-795`) suppresses 45 criteria wholesale, regardless of what the automated part found. The policy is defensible in principle — RGAA conformance requires *all* of a criterion's tests to pass — but it is applied at the wrong granularity. The catalog already stores `automatable_test_count` / `total_test_count` and `test_keys` per criterion. **Carrying results at test level, not criterion level, is the structural fix:** a criterion is decidable automatically when every test it has is either covered by a mechanism or inapplicable on that page. That converts a large share of the 45 into real verdicts — e.g. 6.1 (12/13 tests automatable), 12.6 (5/6), 12.7 (5/6), 11.12 (6/7), 9.2 (6/7), 10.1 (5/6).

## 7. Recommended order

1. **Tier 0** — replace the 40 fictional rule names; any criterion left with no real mechanism must emit `NotTested`, never `Pass`. Regression test: assert every name in the mapping exists in `axe_rules.json`.
2. **Blocker (a)** — `agent-error` must not overwrite; deterministic evidence wins over an LLM `needs_review` where the mechanism covers the whole criterion.
3. **Tier 1** — map the 49 already-computed rules.
4. **Blocker (b)** — test-level results, enabling Tier 3.
5. **Tier 2** — the nine mechanisms, cheapest first (`vnu` for 8.2, computed-style focus for 10.7, real resize for 10.4/10.11).

Corrections to the catalog data found on the way: **2.1** is labelled `NotAutomatable`/`IaAssiste` but is exactly what axe `frame-title` decides; **12.3** is labelled `NotAutomatable` with `3/3` automatable tests, an internal contradiction.

## Sources

- W3C ACT Rules index — <https://www.w3.org/WAI/standards-guidelines/act/rules/> (`73f2c2` autocomplete, `bf051a`/`ucwvc8`/`de46e4`/`off6ek` language, `a25f45`/`d0f69e` tables, `oj04fd` focus visible, `cae760` iframe name, `b4f0c3` meta viewport, `b33eff` orientation)
- axe-core 4.9.1 rule metadata, `RGAAv4` tags — `crates/rgaa-core/data/rgaa-4.1.2/axe_rules.json`
- RGAA 4.1.2 criteria and tests — <https://accessibilite.numerique.gouv.fr/methode/criteres-et-tests/>
