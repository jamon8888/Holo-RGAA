#!/usr/bin/env python3
"""Régénère skills/criteria/SKILL.md à partir du référentiel embarqué dans rgaa-core.

Source unique : rgaa-core/data/rgaa-4.1.2/criteres.json (texte officiel RGAA 4.1.2)
et engine_plan.json (moteur propriétaire de chaque critère). Le fichier produit ne doit
jamais être édité à la main : plugin-contract.sh le régénère et échoue s'il diffère.

Usage : python3 -I gen-criteria-skill.py [--check]
"""
import json
import re
import sys
from pathlib import Path

PLUGIN = Path(__file__).resolve().parent.parent
DATA = PLUGIN.parent.parent / "crates" / "rgaa-core" / "data" / "rgaa-4.1.2"
OUT = PLUGIN / "skills" / "criteria" / "SKILL.md"

ENGINE = {
    "AxeCore": "axe-core",
    "Deterministic": "Contrôles déterministes",
    "Holo": "Preuve + IA (revue humaine)",
    "Human": "Humain (pré-tri machine)",
}


def plain(md: str) -> str:
    text = re.sub(r"\[([^\]]*)\]\([^)]*\)", r"\1", md)
    return " ".join(text.replace("|", "\\|").split())


def main() -> int:
    criteres = json.loads((DATA / "criteres.json").read_text(encoding="utf-8"))
    plan = {e["criterion_id"]: e for e in json.loads((DATA / "engine_plan.json").read_text(encoding="utf-8"))}
    counts: dict[str, int] = {}
    lines = [
        "---",
        "name: criteria",
        "description: Référence des 106 critères RGAA 4.1.2 — numéro, intitulé officiel, moteur qui porte le verdict, critères WCAG 2.1 associés. Use when the user asks what a criterion requires, how it is tested, or which criteria cover a topic.",
        "---",
        "",
        "<!-- FICHIER GÉNÉRÉ par scripts/gen-criteria-skill.py depuis rgaa-core/data/rgaa-4.1.2 — ne pas éditer à la main. -->",
        "",
        "# criteria — référentiel RGAA 4.1.2",
        "",
        "Intitulés : texte officiel du RGAA 4.1.2 (DINUM), sans les liens du glossaire. Pour la méthode",
        "officielle d'un test, appeler l'outil MCP `get_test_methodology` avec la clé du test (ex. `1.2.3`) :",
        "il retourne la méthode du RGAA, tirée de `RGAA/methodologies.json` (DISIC/",
        "accessibilite.numerique.gouv.fr, commit épinglé).",
        "",
        "**Moteur** : qui porte le verdict dans Holo-RGAA (`engine_plan.json`). Le moteur ne dit pas le",
        "niveau d'automatisation de chaque test : un test *Assisté* rend toujours `review` (la machine",
        "collecte la preuve, un relecteur décide), quel que soit le moteur. Aucun intitulé de ce",
        "fichier ne doit être reformulé de mémoire : en cas de doute, citer le texte officiel.",
        "",
    ]
    for topic in criteres["topics"]:
        n = topic["number"]
        lines += [f"## {n}. {topic['topic']}", "", "| Critère | Intitulé | Moteur | WCAG 2.1 |", "|---|---|---|---|"]
        for c in topic["criteria"]:
            crit = c["criterium"]
            cid = f"{n}.{crit['number']}"
            engine = plan[cid]["primary"]
            counts[engine] = counts.get(engine, 0) + 1
            wcag = []
            for ref in crit.get("references", []):
                wcag += [w.split(" ")[0] for w in ref.get("wcag", [])]
            lines.append(f"| {cid} | {plain(crit['title'])} | {ENGINE[engine]} | {', '.join(wcag) or '—'} |")
        lines.append("")
    total = sum(counts.values())
    lines += ["## Répartition par moteur", "", "| Moteur | Critères |", "|---|---|"]
    for key in ("AxeCore", "Deterministic", "Holo", "Human"):
        lines.append(f"| {ENGINE[key]} | {counts.get(key, 0)} |")
    lines += [f"| **Total** | **{total}** |", ""]
    content = "\n".join(lines)
    if "--check" in sys.argv:
        current = OUT.read_text(encoding="utf-8") if OUT.exists() else ""
        if current != content:
            print("skills/criteria/SKILL.md n'est pas à jour : lancer scripts/gen-criteria-skill.py", file=sys.stderr)
            return 1
        return 0
    OUT.write_text(content, encoding="utf-8")
    return 0


if __name__ == "__main__":
    sys.exit(main())
