---
name: pertinence
description: Présenter les preuves text (signaux, extraits, similarité au contexte) sans écrire de statut. Use for /rgaa:pertinence.
---

# pertinence (Codex)

1. Appeler `list_analysis_plugins`. Si `text` est absent, l'expliquer (feature `plugin-text`). Si les plongements sont absents, afficher `rgaa models pull text` et s'arrêter.
2. Appeler `run_analysis_plugin { "plugin": "text", "url": "<url>" }`.
3. Trier par similarité croissante ; afficher un tableau des signaux (texte, contexte, similarité, ancre de hasard) et l'extrait. Avis du modèle replié, « indicatif » (R3).
4. **N'écrire aucun statut (R7)** : préparer la décision ; le relecteur la pose via la page de revue ou `decide` (D2).
