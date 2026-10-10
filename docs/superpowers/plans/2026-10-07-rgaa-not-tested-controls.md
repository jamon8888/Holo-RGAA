# Plan — sondes RGAA pour les critères non testés

## Objectif
Remplacer les résultats `NotTested` des deux pages auditées sur danijelagracner.com par des contrôles automatisés adaptés, sans transformer l'absence de signal en preuve de conformité. Les critères qui exigent une vérification exhaustive ou un jugement restent `NeedsReview`, avec le résultat du pré-triage.

## Critères visés
1.7, 7.2, 7.3, 8.1, 8.9, 9.4, 10.4, 10.11, 10.12, 12.2, 12.5, 12.9, 12.10, 12.11, 13.1, 13.2, 13.7, 13.8, 13.9, 13.10, 13.11 et 13.12.

## Étapes
1. Étendre les sondes gap-fix déclarées dans `mechanisms.toml` pour produire un résultat `review` sur une exécution propre/incomplète, et un `fail` uniquement sur une preuve objective.
2. Ajouter des inventaires DOM adaptés aux critères non couverts : interaction clavier, structure/version de document, présentation, direction du texte, citations, zoom/reflow, délais, clignotement/animations, et orientation.
3. Mettre à jour la documentation d'architecture pour expliquer les limites et le statut partiel de chaque sonde.
4. Vérifier la compilation ciblée, puis relancer l'audit des deux pages et régénérer le rapport HTML avec le nouveau modèle et les résultats des contrôles.

## Garde-fous
- Aucun `Pass` ne sera produit par ces sondes partielles.
- Les `Fail` s'appuieront sur un signal DOM/CSS explicite ; les cas ambigus seront `NeedsReview`.
- Ne pas toucher aux modifications préexistantes relatives au fournisseur MyIA.
