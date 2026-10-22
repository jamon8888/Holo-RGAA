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

    table("effectifs_filieres", out,
          [[("Filière", 1), ("Rentrée 2024", 2), ("Rentrée 2025", 2)],
           [("", 1), ("Hommes", 1), ("Femmes", 1), ("Hommes", 1), ("Femmes", 1)]],
          [["Droit", "412", "596", "428", "631"], ["Informatique", "734", "211", "768", "247"],
           ["Lettres", "188", "402", "174", "389"], ["Biologie", "356", "488", "371", "502"],
           ["Économie", "521", "463", "544", "479"]],
          [230, 150, 150, 150, 150], row_h=36)

    table("tournoi_dom_ext", out,
          [[("Équipe", 1), ("À domicile", 3), ("À l'extérieur", 3)],
           [("", 1), ("Joués", 1), ("Gagnés", 1), ("Perdus", 1), ("Joués", 1), ("Gagnés", 1), ("Perdus", 1)]],
          [["Les Aigles", "9", "7", "2", "9", "4", "5"], ["Olympique Nord", "9", "6", "3", "9", "5", "4"],
           ["Étoile Sud", "9", "5", "4", "9", "2", "7"], ["Racing Est", "9", "3", "6", "9", "3", "6"],
           ["Union Ouest", "9", "2", "7", "9", "1", "8"]],
          [190, 110, 110, 110, 110, 110, 110], row_h=36)

    table("tarifs_postaux", out,
          [[("Poids", 1), ("France", 2), ("International", 2)],
           [("", 1), ("Normal", 1), ("Suivi", 1), ("Normal", 1), ("Suivi", 1)]],
          [["Jusqu'à 20 g", "1,39 €", "3,09 €", "1,96 €", "5,40 €"],
           ["21 à 100 g", "2,78 €", "4,48 €", "3,92 €", "7,36 €"],
           ["101 à 250 g", "4,86 €", "6,56 €", "7,84 €", "11,28 €"],
           ["251 à 500 g", "6,96 €", "8,66 €", "11,76 €", "15,20 €"],
           ["501 g à 1 kg", "9,04 €", "10,74 €", "15,68 €", "19,12 €"]],
          [220, 150, 150, 150, 150], row_h=36)

    table("conso_eau", out,
          [[("Site", 1), ("Bâtiment", 1), ("Été", 2), ("Hiver", 2)],
           [("", 1), ("", 1), ("Volume (m³)", 1), ("Coût (€)", 1), ("Volume (m³)", 1), ("Coût (€)", 1)]],
          [["Nord", "Atelier", "1 240", "3 968", "860", "2 752"],
           [None, "Bureaux", "410", "1 312", "520", "1 664"],
           [None, "Entrepôt", "180", "576", "140", "448"],
           ["Sud", "Atelier", "1 510", "4 832", "1 020", "3 264"],
           [None, "Bureaux", "380", "1 216", "470", "1 504"],
           [None, "Serre", "2 240", "7 168", "1 680", "5 376"]],
          [130, 160, 140, 130, 140, 130], spans={(0, 0): 3, (3, 0): 3}, row_h=36)

    table("population_communes", out,
          [[("Département", 1), ("Commune", 1), ("Population", 2)],
           [("", 1), ("", 1), ("1999", 1), ("2024", 1)]],
          [["Ardèche", "Aubenas", "11 018", "12 406"], [None, "Privas", "10 080", "8 190"],
           [None, "Annonay", "18 525", "16 142"], ["Gers", "Auch", "23 136", "21 838"],
           [None, "Condom", "7 251", "6 884"], [None, "Fleurance", "6 273", "6 015"]],
          [170, 200, 160, 160], spans={(0, 0): 3, (3, 0): 3}, row_h=36)

    table("musee_horaires", out,
          [[("Jour", 1), ("Basse saison", 2), ("Haute saison", 2)],
           [("", 1), ("Ouverture", 1), ("Fermeture", 1), ("Ouverture", 1), ("Fermeture", 1)]],
          [["Mardi", "10 h", "17 h", "9 h 30", "18 h 30"], ["Mercredi", "10 h", "17 h", "9 h 30", "19 h"],
           ["Jeudi", "10 h", "17 h", "9 h 30", "18 h 30"], ["Vendredi", "10 h", "18 h", "9 h 30", "20 h"],
           ["Samedi", "9 h 30", "18 h", "9 h", "20 h"], ["Dimanche", "9 h 30", "17 h", "9 h", "19 h"]],
          [170, 160, 160, 160, 160], row_h=36)

    table("ventes_produits", out,
          [[("Catégorie", 1), ("Produit", 1), ("Ventes (unités)", 4)],
           [("", 1), ("", 1), ("T1", 1), ("T2", 1), ("T3", 1), ("T4", 1)]],
          [["Jardin", "Tondeuse", "412", "638", "521", "190"], [None, "Arrosoir", "230", "410", "355", "120"],
           [None, "Sécateur", "180", "295", "240", "160"], ["Cuisine", "Robot", "310", "280", "295", "520"],
           [None, "Casserole", "420", "390", "405", "610"], [None, "Couteau", "250", "240", "260", "480"]],
          [150, 180, 125, 125, 125, 125], spans={(0, 0): 3, (3, 0): 3}, row_h=36)

    table("election_bureaux", out,
          [[("Bureau", 1), ("Mercier", 2), ("Vasseur", 2), ("Blanc", 2)],
           [("", 1), ("Voix", 1), ("%", 1), ("Voix", 1), ("%", 1), ("Voix", 1), ("%", 1)]],
          [["Bureau 1", "412", "41,2", "358", "35,8", "230", "23,0"],
           ["Bureau 2", "389", "38,9", "401", "40,1", "210", "21,0"],
           ["Bureau 3", "455", "45,5", "322", "32,2", "223", "22,3"],
           ["Bureau 4", "298", "29,8", "437", "43,7", "265", "26,5"],
           ["Bureau 5", "376", "37,6", "390", "39,0", "234", "23,4"]],
          [150, 110, 105, 110, 105, 110, 105], row_h=36)

    table("stock_entrepots", out,
          [[("Entrepôt", 1), ("Référence", 1), ("Janvier", 2), ("Février", 2)],
           [("", 1), ("", 1), ("Entrées", 1), ("Sorties", 1), ("Entrées", 1), ("Sorties", 1)]],
          [["Lyon", "A-12", "120", "95", "140", "130"], [None, "B-07", "60", "45", "55", "70"],
           ["Rouen", "A-12", "80", "72", "90", "85"], [None, "C-33", "45", "38", "50", "41"],
           [None, "D-01", "210", "190", "230", "205"]],
          [150, 170, 130, 130, 130, 130], spans={(0, 0): 2, (2, 0): 3}, row_h=36)

    return sorted(os.listdir(out))


if __name__ == "__main__":
    if len(sys.argv) != 2:
        sys.exit(__doc__)
    print(make(sys.argv[1]))
