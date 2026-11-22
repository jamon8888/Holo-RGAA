---
name: langue
description: Présenter les preuves du plugin `lang` pour les critères 8.4 et 8.8 — langue par défaut et changements de langue — sous forme de liste de signaux et d'extraits. Use when the user asks for /rgaa:langue, to review language evidence, or to check default language and language switches.
---

# langue — preuves du plugin `lang`

## Principe

Cette skill **présente des preuves**, elle ne rend jamais de verdict et **n'écrit jamais de statut** (R7). Le test reste `review` jusqu'à ce qu'un relecteur prenne sa décision sur la page de revue ou par la commande `decide` (confirmation hors bande, décision D2). L'avis du modèle, s'il existe, est replié sous « avis du modèle (indicatif) » (R3) : il sert à ordonner la file, pas à décider.

## Prérequis

1. Appeler d'abord `list_analysis_plugins`. Si `lang` n'est pas compilé, l'expliquer : activer la feature `plugin-lang` puis reconstruire `rgaa-mcp`. Ne jamais inventer de preuve.
2. Le modèle de détection est intégré au crate : aucun `rgaa models pull` n'est nécessaire. Si `list_analysis_plugins` signale malgré tout un modèle absent, l'expliquer et s'arrêter.

## Procédure

1. Appeler `run_analysis_plugin { "plugin": "lang", "url": "<url>" }`.
2. Trier les preuves par sélecteur, puis par extrait (ordre déterministe).
3. Afficher un tableau des signaux : `lang` déclaré, `lang` détecté sur l'extrait, concordance, et position dans le document.
4. Joindre l'extrait incriminé (texte voisin, au plus 300 caractères) pour chaque changement de langue (8.8) ou incohérence de langue par défaut (8.4).
5. Distinguer `Mesuree` (attribut `lang` lu dans le DOM) de `Estimee` (détection sur le texte).

## Présentation des preuves

```
8.4 · <html lang="en"> · Mesuree
Langue déclarée : en · Langue détectée : fr · Extrait : « Nos ventes… »
8.8 · #content blockquote · Estimee
Passage en anglais non signalé : "See our terms"
```

## Décision de revue (D2)

Préparer la décision, ne pas l'écrire : **aucun outil exposé à l'agent n'écrit de statut** (R7). Le relecteur pose `conforme` / `non conforme` / `non applicable` sur son compte personnel, dans la page de revue ou via `decide` avec confirmation hors bande (D2). L'avis du modèle, s'il existe, est replié sous « avis du modèle (indicatif) » (R3).

## Limites

- La détection est une estimation : un texte court ou technique peut être mal classé.
- Ne pas conclure sur 8.3 (présence de l'attribut `lang`) à partir d'un extrait seul.
- Ne pas corriger la page : préparer la décision uniquement.
