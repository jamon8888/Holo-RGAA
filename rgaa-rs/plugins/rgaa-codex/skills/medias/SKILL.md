---
name: medias
description: Présenter les preuves media (passages manquants, transcription Whisper face à l'extrait fourni) sans lecture automatique ni écriture de statut. Use for /rgaa:medias.
---

# medias (Codex)

1. Appeler `list_analysis_plugins`. Si `media` est absent, l'expliquer (feature `plugin-media`). Si VAD/Whisper est absent, afficher `rgaa models pull media` et s'arrêter.
2. Appeler `run_analysis_plugin { "plugin": "media", "url": "<url>" }`.
3. Pour chaque passage manquant, ouvrir le lecteur à l'horodatage (`#t=120` ou `currentTime`) via **Codex for Chrome**, uniquement sur demande : jamais de lecture automatique. Afficher l'extrait Whisper face à l'extrait fourni.
4. **N'écrire aucun statut (R7)** : préparer la décision ; le relecteur la pose via la page de revue ou `decide` (D2). Avis du modèle replié, « indicatif » (R3).
