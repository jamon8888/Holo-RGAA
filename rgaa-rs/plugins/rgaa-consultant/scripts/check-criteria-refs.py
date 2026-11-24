#!/usr/bin/env python3
"""Garde-fou des références de critères RGAA dans les skills, commandes et agents.

1. Tout numéro de critère cité (tableaux, `criterion_mapping`, « RGAA x.y ») existe dans
   le référentiel officiel embarqué (rgaa-core/data/rgaa-4.1.2/criteres.json).
2. Les types de tests guidés citent les critères que le texte du RGAA 4.1.2 leur attribue
   (oracle écrit à la main depuis la méthode officielle, pas dérivé de la skill).

Usage : python3 -I check-criteria-refs.py   (code de sortie 1 en cas d'écart)
"""
import json
import re
import sys
from pathlib import Path

PLUGIN = Path(__file__).resolve().parent.parent
DATA = PLUGIN.parent.parent / "crates" / "rgaa-core" / "data" / "rgaa-4.1.2" / "criteres.json"

# Oracle : intitulés officiels RGAA 4.1.2 (7.3 scripts au clavier, 10.3 compréhensible sans
# styles, 10.7 focus visible, 12.8 ordre de tabulation, 12.9 piège au clavier, 3.2/3.3
# contrastes). Les cibles tactiles ne sont pas un critère RGAA 4.1.2 (WCAG 2.2, 2.5.8).
GUIDED = {
    "keyboard-navigation": {"7.3", "12.8", "12.9"},
    "focus-visibility": {"10.7"},
    "color-contrast": {"3.2", "3.3"},
    "touch-targets": set(),
    "reading-order": {"10.3", "12.8"},
}

ID = r"(?<![\d.])(1[0-3]|[1-9])\.(\d{1,2})(?![\d.])"


def catalog() -> set[str]:
    data = json.loads(DATA.read_text(encoding="utf-8"))
    return {f"{t['number']}.{c['criterium']['number']}" for t in data["topics"] for c in t["criteria"]}


def main() -> int:
    known = catalog()
    errors: list[str] = []
    files = [p for d in ("skills", "commands", "agents") for p in (PLUGIN / d).rglob("*.md")]
    for path in files:
        if path.match("skills/criteria/SKILL.md"):
            continue  # généré depuis le référentiel, vérifié par gen-criteria-skill.py --check
        in_mapping = False
        for n, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if stripped.startswith("criterion_mapping:"):
                in_mapping = True
                continue
            candidates: list[str] = []
            if in_mapping and re.fullmatch(r"- " + ID, stripped):
                candidates = [f"{a}.{b}" for a, b in re.findall(r"(\d+)\.(\d+)", stripped)]
            else:
                in_mapping = in_mapping and stripped.startswith("- ")
                if line.lstrip().startswith("|"):
                    for cell in line.split("|"):
                        cell = cell.strip()
                        if re.fullmatch(r"(?:\d+\.\d+(?:[–-]\d+\.\d+)?)(?:,\s*\d+\.\d+(?:[–-]\d+\.\d+)?)*", cell):
                            candidates += [f"{a}.{b}" for a, b in re.findall(r"(\d+)\.(\d+)", cell)]
                candidates += [f"{a}.{b}" for a, b in re.findall(r"RGAA\s+" + ID, line)]
            for cid in candidates:
                if cid not in known:
                    errors.append(f"{path.relative_to(PLUGIN)}:{n}: critère {cid} inexistant dans le RGAA 4.1.2")
    guided = (PLUGIN / "skills" / "guided-test" / "SKILL.md").read_text(encoding="utf-8")
    for test, expected in GUIDED.items():
        row = re.search(r"^\|\s*`" + re.escape(test) + r"`\s*\|([^|]*)\|", guided, re.M)
        if not row:
            errors.append(f"guided-test : type `{test}` absent du tableau")
            continue
        got = {f"{a}.{b}" for a, b in re.findall(ID, row.group(1))}
        if got != expected:
            errors.append(f"guided-test : `{test}` cite {sorted(got)}, le RGAA 4.1.2 attend {sorted(expected)}")
    for e in errors:
        print(f"❌ {e}")
    return 1 if errors else 0


if __name__ == "__main__":
    sys.exit(main())
