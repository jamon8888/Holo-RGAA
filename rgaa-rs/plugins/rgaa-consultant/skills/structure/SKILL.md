---
name: structure
description: Présenter les preuves du plugin `layout` pour les critères 5.3, 5.6, 7.1, 7.5, 9.1, 9.2, 10.3, 10.9 et 10.10 — écarts entre rendu visuel et structure exposée. Use when the user asks for /rgaa:structure, to review heading or table structure evidence, or to compare the rendered page with its accessibility tree.
---

# structure — preuves du plugin `layout`

## Principe

Cette skill **présente des preuves**, elle ne rend jamais de verdict et **n'écrit jamais de statut** (R7). Le test reste `review` jusqu'à ce qu'un relecteur prenne sa décision sur la page de revue ou par la commande `decide` (confirmation hors bande, décision D2). L'avis du modèle, s'il existe, est replié sous « avis du modèle (indicatif) » (R3) : il sert à ordonner la file, pas à décider.

## Prérequis

1. Appeler d'abord `list_analysis_plugins`. Si `layout` n'est pas compilé, l'expliquer : activer la feature `plugin-layout` puis reconstruire `rgaa-mcp`. Ne jamais inventer de preuve.
2. Le LLM vision est optionnel et aucun modèle n'est à télécharger pour les mesures DOM/AXTree. Si `list_analysis_plugins` signale un modèle absent, le signaler et continuer avec les seules preuves `Mesuree`.

## Procédure

1. Appeler `run_analysis_plugin { "plugin": "layout", "url": "<url>" }`.
2. Trier les preuves par critère, puis par sélecteur (ordre déterministe).
3. Afficher un tableau des signaux : ce qui est **vu** (ordre visuel, groupes, en-têtes) face à ce qui est **exposé** (DOM, AXTree), avec l'écart mesuré.
4. Joindre l'extrait concerné : titres et leur niveau (9.1), hiérarchie (9.2), en-têtes de tableau (5.6), messages de statut (7.5), forme/taille/position (10.9/10.10).
5. Reporter `portee` : `Mesuree` pour les écarts DOM/AXTree, `Estimee` pour un départage par modèle.

## Présentation des preuves

```
9.1 · main h2.sous-titre · Mesuree
Vu : titre de niveau 2 visuel · Exposé : <h4> · écart de 2 niveaux
10.3 · main section · Mesuree
Ordre visuel : A, B, C · Ordre DOM : A, C, B
```

## Décision de revue (D2)

Préparer la décision, ne pas l'écrire : **aucun outil exposé à l'agent n'écrit de statut** (R7). Le relecteur pose `conforme` / `non conforme` / `non applicable` sur son compte personnel, dans la page de revue ou via `decide` avec confirmation hors bande (D2). L'avis du modèle, s'il existe, est replié sous « avis du modèle (indicatif) » (R3).

## Limites

- L'AXTree dépend du navigateur : ne pas généraliser un écart à tous les environnements.
- Ne pas confondre 7.1 (compatibilité avec les technologies d'assistance) et 7.2 (alternative de script) — ce dernier est couvert par `pertinence`.
- Ne pas modifier la structure de la page : préparer la décision uniquement.
