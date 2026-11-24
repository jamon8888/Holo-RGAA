---
name: contraste
description: Présenter les preuves de contraste calculées par le plugin `contrast` pour les critères 3.2 et 3.3 — ratio croissant, seuil, états et vignette avec masque texte. Use when the user asks for /rgaa:contraste, to review contrast evidence on a URL, or to prepare a review decision on RGAA 3.2 / 3.3.
---

# contraste — preuves du plugin `contrast`

## Principe

Cette skill **présente des preuves**, elle ne rend jamais de verdict et **n'écrit jamais de statut** (R7). Le test reste `review` jusqu'à ce qu'un relecteur prenne sa décision sur la page de revue ou par la commande `decide` (confirmation hors bande, décision D2). L'avis du modèle, s'il existe, est replié sous « avis du modèle (indicatif) » (R3) : il sert à ordonner la file, pas à décider.

## Prérequis

1. Appeler d'abord `list_analysis_plugins`. Si `contrast` n'est pas compilé, l'expliquer : activer la feature `plugin-contrast` puis reconstruire `rgaa-mcp`. Ne jamais inventer de preuve.
2. Ce plugin n'a aucun modèle à télécharger. Si `list_analysis_plugins` signale un binaire absent, l'expliquer et arrêter la revue.

## Procédure

1. Appeler `run_analysis_plugin { "plugin": "contrast", "url": "<url>" }`.
2. **Trier les preuves par `mesures.ratio` croissant** : les plus faibles en tête. Conserver l'ordre d'origine pour les ratios égaux (déterminisme).
3. Demander à **Claude for Chrome** de surligner chaque élément dans l'onglet du client via `mesures.sujet.selecteur` / `xpath`, sans modifier la page audité.
4. Pour chaque preuve, afficher : ratio, seuil (`4,5:1` ou `3:1` si `texte_grand`), couleurs avant/fond, état (`repos` / `hover` / `focus`), étape (`A` CSS, `B` pixels, `C` composant) et la vignette avec masque texte quand un artefact existe.
5. Garder `portee` visible : `Mesuree` (étape A) peut alimenter un test automatique, `Estimee` (B/C) mène toujours à la revue.

## Présentation des preuves

```
3.2 · a.lien-gris · repos · étape A · Mesuree
#777777 sur #ffffff — ratio 4,48:1  (seuil 4,5:1)  ⚠ à revoir
```

Le ratio vient de la mesure du plugin, jamais d'une estimation visuelle. Les seuils et l'oracle sont ceux de la spec P-01 §6 (WebAIM Contrast Checker).

## Décision de revue (D2)

Préparer la décision, ne pas l'écrire : **aucun outil exposé à l'agent n'écrit de statut** (R7). Le relecteur pose `conforme` / `non conforme` / `non applicable` sur son compte personnel, dans la page de revue ou via `decide` avec confirmation hors bande (D2). Le `indice` (`Suspect`, `Neutre`, `Rassurant`) est un simple ordonnancement de la file, pas un verdict.

## Limites

- Ne pas conclure sur un texte désactivé ou décoratif (`indice: Neutre`).
- Les états `hover` / `focus` ne sont affichés que si l'orchestrateur les a capturés (`etat` présent dans `mesures`).
- Ne pas rejouer soi-même une animation pour mesurer un contraste : utiliser les vignettes de la preuve.

## Skills liées

- `daltonisme` — information donnée uniquement par la couleur (3.1).
- `guided-test` — protocole manuel de vérification des contrastes.
