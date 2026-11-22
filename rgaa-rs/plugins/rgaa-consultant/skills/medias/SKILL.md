---
name: medias
description: Présenter les preuves du plugin `media` pour les critères 4.2, 4.4, 4.6 et 4.9 — passages manquants, transcription Whisper et lecteur ouvert à l'horodatage. Use when the user asks for /rgaa:medias, to review audio/video evidence, or to compare a provided transcript with the Whisper transcript.
---

# medias — preuves du plugin `media`

## Principe

Cette skill **présente des preuves**, elle ne rend jamais de verdict et **n'écrit jamais de statut** (R7). Le test reste `review` jusqu'à ce qu'un relecteur prenne sa décision sur la page de revue ou par la commande `decide` (confirmation hors bande, décision D2). L'avis du modèle, s'il existe, est replié sous « avis du modèle (indicatif) » (R3) : il sert à ordonner la file, pas à décider.

## Prérequis

1. Appeler d'abord `list_analysis_plugins`. Si `media` n'est pas compilé, l'expliquer : activer la feature `plugin-media` puis reconstruire `rgaa-mcp`. Ne jamais inventer de preuve.
2. Vérifier `models[]` dans la réponse. Si VAD Silero ou Whisper porte `present: false`, afficher exactement `rgaa models pull media` et s'arrêter là. Ne jamais lancer la transcription « à vue ».

## Procédure

1. Appeler `run_analysis_plugin { "plugin": "media", "url": "<url>" }`.
2. Trier les preuves par horodatage croissant, puis par type de passage (transcription, sous-titres, audiodescription).
3. Pour chaque **passage manquant ou divergent**, ouvrir le lecteur à l'horodatage : `#t=120` dans l'URL du média, ou `currentTime = 120` via **Claude for Chrome**. Toujours demander avant toute lecture ; ne jamais lancer la lecture automatiquement (RGAA 4.10, page de revue elle-même conforme).
4. Afficher **l'extrait Whisper face à l'extrait fourni** par le client, avec l'horodatage de début/fin, le taux d'erreur de mots connu de la fixture et l'indice.
5. Distinguer les preuves `Mesuree` (présence d'une piste, synchronisation) des preuves `Estimee` (pertinence de la transcription).

## Présentation des preuves

```
4.4 · vidéo /livraison · 02:00–02:14 · Estimee
Fourni  : « Choisissez votre mode de livraison »
Whisper : « Choisissez votre mode de livraison à domicile »
→ passage manquant : « à domicile » (sous-titre coupé)
```

## Décision de revue (D2)

Préparer la décision, ne pas l'écrire : **aucun outil exposé à l'agent n'écrit de statut** (R7). Le relecteur pose `conforme` / `non conforme` / `non applicable` sur son compte personnel, dans la page de revue ou via `decide` avec confirmation hors bande (D2). L'avis du modèle, s'il existe, est replié sous « avis du modèle (indicatif) » (R3).

## Limites

- Ne pas résoudre un CAPTCHA audio ; le signaler comme tel.
- Une transcription automatique peut être exacte sur les mots mais fausse sur le sens : la pertinence reste une décision humaine.
- Ne pas afficher de « verdict IA » : l'extrait Whisper est une mesure, pas une conclusion.
