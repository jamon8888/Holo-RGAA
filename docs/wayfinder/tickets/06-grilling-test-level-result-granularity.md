# Test-level result granularity: per-test outcomes, the `taux_global` denominator, and the applicability gate

- Label: `wayfinder:grilling` (HITL — resolve only through live exchange with the human)
- Status: **open · recommendations below await human confirmation**
- Issue: #203
- Blocked by: #181 · #199 (landed as #204)
- Related: #188 (deterministic-core semantics) · #190 (coverage denominator) · #180 (completion budget) · #201 · #202

## Question

Should a result be carried per RGAA test rather than per criterion — and if so, what does
that change in the schema, the `taux_global` denominator, and the LLM call budget?

## What #201 and #202 added to the evidence

Both landed a **criterion-level approximation of exactly the data this ticket is about**,
and that is the strongest argument for deciding it now.

- `axe_mapping.json` entries carry `coverage: complete | partial`, and
  `GapFixRules::COMPLETE_COVERAGE` carries the same flag for the DOM mechanisms. The flag
  answers one question — *do this criterion's mechanisms decide all of its tests?* — with
  one bit, because the per-test mapping that would answer it properly does not exist.
- The flag is already visibly lossy. 13.1 has fifteen tests and one rule
  (`meta-refresh`), so it is `partial` and can never pass. 11.1 has thirteen tests and is
  declared `complete` only because demoting it would move verdicts in the published
  report. Both are wrong in opposite directions, and only per-test results fix either.
- The two coverage lists are therefore **the migration checklist** for this work: 30
  `complete` and 13 `partial` axe criteria, plus 12 `complete` gap-fix criteria, are
  precisely the declarations that per-test outcomes would replace with computed values.

Restating the facts from the issue, unchanged: the catalog already stores
`automatable_test_count`, `total_test_count` and `test_keys` per criterion and nothing
consumes them except the bulk `NeedsReview` branch at
`crates/rgaa-orchestrator/src/pipeline.rs`; 370 of 693 tests are marked automatable
(53 %) with 21 criteria below 50 %; 121 of 530 slots in the audited run reached
`not_applicable` with no completion.

## Recommendations

### 1. Does `CriterionResult` gain per-test outcomes? — **Yes, additively**

`CriterionResult` keeps its atomic `status`, and gains
`tests: Vec<TestOutcome>` under `#[serde(default, skip_serializing_if = "Vec::is_empty")]`.
`TestOutcome` is `{ test_key, status, source, evidence }`.

The criterion `status` stops being *set* and becomes *derived*: when `tests` is non-empty
it is the reduction over them (any `Fail` → `Fail`; every test of the criterion settled
and at least one of them by a `Pass` → `Pass`; every test settled and all of them
deterministically inapplicable → `NotApplicable`; otherwise `NeedsReview`/`NotTested`
per #188's rules). Requiring at least one `Pass` matters: an empty applicable set must
not reduce to `Pass`, or a criterion would report conformance without a single
applicable test having passed. When `tests` is empty the status is whatever the
mechanism set, exactly as today.

Why additive rather than replacing the atomic verdict:

- **No storage migration.** `rgaa-storage` already persists `violations` as a JSON
  column; `tests` rides the same way. The union schema's column set does not change.
- **Every export surface keeps working** unchanged — HTML report, declaration, TUI, MCP,
  audit bundle. They read `status`, which still exists and still means the same thing.
- **RGAA conformance is defined per criterion.** The atomic verdict is not a modelling
  shortcut to be removed; it is what an opposable report must state. Per-test outcomes
  are how it is *justified*, not what replaces it.

The same field then lets the #201/#202 coverage flags be **computed** rather than
declared: a criterion is `complete` exactly when every applicable test has a mechanism.
That is the payoff, and it is why this is worth the schema change.

### 2. Decision rule for a partly covered criterion — **deterministic inapplicability only**

A criterion is decidable automatically **iff every test its mechanisms do not cover is
inapplicable on that page**, and inapplicability may be established **only by a
deterministic query** — never by an LLM.

The asymmetry is deliberate, and it is #199's objection restated: a model may *fail* a
test, and may *flag* one for review, but it may not make a criterion conform by
declaring the uncovered tests out of scope. Allowing that would rest a published
conformance claim on an unreproducible judgement — the same defect as a `Pass` that
cannot fail, reached by a longer route.

### 3. Does this reopen #188? — **No. Let #188 land, accept a second pass**

#188's semantics (`NotTested` when no test executed, `NeedsReview` on axe `incomplete`)
restate at test granularity without changing meaning — they are the same two rules at a
finer grain, and the reduction in recommendation 1 is where they get applied. Blocking
#188 on this ticket stalls the deterministic-core fix for a mechanical lift. Sequence:
#188 lands at criterion granularity, then the per-test pass moves those two rules down
one level.

### 4. Reconciling with #190 — **tests for coverage, criteria for the rate, always published together**

- `coverage_percent` counts validated **tests**, per #190's settled denominator.
  Unchanged.
- `taux_global` counts **criteria**, because that is the unit RGAA conformance and the
  *Déclaration d'accessibilité* are defined in.

They are not derivable from one another, and **publishing either alone is what produced
the `taux_global: 81.08` claim** over a run where two thirds of the passes could not
fail. So the report leads with the criterion rate and carries the test coverage
immediately beside it as a qualifier that cannot be separated from it:

> *X % conforme, établi sur Y % de la surface testable.*

A conformance rate with no coverage figure next to it should be treated as a defect, not
a formatting choice.

### 5. The applicability gate in front of the 32 `IaAssiste` criteria — **in scope, and free of the cap**

In scope for #180's flow, and a deterministic `not_applicable` **does not** count against
the 10-completions-per-page cap, because it consumes no completion. That is the entire
point: the gate spends DOM queries to buy back budget. The 121 of 530 slots that already
reach `not_applicable` without an LLM call show the mechanism is not speculative.

The pertinence criteria (1.3, 1.7, 4.2, 5.5, 8.4, 11.2, 11.9, 13.6, …) all have the shape
*deterministic candidate search, then judgement per candidate*. Gating them removes calls
rather than answering them, which is why this belongs with the budget work and not after
it.

### 6. Ceiling — what this pass explicitly does not rebuild

- **No change to the storage union's column set.** `tests` rides in JSON beside
  `violations`.
- **No per-test rendering** beyond a collapsed disclosure under each criterion in the
  HTML report. No new export format, no per-test rows in the declaration.
- **No re-derivation of the catalog.** `test_keys` as shipped in
  `automatable_criteres.json` is the test identity; this pass consumes it, it does not
  re-source it from the RGAA reference.
- **No re-assessment of the 42 `complete` coverage declarations** — the 30 axe plus the
  12 gap-fix ones, not the `partial` entries — from #201/#202 in the same change.
  They are replaced by computed values only once the reduction is in place and tested,
  so the two changes stay separately reviewable.
- **No change to the LLM prompt shape.** Per-test prompting is a later question; this
  pass routes existing criterion-level completions and records their results per test
  where the mechanism knows which test it answered.

## Fog patch found while writing this

`automatable_criteres.json` had **five** rows whose `classification` contradicted its own
counts, all labelled `NotAutomatable` with every test automatable: **4.9, 5.5, 10.2,
10.14** and **12.3**. #201 fixed 12.3 (its AC4 named it), so **four** remain — 4.9, 5.5,
10.2 and 10.14. It left those, because
none carries axe rules and relabelling them moves `coverage_percent` on grounds #201 does
not discuss. A test now pins them as known-wrong with an exception list, so the next
person meets them deliberately. They should be resolved before per-test counting starts —
recommendation 1 makes the counts load-bearing.
