---
name: contraste
description: Présenter les preuves de contraste du plugin contrast (ratio croissant, seuil, vignette, états) sans écrire de statut. Use for /rgaa:contraste.
---

# contraste (Codex)

1. Appeler `list_analysis_plugins`. Si `contrast` est absent, l'expliquer (feature `plugin-contrast`) ; ne rien inventer.
2. Appeler `run_analysis_plugin { "plugin": "contrast", "url": "<url>" }`.
3. Trier par `mesures.ratio` croissant ; afficher ratio, seuil, couleurs, état (`repos`/`hover`/`focus`), étape et vignette. Surligner l'élément dans l'onglet du client via **Codex for Chrome**.
4. **N'écrire aucun statut (R7)** : préparer la décision ; le relecteur la pose dans la page de revue ou via `decide` (D2, confirmation hors bande). L'avis du modèle reste replié, « indicatif » (R3).
