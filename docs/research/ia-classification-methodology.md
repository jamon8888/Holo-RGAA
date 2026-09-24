# IA-assisté set fidelity to official RGAA 4.1.2 methodology — research findings

Investigation for issue jamon8888/Holo-RGAA#182 (child of wayfinder map #180).
Question: is the 32-criterion `Classification::IaAssiste` set in
`rgaa-rs/crates/rgaa-core/src/criteria.rs` faithful to official RGAA 4.1.2
methodology, or hand-authored?

## Findings

1. **The official methodology contains no per-test automation flags at all.**
   DINUM publishes `RGAA/methodologies.json` (258 entries, one per official test,
   key = `criterion.test` e.g. `1.1.1`); every value is a French procedure string.
   253/258 entries end with the pass condition "Si c'est le cas, **le test est
   validé**" — i.e. `validé` means *the test passed*, not *the test is automatable*.
   There is no `automatable`, `validé`-as-automation, or any structured boolean
   field anywhere in the file (all 258 values are plain strings).
   Source: https://raw.githubusercontent.com/DISIC/accessibilite.numerique.gouv.fr/master/RGAA/methodologies.json

2. **`IMPLEMENTATION_PLAN.md` misreads that language as an automation rule.**
   Lines 30–32 claim: "Tests are per-criterion; each is `validé` (automatable) or
   `non validé` (needs human / assistive‑tech judgement) … the current
   `Classification` enum is a coarse, incorrect proxy." The official text does not
   say this. Officially, a *criterion* is validé when **all of its tests passed**
   on the page ("un critère est validé … lorsque tous les éléments de la page ont
   passé avec succès les tests") — a conformance outcome, not an automation class.
   Sources: `IMPLEMENTATION_PLAN.md:30-32`;
   https://accessibilite.numerique.gouv.fr/obligations/evaluation-conformite/

3. **`methodologie.json` — cited as a source of truth — is neither vendored nor
   its upstream filename.** `IMPLEMENTATION_PLAN.md:26` and
   `docs/superpowers/specs/2026-08-24-data-sourcing-production-readiness-design.md:30,40`
   cite `RGAA/methodologie.json`; the upstream repo actually ships
   `RGAA/methodologies.json` (plural), and neither file exists under
   `rgaa-rs/crates/rgaa-core/data/rgaa-4.1.2/` (which contains only
   `criteres.json`, `automatable_criteres.json`, `axe_mapping.json`,
   `axe_rules.json`). Glossaire is also not vendored despite the design doc.
   Sources: repo directory listing; DISIC repo contents API
   (https://api.github.com/repos/DISIC/accessibilite.numerique.gouv.fr/contents/RGAA);
   design doc lines 29–40.

4. **`criteres.json` has no automation field.** Each criterion object has only
   `number`, `title`, `tests`, `references` — confirmed identical to DINUM's
   upstream `RGAA/criteres.json` (byte-for-byte). So no criterion-level 32/73/1
   split can be derived from it. (`IMPLEMENTATION_PLAN.md:107` itself concedes
   "`criteres.json` has **no** automatable flag".)
   Sources: `rgaa-rs/crates/rgaa-core/data/rgaa-4.1.2/criteres.json`;
   https://raw.githubusercontent.com/DISIC/accessibilite.numerique.gouv.fr/master/RGAA/criteres.json

5. **The 32/73/1 table was hand-authored at scaffold and hand-edited since — no
   commit derives it from official data.**
   - `3a0fb9f` (2026-08-08, scaffold) created the table with **77 Deterministe /
     28 IaAssiste / 1 Manuel**.
   - `d0e65fc` (2026-08-08) flipped 1.4 to Deterministe by hand.
   - `f8d4936` (2026-09-19, "Fix blind spots") flipped 1.4 back plus 2.1, 4.7,
     8.10, 9.1 to IaAssiste → current **73/32/1**.
   The same hand-authored rows appear verbatim in the originating plan
   (`docs/superpowers/plans/2026-08-08-rgaa-rs-asqatasun-replacement.md:299-398`).
   Sources: `git show` of the three commits; plan file lines 299–398;
   current table at `rgaa-rs/crates/rgaa-core/src/criteria.rs:17-124`.

6. **The project's own design doc records that the classifications were mostly
   unconfirmed.** "Only ~20/106 classifications are confirmed with justification."
   Source: `docs/superpowers/specs/2026-08-24-data-sourcing-production-readiness-design.md:12`.

7. **The only automation artifact in the repo is a keyword heuristic that
   disagrees with the IA set — and it is explicitly a guess.** The generator
   (`rgaa-rs/crates/rgaa-data/src/automatability.rs:46-104`) marks a test
   "manual" if its text contains any of ~20 French keywords (`pertinent`,
   `visib`, `sens`, `cohérent`, …) and buckets each criterion as
   Fully/Partially/NotAutomatable. The task brief that produced it says "use a
   simple heuristic" (`.superpowers/sdd/2026-08-24-data-sourcing-production-readiness/task-4-brief.md:47-50`).
   Output: **39 Fully / 45 Partially / 22 Not** (`automatable_criteres.json`
   top-level counts) — not 32/73/1. Unlike `axe_mapping.json`, this file carries
   no provenance block.
   Sources: `automatability.rs:46-104`; task-4 brief lines 47–50;
   `rgaa-rs/crates/rgaa-core/data/rgaa-4.1.2/automatable_criteres.json`.

8. **Cross-tab of the heuristic vs the IA set — concrete criterion mismatches.**
   (IA set from `criteria.rs`, heuristic from `automatable_criteres.json`.)
   - Deterministe but heuristic says NotAutomatable: **10.2, 10.11, 10.14**
     (criteria.rs:75,84,87). Notably `f8d4936` *added gap-fix JS for exactly
     these three*, i.e. the code treats them as deterministic while the repo's
     own automation data says they are not automatable.
   - IaAssiste but heuristic says FullyAutomatable: **3.1** (criteria.rs:29).
   - IaAssiste with *all* tests automatable under the heuristic: **3.1, 4.9,
     5.5, 12.3**.
   - IaAssiste with *zero* automatable tests under the heuristic (15):
     1.4, 2.1, 2.2, 4.4, 4.6, 4.7, 5.2, 8.6, 8.10, 9.1, 10.3, 10.10, 11.3, 11.7, 12.8.
   - Overlap summary: of 32 IA → 18 NotAutomatable, 13 Partially, 1 Fully;
     of 73 Deterministe → 38 Fully, 32 Partially, 3 NotAutomatable.
   Sources: `rgaa-rs/crates/rgaa-core/src/criteria.rs:17-124`;
   `rgaa-rs/crates/rgaa-core/data/rgaa-4.1.2/automatable_criteres.json`.

9. **Mixed automatable/non-automatable tests inside one criterion are real in the
   repo's data, and the single-valued enum collapses them.** The heuristic marks
   **45/106 criteria PartiallyAutomatable** — 13 of them IaAssiste
   (1.3, 1.7, 4.2, 5.3, 7.2, 8.4, 8.8, 9.2, 11.2, 11.8, 11.9, 11.10, 13.6) and
   32 Deterministe (e.g. 1.1 at 12/20, 11.1 at 1/13, 11.9 at 1/14).
   Under *official* data the same condition exists structurally — every criterion
   has 1–9 official tests (258 total, avg 2.43) with independent pass conditions —
   but official data labels none of them automatable, so "mixed automatable /
   non-validé" as an official property is undefined.
   Sources: `automatable_criteres.json` (45 PartiallyAutomatable);
   `methodologies.json` (258 test-level procedures).

10. **The heuristic's `automatable_test_count` does not even count official
    tests.** It iterates every string in each test's condition list
    (`automatability.rs:81-88`), yielding **693 "tests"** across the catalog vs
    the official **258** tests (e.g. criterion 1.1 reports `12/20` while having 8
    official test keys). So the per-test automation arithmetic underneath the
    catalog is not aligned with the official test granularity it claims to measure.
    Sources: sum of `total_test_count` over `automatable_criteres.json` = 693;
    `len(test_keys)` sum = 258 = `len(methodologies.json)`.

11. **methodology.json's test-level rules do NOT imply a criterion-level
    32/73/1 split (nor any criterion-level split).** With no automation flag at
    test level (finding 1) and none at criterion level (finding 4), there is no
    derivation path from official data to any 3-way classification. DINUM's own
    guidance frames tooling as *aids* — "la mise en œuvre de cette procédure peut
    recourir parfois l'usage d'outils spécifiques, autrement un navigateur suffit
    pour effectuer la majorité des tests" — and the Kit d'audit states "Ara n'est
    pas un outil automatique."
    Sources: https://accessibilite.numerique.gouv.fr/ressources/methodologie-de-test/;
    https://accessibilite.numerique.gouv.fr/ressources/kit-audit/;
    `methodologies.json`.

12. **Ticket edge checks — `VISUAL_CRITERIA` vs IA membership: clean.**
    All 14 `VISUAL_CRITERIA` ids (`rgaa-rs/crates/rgaa-agent/src/criteria_defs.rs:680-695`)
    are in the IA set, including **13.6** (criteria.rs:117 = IaAssiste).
    **Zero visual criteria are non-IA.** Two comment-level defects only:
    (a) criteria_defs.rs:694 labels 13.6 "CAPTCHA alternative relevance" — 13.6 is
    "Alternative pertinente contenu cryptique" (definition at criteria_defs.rs:635-638);
    CAPTCHA relevance is 1.4; (b) the design doc labels 13.6 "table linearization"
    (`docs/superpowers/specs/2026-08-20-holo3-agentic-redesign-design.md:184`),
    which is 5.3. Also, the visual list has drifted from that design doc's list
    (doc:190 includes 11.3, omits 5.3 and 12.3; current code is the reverse).
    Sources: criteria_defs.rs:635-638,680-695; criteria.rs:117; design doc lines 184,190.

13. **README's per-topic classification table is wrong in 8 of 10 rows** while its
    totals row (73/32/1) is right — further evidence of hand-maintained data.
    Example mismatches: Images claim 5/4/0 vs actual 6/3/0; Forms claim 10/3/0 vs
    actual 7/6/0; Links row claims IA=1 but topic 6 has 0 IA (and invents a 6.3);
    topics 2, 3, 9 are missing entirely; Navigation range says 12.1–12.14 (catalog
    ends 12.11). Source: `README.md:302-318` vs `criteria.rs:17-124`.

14. **No test pins the 32-set to any external source.** `criteria.rs` tests assert
    only the partition and the count 106 (`criteria.rs:185-231`); nothing
    cross-checks the IA membership against vendored or official data.
    Source: `rgaa-rs/crates/rgaa-core/src/criteria.rs:185-231`.

## Verdict

**Contradicted** (as a fidelity claim). The official RGAA 4.1.2 methodology has
no per-test or per-criterion automation classification — `validé`/`non validé`
in DINUM's data is a pass/fail outcome, not an automation flag — so no official
32/73/1 split exists to be faithful *to*. The set is hand-authored (scaffold
2026-08-08, hand-edited through 2026-09-19), acknowledged in-repo as only
~20/106-justified, and the repo's only automation artifact — a keyword heuristic
producing 39/45/22 — disagrees with it on named criteria (3.1; 10.2/10.11/10.14).
The specific ticket edges are clean: 13.6 is IA, and all 14 `VISUAL_CRITERIA`
are IA (comment typos only). The 32/73/1 split is an engineering routing
heuristic, not a methodology-derived classification, and nothing currently tests
or documents that distinction at the point of use.
