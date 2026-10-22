"""Six tableaux complexes pour le LOT DE VALIDATION du test 5.2.1.

Aucune consigne modèle n'a été ajustée sur ces tableaux : c'est leur raison d'être. Les consignes
v1 à v3 ont été écrites en regardant le lot de 36 cartes `resume_tableau.py` ; un score n'y vaut
donc que comme ajustement. Ce lot-ci n'est vu qu'une fois, après coup.

    python3 images_resume_valid.py OUT_DIR
"""
import os, sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from images_rl import table


def make(out):
    os.makedirs(out, exist_ok=True)

    table("ca_region_canal", out,
          [[("Région", 1), ("En ligne", 2), ("En magasin", 2)],
           [("", 1), ("CA (k€)", 1), ("Commandes", 1), ("CA (k€)", 1), ("Commandes", 1)]],
          [["Bretagne", "412", "1 840", "268", "1 190"], ["Occitanie", "596", "2 470", "381", "1 620"],
           ["Normandie", "287", "1 310", "344", "1 505"], ["Alsace", "198", "905", "412", "1 780"],
           ["Aquitaine", "503", "2 115", "297", "1 288"]],
          [180, 160, 160, 160, 160], row_h=36)

    table("absences_services", out,
          [[("Service", 1), ("Équipe", 1), ("Jours d'absence", 3)],
           [("", 1), ("", 1), ("Maladie", 1), ("Congés", 1), ("Formation", 1)]],
          [["Production", "Ligne A", "48", "126", "12"], [None, "Ligne B", "61", "118", "9"],
           ["Logistique", "Quai", "35", "94", "16"], [None, "Préparation", "52", "107", "7"],
           ["Support", "Accueil", "19", "88", "21"], [None, "Standard", "24", "76", "14"]],
          [150, 170, 160, 160, 160], spans={(0, 0): 2, (2, 0): 2, (4, 0): 2}, row_h=36)

    table("notes_matieres", out,
          [[("Matière", 1), ("Écrit", 2), ("Oral", 2)],
           [("", 1), ("Note", 1), ("Coefficient", 1), ("Note", 1), ("Coefficient", 1)]],
          [["Mathématiques", "14,5", "5", "12,0", "2"], ["Histoire", "11,0", "3", "15,5", "2"],
           ["Anglais", "16,0", "2", "17,5", "3"], ["Physique", "13,5", "4", "11,5", "2"],
           ["Philosophie", "9,5", "3", "13,0", "1"]],
          [200, 140, 170, 140, 170], row_h=36)

    table("trafic_pages", out,
          [[("Page", 1), ("Visiteurs uniques", 2), ("Pages vues", 2)],
           [("", 1), ("T1", 1), ("T2", 1), ("T1", 1), ("T2", 1)]],
          [["Accueil", "18 420", "21 060", "42 310", "48 775"],
           ["Catalogue", "24 180", "27 940", "96 520", "104 830"],
           ["Contact", "3 260", "3 015", "4 180", "3 890"],
           ["Blog", "9 740", "12 380", "16 290", "20 115"],
           ["Recrutement", "2 110", "4 630", "3 040", "7 220"]],
          [200, 150, 150, 150, 150], row_h=36)

    table("recoltes_parcelles", out,
          [[("Exploitation", 1), ("Parcelle", 1), ("2024", 2), ("2025", 2)],
           [("", 1), ("", 1), ("Surface (ha)", 1), ("Rendement", 1),
            ("Surface (ha)", 1), ("Rendement", 1)]],
          [["Les Ormes", "Nord", "12,4", "71", "12,4", "68"], [None, "Sud", "8,9", "64", "9,6", "70"],
           ["La Fontaine", "Est", "15,2", "78", "15,2", "74"], [None, "Ouest", "6,7", "59", "6,7", "63"],
           [None, "Centre", "11,3", "66", "10,8", "69"]],
          [130, 130, 150, 145, 150, 145], spans={(0, 0): 2, (2, 0): 3}, row_h=36)

    table("interventions_equipes", out,
          [[("Équipe", 1), ("Urgentes", 2), ("Planifiées", 2)],
           [("", 1), ("Nombre", 1), ("Délai moyen (h)", 1), ("Nombre", 1), ("Délai moyen (j)", 1)]],
          [["Alpha", "42", "3,2", "118", "6"], ["Beta", "28", "4,8", "96", "9"],
           ["Gamma", "51", "2,6", "134", "5"], ["Delta", "19", "6,1", "77", "11"]],
          [170, 140, 180, 140, 180], row_h=38)

    return sorted(os.listdir(out))


if __name__ == "__main__":
    if len(sys.argv) != 2:
        sys.exit(__doc__)
    print(make(sys.argv[1]))
