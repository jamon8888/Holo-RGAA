---
name: daltonisme
description: Présenter les preuves colorvision (sujets indiscernables, comparaison original/simulation, filtre deutanope) sans écrire de statut. Use for /rgaa:daltonisme.
---

# daltonisme (Codex)

1. Appeler `list_analysis_plugins`. Si `colorvision` est absent, l'expliquer (feature `plugin-colorvision`). Aucun modèle à télécharger.
2. Appeler `run_analysis_plugin { "plugin": "colorvision", "url": "<url>" }`.
3. Afficher original et simulation côte à côte pour chaque sujet indiscernable. Proposer « voir la page comme un deutéranope » : filtre CSS injecté par **Codex for Chrome** avec `types/colorvision-matrices.json`, sans modifier le dépôt.
4. **N'écrire aucun statut (R7)** : préparer la décision ; le relecteur la pose via la page de revue ou `decide` (D2). Avis du modèle replié, « indicatif » (R3).
