#!/usr/bin/env python3
"""
Construit la grille complète des 106 critères RGAA 4.1.2 classés en
déterministe / IA-assisté / manuel, à partir de la source officielle DINUM.

Usage (dans un environnement avec accès réseau à raw.githubusercontent.com,
ex. ta machine ou Claude Code local) :
    pip install requests --break-system-packages
    python3 build_rgaa_grid.py > grille-rgaa-106.csv

Le fichier source est celui référencé par le dépôt officiel :
https://github.com/DISIC/accessibilite.numerique.gouv.fr/tree/main/RGAA
"""

import csv
import re
import sys

import requests

SOURCE_URL = (
    "https://raw.githubusercontent.com/DISIC/"
    "accessibilite.numerique.gouv.fr/main/RGAA/criteres.json"
)

# --- Règle de classification --------------------------------------------
#
# DETERMINISTE : le test se résout par inspection structurelle du DOM/CSSOM
#   sans jugement sur le contenu -- présence/absence d'un attribut, structure
#   de balise, valeur numérique (contraste, ratio), langue déclarée, ordre
#   de tabulation calculé, etc. Couvre aussi les techniques d'interaction
#   scriptée (Phase 2 du plan : simulation clavier, soumission de formulaire,
#   reflow) tant que le verdict reste une comparaison structurelle, pas un
#   jugement de sens.
#
# IA_ASSISTE : le test exige un jugement sémantique sur un contenu textuel
#   donné (pertinence d'un alt, d'un titre de cadre, d'une transcription,
#   cohérence d'un intitulé de lien hors contexte). Objectivement évaluable
#   par comparaison contenu/contexte, mais pas par une règle simple --
#   candidat Holo3/GLiNER sous gate de confiance (Phase 3 du plan).
#
# MANUEL : nécessite un test humain réel (lecteur d'écran, jugement éditorial
#   non vérifiable automatiquement, cas d'usage en situation) -- reste en
#   checklist assistée (Phase 6, gate de signature humaine).
#
# Mots-clés heuristiques utilisés pour un premier classement automatique.
# CE CLASSEMENT AUTOMATIQUE EST UN POINT DE DEPART, PAS UN VERDICT FINAL --
# chaque ligne doit être relue à la main, en particulier les critères qui
# mêlent plusieurs tests de nature différente (ex. 1.1 est structurel pour
# la présence, mais 1.3 sur le même thème est sémantique pour la pertinence).

MOTS_IA = [
    "pertinent", "pertinente", "pertinents", "pertinentes",
    "cohérent", "cohérente", "cohérence",
    "compréhensible", "compréhensible",
]
MOTS_MANUEL = [
    "restituée par les technologies d’assistance",
    "restitué par les technologies d’assistance",
    "correctement restitué",
    "logique",
]


def classer(titre: str) -> str:
    t = titre.lower()
    if any(m in t for m in MOTS_MANUEL):
        return "MANUEL (à confirmer)"
    if any(m in t for m in MOTS_IA):
        return "IA_ASSISTE (à confirmer)"
    return "DETERMINISTE (à confirmer)"


def main():
    try:
        resp = requests.get(SOURCE_URL, timeout=30)
        resp.raise_for_status()
        data = resp.json()
    except Exception as exc:  # noqa: BLE001
        print(f"Échec de récupération de la source officielle : {exc}", file=sys.stderr)
        print(
            "Vérifie l'accès réseau à raw.githubusercontent.com depuis cet environnement.",
            file=sys.stderr,
        )
        sys.exit(1)

    writer = csv.writer(sys.stdout)
    writer.writerow([
        "thematique_num", "thematique_nom", "critere_num", "critere_id",
        "titre", "nb_tests", "classification_proposee", "wcag_refs",
    ])

    total = 0
    for topic in data["topics"]:
        tnum = topic["number"]
        tname = topic["topic"]
        for crit in topic["criteria"]:
            c = crit["criterium"]
            cnum = c["number"]
            crit_id = f"{tnum}.{cnum}"
            titre = re.sub(r"\[([^\]]+)\]\([^)]+\)", r"\1", c["title"])  # strip liens md
            nb_tests = len(c.get("tests", {}))
            wcag_refs = []
            for ref in c.get("references", []):
                if "wcag" in ref:
                    wcag_refs.extend(ref["wcag"])
            writer.writerow([
                tnum, tname, cnum, crit_id, titre, nb_tests,
                classer(titre), " | ".join(wcag_refs),
            ])
            total += 1

    print(f"# Total critères traités : {total} (doit être 106)", file=sys.stderr)


if __name__ == "__main__":
    main()
