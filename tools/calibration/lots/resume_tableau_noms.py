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
NEUVES, sur quatre tableaux que l'étiqueteur n'a jamais vus (images_resume.py les dessinait déjà).
Douze pertinentes, douze non pertinentes : repère trivial de 50 %. Quatre ancres dont une fausse.

    python3 resume_tableau_noms.py OUT_DIR
"""
import html, json, os, random, sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import images_resume

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
    # --- meteo_villes : Ville x (Températures : Minimale/Maximale ; Précipitations : Hauteur/Jours) ---
    ("meteo_villes",
     "Relevés climatiques de six villes, Brest, Strasbourg, Nice, Lille, Clermont-Ferrand et "
     "Bordeaux ; pour chacune, les températures minimale et maximale, puis les précipitations, en "
     "hauteur et en nombre de jours de pluie.",
     "P", "villes et colonnes nommées", 0),
    ("meteo_villes",
     "Températures, minimale et maximale, et précipitations, en hauteur et en jours de pluie, de "
     "Brest, Strasbourg, Nice, Lille, Clermont-Ferrand et Bordeaux, à raison d'une ville par ligne.",
     "P", "villes et colonnes nommées, formulation brève", 0),
    ("meteo_villes",
     "Climat de six villes. Les lignes sont les villes : Brest, Strasbourg, Nice, Lille, "
     "Clermont-Ferrand et Bordeaux ; les colonnes sont groupées en températures, minimale puis "
     "maximale, et en précipitations, hauteur puis jours de pluie.",
     "P", "villes et colonnes nommées, formulation longue", 0),
    ("meteo_villes",
     "Relevés climatiques de six villes ; pour chacune, les températures et les précipitations.",
     "N", "villes non nommées, colonnes non détaillées", 0),
    ("meteo_villes",
     "Températures minimale et maximale, et précipitations en hauteur et en jours de pluie, de six "
     "villes françaises.",
     "N", "colonnes nommées mais villes non nommées", 0),
    ("meteo_villes",
     "Prévisions météorologiques de la semaine à venir pour la ville de Brest.",
     "N", "factuellement faux (ancre)", 1),

    # --- horaires_trains : Gare x (Semaine, Week-end) x (Matin, Soir) ---
    ("horaires_trains",
     "Horaires de passage des trains dans cinq gares, Auteuil, Belleville, Chantemerle, Dourdan et "
     "Épinay ; pour chacune, l'heure du matin et l'heure du soir, en semaine puis le week-end.",
     "P", "gares et colonnes nommées", 0),
    ("horaires_trains",
     "Passages du matin et du soir, en semaine et le week-end, aux gares d'Auteuil, Belleville, "
     "Chantemerle, Dourdan et Épinay.",
     "P", "gares et colonnes nommées, formulation brève", 0),
    ("horaires_trains",
     "Horaires de trains. Chaque ligne correspond à une gare : Auteuil, Belleville, Chantemerle, "
     "Dourdan, Épinay ; les colonnes sont groupées en semaine puis week-end, et chacun de ces deux "
     "cas se décompose en matin puis soir.",
     "P", "gares et colonnes nommées, formulation longue", 0),
    ("horaires_trains",
     "Horaires de passage des trains dans cinq gares, le matin et le soir, en semaine et le "
     "week-end.",
     "N", "colonnes nommées mais gares non nommées", 0),
    ("horaires_trains",
     "Horaires des trains aux gares d'Auteuil, Belleville, Chantemerle, Dourdan et Épinay.",
     "N", "gares nommées mais colonnes omises", 0),
    ("horaires_trains",
     "Tableau des horaires.",
     "N", "générique (ancre)", 1),

    # --- bulletin_trimestres : Élève x (Trimestre 1, Trimestre 2) x (Moyenne, Rang) ---
    ("bulletin_trimestres",
     "Résultats scolaires de six élèves, Alice Berger, Bruno Caron, Chloé Dufour, David Esnault, "
     "Elsa Favre et Farid Guérin ; pour chacun, la moyenne et le rang aux premier et deuxième "
     "trimestres.",
     "P", "élèves et colonnes nommés (ancre)", 1),
    ("bulletin_trimestres",
     "Moyennes et rangs, au trimestre 1 puis au trimestre 2, d'Alice Berger, Bruno Caron, Chloé "
     "Dufour, David Esnault, Elsa Favre et Farid Guérin.",
     "P", "élèves et colonnes nommés, formulation brève", 0),
    ("bulletin_trimestres",
     "Bulletin de six élèves : Alice Berger, Bruno Caron, Chloé Dufour, David Esnault, Elsa Favre, "
     "Farid Guérin. Les colonnes sont groupées par trimestre, le premier puis le second, chacun "
     "donnant la moyenne puis le rang.",
     "P", "élèves et colonnes nommés, formulation longue", 0),
    ("bulletin_trimestres",
     "Résultats scolaires de six élèves ; pour chacun, la moyenne et le rang à deux trimestres.",
     "N", "élèves non nommés", 0),
    ("bulletin_trimestres",
     "Moyennes et rangs de la classe, pour chaque trimestre.",
     "N", "élèves non nommés, structure partielle", 0),
    ("bulletin_trimestres",
     "Emploi du temps de la classe, avec les matières et les salles.",
     "N", "factuellement faux", 0),

    # --- budget_services : Service/Poste x (2024, 2025) x (Prévu, Réalisé) ---
    ("budget_services",
     "Budget de deux services, Technique et Culture, détaillé en trois postes chacun ; pour chaque "
     "poste, le montant prévu et le montant réalisé en 2024 puis en 2025.",
     "P", "services, années et colonnes nommés ; postes comptés", 0),
    ("budget_services",
     "Montants prévus et réalisés, en 2024 et en 2025, des postes de dépense des services "
     "Technique et Culture.",
     "P", "services, années et colonnes nommés, formulation brève", 0),
    ("budget_services",
     "Budget par service. Les lignes sont groupées en deux services, Technique et Culture, chacun "
     "subdivisé en postes ; les colonnes sont groupées par année, 2024 puis 2025, et chaque année "
     "se décompose en prévu et réalisé.",
     "P", "services, années et colonnes nommés, formulation longue", 0),
    ("budget_services",
     "Budget de deux services, détaillé en trois postes chacun ; pour chaque poste, le prévu et le "
     "réalisé de deux années.",
     "N", "services et années non nommés", 0),
    ("budget_services",
     "Budget des services Technique et Culture, poste par poste.",
     "N", "services nommés mais colonnes omises", 0),
    ("budget_services",
     "Le budget de la collectivité.",
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
    images_resume.make(os.path.join(out, "img"))
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
