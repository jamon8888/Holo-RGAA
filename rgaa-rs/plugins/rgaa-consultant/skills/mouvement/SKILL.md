---
name: mouvement
description: Présenter les preuves du plugin `motion` pour les critères 13.7 et 13.1 — courbe L(t), bande de vignettes statiques et avertissement photosensible avant toute replay. Use when the user asks for /rgaa:mouvement, to review flash or motion evidence, or to inspect time-limit controls.
---

# mouvement — preuves du plugin `motion`

## ⚠️ Avertissement photosensible

**Ne rejoue jamais l'animation incriminée à l'écran sans avertissement explicite et accord du relecteur.** Un contenu en flash (13.7) peut provoquer une crise photosensible chez le relecteur. Afficher d'abord les **vignettes statiques** et la courbe `L(t)` ; ne demander une lecture que si le relecteur la réclame explicitement, après avoir été prévenu.

## Principe

Cette skill **présente des preuves**, elle ne rend jamais de verdict et **n'écrit jamais de statut** (R7). Le test reste `review` jusqu'à ce qu'un relecteur prenne sa décision sur la page de revue ou par la commande `decide` (confirmation hors bande, décision D2). L'avis du modèle, s'il existe, est replié sous « avis du modèle (indicatif) » (R3) : il sert à ordonner la file, pas à décider.

## Prérequis

1. Appeler d'abord `list_analysis_plugins`. Si `motion` n'est pas compilé, l'expliquer : activer la feature `plugin-motion` puis reconstruire `rgaa-mcp`. Ne jamais inventer de preuve.
2. Ce plugin n'a aucun modèle à télécharger. Le sidecar `ffmpeg` sert uniquement à extraire les vignettes des vidéos ; si `list_analysis_plugins` le signale absent, l'expliquer et arrêter la revue vidéo.

## Procédure

1. Appeler `run_analysis_plugin { "plugin": "motion", "url": "<url>" }`.
2. Trier les preuves par pic de luminosité décroissant (`mesures.pic_lt`), puis par horodatage.
3. Afficher la **courbe `L(t)`** (luminance en fonction du temps) fournie en artefact SVG, en surlignant les franchissements du seuil de flash.
4. Afficher la **bande de vignettes statiques** uniquement : trois exceptions par seconde autour du pic, avec horodatage. Aucune lecture automatique.
5. Pour 13.1, afficher la durée des limites de temps et les moyens de contrôle exposés par la page, tels que mesurés.
6. Si le relecteur demande malgré tout une replay : rappeler l'avertissement photosensible, utiliser Claude for Chrome pour rejouer **une seule fois**, à vitesse réduite, puis s'arrêter.

## Présentation des preuves

```
13.7 · /accueil/banniere · 00:03.2 · Mesuree
Courbe L(t) : pic 0,92 → 0,08 en 90 ms  (seuil de flash franchi)
[▣ 00:03.1] [▣ 00:03.2] [▣ 00:03.3]   vignettes statiques
⚠️ Risque photosensible — ne pas rejouer sans accord explicite.
```

## Décision de revue (D2)

Préparer la décision, ne pas l'écrire : **aucun outil exposé à l'agent n'écrit de statut** (R7). Le relecteur pose `conforme` / `non conforme` / `non applicable` sur son compte personnel, dans la page de revue ou via `decide` avec confirmation hors bande (D2). L'indice ne remplace jamais cette décision.

## Limites

- Les vignettes sont des extraits : elles ne prouvent pas l'absence de flash entre deux instants.
- Ne pas conclure sur 13.8 (contenu clignotant contrôlable) à partir de la seule courbe : préparer une preuve complémentaire.
- Aucune lecture automatique de média, ici ni dans la page de revue.
