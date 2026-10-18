#!/usr/bin/env python3
"""Lot T — test RGAA 5.2.1 : pertinence du résumé d'un tableau de données complexe.

Remplace les 8 cartes 5.2.1 du lot R, écartées par l'ADR 0002 : elles avaient été étiquetées
selon un critère plus strict que le référentiel, et deux d'entre elles portaient sur un tableau
qui n'était pas complexe, donc hors du champ du test.

DIFFÉRENCE DE NATURE avec les lots C, D, G et R : **la feuille énonce la définition** du
glossaire RGAA. On ne cherche donc plus à découvrir le critère de l'étiqueteur, on mesure
l'application d'une règle écrite. C'est voulu (ADR 0002) et cela change ce que le lot prouve.

Huit cartes, quatre pertinentes et quatre non pertinentes attendues : repère trivial de 50 %.
Trois ancres. Quatre tableaux complexes, deux cartes chacun. `pred` est la lecture du
référentiel, pas une vérité d'étiquetage.

    python3 resume_tableau.py OUT_DIR
"""
import html, json, os, sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import images_resume

LABEL = "Résumé proposé"
QUESTION = "Ce résumé de tableau est-il pertinent ?"
CTX = ("Voir l'image : le tableau de données complexe auquel ce résumé est associé. "
       "Le tableau est montré sans son résumé.")

# (image, résumé proposé, prédiction P/N, nature, ancre ?)
ROWS = [
    ("meteo_villes",
     "Relevés météorologiques de six villes : pour chaque ville, la température minimale et la "
     "température maximale en degrés Celsius, puis les précipitations exprimées en hauteur d'eau "
     "et en nombre de jours de pluie.",
     "P", "nature et structure", 0),
    ("meteo_villes",
     "La météo dans plusieurs villes françaises.",
     "N", "nature seule, rien sur la structure", 0),

    ("horaires_trains",
     "Horaires de départ des trains dans cinq gares, donnés séparément pour la semaine et pour le "
     "week-end, et pour chacun de ces deux cas le départ du matin et celui du soir.",
     "P", "nature et structure", 0),
    ("horaires_trains",
     "Tarifs des abonnements de transport selon l'âge du voyageur.",
     "N", "factuellement faux (ancre)", 1),

    ("bulletin_trimestres",
     "Résultats de six élèves sur deux trimestres : pour chaque trimestre, la moyenne obtenue et "
     "le rang dans la classe.",
     "P", "nature et structure", 0),
    ("bulletin_trimestres",
     "Tableau récapitulatif des résultats.",
     "N", "générique (ancre)", 1),

    ("budget_services",
     "Budget de deux services, Technique et Culture, détaillé par poste de dépense ; pour chaque "
     "poste, le montant prévu et le montant réalisé, en 2024 puis en 2025.",
     "P", "nature et structure (ancre)", 1),
    ("budget_services",
     "Le service Technique a mieux tenu son budget que le service Culture.",
     "N", "avis, pas une description", 0),
]

DEFINITION = (
    "Un résumé est un passage de texte associé à un tableau de données complexe. Il permet de "
    "donner des informations sur la nature et la structure du tableau afin d'en faciliter "
    "l'utilisation par les utilisateurs de technologies d'assistance par exemple."
)


def rubric():
    return f"""<p>Chaque carte porte sur le <strong>test RGAA 5.2.1</strong>, mot pour mot :
<em>« Pour chaque tableau de données complexe ayant un résumé, celui-ci est-il pertinent ? »</em></p>
<p><strong>Définition du glossaire RGAA 4.1.2</strong>, mot pour mot :<br>
<em>« {html.escape(DEFINITION)} »</em></p>
<p>Autrement dit, le résumé dit <strong>de quoi il s'agit</strong> et <strong>comment le tableau est
bâti</strong>. Il n'a <strong>pas</strong> à reprendre les valeurs des cellules : celles-ci sont portées
par l'association de chaque cellule à ses en-têtes (critères 5.6 et 5.7).</p>
<p>Les quatre tableaux montrés sont des tableaux de données complexes : leurs en-têtes ne tiennent
pas dans la seule première ligne et la seule première colonne.</p>
<p>Jugez chaque résumé <strong>selon cette définition</strong>, et dites dans la justification ce qui,
dans l'image, vous a fait répondre.</p>"""


def build(out):
    os.makedirs(out, exist_ok=True)
    images_resume.make(os.path.join(out, "img"))
    items = [dict(id=f"T{n:03d}", qid=f"t{n:02d}", image=f"img/{img}.jpg", ctx=CTX, alt=txt,
                  pred=p, kind=k, anchor=bool(a), test="5.2.1", label=LABEL, question=QUESTION)
             for n, (img, txt, p, k, a) in enumerate(ROWS, 1)]
    public = [{k: x[k] for k in ("qid", "id", "image", "ctx", "alt", "test", "label", "question")}
              for x in items]
    key = {x["qid"]: {k: x[k] for k in ("id", "pred", "kind", "anchor", "test")} for x in items}
    json.dump(public, open(os.path.join(out, "items.json"), "w", encoding="utf-8"),
              ensure_ascii=False, indent=1)
    json.dump(key, open(os.path.join(out, "key.json"), "w", encoding="utf-8"),
              ensure_ascii=False, indent=1)
    open(os.path.join(out, "rubric.html"), "w", encoding="utf-8").write(rubric())
    p = sum(1 for x in items if x["pred"] == "P")
    print(f"{len(items)} cartes | attendues pertinentes {p}, non pertinentes {len(items) - p} | "
          f"repère trivial {max(p, len(items) - p) / len(items):.0%} | "
          f"ancres {sum(x['anchor'] for x in items)}")


if __name__ == "__main__":
    if len(sys.argv) != 2:
        sys.exit(__doc__)
    build(sys.argv[1])
