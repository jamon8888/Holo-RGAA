#!/usr/bin/env python3
"""Lot de calibration — famille « description détaillée » (test RGAA 1.7.x).

Neuf images de données, trois ou quatre descriptions chacune (une ou deux complètes de rédaction
différente, une partielle ou vague, une fausse), plus quatre ancres : 39 cartes, 18 attendues
pertinentes, donc un repère trivial de 53,8 % (protocole §3 : >= 36 cartes, <= 60 %).

Aucune règle de pertinence n'est écrite dans la feuille : seul le texte officiel du test y
figure. La dernière fois qu'une règle a été imposée (#339), elle était fausse, et c'est la
justification de l'étiqueteur qui a établi le critère. `pred` ci-dessous est ma prédiction,
pas une vérité ; l'étiquetage humain fait référence.

    python3 description_detaillee.py OUT_DIR
"""
import json, os, random, sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import images

CTX = {
    "bar_h": "Mobilité — Le graphique ci-dessous présente la façon dont les habitants se rendent à leur travail.",
    "donut": "Énergie — Le graphique ci-dessous présente l'origine de l'électricité consommée dans la région en 2024.",
    "timeline": "Le projet — Notre histoire en quelques dates.",
    "orgchart": "L'association — Qui fait quoi ? Voici notre organisation.",
    "plan": "Venir à la bibliothèque — Le plan ci-dessous montre le trajet depuis la gare.",
    "bins": "Tri des déchets — Les couleurs des bacs de la commune.",
    "banner_soldes": "Accueil de la librairie Page 12 — Offres du moment.",
    "logo_atelier": "À propos — L'Atelier Lumière, créateur de luminaires depuis 2015.",
    "tarifs": "Piscine municipale — Horaires et tarifs. L'image ci-dessous indique les prix d'entrée.",
}

# (image, description détaillée, prédiction P/N, nature, ancre ?)
ROWS = [
    ("bar_h", "Histogramme horizontal à quatre barres, de la plus longue à la plus courte. La voiture domine avec 48 % des déplacements domicile-travail, suivie des transports en commun (27 %), de la marche (13 %) et du vélo (12 %). L'axe horizontal va de 0 à 58 %.", "P", "complète, structurée", 0),
    ("bar_h", "Quatre barres horizontales : voiture 48 %, transports en commun 27 %, marche 13 %, vélo 12 %, par ordre décroissant.", "P", "complète, brève", 0),
    ("bar_h", "La voiture est le mode de transport le plus utilisé.", "N", "tête de classement seule", 0),
    ("bar_h", "Histogramme vertical montrant l'évolution des ventes de vélos entre 2019 et 2024.", "N", "faux sujet", 0),
    ("donut", "Diagramme en anneau à trois secteurs : nucléaire 65 % (bleu), énergies renouvelables 25 % (vert), gaz 10 % (orange). Le titre est « Origine de l'électricité en 2024 ».", "P", "complète, structurée", 0),
    ("donut", "Répartition de l'électricité en 2024 : le nucléaire en fournit près des deux tiers (65 %), les renouvelables un quart (25 %) et le gaz le reste (10 %).", "P", "complète, rédigée", 0),
    ("donut", "Un graphique circulaire coloré.", "N", "générique", 0),
    ("donut", "Nucléaire 25 %, renouvelables 65 %, gaz 10 %.", "N", "valeurs inversées", 0),
    ("timeline", "Frise chronologique en quatre étapes : 2019, l'idée ; 2021, le prototype ; 2023, le lancement ; 2024, le cap des 10 000 utilisateurs. Les jalons alternent au-dessus et au-dessous d'une ligne horizontale.", "P", "complète, structurée", 0),
    ("timeline", "Quatre jalons du projet : idée (2019), prototype (2021), lancement (2023), 10 000 utilisateurs (2024).", "P", "complète, brève", 0),
    ("timeline", "Le projet a connu plusieurs étapes importantes.", "N", "sans jalons", 0),
    ("timeline", "Frise chronologique de l'histoire de l'entreprise de 1990 à 2010.", "N", "faux sujet", 0),
    ("orgchart", "Organigramme de l'association en trois cases : Claire Morel, présidente, en haut ; en dessous, reliés à elle, Hugo Faure, trésorier, à gauche, et Inès Roche, secrétaire, à droite.", "P", "complète, structurée", 0),
    ("orgchart", "Claire Morel préside l'association ; Hugo Faure (trésorier) et Inès Roche (secrétaire) sont placés sous sa responsabilité.", "P", "complète, rédigée", 0),
    ("orgchart", "Organigramme de l'association.", "N", "titre seul", 0),
    ("orgchart", "Hugo Faure préside l'association, avec Claire Morel comme trésorière et Inès Roche comme secrétaire.", "N", "rôles permutés", 0),
    ("plan", "Plan en trois étapes alignées de gauche à droite : la gare, la mairie (à 600 m de la gare), puis la bibliothèque (à 200 m de la mairie), soit 800 m au total.", "P", "complète, structurée", 0),
    ("plan", "Depuis la gare, marcher 600 m jusqu'à la mairie, puis 200 m jusqu'à la bibliothèque.", "P", "complète, brève", 0),
    ("plan", "Le plan montre comment aller à la bibliothèque.", "N", "sans distances", 0),
    ("plan", "La bibliothèque est à 50 m de la gare, sans détour.", "N", "faux", 0),
    ("bins", "Trois bacs de tri côte à côte : le bac jaune pour les emballages, le bac vert pour le verre et le bac gris pour les ordures ménagères.", "P", "complète, structurée", 0),
    ("bins", "Des bacs de couleurs différentes.", "N", "générique", 0),
    ("bins", "Le bac bleu est destiné au papier et au carton.", "N", "faux sujet", 0),
    ("banner_soldes", "Bannière bordeaux annonçant les soldes d'hiver à −30 %, du 8 au 28 janvier, à la librairie Page 12.", "P", "complète", 0),
    ("banner_soldes", "Soldes d'hiver : 30 % de réduction du 8 au 28 janvier, Librairie Page 12 (texte blanc et jaune sur fond bordeaux).", "P", "complète, avec mise en forme", 0),
    ("banner_soldes", "Une bannière promotionnelle.", "N", "générique", 0),
    ("banner_soldes", "Offre de bienvenue : −50 % sur le premier achat jusqu'au 31 mars.", "N", "faux", 0),
    ("logo_atelier", "Logo de l'Atelier Lumière : une ampoule jaune sur un culot gris, entourée de quatre traits de rayonnement, à gauche des mots « Atelier » et « Lumière » écrits en deux lignes.", "P", "complète", 0),
    ("logo_atelier", "Ampoule stylisée à gauche, nom « Atelier Lumière » à droite en deux lignes, la seconde en doré.", "P", "complète, brève", 0),
    ("logo_atelier", "Logo.", "N", "générique", 0),
    ("logo_atelier", "Logo d'une boulangerie, avec un croissant doré.", "N", "faux sujet", 0),
    ("tarifs", "Tableau des tarifs d'entrée de la piscine municipale : adulte 4,50 € ; enfant de 3 à 17 ans 2,80 € ; gratuit pour les moins de 3 ans.", "P", "complète, structurée", 0),
    ("tarifs", "Trois lignes de tarifs : adulte 4,50 €, enfant (3 à 17 ans) 2,80 €, moins de 3 ans gratuit.", "P", "complète, brève", 0),
    ("tarifs", "Les tarifs d'entrée de la piscine.", "N", "sans valeurs", 0),
    ("tarifs", "Adulte 2,80 €, enfant 4,50 €, gratuit pour les moins de 3 ans.", "N", "valeurs inversées", 0),
    # ancres : verdict non discutable sous l'une ou l'autre lecture
    ("bar_h", "Voir l'image.", "N", "renvoi vide (ancre)", 1),
    ("bar_h", "La marche est le mode de transport le plus utilisé, avec 70 % des déplacements.", "N", "factuellement faux (ancre)", 1),
    ("donut", "Lorem ipsum dolor sit amet, consectetur adipiscing elit.", "N", "texte sans rapport (ancre)", 1),
    ("bins", "Trois bacs de tri : jaune (emballages), vert (verre), gris (ordures ménagères).", "P", "complète et exacte (ancre)", 1),
]

RUBRIC = """<p>Pour chaque carte, regardez l'image, lisez le contexte de la page, puis dites si la
<strong>description détaillée proposée est pertinente</strong>.</p>
<p>Test RGAA 1.7.1, mot pour mot : <em>« Chaque image porteuse d'information, ayant une description
détaillée, vérifie-t-elle ces conditions ? — La description détaillée est pertinente. »</em></p>
<p><strong>Aucune règle supplémentaire n'est donnée, volontairement.</strong> Jugez selon votre pratique
d'audit. C'est ce que vous écrivez dans la justification qui établit le critère retenu.</p>
<p>Toutes les images sont considérées comme <strong>informatives</strong>.</p>"""


def build(out):
    images.make(os.path.join(out, "img"))
    items = [dict(id=f"G{n:03d}", image=f"img/{img}.jpg", ctx=CTX[img], alt=txt, pred=p, kind=k, anchor=bool(a),
                  test="1.7.1", label="Description détaillée proposée",
                  question="Cette description détaillée est-elle pertinente ?")
             for n, (img, txt, p, k, a) in enumerate(ROWS, 1)]
    random.Random(1700).shuffle(items)
    for n, x in enumerate(items, 1):
        x["qid"] = f"g{n:02d}"
    public = [{k: x[k] for k in ("qid", "id", "image", "ctx", "alt", "test", "label", "question")} for x in items]
    key = {x["qid"]: {k: x[k] for k in ("id", "pred", "kind", "anchor", "test")} for x in items}
    json.dump(public, open(os.path.join(out, "items.json"), "w", encoding="utf-8"), ensure_ascii=False, indent=1)
    json.dump(key, open(os.path.join(out, "key.json"), "w", encoding="utf-8"), ensure_ascii=False, indent=1)
    open(os.path.join(out, "rubric.html"), "w", encoding="utf-8").write(RUBRIC)
    p = sum(1 for x in items if x["pred"] == "P")
    print(f"{len(items)} cartes | attendues pertinentes {p}, non pertinentes {len(items)-p} | "
          f"repère trivial {max(p, len(items)-p)/len(items):.1%} | ancres {sum(x['anchor'] for x in items)}")


if __name__ == "__main__":
    if len(sys.argv) != 2:
        sys.exit(__doc__)
    build(sys.argv[1])
