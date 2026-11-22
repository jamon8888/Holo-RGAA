---
name: structure
description: Présenter les preuves layout (écarts entre rendu visuel et structure exposée) sous forme de signaux et d'extraits, sans écrire de statut. Use for /rgaa:structure.
---

# structure (Codex)

1. Appeler `list_analysis_plugins`. Si `layout` est absent, l'expliquer (feature `plugin-layout`). Aucun modèle à télécharger pour les mesures DOM/AXTree.
2. Appeler `run_analysis_plugin { "plugin": "layout", "url": "<url>" }`.
3. Afficher un tableau des signaux (vu vs exposé : titres 9.1/9.2, tableaux 5.6, ordre 10.3, forme/position 10.9/10.10) avec l'extrait et l'écart mesuré.
4. **N'écrire aucun statut (R7)** : préparer la décision ; le relecteur la pose via la page de revue ou `decide` (D2). Avis du modèle replié, « indicatif » (R3).
