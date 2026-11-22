---
name: daltonisme
description: Présenter les preuves du plugin `colorvision` pour le critère 3.1 — sujets indiscernables, comparaison original/simulation et filtre de page entière. Use when the user asks for /rgaa:daltonisme, to review colour-vision evidence, or to check that information is not conveyed by colour alone.
---

# daltonisme — preuves du plugin `colorvision`

## Principe

Cette skill **présente des preuves**, elle ne rend jamais de verdict et **n'écrit jamais de statut** (R7). Le test reste `review` jusqu'à ce qu'un relecteur prenne sa décision sur la page de revue ou par la commande `decide` (confirmation hors bande, décision D2). L'avis du modèle, s'il existe, est replié sous « avis du modèle (indicatif) » (R3) : il sert à ordonner la file, pas à décider.

## Prérequis

1. Appeler d'abord `list_analysis_plugins`. Si `colorvision` n'est pas compilé, l'expliquer : activer la feature `plugin-colorvision` puis reconstruire `rgaa-mcp`. Ne jamais inventer de preuve.
2. Ce plugin n'a aucun modèle à télécharger : les matrices sont embarquées dans `types/colorvision-matrices.json`. Si ce fichier est absent, le signaler et arrêter la revue.

## Procédure

1. Appeler `run_analysis_plugin { "plugin": "colorvision", "url": "<url>" }`.
2. Trier les preuves par nombre de sujets « indiscernables » décroissant, puis par sélecteur (ordre déterministe).
3. Pour chaque sujet, afficher **côte à côte l'original et la simulation** (protan / deutan / tritan) avec la vignette fournie en artefact, les couleurs d'origine et les couleurs simulées, et le signal qui les distingue (forme, texte, position).
4. Proposer, sans l'imposer : « voir la page entière comme un deutéranope ». Injecter alors le filtre CSS via **Claude for Chrome** en appliquant les matrices de `types/colorvision-matrices.json`, uniquement dans l'onglet du client et sans écrire dans le dépôt.
5. Rappeler que 3.1 porte sur *l'information*, pas sur le confort de lecture : deux couleurs différentes ne suffisent pas à conclure.

## Présentation des preuves

```
3.1 · .badge-statut · deutan · Estimee
[original]  #1f5f8b / #d8dde4
[simulé]    #7a7a7a / #7a7a7a   → indiscernable
Signal distinctif : libellé « En retire » présent dans le badge
```

## Décision de revue (D2)

Préparer la décision, ne pas l'écrire : **aucun outil exposé à l'agent n'écrit de statut** (R7). Le relecteur pose `conforme` / `non conforme` / `non applicable` sur son compte personnel, dans la page de revue ou via `decide` avec confirmation hors bande (D2). L'avis du modèle, s'il existe, est replié sous « avis du modèle (indicatif) » (R3) : il sert à ordonner la file, pas à décider.

## Limites

- La simulation est une estimation : elle ne remplace pas l'avis d'un relecteur ayant une déficience visuelle.
- Ne pas modifier la page auditée de façon permanente ; le filtre est une visualisation temporaire.
- Ne pas confondre 3.1 (couleur seule) et 3.3 (contraste) : utiliser `contraste` pour 3.3.
