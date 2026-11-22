---
name: mouvement
description: Présenter les preuves motion (courbe L(t), vignettes statiques) avec avertissement photosensible avant toute replay, sans écrire de statut. Use for /rgaa:mouvement.
---

# mouvement (Codex)

⚠️ **Risque photosensible** : ne rejoue jamais l'animation incriminée sans avertissement explicite et accord du relecteur. Vignettes statiques et courbe `L(t)` d'abord.

1. Appeler `list_analysis_plugins`. Si `motion` est absent, l'expliquer (feature `plugin-motion`). Aucun modèle à télécharger ; le sidecar `ffmpeg` sert aux vignettes.
2. Appeler `run_analysis_plugin { "plugin": "motion", "url": "<url>" }`.
3. Trier par pic de luminosité décroissant ; afficher courbe, bande de vignettes **statiques** et limites de temps. Replay unique, à vitesse réduite, seulement sur demande via **Codex for Chrome**.
4. **N'écrire aucun statut (R7)** : préparer la décision ; le relecteur la pose via la page de revue ou `decide` (D2). Avis du modèle replié, « indicatif » (R3).
