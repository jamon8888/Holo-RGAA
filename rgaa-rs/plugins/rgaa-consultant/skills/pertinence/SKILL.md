---
name: pertinence
description: Présenter les preuves du plugin `text` pour les critères de pertinence textuelle (1.5, 2.2, 5.2, 5.5, 6.1, 7.2, 8.6, 11.x, 12.3, 13.4, 13.6) — signaux, extraits et similarité au contexte. Use when the user asks for /rgaa:pertinence, to review text-relevance evidence, or to prepare a review decision on a label, title or link.
---

# pertinence — preuves du plugin `text`

## Principe

Cette skill **présente des preuves**, elle ne rend jamais de verdict et **n'écrit jamais de statut** (R7). Le test reste `review` jusqu'à ce qu'un relecteur prenne sa décision sur la page de revue ou par la commande `decide` (confirmation hors bande, décision D2). L'avis du modèle, s'il existe, est replié sous « avis du modèle (indicatif) » (R3) : il sert à ordonner la file, pas à décider.

## Prérequis

1. Appeler d'abord `list_analysis_plugins`. Si `text` n'est pas compilé, l'expliquer : activer la feature `plugin-text` puis reconstruire `rgaa-mcp`. Ne jamais inventer de preuve.
2. Vérifier `models[]` dans la réponse. Si les plongements texte locaux portent `present: false`, afficher exactement `rgaa models pull text` et s'arrêter là. Ne pas remplacer la similarité par une appréciation visuelle.

## Procédure

1. Appeler `run_analysis_plugin { "plugin": "text", "url": "<url>" }`.
2. Trier les preuves par similarité au contexte croissante, puis par sélecteur (ordre déterministe).
3. Afficher un tableau des signaux : texte proposé, contexte de la page, similarité calculée et ancre de hasard, règles déterministes déclenchées.
4. Joindre l'extrait incriminé (libellé, titre, résumé de tableau, intitulé de bouton) avec son sélecteur.
5. Reporter `portee` : `Mesuree` pour les règles déterministes, `Estimee` pour les plongements et l'avis du modèle. Ce dernier reste replié sous « avis du modèle (indicatif) » (R3).

## Présentation des preuves

```
8.6 · <title> · Mesuree
Titre « Accueil » · Contenu principal : « Commandes passées et suivi »
8.6 · <title> · Estimee
similarité 0,18 < ancre de hasard 0,31 → Suspect
```

## Décision de revue (D2)

Préparer la décision, ne pas l'écrire : **aucun outil exposé à l'agent n'écrit de statut** (R7). Le relecteur pose `conforme` / `non conforme` / `non applicable` sur son compte personnel, dans la page de revue ou via `decide` avec confirmation hors bande (D2). L'avis du modèle, s'il existe, est replié sous « avis du modèle (indicatif) » (R3).

## Limites

- Une similarité élevée ne prouve pas la pertinence : le relecteur reste seul juge du sens.
- Ne pas conclure sur un texte dont l'extrait de contexte est absent de la preuve.
- Ne pas modifier les textes de la page : préparer la décision uniquement.
