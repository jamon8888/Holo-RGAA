#!/usr/bin/env python3
"""Lot T2 — test RGAA 5.2.1 : pertinence du résumé d'un tableau de données complexe.

Remplace les 8 cartes 5.2.1 du lot R, écartées par l'ADR 0002 : elles avaient été étiquetées
selon un critère plus strict que le référentiel, et deux d'entre elles portaient sur un tableau
qui n'était pas complexe, donc hors du champ du test.

Le lot T (mêmes tests, quatre autres tableaux) n'a rien mesuré : l'étiqueteur n'avait pas vu
l'encadré de règle en tête de feuille et a rejeté les 8 cartes, ancre positive comprise. D'où
deux changements ici : **la règle est rappelée sur CHAQUE carte**, juste au-dessus de la
question (champ `rule` de `sheet.py`), et les 8 cartes sont neuves, les anciennes étant
désormais connues de l'étiqueteur.

DIFFÉRENCE DE NATURE avec les lots C, D, G et R : **la feuille énonce la définition** du
glossaire RGAA. On ne cherche donc plus à découvrir le critère de l'étiqueteur, on mesure
l'application d'une règle écrite. C'est voulu (ADR 0002) et cela change ce que le lot prouve.

Trente-six cartes, dix-huit pertinentes et dix-huit non pertinentes attendues : repère trivial de
50 %. Six ancres. Neuf tableaux complexes. `pred` est la lecture du
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

RULE = ("Un résumé donne la NATURE et la STRUCTURE du tableau. Il n'a PAS à reprendre les valeurs "
        "des cellules : celles-ci sont portées par l'association de chaque cellule à ses en-têtes "
        "(critères 5.6 et 5.7). Définition du glossaire RGAA 4.1.2.")

# (image, résumé proposé, prédiction P/N, nature, ancre ?)
ROWS = [
    ("effectifs_filieres",
     "Effectifs de cinq filières aux rentrées 2024 et 2025 ; pour chaque rentrée, le nombre "
     "d'hommes et le nombre de femmes sont donnés séparément.",
     "P", "nature et structure", 0),
    ("effectifs_filieres",
     "Les effectifs étudiants de l'établissement.",
     "N", "nature seule, rien sur la structure", 0),

    ("tournoi_dom_ext",
     "Bilan de cinq équipes d'un tournoi, distingué selon que les matchs ont été joués à domicile "
     "ou à l'extérieur ; dans chacun de ces deux cas figurent le nombre de matchs joués, gagnés "
     "et perdus.",
     "P", "nature et structure", 0),
    ("tournoi_dom_ext",
     "Calendrier des prochaines rencontres du championnat, avec la date et le lieu de chaque match.",
     "N", "factuellement faux (ancre)", 1),

    ("tarifs_postaux",
     "Tarifs d'affranchissement par tranche de poids, pour la France puis pour l'international, "
     "chaque destination étant déclinée en envoi normal et en envoi suivi.",
     "P", "nature et structure", 0),
    ("tarifs_postaux",
     "Tableau des tarifs.",
     "N", "générique (ancre)", 1),

    ("conso_eau",
     "Consommation d'eau de deux sites, Nord et Sud, détaillée bâtiment par bâtiment ; pour chaque "
     "bâtiment, le volume consommé et son coût, en été puis en hiver.",
     "P", "nature et structure (ancre)", 1),
    ("conso_eau",
     "La serre du site Sud consomme à elle seule plus d'eau que tous les bureaux réunis.",
     "N", "avis tiré des valeurs, pas une description", 0),
]

# Deuxième série (u09 à u36) : cinq autres tableaux complexes, et surtout des cartes DIFFICILES.
# Le lot T2 à 8 cartes ne discriminait rien (étiqueteur, modèle et clé d'accord sur les 8).
# D'où quatre familles de cas limites : structure partielle (un seul des deux niveaux d'en-têtes),
# structure fausse, valeurs citées en plus d'un résumé par ailleurs valable (le référentiel ne
# les interdit pas), et description purement visuelle sans la nature.
ROWS += [
    # --- population_communes : Département/Commune x 1999/2024 ---
    ("population_communes",
     "Population de six communes réparties entre deux départements, l'Ardèche et le Gers ; pour "
     "chaque commune, le nombre d'habitants en 1999 puis en 2024.",
     "P", "nature et structure", 0),
    ("population_communes",
     "Évolution démographique de six communes de l'Ardèche et du Gers entre les recensements de "
     "1999 et de 2024 ; chaque ligne porte les deux relevés, par exemple 11 018 puis 12 406 "
     "habitants pour Aubenas.",
     "P", "nature et structure, valeurs citées en plus", 0),
    ("population_communes",
     "Nombre d'habitants, en 1999 et en 2024, de six communes groupées par département.",
     "P", "nature et structure, formulation brève", 0),
    ("population_communes",
     "La population des communes françaises.",
     "N", "nature seule, rien sur la structure", 0),
    ("population_communes",
     "Population de six communes, une ligne par commune.",
     "N", "structure partielle : les colonnes sont passées sous silence", 0),
    ("population_communes",
     "Population de six communes relevée chaque année de 1999 à 2024.",
     "N", "structure fausse : deux relevés, pas une série annuelle", 0),

    # --- musee_horaires : Jour x (Basse/Haute saison) x (Ouverture/Fermeture) ---
    ("musee_horaires",
     "Horaires d'ouverture et de fermeture d'un musée pour chaque jour de la semaine, donnés "
     "séparément en basse saison et en haute saison.",
     "P", "nature et structure", 0),
    ("musee_horaires",
     "Pour la basse saison comme pour la haute saison, le tableau indique, jour par jour, l'heure "
     "d'ouverture et l'heure de fermeture du musée.",
     "P", "nature et structure, ordre inversé", 0),
    ("musee_horaires",
     "Horaires du musée. Les lignes correspondent aux jours d'ouverture, du mardi au dimanche ; "
     "les colonnes sont groupées par saison, basse puis haute, et chaque saison se décompose en "
     "heure d'ouverture et heure de fermeture.",
     "P", "nature et structure, formulation longue", 0),
    ("musee_horaires",
     "Tableau de six lignes et cinq colonnes, avec des en-têtes sur deux niveaux.",
     "N", "structure visuelle sans la nature", 0),
    ("musee_horaires",
     "Informations pratiques.",
     "N", "générique (ancre)", 1),
    ("musee_horaires",
     "Le musée ouvre plus tard et ferme plus tôt en basse saison qu'en haute saison.",
     "N", "constat tiré des valeurs, pas une description", 0),

    # --- ventes_produits : Catégorie/Produit x T1..T4 ---
    ("ventes_produits",
     "Ventes en unités de six produits, groupés en deux catégories, Jardin et Cuisine ; pour "
     "chaque produit, les ventes de chacun des quatre trimestres.",
     "P", "nature et structure", 0),
    ("ventes_produits",
     "Ventes trimestrielles de six produits classés par catégorie, avec une colonne par trimestre.",
     "P", "nature et structure, formulation brève", 0),
    ("ventes_produits",
     "Détail des ventes, en unités, de six produits répartis en deux catégories ; chaque ligne "
     "couvre les quatre trimestres de l'année.",
     "P", "nature et structure", 0),
    ("ventes_produits",
     "Les ventes de l'année.",
     "N", "nature seule, rien sur la structure", 0),
    ("ventes_produits",
     "Ventes de six produits, trimestre par trimestre.",
     "N", "structure partielle : le regroupement par catégorie est omis", 0),
    ("ventes_produits",
     "Consultez ce tableau pour connaître le détail de nos ventes.",
     "N", "consigne de lecture", 0),

    # --- election_bureaux : Bureau x (3 candidats) x (Voix/%) ---
    ("election_bureaux",
     "Résultats d'une élection dans cinq bureaux de vote ; pour chacun des trois candidats, "
     "Mercier, Vasseur et Blanc, le nombre de voix et le pourcentage obtenus.",
     "P", "nature et structure", 0),
    ("election_bureaux",
     "Voix et pourcentages obtenus par trois candidats dans chacun des cinq bureaux de vote.",
     "P", "nature et structure, formulation brève", 0),
    ("election_bureaux",
     "Résultats par bureau de vote. Chaque ligne correspond à un bureau ; les colonnes sont "
     "groupées par candidat, et chaque candidat occupe deux colonnes, l'une pour les voix, "
     "l'autre pour le pourcentage.",
     "P", "nature et structure, formulation longue", 0),
    ("election_bureaux",
     "Liste des bureaux de vote de la commune, avec leur adresse et leurs horaires d'ouverture.",
     "N", "factuellement faux (ancre)", 1),
    ("election_bureaux",
     "Nombre de voix obtenues par chaque candidat dans les cinq bureaux.",
     "N", "structure partielle : la colonne des pourcentages est omise", 0),

    # --- stock_entrepots : Entrepôt/Référence x (Janvier/Février) x (Entrées/Sorties) ---
    ("stock_entrepots",
     "Mouvements de stock de cinq références réparties sur deux entrepôts, Lyon et Rouen ; pour "
     "chaque référence, les entrées et les sorties de janvier puis de février.",
     "P", "nature et structure (ancre)", 1),
    ("stock_entrepots",
     "Entrées et sorties de stock, par entrepôt et par référence, pour les mois de janvier et "
     "de février.",
     "P", "nature et structure, formulation brève", 0),
    ("stock_entrepots",
     "Mouvements de stock de cinq références, avec le solde de chaque mois.",
     "N", "structure fausse : il n'y a pas de colonne de solde", 0),
    ("stock_entrepots",
     "Le stock des entrepôts.",
     "N", "nature seule, rien sur la structure", 0),
    ("stock_entrepots",
     "La référence A-12 est celle qui tourne le plus vite à Lyon.",
     "N", "constat tiré des valeurs, pas une description", 0),
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
dans l'image, vous a fait répondre. <strong>La règle est rappelée sur chaque carte</strong>, juste
au-dessus de la question.</p>"""


def build(out):
    os.makedirs(out, exist_ok=True)
    images_resume.make(os.path.join(out, "img"))
    items = [dict(id=f"U{n:03d}", qid=f"u{n:02d}", image=f"img/{img}.jpg", ctx=CTX, alt=txt,
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
          f"ancres {sum(x['anchor'] for x in items)}")


if __name__ == "__main__":
    if len(sys.argv) != 2:
        sys.exit(__doc__)
    build(sys.argv[1])
