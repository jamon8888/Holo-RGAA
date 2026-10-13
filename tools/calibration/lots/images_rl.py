#!/usr/bin/env python3
"""Vingt images SYNTHÉTIQUES pour le lot R : un tableau, un groupe de champs, une liste ou une page.

Règle commune : l'image montre l'élément SANS le texte jugé (titre, résumé, légende, libellé de
groupe, titre de page). Ce texte est celui de la carte ; le dessiner reviendrait à le donner deux fois
et, pour un contexte trompeur, à le contredire. Les valeurs sont inventées, aucune licence.

    python3 images_rl.py OUT_DIR      # écrit OUT_DIR/<nom>.jpg, renvoie la liste
"""
import os, sys
from PIL import Image, ImageDraw, ImageFont

FB = "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf"
FR = "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf"
W = 880


def font(bold=False, size=20):
    return ImageFont.truetype(FB if bold else FR, size)


def table(name, out, head, rows, widths, spans=None, row_h=34, fills=None):
    """head : lignes d'en-tête, chacune une liste de (texte, colspan). spans : {(r, c): rowspan}
    sur les lignes du corps. fills : {(r, c): couleur}."""
    spans, fills = spans or {}, fills or {}
    nh = len(head)
    h = (nh + len(rows)) * row_h + 24
    im = Image.new("RGB", (W, h), "white"); d = ImageDraw.Draw(im)
    x0, f, fb = 20, font(False, 18), font(True, 18)
    xs = [x0]
    for w in widths:
        xs.append(xs[-1] + w)

    def cell(x1, y1, x2, y2, text, bold, fill):
        d.rectangle((x1, y1, x2, y2), fill=fill, outline="#555")
        ff = fb if bold else f
        tw = d.textlength(text, font=ff)
        d.text((x1 + (x2 - x1 - tw) / 2, y1 + (y2 - y1 - 18) / 2 - 2), text, fill="#111", font=ff)

    for r, line in enumerate(head):
        c = 0
        for text, span in line:
            cell(xs[c], 12 + r * row_h, xs[c + span], 12 + (r + 1) * row_h, text, True, "#dce6f1")
            c += span
    covered = set()
    for r, line in enumerate(rows):
        for c, text in enumerate(line):
            if (r, c) in covered or text is None:
                continue
            rs = spans.get((r, c), 1)
            for k in range(1, rs):
                covered.add((r + k, c))
            y1 = 12 + (nh + r) * row_h
            cell(xs[c], y1, xs[c + 1], y1 + rs * row_h, text, c == 0, fills.get((r, c), "#fafafa" if r % 2 else "white"))
    im.save(f"{out}/{name}.jpg", quality=88)


def page(name, out, blocks, h=330):
    """Corps de page SANS barre de titre : seul le contenu visible, comme sous un onglet masqué."""
    im = Image.new("RGB", (W, h), "white"); d = ImageDraw.Draw(im)
    d.rectangle((0, 0, W, 8), fill="#1f6fb2")
    y = 30
    for kind, text in blocks:
        if kind == "h1":
            d.text((40, y), text, fill="#111", font=font(True, 34)); y += 62
        elif kind == "p":
            d.text((40, y), text, fill="#333", font=font(False, 20)); y += 34
        elif kind == "li":
            d.text((60, y), "• " + text, fill="#333", font=font(False, 20)); y += 32
        elif kind == "field":
            d.text((40, y), text, fill="#333", font=font(False, 20))
            d.rectangle((300, y - 4, 760, y + 28), outline="#777", fill="#fafafa"); y += 48
        elif kind == "btn":
            d.rectangle((40, y, 240, y + 40), fill="#1f6fb2"); d.text((70, y + 8), text, fill="white", font=font(True, 20)); y += 56
    im.save(f"{out}/{name}.jpg", quality=88)


def group(name, out, title_gap, rows, kind):
    """Groupe de champs : cadre de fieldset dont la légende est volontairement absente (espace vide)."""
    h = 70 + len(rows) * 52
    im = Image.new("RGB", (W, h), "white"); d = ImageDraw.Draw(im)
    d.rectangle((40, 30, 820, h - 20), outline="#555", width=2)
    d.rectangle((60, 22, 280, 38), fill="white")  # emplacement de la légende, laissé vide
    y = 58
    for r in rows:
        if kind == "text":
            d.text((70, y + 4), r, fill="#222", font=font(False, 20)); d.rectangle((330, y, 780, y + 34), outline="#777", fill="#fafafa")
        elif kind == "radio":
            d.ellipse((70, y + 4, 92, y + 26), outline="#333", width=2); d.text((105, y + 2), r, fill="#222", font=font(False, 21))
        else:
            d.rectangle((70, y + 4, 92, y + 26), outline="#333", width=2); d.text((105, y + 2), r, fill="#222", font=font(False, 21))
        y += 52
    im.save(f"{out}/{name}.jpg", quality=88)


def listbox(name, out, select, others, items, h_extra=0):
    """Liste de choix ouverte : le groupe jugé est dessiné SANS son libellé (ligne grisée vide)."""
    n = len(others) + len(items) + 2
    h = 90 + n * 34 + h_extra
    im = Image.new("RGB", (W, h), "white"); d = ImageDraw.Draw(im)
    d.text((40, 20), select, fill="#222", font=font(True, 20))
    d.rectangle((40, 52, 420, 52 + n * 34 + 6), outline="#555", fill="#fafafa")
    y = 58
    for o in others:
        d.text((60, y + 4), o, fill="#222", font=font(False, 19)); y += 34
    d.rectangle((41, y, 419, y + 30), fill="#e6e6e6")  # ligne du libellé de groupe, masqué
    y += 34
    for it in items:
        d.text((90, y + 4), it, fill="#222", font=font(False, 19)); y += 34
    im.save(f"{out}/{name}.jpg", quality=88)


def make(out):
    os.makedirs(out, exist_ok=True)
    reg = ["Bretagne", "Normandie", "Occitanie", "Grand Est", "Hauts-de-France", "Pays de la Loire",
           "Centre-Val de Loire", "Bourgogne-Franche-Comté", "Nouvelle-Aquitaine", "Auvergne-Rhône-Alpes",
           "Provence-Alpes-Côte d'Azur", "Île-de-France"]
    el = [(412, 398, 305, 281, 98, 87), (356, 349, 210, 198, 77, 70), (498, 487, 402, 391, 121, 109),
          (601, 588, 530, 512, 143, 130), (702, 690, 611, 598, 160, 149), (433, 421, 288, 276, 104, 95),
          (301, 295, 196, 187, 66, 59), (288, 279, 183, 177, 71, 63), (655, 643, 478, 466, 151, 139),
          (1012, 997, 720, 705, 190, 176), (734, 722, 541, 530, 129, 118), (1340, 1318, 905, 889, 210, 195)]
    table("energie_regions", out,
          [[("Région", 1), ("Électricité", 2), ("Gaz", 2), ("Fioul", 2)],
           [("", 1)] + [("2022", 1), ("2023", 1)] * 3],
          [[r] + [str(v) for v in e] for r, e in zip(reg, el)],
          [300, 90, 90, 90, 90, 90, 90], row_h=30)
    ed = ["8 h", "9 h", "10 h", "11 h", "12 h", "13 h", "14 h", "15 h", "16 h"]
    grid = [["Maths", "Français", "Histoire", "Anglais", "Maths"],
            [None, "Français", "Sport", "Anglais", "Physique"],
            ["Physique", "Maths", "TP physique", "Arts", "Français"],
            ["Anglais", "Histoire", None, "Latin", "Histoire"],
            ["Pause", "Pause", "Pause", "Pause", "Pause"],
            ["Latin", "SVT", "Techno", "Maths", "Sport"],
            ["Étude", "Histoire", "Musique", "Étude", None],
            ["Anglais", "Maths", "Français", "SVT", "Étude"],
            ["Étude", "Étude", "Étude", "Étude", "Étude"]]
    table("emploi_du_temps", out, [[("", 1), ("Lundi", 1), ("Mardi", 1), ("Mercredi", 1), ("Jeudi", 1), ("Vendredi", 1)]],
          [[h] + g for h, g in zip(ed, grid)], [90, 150, 150, 150, 150, 150],
          spans={(0, 1): 2, (2, 3): 2, (5, 5): 2}, row_h=34)
    table("forfaits", out,
          [[("Opérateur", 1), ("Tarif", 2), ("Contenu", 2)],
           [("", 1), ("Prix mensuel", 1), ("Engagement", 1), ("Data", 1), ("Appels", 1)]],
          [["A", "9,99 €", "Sans", "20 Go", "Illimités"], ["B", "14,99 €", "12 mois", "100 Go", "Illimités"],
           ["C", "19,99 €", "Sans", "200 Go", "Illimités"], ["D", "7,99 €", "24 mois", "5 Go", "2 h"]],
          [140, 160, 160, 160, 160], row_h=38)
    table("bareme", out,
          [[("Tranche de revenu", 1), ("Taux applicable", 3)],
           [("", 1), ("Personne seule", 1), ("Couple", 1), ("Couple avec enfants", 1)]],
          [["Jusqu'à 11 000 €", "0 %", "0 %", "0 %"], ["11 001 à 28 000 €", "11 %", "8 %", "5 %"],
           ["28 001 à 78 000 €", "30 %", "24 %", "18 %"], ["Au-delà de 78 000 €", "41 %", "35 %", "29 %"]],
          [260, 190, 190, 190], row_h=38)
    table("classement", out, [[("Équipe", 1), ("Points", 1), ("Victoires", 1), ("Défaites", 1)]],
          [["Les Aigles", "24", "8", "1"], ["Olympique Nord", "21", "7", "2"], ["Étoile Sud", "15", "5", "4"],
           ["Racing Est", "9", "3", "6"], ["Union Ouest", "3", "1", "8"]], [300, 150, 150, 150], row_h=40)
    table("dechetterie", out, [[("Jour", 1), ("Été", 1), ("Hiver", 1)]],
          [["Lundi", "9 h – 19 h", "9 h – 17 h"], ["Mardi", "Fermé", "Fermé"], ["Mercredi", "9 h – 19 h", "9 h – 17 h"],
           ["Jeudi", "9 h – 19 h", "9 h – 17 h"], ["Vendredi", "9 h – 19 h", "9 h – 17 h"],
           ["Samedi", "9 h – 20 h", "9 h – 18 h"], ["Dimanche", "9 h – 12 h", "Fermé"]], [220, 220, 220], row_h=36)
    table("idf", out, [[("Département", 1), ("Population", 1), ("Superficie (km²)", 1)]],
          [["Paris (75)", "2 133 111", "105"], ["Seine-et-Marne (77)", "1 454 190", "5 915"], ["Yvelines (78)", "1 455 105", "2 284"],
           ["Essonne (91)", "1 306 000", "1 804"], ["Hauts-de-Seine (92)", "1 653 000", "176"],
           ["Seine-Saint-Denis (93)", "1 685 000", "236"], ["Val-de-Marne (94)", "1 407 000", "245"], ["Val-d'Oise (95)", "1 262 000", "1 246"]],
          [320, 220, 220], row_h=34)
    mois = ["Janvier", "Février", "Mars", "Avril", "Mai", "Juin", "Juillet", "Août", "Septembre", "Octobre", "Novembre", "Décembre"]
    ca = [41200, 38800, 45100, 47900, 52300, 49800, 44100, 36500, 51200, 56800, 63400, 71900]
    cm = [412, 388, 451, 479, 523, 498, 441, 365, 512, 568, 634, 719]
    table("ca_mensuel", out, [[("Mois", 1), ("Chiffre d'affaires (€)", 1), ("Commandes", 1)]],
          [[m, f"{a:,}".replace(",", " "), str(c)] for m, a, c in zip(mois, ca, cm)], [220, 260, 200], row_h=30)

    page("page_cni", out, [("h1", "Renouveler sa carte d'identité"), ("p", "Pièces à fournir :"), ("li", "ancienne carte d'identité"),
                           ("li", "justificatif de domicile de moins de 6 mois"), ("li", "photo d'identité récente"), ("p", "Délai moyen : 3 semaines.")])
    page("page_contact", out, [("h1", "Nous écrire"), ("field", "Votre nom"), ("field", "Votre adresse e-mail"), ("field", "Votre message"), ("btn", "Envoyer")], h=380)
    page("page_tarte", out, [("h1", "Tarte aux pommes"), ("p", "Ingrédients : 1 pâte, 5 pommes, 60 g de sucre, 30 g de beurre."), ("p", "1. Étaler la pâte, disposer les pommes."),
                             ("p", "2. Saupoudrer de sucre, ajouter le beurre."), ("p", "3. Cuire 35 minutes à 180 °C.")])
    page("page_recherche", out, [("h1", "12 résultats"), ("p", "Pour la recherche : vélo"), ("li", "Le vélo dans la ville — exposition"),
                                 ("li", "Atelier de réparation de vélos"), ("li", "Guide des pistes cyclables de Saint-Aubin")], h=360)

    group("adresse", out, "", ["Rue", "Code postal", "Ville", "Pays"], "text")
    group("paiement", out, "", ["Carte bancaire", "Virement", "Chèque"], "radio")
    group("identite", out, "", ["Nom", "Prénom", "Date de naissance"], "text")
    group("abonnements", out, "", ["Actualités", "Offres promotionnelles", "Invitations aux événements"], "check")

    listbox("liste_fruits", out, "Fruits", ["Choisir un fruit"], ["pomme", "poire", "coing"])
    listbox("liste_pays", out, "Pays", ["Choisir un pays"], ["France", "Belgique", "Suisse"])
    listbox("liste_taille", out, "Taille", ["Choisir une taille"], ["S", "M", "L", "XL"])
    listbox("liste_ville", out, "Ville", ["Choisir une ville"], ["Marseille", "Nice", "Toulon"])
    return sorted(os.listdir(out))


if __name__ == "__main__":
    if len(sys.argv) != 2:
        sys.exit(__doc__)
    print(make(sys.argv[1]))
