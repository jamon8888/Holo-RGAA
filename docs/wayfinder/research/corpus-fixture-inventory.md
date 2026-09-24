# Corpus & fixture asset inventory

Resolves ticket #185 ("Inventory existing corpus and fixture assets", child of map #183). Inventory only — no design, no protocol decisions (owned by #189). Verified against HEAD `0677195` (master) by reading source and files, not README claims.

## 1. `rgaa-test-corpus` crate

Path: `rgaa-rs/crates/rgaa-test-corpus/` (workspace member; deps: `rgaa-core`, `serde`, `serde_json`).

| Metric | Value |
|---|---|
| HTML pages | **37** (`criteria/*.html`) |
| Distinct criteria covered | **26** of 106 |
| Expected-status mix | Fail 19 · Pass 16 · NotApplicable 2 · NotTested 0 |
| Case-kind mix | Standard 35 · Adversarial 2 |
| Verdict authorship | filename only, no in-page metadata |

### Verdict field schema (`src/lib.rs`)

```rust
pub struct TestPage {
    pub criterion_id: String,   // parsed from filename prefix ("1.1", "13.3", …)
    pub name: String,           // filename minus .html
    pub html_path: String,
    pub description: String,    // "Test page: {filename}" — boilerplate
    pub expected_status: String, // "Pass" | "Fail" | "NotApplicable" | "NotTested"
    pub kind: CaseKind,          // Standard | Adversarial
}
```

- Filename convention: `{criterion_id}-{slug}-{status}[-adversarial].html`; status parsed by whole-segment match (`na` > `pass` > `fail`, else `NotTested`). `-adversarial` sets `CaseKind::Adversarial` independently of status.
- `TestPage::classification()` resolves the criterion against `rgaa_core::RgaaCriteria::all()` at runtime.
- Statuses are plain strings, not an enum; the corpus vocabulary (4 values) is a subset of runtime `rgaa_core::CriterionStatus` (Pass/Fail/NotApplicable/NotTested/NeedsReview/Error — no NeedsReview, no Error in corpus labels).

### `-adversarial` usage (2 pages)

| File | Criterion | Classification | Expected |
|---|---|---|---|
| `1.1-prompt-injection-adversarial-fail.html` | 1.1 | Deterministe | Fail |
| `9.1-fake-heading-adversarial-fail.html` | 9.1 | IaAssiste | Fail |

Both added in `c7ba39a` (2026-09-20, "#131 cas adversariaux"). Unit tests enforce: ≥2 adversarial pages, every adversarial page has a definite (non-`NotTested`) status, and kind/status vary independently (a standard-Fail and an adversarial-Fail both exist).

### How consuming tests use it

**No code outside the crate consumes it.** Zero other `Cargo.toml` declares `rgaa-test-corpus` as a dependency; zero references to `rgaa_test_corpus` / `TestCorpus` / `TestPage` exist outside `crates/rgaa-test-corpus/`. The only tests that load it are the crate's own 7 unit tests in `src/lib.rs` (`load_corpus()` → assertions on IA-assiste/NA/adversarial coverage).

Nearest conceptual consumer: `rgaa-agent/src/rag/harness.rs` — `BaselineCase.expected` is documented as "The expected status from the test corpus's own labels", and its tests hand-build `CriterionStatus` labels in the same shape — but `rgaa-agent/Cargo.toml` does not depend on the corpus crate; the wiring is comment-only, not implemented. README.md:283 describes the crate as "Fixtures for regression tests".

### Per-criterion page inventory (26 criteria)

| Criterion | Class | Pages (status·kind) |
|---|---|---|
| 1.1 | Det | fail·std, pass·std, fail·**adv** |
| 1.2 | Det | fail·std, pass·std |
| 2.1 | **IA** | fail·std, pass·std |
| 3.2 | Det | fail·std, pass·std |
| 4.3 | Det | fail·std, pass·std |
| 4.5 | Det | pass·std |
| 4.7 | **IA** | na·std, fail·std |
| 4.10 | Det | fail·std |
| 4.11 | Det | fail·std |
| 5.1 | Det | na·std, pass·std |
| 6.1 | Det | pass·std |
| 7.1 | Det | fail·std |
| 7.3 | Det | fail·std |
| 8.1 | Det | pass·std |
| 8.2 | Det | pass·std |
| 8.5 | Det | fail·std, pass·std |
| 8.7 | Det | fail·std |
| 9.1 | **IA** | pass·std, fail·**adv** |
| 9.3 | Det | fail·std |
| 10.1 | Det | fail·std |
| 10.4 | Det | pass·std |
| 11.1 | Det | fail·std |
| 11.2 | **IA** | fail·std, pass·std |
| 12.1 | Det | pass·std |
| 13.1 | Det | fail·std |
| 13.3 | Det | pass·std |

## 2. Other fixtures repo-wide

The external review's "~37 fixtures over 26 criteria" **is exactly the `rgaa-test-corpus` crate** — verified: 37 files / 26 distinct criterion IDs. There is no second hidden set of that size.

| Location | Count | What it is | Verdict-bearing? |
|---|---|---|---|
| `rgaa-rs/crates/rgaa-test-corpus/criteria/*.html` | 37 | Per-criterion synthetic HTML pages | Yes (filename status) |
| `rgaa-rs/crates/rgaa-remediation/tests/fixtures/{react,next,vue,angular}/…` | 4 | Framework source snippets for patch-proposal tests (`include_str!`) | No — remediation, not verdicts |
| `docs/wayfinder/prototype/*.PROTOTYPE.html` + `audit-payload.PROTOTYPE.schema.json` | 3 | Wayfinder declaration prototype output | No — not test fixtures |
| `rgaa-rs/crates/rgaa-core/data/rgaa-4.1.2/*.json` | 4 | Catalog/reference data (`criteres`, `automatable_criteres`, `axe_mapping`, `axe_rules`) | No — input data |

**Real total: 41 fixture files** (37 + 4); 44 if the 3 wayfinder prototype files are counted, but those are deliverables, not test fixtures.

Referenced-but-absent fixtures (docs/plans promise them, HEAD has none):

- `.github/tests/policy-fixtures.sh` — named in `docs/superpowers/plans/2026-08-18-claude-code-rgaa-remediation-plugin.md`; `.github/tests/` does not exist.
- `test_fixtures/axe_output.json` — example path in `CLAUDE.md:174`; no `test_fixtures/` dir exists.
- `rgaa-linter/fixtures/{pass,fail}/*` — designed in `rgaa-rs/docs/superpowers/plans/2026-08-24-rgaa-linter.md`; crate itself does not exist.
- No checked-in cassette files (`rgaa-holo/src/cassette.rs` is code-only) and no checked-in baseline-report JSON (harness test `baseline_report_round_trips_through_json_for_a_locked_fixture` proves format readiness but locks nothing yet).

Inline (non-file) test HTML exists in code — obscura `data:text/html,…` URLs, remediation `element_html` strings, MCP test payloads — but carries no verdict labels.

## 3. Gap list (as-implemented `criteria.rs` at HEAD)

Catalog partition, parsed from the `CLASSIFICATION` array (106 entries): **73 Deterministe · 32 IaAssiste · 1 Manuel (7.5)**. The 32-IaAssiste set is the as-implemented split; ticket #182 (fidelity of that split to official methodology) is closed/contested — this inventory reports the code, not its correctness.

Coverage against the 26 fixture criteria: 22 Deterministe + 4 IaAssiste covered; 0 Manuel covered.

### Deterministic criteria with ZERO fixture representation (51 of 73)

```
1.5 1.6 1.8 1.9 3.3 4.1 4.8 4.12 4.13 5.4 5.6 5.7 5.8 6.2 7.4 8.3 8.9 9.4
10.2 10.5 10.6 10.7 10.8 10.9 10.11 10.12 10.13 10.14 11.4 11.5 11.6 11.11
11.12 11.13 12.2 12.4 12.5 12.6 12.7 12.9 12.10 12.11 13.2 13.4 13.5 13.7
13.8 13.9 13.10 13.11 13.12
```

### Current-`IaAssiste` criteria with ZERO fixture representation (28 of 32)

Covered: `2.1 4.7 9.1 11.2`. Zero representation:

```
1.3 1.4 1.7 2.2 3.1 4.2 4.4 4.6 4.9 5.2 5.3 5.5 7.2 8.4 8.6 8.8 8.10 9.2
10.3 10.10 11.3 11.7 11.8 11.9 11.10 12.3 12.8 13.6
```

### Other

- `7.5` (Manuel): zero fixtures — the only Manuel criterion, outside both lists above.

**Totals: 79 of 106 criteria (51 det + 28 IA + 1 manuel) have zero fixture representation; 27 have ≥1 page.**

## 4. Verdict schema today vs. a human-verdict reference set

### What exists

- **Unit of judgment:** one `expected_status` per HTML **page** (which targets one named criterion). Not per RGAA *test* (each criterion has multiple tests in the référentiel), not per page×criterion matrix.
- **Vocabulary:** `Pass` / `Fail` / `NotApplicable` / `NotTested` as `String`, encoded **in the filename**; no sidecar metadata, no verdict text inside the HTML (pages are bare markup + French heading).
- **Author (git provenance):** created 2026-08-24 by `Test <test@test.com>` (initial 10 pages → expansion to 30 criteria, `1691d3d`/`568c6c3`/`1a5d6ce`); extended 2026-09-20 by `John Wayne <johnwayne75011@gmail.com>` (`c7ba39a`: +6 pages — 2 adversarial, 2 NA, 11.2 pair — plus harness). No in-artifact author field; authorship recoverable only from git history.
- **Nature of pages:** all synthetic minimal HTML authored in-repo; **zero real-world pages**.

### Missing for a human-verdict reference set (absences only — protocol is #189's)

| Gap | Current state |
|---|---|
| Human author / reviewer identity on the verdict | Absent (only git author of the file) |
| Verdict date / revision per verdict | Absent |
| Rationale / justification / evidence behind expected status | Absent |
| Per-test granularity (criterion → its tests) | Absent — status is per page |
| `NeedsReview` (and `Error`) in label vocabulary | Absent from corpus (present in runtime `CriterionStatus`) |
| Provenance marker: synthetic vs real page, source URL | Absent — all synthetic, no URL field |
| Structured metadata (sidecar JSON/YAML) | Absent — filename is the schema |
| Confidence / second-rater / disagreement fields | Absent |
| Label for criteria a page incidentally exercises | Absent — one criterion_id per file |
| Real-page corpus (any) | 0 pages |

Reusable as-is: the 37 pages + filename convention + `CaseKind` axis + loader API; anything requiring human sign-off, rationale, or per-test verdicts must be added as new schema — nothing to migrate, because nothing of that shape exists yet.
