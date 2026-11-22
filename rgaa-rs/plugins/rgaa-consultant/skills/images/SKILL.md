---
name: images
description: Présenter les preuves du plugin `images` pour les critères 1.2, 1.3, 1.4, 1.7 et 1.8 — regroupement par pHash, vignette, alt, texte OCR et similarité. Use when the user asks for /rgaa:images, to review image-alternative evidence, or to prepare a batched review decision after confirmation.
---

# images — preuves du plugin `images`

## Principe

Cette skill **présente des preuves**, elle ne rend jamais de verdict et **n'écrit jamais de statut** (R7). Le test reste `review` jusqu'à ce qu'un relecteur prenne sa décision sur la page de revue ou par la commande `decide` (confirmation hors bande, décision D2). L'avis du modèle est replié sous « **avis du modèle (indicatif)** » (R3) : il sert à ordonner la file, pas à décider.

## Prérequis

1. Appeler d'abord `list_analysis_plugins`. Si `images` n'est pas compilé, l'expliquer : activer la feature `plugin-images` puis reconstruire `rgaa-mcp`. Ne jamais inventer de preuve.
2. Vérifier `models[]` dans la réponse. Si OCR ou CLIP porte `present: false`, afficher exactement `rgaa models pull images` et s'arrêter là. Les règles déterministes (`Mesuree`) restent consultables sans modèle, mais l'OCR et la similarité doivent être signalées comme « preuve manquante » (spec P-03 §7.1).

## Procédure

1. Appeler `run_analysis_plugin { "plugin": "images", "url": "<url>" }`.
2. **Regrouper par pHash** (`occurrences_site`) en libellant le groupe « ce logo, 41 pages ». Trier les groupes par nombre d'occurrences décroissant, puis les images par sélecteur.
3. Pour chaque image, afficher : la vignette (artefact), le nom accessible / `alt`, le texte OCR surligné (`mesures.ocr.texte`, `couverture`), la similarité `mesures.clip.similarite_alt` face à `ancre_hasard`, la classe zero-shot, les règles déterministes déclenchées et l'indice.
4. Afficher l'avis du modèle uniquement dans un bloc replié intitulé « avis du modèle (indicatif) » — jamais comme conclusion.
5. Une décision peut être **appliquée en lot à toutes les occurrences d'un même pHash, mais seulement après confirmation explicite du relecteur** (R5). Ne jamais appliquer de lot automatiquement.

## Présentation des preuves

```
1.3 · header a.logo img · Estimee · « ce logo, 41 pages »
alt « Logo » · OCR « SOLDES -50% » (couverture 0,62)
similarité alt 0,19 < ancre de hasard 0,22 → Suspect
<details> avis du modèle (indicatif) : « pertinent ? non — manque le pourcentage de remise » </details>
```

## Décision de revue (D2)

Préparer la décision, ne pas l'écrire : **aucun outil exposé à l'agent n'écrit de statut** (R7). Le relecteur pose `conforme` / `non conforme` / `non applicable` sur son compte personnel, dans la page de revue ou via `decide` avec confirmation hors bande (D2). L'application en lot est une confirmation supplémentaire, pas une décision implicite.

## Limites

- Ne jamais OCRiser un CAPTCHA (1.4) : présenter seulement la question posée au modèle sur la nature et la fonction de l'image.
- `Estimee` mène toujours à la revue ; une similarité au-dessus de l'ancre n'est pas une preuve de conformité.
- Ne pas conclure sur l'accessibilité d'une image dont le nom accessible manque dans la preuve : demander une preuve complémentaire.
