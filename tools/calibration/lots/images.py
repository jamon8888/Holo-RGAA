#!/usr/bin/env python3
"""Neuf images de données SYNTHÉTIQUES, générées par code : rien à commiter, aucune licence.

Chaque image porte un contenu connu et des libellés UNIQUES. Cette dernière règle vient d'un
défaut du prototype #339 : matplotlib fusionne les libellés d'axe répétés en une seule
catégorie (« J F M A M J J A… » devient un axe à catégories uniques), ce qui a produit un
graphique faux dont la « vérité terrain » contredisait l'image. Deux modèles avaient raison
et la clé avait tort.

    python3 images.py OUT_DIR      # écrit OUT_DIR/<nom>.jpg
"""
import os, sys
try:
    import matplotlib
except ModuleNotFoundError as exc:  # message utile : l'import est a trois niveaux de profondeur
    raise SystemExit(
        "matplotlib est absent, et les schemas de la famille « description detaillee » "
        "(1.7.x) en dependent.\n"
        "Installer les dependances de l'outillage :\n"
        "    python3 -m venv .venv && . .venv/bin/activate\n"
        "    pip install -r tools/calibration/requirements.txt"
    ) from exc
matplotlib.use("Agg")
import matplotlib.pyplot as plt
from matplotlib.patches import FancyArrowPatch, FancyBboxPatch, Rectangle
from PIL import Image, ImageDraw, ImageFont

FB = "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf"
FR = "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf"
FS = "/usr/share/fonts/truetype/dejavu/DejaVuSerif-Bold.ttf"


def make(out):
    os.makedirs(out, exist_ok=True)

    def save(fig, name):
        fig.tight_layout()
        fig.savefig(f"{out}/{name}.jpg", format="jpg", dpi=110, pil_kwargs={"quality": 84})
        plt.close(fig)

    fig, ax = plt.subplots(figsize=(8, 4.5))
    lab, val = ["Voiture", "Transports en commun", "Marche", "Vélo"], [48, 27, 13, 12]
    ax.barh(range(4), val, color=["#e45756", "#4c78a8", "#54a24b", "#f58518"])
    ax.set_yticks(range(4)); ax.set_yticklabels(lab); ax.invert_yaxis()
    for i, v in enumerate(val):
        ax.text(v + 0.8, i, f"{v} %", va="center")
    ax.set_xlim(0, 58); ax.set_xlabel("Part des déplacements domicile-travail (%)")
    ax.set_title("Mode de transport domicile-travail"); save(fig, "bar_h")

    fig, ax = plt.subplots(figsize=(8, 4.5))
    ax.pie([65, 25, 10], labels=["Nucléaire 65 %", "Renouvelables 25 %", "Gaz 10 %"],
           colors=["#4c78a8", "#54a24b", "#f58518"], startangle=90, wedgeprops=dict(width=0.42, edgecolor="white"))
    ax.set_title("Origine de l'électricité en 2024"); save(fig, "donut")

    fig, ax = plt.subplots(figsize=(8, 3.6))
    yrs, txt = [2019, 2021, 2023, 2024], ["Idée", "Prototype", "Lancement", "10 000 utilisateurs"]
    ax.hlines(0, 2018.5, 2024.6, color="#444", lw=2); ax.plot(yrs, [0] * 4, "o", ms=12, color="#e45756")
    for y, t, k in zip(yrs, txt, (1, -1, 1, -1)):
        ax.annotate(f"{y}\n{t}", (y, 0), (y, 0.55 * k), ha="center", va="center",
                    arrowprops=dict(arrowstyle="-", color="#888"), fontsize=12)
    ax.set_ylim(-1, 1); ax.set_xlim(2018.3, 2024.9); ax.axis("off"); ax.set_title("Historique du projet"); save(fig, "timeline")

    fig, ax = plt.subplots(figsize=(8, 4.5)); ax.set_xlim(0, 10); ax.set_ylim(0, 6); ax.axis("off")

    def box(x, y, t1, t2, c):
        ax.add_patch(FancyBboxPatch((x - 1.6, y - 0.7), 3.2, 1.4, boxstyle="round,pad=0.05", fc=c, ec="#333"))
        ax.text(x, y + 0.2, t1, ha="center", fontweight="bold"); ax.text(x, y - 0.3, t2, ha="center", fontsize=10)

    box(5, 5, "Claire Morel", "Présidente", "#cfe2f3"); box(2.4, 1.6, "Hugo Faure", "Trésorier", "#d9ead3")
    box(7.6, 1.6, "Inès Roche", "Secrétaire", "#fff2cc")
    for x in (2.4, 7.6):
        ax.add_patch(FancyArrowPatch((5, 4.25), (x, 2.35), arrowstyle="-", color="#333"))
    ax.set_title("Organisation de l'association"); save(fig, "orgchart")

    fig, ax = plt.subplots(figsize=(8, 3.6)); ax.set_xlim(0, 10); ax.set_ylim(0, 4); ax.axis("off")
    for x, t, c in ((1.3, "Gare", "#f4cccc"), (5, "Mairie", "#cfe2f3"), (8.7, "Bibliothèque", "#d9ead3")):
        ax.add_patch(FancyBboxPatch((x - 1.1, 1.4), 2.2, 1.2, boxstyle="round,pad=0.05", fc=c, ec="#333"))
        ax.text(x, 2, t, ha="center", va="center", fontweight="bold")
    for a, b, t in ((2.5, 3.8, "600 m"), (6.2, 7.5, "200 m")):
        ax.add_patch(FancyArrowPatch((a, 2), (b, 2), arrowstyle="-|>", mutation_scale=18, color="#333"))
        ax.text((a + b) / 2, 2.35, t, ha="center")
    ax.set_title("Plan d'accès à la bibliothèque"); save(fig, "plan")

    fig, ax = plt.subplots(figsize=(8, 4.2)); ax.set_xlim(0, 9); ax.set_ylim(0, 5); ax.axis("off")
    for x, c, t in ((1.5, "#f2c744", "Emballages"), (4.5, "#5aa454", "Verre"), (7.5, "#8c8c8c", "Ordures ménagères")):
        ax.add_patch(Rectangle((x - 0.9, 1), 1.8, 2.6, fc=c, ec="#222")); ax.add_patch(Rectangle((x - 1.0, 3.6), 2.0, 0.35, fc=c, ec="#222"))
        ax.text(x, 0.45, t, ha="center", fontsize=12)
    ax.set_title("Les couleurs des bacs de tri"); save(fig, "bins")

    im = Image.new("RGB", (880, 220), "#8c1d40"); d = ImageDraw.Draw(im)
    d.text((40, 30), "SOLDES D'HIVER −30 %", fill="white", font=ImageFont.truetype(FB, 52))
    d.text((40, 125), "du 8 au 28 janvier — Librairie Page 12", fill="#ffe08a", font=ImageFont.truetype(FR, 32))
    im.save(f"{out}/banner_soldes.jpg", quality=88)

    im = Image.new("RGB", (880, 330), "white"); d = ImageDraw.Draw(im)
    d.ellipse((90, 70, 230, 210), fill="#f6c945", outline="#a97c00", width=5)
    d.rectangle((135, 210, 185, 255), fill="#555"); d.rectangle((120, 255, 200, 270), fill="#333")
    for ln in ((60, 60, 90, 90), (265, 60, 235, 90), (50, 140, 85, 140), (270, 140, 235, 140)):
        d.line(ln, fill="#a97c00", width=6)
    d.text((310, 105), "Atelier", fill="#222", font=ImageFont.truetype(FS, 64))
    d.text((310, 185), "Lumière", fill="#a97c00", font=ImageFont.truetype(FS, 64))
    im.save(f"{out}/logo_atelier.jpg", quality=88)

    im = Image.new("RGB", (880, 330), "white"); d = ImageDraw.Draw(im)
    f, fb = ImageFont.truetype(FR, 34), ImageFont.truetype(FB, 36)
    d.rectangle((40, 30, 840, 100), fill="#1f6fb2"); d.text((60, 42), "Tarifs d'entrée — piscine municipale", fill="white", font=fb)
    for i, (a, b) in enumerate((("Adulte", "4,50 €"), ("Enfant (3 à 17 ans)", "2,80 €"), ("Moins de 3 ans", "Gratuit"))):
        y = 115 + i * 68; d.line((40, y + 62, 840, y + 62), fill="#bbb", width=2)
        d.text((60, y + 10), a, fill="#222", font=f); d.text((640, y + 10), b, fill="#222", font=fb)
    im.save(f"{out}/tarifs.jpg", quality=88)
    return sorted(os.listdir(out))


if __name__ == "__main__":
    if len(sys.argv) != 2:
        sys.exit(__doc__)
    print("images générées :", make(sys.argv[1]))
