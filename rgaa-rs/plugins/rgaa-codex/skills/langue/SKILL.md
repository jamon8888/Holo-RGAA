---
name: langue
description: Présenter les preuves lang (langue par défaut et changements) sous forme de signaux et d'extraits, sans écrire de statut. Use for /rgaa:langue.
---

# langue (Codex)

1. Appeler `list_analysis_plugins`. Si `lang` est absent, l'expliquer (feature `plugin-lang`). Modèle intégré : aucun `rgaa models pull`.
2. Appeler `run_analysis_plugin { "plugin": "lang", "url": "<url>" }`.
3. Afficher un tableau des signaux (`lang` déclaré / détecté) et l'extrait pour chaque changement de langue (8.8) ou incohérence (8.4).
4. **N'écrire aucun statut (R7)** : préparer la décision ; le relecteur la pose via la page de revue ou `decide` (D2). Avis du modèle replié, « indicatif » (R3).
