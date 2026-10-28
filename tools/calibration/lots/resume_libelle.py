#!/usr/bin/env python3
"""Lot de calibration — famille « résumé et libellé » (tests RGAA 5.5.1, 8.6.1, 11.7.1, 11.8.3).

Des tests qui posent la même question sur le même type de preuve : un court texte, produit
par l'auteur de la page, désigne-t-il correctement ce qu'il étiquette ? Huit cartes par test,
quatre pertinentes et quatre non pertinentes attendues pour chacun : repère trivial de 50 %
(protocole §3 : <= 60 %).

5.2.1 ne fait PLUS partie de cette famille. L'ADR 0002 a retenu la définition du glossaire
RGAA — le résumé porte la nature et la structure du tableau, pas le contenu des cellules — et
l'étiquetage du lot R appliquait un critère plus strict. Ses 8 cartes 5.2.1 sont donc invalides,
et deux d'entre elles portaient en outre sur un tableau non complexe, hors du champ du test.
La famille « résumé de tableau » est désormais mesurée à part, par `resume_tableau.py`.

Les cartes 5.2.1 restent dans `ROWS` pour pouvoir reproduire le lot R d'origine à l'identique
(`--avec-5-2-1-retire`), ce que demande toute relecture des mesures publiées. Elles sont exclues
par défaut : les régénérer sans le savoir reconstituerait du matériel écarté.

Chaque contexte est une IMAGE (`images_rl.py`) : le tableau, le groupe de champs, la liste ou la page
que le texte désigne, dessiné SANS ce texte. Un contexte seulement écrit prédisait la réponse et ne
laissait rien à vérifier. Aucune règle de
pertinence n'est écrite dans la feuille, seulement le texte officiel de chaque test — voir
`description_detaillee.py` pour la raison. `pred` est ma prédiction, pas une vérité.

    python3 resume_libelle.py OUT_DIR [--avec-5-2-1-retire]
"""
import html, json, os, random, shutil, sys, tempfile
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import images_rl

TESTS = {
    "5.2.1": dict(label="Résumé proposé", question="Ce résumé de tableau est-il pertinent ?",
                  official="Pour chaque tableau de données complexe ayant un résumé, celui-ci est-il pertinent ?"),
    "5.5.1": dict(label="Titre proposé", question="Ce titre permet-il d'identifier le contenu du tableau de manière claire et concise ?",
                  official="Pour chaque tableau de données ayant un titre, ce titre permet-il d'identifier le contenu du tableau de données de manière claire et concise ?"),
    "8.6.1": dict(label="Titre de page proposé", question="Ce titre de page est-il pertinent ?",
                  official="Pour chaque page web ayant un titre de page (balise <title>), le contenu de cette balise est-il pertinent ?"),
    "11.7.1": dict(label="Légende proposée", question="Cette légende est-elle pertinente ?",
                   official="Chaque légende associée à un regroupement de champs de même nature est-elle pertinente ?"),
    "11.8.3": dict(label="Libellé du groupe proposé", question="Ce libellé de groupe est-il pertinent ?",
                   official="Pour chaque balise <optgroup> ayant un attribut label, le contenu de l'attribut label est-il pertinent ?"),
}

# (test, contexte, texte proposé, prédiction P/N, nature, ancre ?)
ROWS = [
    # --- 5.2.1 : résumé d'un tableau de données complexe ---
    ("5.2.1", "Tableau complexe : consommation d'énergie par région. Une ligne par région (12 régions) ; trois colonnes regroupées — Électricité, Gaz, Fioul — chacune subdivisée en 2022 et 2023 ; valeurs en GWh.",
     "Consommation d'électricité, de gaz et de fioul, en GWh, pour 12 régions : chaque énergie est détaillée pour 2022 puis 2023.", "P", "décrit la structure et les données", 0),
    ("5.2.1", "Tableau complexe : consommation d'énergie par région. Une ligne par région (12 régions) ; trois colonnes regroupées — Électricité, Gaz, Fioul — chacune subdivisée en 2022 et 2023 ; valeurs en GWh.",
     "Tableau de données.", "N", "générique", 0),
    ("5.2.1", "Tableau complexe : emploi du temps de la classe de 3e B. Colonnes : lundi à vendredi ; lignes : créneaux d'une heure de 8 h à 17 h ; certaines cellules sont fusionnées sur deux créneaux pour les cours de deux heures.",
     "Emploi du temps hebdomadaire de la 3e B, du lundi au vendredi de 8 h à 17 h ; les cours de deux heures occupent deux créneaux fusionnés.", "P", "décrit la structure et les données", 0),
    ("5.2.1", "Tableau complexe : emploi du temps de la classe de 3e B. Colonnes : lundi à vendredi ; lignes : créneaux d'une heure de 8 h à 17 h ; certaines cellules sont fusionnées sur deux créneaux pour les cours de deux heures.",
     "Résultats du championnat de football de la saison.", "N", "factuellement faux (ancre)", 1),
    ("5.2.1", "Tableau complexe : comparatif de quatre forfaits mobiles. Lignes : opérateurs A à D ; colonnes : prix mensuel, volume de data, appels inclus, durée d'engagement ; en-têtes sur deux niveaux.",
     "Comparaison de quatre forfaits mobiles selon le prix mensuel, le volume de data, les appels inclus et la durée d'engagement.", "P", "décrit la structure et les données", 0),
    ("5.2.1", "Tableau complexe : comparatif de quatre forfaits mobiles. Lignes : opérateurs A à D ; colonnes : prix mensuel, volume de data, appels inclus, durée d'engagement ; en-têtes sur deux niveaux.",
     "Le forfait de l'opérateur B est le plus avantageux.", "N", "avis, pas un résumé", 0),
    ("5.2.1", "Tableau complexe : barème d'imposition. Lignes : tranches de revenu ; colonnes : taux pour une personne seule, pour un couple, pour un couple avec enfants.",
     "Taux d'imposition par tranche de revenu, selon que le foyer est une personne seule, un couple ou un couple avec enfants.", "P", "décrit la structure et les données", 0),
    ("5.2.1", "Tableau complexe : barème d'imposition. Lignes : tranches de revenu ; colonnes : taux pour une personne seule, pour un couple, pour un couple avec enfants.",
     "Ce tableau est complexe, merci de le lire attentivement.", "N", "ne décrit rien", 0),
    # --- 5.5.1 : titre d'un tableau de données ---
    ("5.5.1", "Tableau de données : une ligne par mois de 2024 ; deux colonnes — chiffre d'affaires en euros et nombre de commandes.",
     "Chiffre d'affaires et commandes par mois, 2024", "P", "clair et concis", 0),
    ("5.5.1", "Tableau de données : une ligne par mois de 2024 ; deux colonnes — chiffre d'affaires en euros et nombre de commandes.",
     "Tableau 1", "N", "numérotation seule", 0),
    ("5.5.1", "Tableau de données : une ligne par département d'Île-de-France ; deux colonnes — population et superficie.",
     "Population et superficie des départements d'Île-de-France", "P", "clair et concis", 0),
    ("5.5.1", "Tableau de données : une ligne par département d'Île-de-France ; deux colonnes — population et superficie.",
     "Informations", "N", "générique", 0),
    ("5.5.1", "Tableau de données : horaires d'ouverture de la déchetterie, selon les jours de la semaine et la saison (été, hiver).",
     "Horaires d'ouverture de la déchetterie, été et hiver", "P", "clair et concis", 0),
    ("5.5.1", "Tableau de données : horaires d'ouverture de la déchetterie, selon les jours de la semaine et la saison (été, hiver).",
     "Voici le tableau qui présente de manière détaillée l'ensemble des horaires d'ouverture de la déchetterie municipale pour toutes les saisons de l'année et pour tous les jours de la semaine, y compris les jours fériés", "N", "trop long", 0),
    ("5.5.1", "Tableau de données : classement d'un tournoi ; une ligne par équipe ; colonnes — points, victoires, défaites.",
     "Classement du tournoi : points, victoires et défaites par équipe", "P", "clair et concis", 0),
    ("5.5.1", "Tableau de données : classement d'un tournoi ; une ligne par équipe ; colonnes — points, victoires, défaites.",
     "Tarifs de la piscine", "N", "factuellement faux (ancre)", 1),
    # --- 8.6.1 : titre de page ---
    ("8.6.1", "Page : fiche pratique expliquant comment renouveler sa carte d'identité en mairie, avec les pièces à fournir et les délais.",
     "Renouveler sa carte d'identité — Mairie de Villeneuve", "P", "décrit la page", 0),
    ("8.6.1", "Page : fiche pratique expliquant comment renouveler sa carte d'identité en mairie, avec les pièces à fournir et les délais.",
     "Accueil", "N", "générique", 0),
    ("8.6.1", "Page : recette de la tarte aux pommes, avec les ingrédients et les étapes.",
     "Tarte aux pommes : recette facile — Cuisine de Léa", "P", "décrit la page", 0),
    ("8.6.1", "Page : recette de la tarte aux pommes, avec les ingrédients et les étapes.",
     "Sans titre", "N", "titre par défaut (ancre)", 1),
    ("8.6.1", "Page : formulaire de contact de l'association Les Amis du Parc.",
     "Contact — Les Amis du Parc", "P", "décrit la page", 0),
    ("8.6.1", "Page : formulaire de contact de l'association Les Amis du Parc.",
     "Page 3", "N", "numérotation seule", 0),
    ("8.6.1", "Page : résultats de recherche pour le mot « vélo » sur le site de la médiathèque de Saint-Aubin.",
     "Résultats pour « vélo » — Médiathèque de Saint-Aubin", "P", "décrit la page", 0),
    ("8.6.1", "Page : résultats de recherche pour le mot « vélo » sur le site de la médiathèque de Saint-Aubin.",
     "index.html", "N", "nom de fichier", 0),
    # --- 11.7.1 : légende d'un regroupement de champs ---
    ("11.7.1", "Groupe de quatre champs : Rue, Code postal, Ville, Pays — l'adresse à laquelle la commande sera livrée.",
     "Adresse de livraison", "P", "nomme le groupe (ancre)", 1),
    ("11.7.1", "Groupe de quatre champs : Rue, Code postal, Ville, Pays — l'adresse à laquelle la commande sera livrée.",
     "Informations", "N", "générique", 0),
    ("11.7.1", "Groupe de trois boutons radio : Carte bancaire, Virement, Chèque.",
     "Mode de paiement", "P", "nomme le groupe", 0),
    ("11.7.1", "Groupe de trois boutons radio : Carte bancaire, Virement, Chèque.",
     "Choisissez", "N", "verbe seul", 0),
    ("11.7.1", "Groupe de trois champs : Nom, Prénom, Date de naissance — l'identité du demandeur.",
     "État civil", "P", "nomme le groupe", 0),
    ("11.7.1", "Groupe de trois champs : Nom, Prénom, Date de naissance — l'identité du demandeur.",
     "Remplissez les champs ci-dessous avec le plus grand soin, tous les champs sont obligatoires sauf mention contraire", "N", "consigne, pas une légende", 0),
    ("11.7.1", "Groupe de trois cases à cocher : Actualités, Offres promotionnelles, Invitations aux événements.",
     "Abonnements à la lettre d'information", "P", "nomme le groupe", 0),
    ("11.7.1", "Groupe de trois cases à cocher : Actualités, Offres promotionnelles, Invitations aux événements.",
     "Adresse de livraison", "N", "factuellement faux (ancre)", 1),
    # --- 11.8.3 : libellé d'un <optgroup> ---
    ("11.8.3", "Liste de choix « Pays » ; un groupe contient : France, Belgique, Suisse.",
     "Pays francophones", "P", "nomme le groupe", 0),
    ("11.8.3", "Liste de choix « Pays » ; un groupe contient : France, Belgique, Suisse.",
     "Groupe 1", "N", "numérotation seule", 0),
    ("11.8.3", "Liste de choix « Fruits » ; un groupe contient : pomme, poire, coing.",
     "Fruits à pépins", "P", "nomme le groupe", 0),
    ("11.8.3", "Liste de choix « Fruits » ; un groupe contient : pomme, poire, coing.",
     "Autres", "N", "générique", 0),
    ("11.8.3", "Liste de choix « Taille » ; un groupe contient : S, M, L, XL.",
     "Tailles adultes", "P", "nomme le groupe", 0),
    ("11.8.3", "Liste de choix « Taille » ; un groupe contient : S, M, L, XL.",
     "Veuillez sélectionner l'une des options proposées ci-dessous en fonction de votre morphologie", "N", "consigne, pas un libellé", 0),
    ("11.8.3", "Liste de choix « Ville » ; un groupe contient : Marseille, Nice, Toulon.",
     "Villes de Provence-Alpes-Côte d'Azur", "P", "nomme le groupe", 0),
    ("11.8.3", "Liste de choix « Ville » ; un groupe contient : Marseille, Nice, Toulon.",
     "Pays francophones", "N", "factuellement faux (ancre)", 1),
]


# début du contexte écrit -> image qui le remplace
IMG = {
    "Tableau complexe : consommation": "energie_regions", "Tableau complexe : emploi du temps": "emploi_du_temps",
    "Tableau complexe : comparatif": "forfaits", "Tableau complexe : barème": "bareme",
    "Tableau de données : classement": "classement", "Tableau de données : horaires": "dechetterie",
    "Tableau de données : une ligne par département": "idf", "Tableau de données : une ligne par mois": "ca_mensuel",
    "Page : fiche pratique": "page_cni", "Page : formulaire de contact": "page_contact",
    "Page : recette": "page_tarte", "Page : résultats de recherche": "page_recherche",
    "Groupe de quatre champs": "adresse", "Groupe de trois boutons radio": "paiement",
    "Groupe de trois champs": "identite", "Groupe de trois cases à cocher": "abonnements",
    "Liste de choix « Fruits »": "liste_fruits", "Liste de choix « Pays »": "liste_pays",
    "Liste de choix « Taille »": "liste_taille", "Liste de choix « Ville »": "liste_ville",
}
CTX_IMAGE = "Voir l'image : l'élément de la page dont le texte proposé est le titre, le résumé, la légende ou le libellé."


def image_of(ctx):
    hits = [v for k, v in IMG.items() if ctx.startswith(k)]
    assert len(hits) == 1, ctx
    return hits[0]


NOMBRES = {1: "un", 2: "deux", 3: "trois", 4: "quatre", 5: "cinq"}


def rubric(tests=None):
    """La rubrique que lit l'etiqueteur : elle ne doit nommer QUE les tests presents dans le lot.
    Un test retire (ADR 0002) qui y figurerait serait presente comme faisant partie de la mesure."""
    tests = list(TESTS) if tests is None else [t for t in TESTS if t in set(tests)]
    lis = "".join(f"<li><strong>{t}</strong> — <em>{html.escape(TESTS[t]['official'])}</em></li>" for t in tests)
    nombre = NOMBRES.get(len(tests), str(len(tests)))
    return f"""<p>Chaque carte porte sur l'un de ces {nombre} tests RGAA, indiqué sur la carte. Lisez le contexte, puis
dites si le texte proposé est pertinent. L'image montre l'élément concerné sans ce texte
(tableau sans titre ni résumé, groupe sans légende, liste sans libellé de groupe, page sans titre d'onglet). Texte officiel de chaque test :</p><ul>{lis}</ul>
<p><strong>Aucune règle supplémentaire n'est donnée, volontairement.</strong> Jugez selon votre pratique
d'audit. C'est ce que vous écrivez dans la justification qui établit le critère retenu.</p>"""


RETIRED_TESTS = ("5.2.1",)  # ADR 0002 — voir le docstring


def build(out, include_retired=False):
    """Ecrit le lot dans un repertoire de travail unique, puis le publie d'un bloc.

    items.json, key.json, rubric.html et img/ forment un ensemble : un lot dont `key.json` vient
    d'une variante et `items.json` d'une autre ferait corriger des cartes avec la cle d'un autre
    lot, sans qu'aucun controle statique ne le voie. Deux constructions concurrentes, ou une
    construction interrompue, ne doivent donc jamais laisser un repertoire hybride."""
    out = os.path.abspath(out)
    parent = os.path.dirname(out)
    os.makedirs(parent, exist_ok=True)
    stage = tempfile.mkdtemp(prefix=os.path.basename(out) + ".build-", dir=parent)
    try:
        _write(stage, include_retired)
        trash = None
        if os.path.lexists(out):
            trash = tempfile.mkdtemp(prefix=os.path.basename(out) + ".old-", dir=parent)
            os.rmdir(trash)
            os.rename(out, trash)
        os.rename(stage, out)
        if trash:
            shutil.rmtree(trash, ignore_errors=True)
    except BaseException:
        shutil.rmtree(stage, ignore_errors=True)
        raise


def _write(out, include_retired):
    os.makedirs(out, exist_ok=True)
    # La numerotation R### suit ROWS en entier, pour que les identifiants d'une carte
    # conservee soient les memes avec et sans la tranche retiree.
    rows = [(n, r) for n, r in enumerate(ROWS, 1)
            if include_retired or r[0] not in RETIRED_TESTS]
    items = [dict(id=f"R{n:03d}", ctx=ctx, alt=txt, pred=p, kind=k, anchor=bool(a), test=t,
                  label=TESTS[t]["label"], question=TESTS[t]["question"])
             for n, (t, ctx, txt, p, k, a) in rows]
    for x in items:
        x["image"] = f"img/{image_of(x['ctx'])}.jpg"
        x["ctx_ecrit"] = x["ctx"]
    random.Random(5200).shuffle(items)
    for n, x in enumerate(items, 1):
        x["qid"] = f"r{n:02d}"
    for x in items:
        x["ctx"] = CTX_IMAGE
    public = [{k: x[k] for k in ("qid", "id", "image", "ctx", "alt", "test", "label", "question")} for x in items]
    key = {x["qid"]: {k: x[k] for k in ("id", "pred", "kind", "anchor", "test")} for x in items}
    images_rl.make(os.path.join(out, "img"))
    json.dump(public, open(os.path.join(out, "items.json"), "w", encoding="utf-8"), ensure_ascii=False, indent=1)
    json.dump(key, open(os.path.join(out, "key.json"), "w", encoding="utf-8"), ensure_ascii=False, indent=1)
    open(os.path.join(out, "rubric.html"), "w", encoding="utf-8").write(rubric({x["test"] for x in items}))
    p = sum(1 for x in items if x["pred"] == "P")
    par_test = {t: n for t in TESTS if (n := sum(1 for x in items if x["test"] == t))}
    retire = "" if include_retired else f" | retire (ADR 0002) {', '.join(RETIRED_TESTS)}"
    print(f"{len(items)} cartes | attendues pertinentes {p}, non pertinentes {len(items)-p} | "
          f"repère trivial {max(p, len(items)-p)/len(items):.1%} | ancres {sum(x['anchor'] for x in items)} | "
          f"par test {par_test}{retire}")


if __name__ == "__main__":
    args = [a for a in sys.argv[1:] if a != "--avec-5-2-1-retire"]
    if len(args) != 1:
        sys.exit(__doc__)
    build(args[0], include_retired="--avec-5-2-1-retire" in sys.argv[1:])
