---
name: images
description: Présenter les preuves images (pHash, vignette, alt, OCR, similarité) et proposer un lot après confirmation, sans écrire de statut. Use for /rgaa:images.
---

# images (Codex)

1. Appeler `list_analysis_plugins`. Si `images` est absent, l'expliquer (feature `plugin-images`). Si OCR/CLIP est absent (`present: false`), afficher `rgaa models pull images` et s'arrêter.
2. Appeler `run_analysis_plugin { "plugin": "images", "url": "<url>" }`.
3. Regrouper par pHash (« ce logo, 41 pages ») ; afficher vignette, alt, texte OCR, similarité face à l'ancre et occurrences. Avis du modèle replié sous « avis du modèle (indicatif) » (R3).
4. Lot possible **après confirmation explicite du relecteur** uniquement (R5). **N'écrire aucun statut (R7)** : décision par la page de revue ou `decide` (D2).
