#!/usr/bin/env python3
"""Generate the coverage map from engine_plan.json.

The map answers the tooling layer's own question: for each RGAA criterion,
which instrument decides it, and what a *missing* finding proves.

Never edit the generated file by hand. Regenerate:

    ./scripts/generate-coverage-map.py

Check for drift (what CI runs):

    ./scripts/generate-coverage-map.py --check
"""

import argparse
import json
import sys
from pathlib import Path

PLUGIN_ROOT = Path(__file__).resolve().parent.parent
REPO_ROOT = PLUGIN_ROOT.parents[2]
SOURCE = (
    REPO_ROOT / "rgaa-rs/crates/rgaa-core/data/rgaa-4.1.2/engine_plan.json"
)
TARGET = PLUGIN_ROOT / "skills/usage/references/coverage-map.md"

# Engines the remediation loop never invokes: Holo needs the LLM, which only
# audit_url calls; Human needs a person.
OUT_OF_REACH = ("Holo", "Human")

NOTHING = "nothing"
PARTIAL = "partial"
CONFORMING = "conforming"

VERDICT_LABEL = {
    NOTHING: "**nothing** — not evaluated here",
    PARTIAL: "**nothing conclusive** — residual judgement",
    CONFORMING: "conforming",
}


def verdict(entry):
    """What a missing finding proves for this criterion."""
    if entry["primary"] in OUT_OF_REACH:
        return NOTHING
    residual = bool(entry.get("holo", "").strip()) or bool(
        entry.get("human_residual", "").strip()
    )
    return PARTIAL if residual else CONFORMING


def sort_key(criterion_id):
    theme, number = criterion_id.split(".", 1)
    return (int(theme), int(number))


def render(entries):
    entries = sorted(entries, key=lambda e: sort_key(e["criterion_id"]))
    counts = {NOTHING: 0, PARTIAL: 0, CONFORMING: 0}
    for entry in entries:
        counts[verdict(entry)] += 1

    out = []
    w = out.append
    w("<!-- GENERATED FILE — do not edit.")
    w("     Source: rgaa-rs/crates/rgaa-core/data/rgaa-4.1.2/engine_plan.json")
    w("     Regenerate: ./scripts/generate-coverage-map.py -->")
    w("")
    w("# Coverage map")
    w("")
    w("For each RGAA 4.1.2 criterion: which instrument decides it, and **what a")
    w("missing finding proves**. This is the tooling layer's own question — what a")
    w("criterion *requires* is domain knowledge and lives elsewhere.")
    w("")
    w("Generated from the engine plan, so it cannot drift from the engines.")
    w("")
    w("## The one number that matters")
    w("")
    w(f"Of {len(entries)} criteria, a clean result lets you conclude **conforming**")
    w(f"for **{counts[CONFORMING]}**.")
    w("")
    w(f"- **{counts[CONFORMING]}** fully decided here — silence means conforming.")
    w(f"- **{counts[PARTIAL]}** partially covered — these tools can find violations,")
    w("  but silence proves **nothing**: a residual judgement remains.")
    w(f"- **{counts[NOTHING]}** never evaluated by this loop — `Holo` needs the LLM")
    w("  (only `audit_url` calls it), `Human` needs a person.")
    w("")
    w(f"So {counts[PARTIAL] + counts[NOTHING]} of {len(entries)} criteria cannot be")
    w("called conforming on the strength of a clean loop. Report them as residue.")
    w("")
    w("## Legend")
    w("")
    w("| Column | Meaning |")
    w("|---|---|")
    w("| Engine | the instrument that primarily decides this criterion |")
    w("| Axe rules | axe-core rules mapped to it, if any |")
    w("| Silence proves | what a *missing* finding lets you conclude |")
    w("")
    w("## Criteria")
    w("")
    w("| Criterion | Engine | Axe rules | Silence proves |")
    w("|---|---|---|---|")
    for entry in entries:
        rules = ", ".join(f"`{r}`" for r in entry.get("axe_rules", [])) or "—"
        w(
            f"| {entry['criterion_id']} | `{entry['primary']}` | {rules} "
            f"| {VERDICT_LABEL[verdict(entry)]} |"
        )
    w("")
    return "\n".join(out)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--check",
        action="store_true",
        help="exit non-zero if the generated file is stale (for CI)",
    )
    args = parser.parse_args()

    if not SOURCE.exists():
        print(f"source not found: {SOURCE}", file=sys.stderr)
        return 2

    entries = json.loads(SOURCE.read_text(encoding="utf-8"))
    rendered = render(entries)

    if args.check:
        if not TARGET.exists():
            print(f"{TARGET.name} has never been generated", file=sys.stderr)
            return 1
        if TARGET.read_text(encoding="utf-8") != rendered:
            print(
                f"{TARGET.name} is stale — engine_plan.json changed.\n"
                "Run ./scripts/generate-coverage-map.py and commit the result.",
                file=sys.stderr,
            )
            return 1
        print(f"{TARGET.name} is up to date ({len(entries)} criteria)")
        return 0

    TARGET.parent.mkdir(parents=True, exist_ok=True)
    TARGET.write_text(rendered, encoding="utf-8")
    print(f"wrote {TARGET.relative_to(PLUGIN_ROOT)} ({len(entries)} criteria)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
