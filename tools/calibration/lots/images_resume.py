#!/usr/bin/env python3
"""Quatre tableaux de données COMPLEXES au sens du RGAA, pour le test 5.2.1.

Complexe = les en-têtes ne tiennent pas dans la seule première ligne et/ou première colonne
(glossaire RGAA 4.1.2). Ici : trois tableaux à en-têtes de colonnes sur deux niveaux, et un
tableau dont les en-têtes de lignes occupent deux colonnes. Des cellules fusionnées dans le
corps ne suffiraient pas : c'est l'erreur du premier lot R, où l'emploi du temps n'était pas
un tableau complexe et sortait donc du champ du test.

Le tableau est dessiné SANS résumé : le résumé est le texte jugé par la carte.

    python3 images_resume.py OUT_DIR
"""
import os, sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from images_rl import table


def make(out):
    os.makedirs(out, exist_ok=True)

    table("meteo_villes", out,
          [[("Ville", 1), ("Températures (°C)", 2), ("Précipitations", 2)],
           [("", 1), ("Minimale", 1), ("Maximale", 1), ("Hauteur (mm)", 1), ("Jours de pluie", 1)]],
          [["Brest", "7", "14", "132", "19"], ["Strasbourg", "2", "11", "48", "11"],
           ["Nice", "9", "17", "61", "6"], ["Lille", "4", "12", "74", "15"],
           ["Clermont-Ferrand", "3", "13", "57", "10"], ["Bordeaux", "6", "16", "89", "13"]],
          [230, 140, 140, 150, 150], row_h=36)

    table("horaires_trains", out,
          [[("Gare", 1), ("Semaine", 2), ("Week-end", 2)],
           [("", 1), ("Matin", 1), ("Soir", 1), ("Matin", 1), ("Soir", 1)]],
          [["Auteuil", "6 h 12", "18 h 40", "8 h 05", "19 h 15"],
           ["Belleville", "6 h 27", "18 h 55", "8 h 20", "19 h 30"],
           ["Chantemerle", "6 h 41", "19 h 08", "8 h 34", "19 h 44"],
           ["Dourdan", "6 h 58", "19 h 22", "8 h 51", "20 h 01"],
           ["Épinay", "7 h 14", "19 h 37", "9 h 06", "20 h 18"]],
          [220, 150, 150, 150, 150], row_h=38)

    table("bulletin_trimestres", out,
          [[("Élève", 1), ("Trimestre 1", 2), ("Trimestre 2", 2)],
           [("", 1), ("Moyenne", 1), ("Rang", 1), ("Moyenne", 1), ("Rang", 1)]],
          [["Alice Berger", "15,2", "2", "16,1", "1"], ["Bruno Caron", "11,8", "9", "12,4", "8"],
           ["Chloé Dufour", "16,4", "1", "15,7", "2"], ["David Esnault", "9,6", "14", "10,9", "12"],
           ["Elsa Favre", "13,1", "5", "12,8", "6"], ["Farid Guérin", "12,5", "7", "14,2", "4"]],
          [240, 140, 120, 140, 120], row_h=36)

    table("budget_services", out,
          [[("Service", 1), ("Poste", 1), ("2024", 2), ("2025", 2)],
           [("", 1), ("", 1), ("Prévu", 1), ("Réalisé", 1), ("Prévu", 1), ("Réalisé", 1)]],
          [["Technique", "Entretien", "48 000", "51 200", "52 000", "49 800"],
           [None, "Véhicules", "26 000", "24 700", "27 500", "28 900"],
           [None, "Outillage", "12 000", "11 400", "12 500", "13 100"],
           ["Culture", "Programmation", "63 000", "59 800", "65 000", "66 400"],
           [None, "Médiation", "19 000", "20 300", "21 000", "19 700"],
           [None, "Communication", "14 000", "13 100", "14 500", "15 200"]],
          [150, 180, 130, 130, 130, 130], spans={(0, 0): 3, (3, 0): 3}, row_h=36)

    return sorted(os.listdir(out))


if __name__ == "__main__":
    if len(sys.argv) != 2:
        sys.exit(__doc__)
    print(make(sys.argv[1]))
