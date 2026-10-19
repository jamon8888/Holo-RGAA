#!/usr/bin/env python3
"""Lot V — LOT DE VALIDATION du test 5.2.1 : jamais vu par aucune consigne.

Les consignes v1, v2 et v3 ont été écrites en regardant les 36 cartes de `resume_tableau.py`.
Un score obtenu sur ce lot-là ne vaut donc que comme ajustement. Ce lot-ci sert à mesurer la
consigne arrêtée **une seule fois**, sur du matériel neuf : six tableaux complexes inédits et
trente-six résumés inédits. Il perd cette valeur dès qu'une consigne est retouchée après l'avoir vu.

Trente-six cartes, dix-huit pertinentes et dix-huit non pertinentes attendues : repère trivial de
50 %. Quatre ancres. Les familles de défauts reprennent celles du lot d'ajustement : niveau omis,
structure fausse, sujet seul, générique, consigne de lecture, constat tiré des valeurs.

SIX CARTES TRANCHENT UNE AMBIGUÏTÉ CONNUE. Sur le lot d'ajustement, l'étiqueteur accepte « par
entrepôt et par référence » (u33) mais rejette « classés par catégorie » (u22) et « groupées par
département » (u11), des formulations de même forme. Une carte par tableau emploie ici cette forme
— un niveau désigné comme dimension, sans effectif ni énumération — et leurs étiquettes diront
laquelle des deux lectures est la bonne. `pred` suit la lecture de la consigne v3 (pertinent).

    python3 resume_tableau_valid.py OUT_DIR
"""
import html, json, os, sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import images_resume_valid
from resume_tableau import CTX, DEFINITION, LABEL, QUESTION, RULE

# (image, résumé proposé, prédiction P/N, nature, ancre ?)
ROWS = [
    # --- ca_region_canal : Région x (En ligne / En magasin) x (CA / Commandes) ---
    ("ca_region_canal",
     "Chiffre d'affaires et nombre de commandes de cinq régions, donnés séparément pour les ventes "
     "en ligne et pour les ventes en magasin.",
     "P", "nature et structure (ancre)", 1),
    ("ca_region_canal",
     "Pour chacun des deux canaux de vente, en ligne et en magasin, le tableau indique, région par "
     "région, le chiffre d'affaires et le nombre de commandes.",
     "P", "nature et structure, forme distributive", 0),
    ("ca_region_canal",
     "Ventes par région, réparties par canal, avec pour chaque canal le chiffre d'affaires et le "
     "nombre de commandes.",
     "P", "dimension nommée sans effectif — carte d'arbitrage", 0),
    ("ca_region_canal",
     "Chiffre d'affaires de cinq régions, pour les ventes en ligne et pour les ventes en magasin.",
     "N", "niveau omis : le nombre de commandes", 0),
    ("ca_region_canal",
     "Les ventes de l'entreprise.",
     "N", "sujet seul, rien sur la structure", 0),
    ("ca_region_canal",
     "Tableau de cinq lignes et de cinq colonnes, avec des en-têtes sur deux niveaux.",
     "N", "mise en page sans le sujet", 0),

    # --- absences_services : Service/Équipe x (Maladie / Congés / Formation) ---
    ("absences_services",
     "Jours d'absence de six équipes réparties en trois services ; pour chaque équipe, les absences "
     "pour maladie, pour congés et pour formation.",
     "P", "nature et structure", 0),
    ("absences_services",
     "Absences décomptées en jours, service par service et équipe par équipe, selon qu'il s'agit de "
     "maladie, de congés ou de formation.",
     "P", "nature et structure, forme distributive", 0),
    ("absences_services",
     "Nombre de jours d'absence par service et par équipe, détaillé par motif.",
     "P", "dimension nommée sans effectif — carte d'arbitrage", 0),
    ("absences_services",
     "Jours d'absence de six équipes, selon le motif.",
     "N", "niveau omis : le service", 0),
    ("absences_services",
     "Liste du personnel avec les coordonnées de chaque salarié.",
     "N", "factuellement faux (ancre)", 1),
    ("absences_services",
     "Le service Production cumule le plus de jours d'absence.",
     "N", "constat tiré des valeurs", 0),

    # --- notes_matieres : Matière x (Écrit / Oral) x (Note / Coefficient) ---
    ("notes_matieres",
     "Notes et coefficients de cinq matières, donnés séparément pour l'épreuve écrite et pour "
     "l'épreuve orale.",
     "P", "nature et structure", 0),
    ("notes_matieres",
     "Matière par matière, la note obtenue et le coefficient appliqué, à l'écrit puis à l'oral.",
     "P", "nature et structure, forme distributive", 0),
    ("notes_matieres",
     "Résultats par matière, répartis par type d'épreuve, avec la note et le coefficient.",
     "P", "dimension nommée sans effectif — carte d'arbitrage", 0),
    ("notes_matieres",
     "Notes obtenues dans cinq matières, à l'écrit et à l'oral.",
     "N", "niveau omis : le coefficient", 0),
    ("notes_matieres",
     "Les résultats de l'élève.",
     "N", "sujet seul, rien sur la structure", 0),
    ("notes_matieres",
     "Relevé des notes de cinq matières, chacune évaluée sur trois épreuves.",
     "N", "structure fausse : deux épreuves, pas trois", 0),

    # --- trafic_pages : Page x (Visiteurs uniques / Pages vues) x (T1 / T2) ---
    ("trafic_pages",
     "Fréquentation de cinq pages d'un site, avec pour chacune les visiteurs uniques et les pages "
     "vues, au premier puis au deuxième trimestre.",
     "P", "nature et structure", 0),
    ("trafic_pages",
     "Page par page, le nombre de visiteurs uniques et le nombre de pages vues, relevés sur deux "
     "trimestres.",
     "P", "nature et structure, forme distributive", 0),
    ("trafic_pages",
     "Audience par page, déclinée par indicateur et par trimestre.",
     "P", "dimension nommée sans effectif — carte d'arbitrage", 0),
    ("trafic_pages",
     "Visiteurs uniques de cinq pages, au premier et au deuxième trimestre.",
     "N", "niveau omis : les pages vues", 0),
    ("trafic_pages",
     "Tableau de statistiques.",
     "N", "générique (ancre)", 1),
    ("trafic_pages",
     "La page Catalogue attire bien plus de visiteurs que la page Accueil.",
     "N", "constat tiré des valeurs", 0),

    # --- recoltes_parcelles : Exploitation/Parcelle x (2024 / 2025) x (Surface / Rendement) ---
    ("recoltes_parcelles",
     "Surface et rendement de cinq parcelles réparties entre deux exploitations, Les Ormes et La "
     "Fontaine ; pour chaque parcelle, les relevés de 2024 puis ceux de 2025.",
     "P", "nature et structure (ancre)", 1),
    ("recoltes_parcelles",
     "Exploitation par exploitation et parcelle par parcelle, la surface et le rendement relevés "
     "en 2024 puis en 2025.",
     "P", "nature et structure, forme distributive", 0),
    ("recoltes_parcelles",
     "Surfaces et rendements par exploitation et par parcelle, pour deux campagnes.",
     "P", "dimension nommée sans effectif — carte d'arbitrage", 0),
    ("recoltes_parcelles",
     "Rendement de cinq parcelles en 2024 et en 2025.",
     "N", "deux niveaux omis : la surface et l'exploitation", 0),
    ("recoltes_parcelles",
     "Les parcelles de l'exploitation.",
     "N", "sujet seul, rien sur la structure", 0),
    ("recoltes_parcelles",
     "Surface et rendement de cinq parcelles, relevés chaque année depuis 2020.",
     "N", "structure fausse : deux campagnes, pas une série annuelle", 0),

    # --- interventions_equipes : Équipe x (Urgentes / Planifiées) x (Nombre / Délai moyen) ---
    ("interventions_equipes",
     "Interventions de quatre équipes, distinguées selon qu'elles sont urgentes ou planifiées ; "
     "dans chacun de ces deux cas, leur nombre et leur délai moyen.",
     "P", "nature et structure", 0),
    ("interventions_equipes",
     "Équipe par équipe, le nombre d'interventions et le délai moyen, pour les interventions "
     "urgentes comme pour les interventions planifiées.",
     "P", "nature et structure, forme distributive", 0),
    ("interventions_equipes",
     "Interventions par équipe, réparties par type, avec le nombre et le délai moyen.",
     "P", "dimension nommée sans effectif — carte d'arbitrage", 0),
    ("interventions_equipes",
     "Nombre d'interventions de quatre équipes, urgentes et planifiées.",
     "N", "niveau omis : le délai moyen", 0),
    ("interventions_equipes",
     "Consultez ce tableau pour suivre l'activité des équipes.",
     "N", "consigne de lecture", 0),
    ("interventions_equipes",
     "Interventions de quatre équipes réparties en trois catégories d'urgence.",
     "N", "structure fausse : deux catégories, pas trois", 0),
]


def rubric():
    return f"""<p>Chaque carte porte sur le <strong>test RGAA 5.2.1</strong>, mot pour mot :
<em>« Pour chaque tableau de données complexe ayant un résumé, celui-ci est-il pertinent ? »</em></p>
<p><strong>Définition du glossaire RGAA 4.1.2</strong>, mot pour mot :<br>
<em>« {html.escape(DEFINITION)} »</em></p>
<p>Autrement dit, le résumé dit <strong>de quoi il s'agit</strong> et <strong>comment le tableau est
bâti</strong>. Il n'a <strong>pas</strong> à reprendre les valeurs des cellules : celles-ci sont portées
par l'association de chaque cellule à ses en-têtes (critères 5.6 et 5.7).</p>
<p>Les six tableaux montrés sont des tableaux de données complexes : leurs en-têtes ne tiennent pas
dans la seule première ligne et la seule première colonne.</p>
<p>Jugez chaque résumé <strong>selon cette définition</strong>, et dites dans la justification ce qui,
dans l'image, vous a fait répondre. <strong>La règle est rappelée sur chaque carte</strong>, juste
au-dessus de la question.</p>"""


def build(out):
    os.makedirs(out, exist_ok=True)
    images_resume_valid.make(os.path.join(out, "img"))
    items = [dict(id=f"V{n:03d}", qid=f"v{n:02d}", image=f"img/{img}.jpg", ctx=CTX, alt=txt,
                  pred=p, kind=k, anchor=bool(a), test="5.2.1", label=LABEL, question=QUESTION,
                  rule=RULE)
             for n, (img, txt, p, k, a) in enumerate(ROWS, 1)]
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
          f"ancres {sum(x['anchor'] for x in items)} | "
          f"cartes d'arbitrage {sum(1 for x in items if 'arbitrage' in x['kind'])}")


if __name__ == "__main__":
    if len(sys.argv) != 2:
        sys.exit(__doc__)
    build(sys.argv[1])
