#!/usr/bin/env python3
"""Quatre tableaux de données COMPLEXES (RGAA) que l'étiqueteur n'a JAMAIS vus, pour le lot W.

Un premier jet du lot W réutilisait quatre tableaux d'`images_resume.py` (météo, trains, bulletin,
budget) : ils avaient pourtant déjà été montrés à l'étiqueteur dans la première version du lot T
(commit aa050ec), qui avait rejeté ses 8 cartes. Ce n'était donc pas un test hors échantillon.
Ceux-ci ne figurent dans aucun lot antérieur ; `test_lot_noms.py` le vérifie contre la liste des
images des autres générateurs.

    python3 images_noms.py OUT_DIR
"""
import os, sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from images_rl import table


def make(out):
    os.makedirs(out, exist_ok=True)

    table("rendement_cultures", out,
          [[("Région", 1), ("Blé", 2), ("Maïs", 2)],
           [("", 1), ("Surface (ha)", 1), ("Rendement (t/ha)", 1), ("Surface (ha)", 1), ("Rendement (t/ha)", 1)]],
          [["Bretagne", "182 000", "7,4", "94 000", "9,8"],
           ["Normandie", "231 000", "8,1", "41 000", "9,2"],
           ["Occitanie", "264 000", "5,9", "158 000", "10,4"],
           ["Centre-Val de Loire", "512 000", "7,6", "76 000", "9,5"],
           ["Grand Est", "498 000", "7,9", "103 000", "10,1"]],
          [240, 150, 190, 150, 190], row_h=36)

    table("bibliotheques_prets", out,
          [[("Bibliothèque", 1), ("Adultes", 2), ("Jeunesse", 2)],
           [("", 1), ("Livres", 1), ("DVD", 1), ("Livres", 1), ("DVD", 1)]],
          [["Centrale", "4 120", "880", "2 940", "310"],
           ["Jean-Jaurès", "1 860", "240", "1 330", "95"],
           ["Les Lilas", "970", "130", "1 480", "160"],
           ["Saint-Roch", "1 240", "310", "820", "70"],
           ["Les Tilleuls", "660", "90", "1 050", "120"]],
          [220, 140, 140, 140, 140], row_h=36)

    table("camping_tarifs", out,
          [[("Hébergement", 1), ("Formule", 1), ("Basse saison", 2), ("Haute saison", 2)],
           [("", 1), ("", 1), ("Semaine", 1), ("Week-end", 1), ("Semaine", 1), ("Week-end", 1)]],
          [["Emplacement nu", "Tente", "84 €", "32 €", "154 €", "58 €"],
           [None, "Caravane", "98 €", "38 €", "182 €", "66 €"],
           [None, "Camping-car", "112 €", "42 €", "203 €", "74 €"],
           ["Location", "Mobil-home", "266 €", "96 €", "602 €", "188 €"],
           [None, "Chalet", "329 €", "118 €", "714 €", "224 €"],
           [None, "Gîte", "385 €", "134 €", "826 €", "256 €"]],
          [190, 170, 130, 130, 130, 130], spans={(0, 0): 3, (3, 0): 3}, row_h=36)

    table("labo_analyses", out,
          [[("Site de prélèvement", 1), ("Physico-chimie", 2), ("Contaminants", 2)],
           [("", 1), ("Température (°C)", 1), ("pH", 1), ("Nitrates (mg/L)", 1), ("Phosphates (mg/L)", 1)]],
          [["Les Écluses", "14,2", "7,6", "18", "0,21"],
           ["Pont Neuf", "15,1", "7,4", "26", "0,34"],
           ["Moulin Haut", "11,8", "7,9", "9", "0,08"],
           ["Gué de l'Aube", "13,4", "7,2", "31", "0,47"],
           ["La Source", "9,6", "8,0", "4", "0,03"]],
          [250, 170, 90, 170, 190], row_h=38)

    return sorted(os.listdir(out))


if __name__ == "__main__":
    if len(sys.argv) != 2:
        sys.exit(__doc__)
    print(make(sys.argv[1]))
