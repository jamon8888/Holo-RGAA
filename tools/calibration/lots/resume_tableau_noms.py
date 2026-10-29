#!/usr/bin/env python3
"""Lot W — test RGAA 5.2.1 selon le critère de l'étiqueteur : le résumé NOMME les en-têtes.

Le lot T (36 cartes) a donné un repère trivial de 81 % : l'étiqueteur n'a retenu que 7 résumés,
alors que la lecture du référentiel (ADR 0002) en attendait 18. L'écart n'était pas du bruit :
ses 7 cartes pertinentes sont exactement celles qui nomment les en-têtes (Nord et Sud, Jardin et
Cuisine, Mercier, Vasseur et Blanc, du mardi au dimanche), et ses 11 refus portent le même
motif (« il manque le nom des catégories »). ADR 0003 retient donc SON critère pour ce projet,
plus strict que le glossaire RGAA.

La règle R*, dérivée des 36 étiquettes du lot T :

  - le résumé nomme chaque en-tête dont les libellés ne sont pas de simples numéros d'ordre
    (catégories, sites, candidats, années, jours, prévu/réalisé, noms de villes, de gares...) ;
  - les libellés purement ordinaux (T1 à T4, Bureau 1 à Bureau 5) peuvent n'être que comptés ;
  - le dernier niveau de lignes IMBRIQUÉ dans un regroupement nommé (communes d'un département,
    produits d'une catégorie, postes d'un service) peut n'être que compté ;
  - le résumé ne reprend toujours pas les valeurs des cellules.

CIRCULARITÉ ASSUMÉE : R* a été ajustée sur les 36 étiquettes du lot T, donc l'accord entre R* et
ces 36 étiquettes ne prouve rien. Ce lot-ci est le test hors échantillon : vingt-quatre cartes
NEUVES, sur quatre tableaux (`images_noms.py`) qu'aucun lot antérieur n'a montrés à l'étiqueteur.
Un premier jet réutilisait quatre tableaux d'`images_resume.py` déjà vus dans la première version
du lot T (commit aa050ec) : il n'aurait pas été hors échantillon.
Douze pertinentes, douze non pertinentes : repère trivial de 50 %. Quatre ancres dont une fausse.

    python3 resume_tableau_noms.py OUT_DIR
"""
import html, json, os, random, sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import images_noms

LABEL = "Résumé proposé"
QUESTION = "Ce résumé de tableau est-il pertinent ?"
CTX = ("Voir l'image : le tableau de données complexe auquel ce résumé est associé. "
       "Le tableau est montré sans son résumé.")

RULE = ("Un résumé donne la NATURE et la STRUCTURE du tableau, et NOMME les en-têtes : il cite les "
        "libellés des colonnes et des regroupements (catégories, périodes, candidats, sites...), "
        "pas seulement leur nombre. Des libellés numérotés (T1 à T4, Bureau 1 à 5) et les lignes "
        "imbriquées dans un regroupement nommé peuvent n'être que comptés. Il ne reprend PAS les "
        "valeurs des cellules.")

# (image, résumé proposé, prédiction P/N, nature du cas, ancre ?)
ROWS = [
    # --- rendement_cultures : Région x (Blé, Maïs) x (Surface, Rendement) ---
    ("rendement_cultures",
     "Cultures de blé et de maïs dans cinq régions, Bretagne, Normandie, Occitanie, Centre-Val de "
     "Loire et Grand Est ; pour chaque culture, la surface et le rendement de chaque région.",
     "P", "régions et colonnes nommées", 0),
    ("rendement_cultures",
     "Surfaces et rendements du blé et du maïs en Bretagne, Normandie, Occitanie, Centre-Val de "
     "Loire et Grand Est.",
     "P", "régions et colonnes nommées, formulation brève", 0),
    ("rendement_cultures",
     "Production agricole par région. Les lignes sont les régions : Bretagne, Normandie, Occitanie, "
     "Centre-Val de Loire, Grand Est ; les colonnes sont groupées par culture, le blé puis le maïs, "
     "et chaque culture se décompose en surface puis rendement.",
     "P", "régions et colonnes nommées, formulation longue", 0),
    ("rendement_cultures",
     "Cultures de blé et de maïs dans cinq régions ; pour chaque culture, la surface et le "
     "rendement de chaque région.",
     "N", "régions non nommées", 0),
    ("rendement_cultures",
     "Surface et rendement de cultures dans les régions Bretagne, Normandie, Occitanie, "
     "Centre-Val de Loire et Grand Est.",
     "N", "régions nommées mais cultures non nommées", 0),
    ("rendement_cultures",
     "Calendrier des semis et des récoltes pour la saison à venir.",
     "N", "factuellement faux (ancre)", 1),

    # --- bibliotheques_prets : Bibliothèque x (Adultes, Jeunesse) x (Livres, DVD) ---
    ("bibliotheques_prets",
     "Prêts de cinq bibliothèques, Centrale, Jean-Jaurès, Les Lilas, Saint-Roch et Les Tilleuls ; "
     "pour chacune, les livres et les DVD empruntés par les adultes, puis par la jeunesse.",
     "P", "bibliothèques et colonnes nommées", 0),
    ("bibliotheques_prets",
     "Livres et DVD empruntés, par les adultes et par la jeunesse, dans les bibliothèques "
     "Centrale, Jean-Jaurès, Les Lilas, Saint-Roch et Les Tilleuls.",
     "P", "bibliothèques et colonnes nommées, formulation brève", 0),
    ("bibliotheques_prets",
     "Prêts par bibliothèque : Centrale, Jean-Jaurès, Les Lilas, Saint-Roch, Les Tilleuls, une par "
     "ligne. Les colonnes sont groupées par public, adultes puis jeunesse, et chaque public se "
     "décompose en livres puis DVD.",
     "P", "bibliothèques et colonnes nommées, formulation longue", 0),
    ("bibliotheques_prets",
     "Prêts de cinq bibliothèques ; pour chacune, les livres et les DVD empruntés par les adultes, "
     "puis par la jeunesse.",
     "N", "bibliothèques non nommées", 0),
    ("bibliotheques_prets",
     "Prêts des bibliothèques Centrale, Jean-Jaurès, Les Lilas, Saint-Roch et Les Tilleuls.",
     "N", "bibliothèques nommées mais colonnes omises", 0),
    ("bibliotheques_prets",
     "Tableau des prêts.",
     "N", "générique (ancre)", 1),

    # --- camping_tarifs : Hébergement/Formule x (Basse, Haute saison) x (Semaine, Week-end) ---
    ("camping_tarifs",
     "Tarifs d'un camping, répartis en deux types d'hébergement, Emplacement nu et Location, de "
     "trois formules chacun ; pour chaque formule, le prix à la semaine et le prix du week-end, en "
     "basse saison puis en haute saison.",
     "P", "types, saisons et colonnes nommés ; formules comptées (ancre)", 1),
    ("camping_tarifs",
     "Prix à la semaine et au week-end, en basse et en haute saison, des emplacements nus et des "
     "locations d'un camping.",
     "P", "types, saisons et colonnes nommés, formulation brève", 0),
    ("camping_tarifs",
     "Tarifs d'un camping. Les lignes sont groupées en deux types d'hébergement, Emplacement nu et "
     "Location, chacun subdivisé en formules ; les colonnes sont groupées par saison, basse puis "
     "haute, et chaque saison se décompose en semaine et week-end.",
     "P", "types, saisons et colonnes nommés, formulation longue", 0),
    ("camping_tarifs",
     "Tarifs d'un camping, répartis en deux types d'hébergement de trois formules chacun ; pour "
     "chaque formule, un prix à la semaine et un prix de week-end, selon la saison.",
     "N", "types et saisons non nommés", 0),
    ("camping_tarifs",
     "Tarifs d'un camping pour l'emplacement nu et la location, formule par formule.",
     "N", "types nommés mais colonnes omises", 0),
    ("camping_tarifs",
     "Programme des animations du camping pour l'été.",
     "N", "factuellement faux", 0),

    # --- labo_analyses : Site x (Physico-chimie : Température, pH ; Contaminants : Nitrates, Phosphates) ---
    ("labo_analyses",
     "Analyses de l'eau sur cinq sites de prélèvement, Les Écluses, Pont Neuf, Moulin Haut, Gué de "
     "l'Aube et La Source ; pour chacun, les mesures physico-chimiques, température et pH, puis "
     "les contaminants, nitrates et phosphates.",
     "P", "sites et colonnes nommés", 0),
    ("labo_analyses",
     "Mesures physico-chimiques, température et pH, et contaminants, nitrates et phosphates, des "
     "sites Les Écluses, Pont Neuf, Moulin Haut, Gué de l'Aube et La Source.",
     "P", "sites et colonnes nommés, formulation brève", 0),
    ("labo_analyses",
     "Qualité de l'eau. Une ligne par site de prélèvement : Les Écluses, Pont Neuf, Moulin Haut, "
     "Gué de l'Aube, La Source ; les colonnes sont groupées en physico-chimie, température puis "
     "pH, et en contaminants, nitrates puis phosphates.",
     "P", "sites et colonnes nommés, formulation longue", 0),
    ("labo_analyses",
     "Analyses de l'eau sur cinq sites de prélèvement ; pour chacun, les mesures physico-chimiques "
     "et les contaminants.",
     "N", "sites non nommés, colonnes non détaillées", 0),
    ("labo_analyses",
     "Analyses de l'eau aux sites Les Écluses, Pont Neuf, Moulin Haut, Gué de l'Aube et La Source.",
     "N", "sites nommés mais colonnes omises", 0),
    ("labo_analyses",
     "Les analyses de l'eau.",
     "N", "nature seule, rien sur la structure (ancre)", 1),
]

SEED = 5217


def rubric():
    return f"""<p>Chaque carte porte sur le <strong>test RGAA 5.2.1</strong> :
<em>« Pour chaque tableau de données complexe ayant un résumé, celui-ci est-il pertinent ? »</em></p>
<p>Un résumé donne la <strong>nature</strong> et la <strong>structure</strong> du tableau, et
<strong>nomme les en-têtes</strong> : il cite les libellés des colonnes et des regroupements
(catégories, périodes, candidats, sites…), pas seulement leur nombre. Des libellés numérotés
(T1 à T4, Bureau 1 à 5) et les lignes imbriquées dans un regroupement nommé peuvent n'être que
comptés. Il ne reprend <strong>pas</strong> les valeurs des cellules.</p>
<p>Ce critère est celui que vous avez appliqué au lot précédent (ADR 0003). Il est plus strict que
le glossaire RGAA : jugez chaque résumé selon lui, et dites dans la justification ce qui, dans
l'image, vous a fait répondre. <strong>La règle est rappelée sur chaque carte.</strong></p>"""


def build(out):
    os.makedirs(out, exist_ok=True)
    images_noms.make(os.path.join(out, "img"))
    rows = list(ROWS)
    random.Random(SEED).shuffle(rows)
    items = [dict(id=f"W{n:03d}", qid=f"w{n:02d}", image=f"img/{img}.jpg", ctx=CTX, alt=txt,
                  pred=p, kind=k, anchor=bool(a), test="5.2.1", label=LABEL, question=QUESTION,
                  rule=RULE)
             for n, (img, txt, p, k, a) in enumerate(rows, 1)]
    public = [{k: x[k] for k in ("qid", "id", "image", "ctx", "alt", "test", "label", "question", "rule")}
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
