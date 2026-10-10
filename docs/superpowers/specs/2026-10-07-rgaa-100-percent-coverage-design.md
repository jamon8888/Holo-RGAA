# RGAA — verdict automatique sur 100 % des critères

**Date :** 2026-10-07
**Statut :** conception approuvée en conversation ; spécification à relire avant plan d’implémentation.
**Périmètre :** rgaa-rs, pipeline `rgaa-cli`, représentation des résultats et rapport HTML.

## Objectif convenu

Pour chaque page auditée, le programme doit produire un verdict automatique pour chacun des 106 critères RGAA 4.1.2, y compris lorsqu’un critère demande habituellement un jugement humain.

Le verdict automatique est une prédiction machine. Le système doit distinguer cette prédiction d’un statut vérifié à partir de preuves suffisantes ou confirmé par une revue humaine. Aucun score de conformité officiel ne doit traiter une prédiction IA sans preuve comme une conformité vérifiée.

## Résultats attendus

Chaque résultat de critère conserve ou expose les informations suivantes :

- `automated_verdict` : verdict automatique toujours renseigné dans un audit terminé (`pass`, `fail` ou `not_applicable`). Quand les preuves sont insuffisantes, MyIA doit quand même émettre une prédiction `pass` ou `fail` ; celle-ci est marquée comme estimation.
- `verdict_basis` : type de fondement (`axe`, `deterministic`, `browser`, `model_estimate`, ou combinaison de sources).
- `evidence` : éléments DOM/AX, règles axe, observations de comportement, captures ou éléments utilisés par MyIA. Une estimation sans preuve doit déclarer explicitement cette lacune.
- `confidence` : confiance calibrée sur un corpus annoté, distincte de la confiance autodéclarée par le modèle.
- `review_required` et `review_reason` : indique qu’un jugement humain reste recommandé ou obligatoire, sans supprimer le verdict automatique.
- `verified_status` : statut vérifié, initialement absent ou `needs_review` lorsqu’aucune preuve suffisante ne permet de trancher ; une revue humaine peut le confirmer ou le corriger.
- `review_events` : historique des décisions de revue, avec statut retenu, auteur, date et motif. La prédiction initiale demeure conservée.

Le schéma reste rétrocompatible avec le champ `status` existant pendant une période de transition. Le rapport distingue explicitement la prédiction automatique et le statut vérifié.

## Architecture

### Routage

`engine_plan.json` devient une configuration exécutée par `rgaa-orchestrator`, et non une simple proposition de répartition. Chaque critère indique son mécanisme principal, ses mécanismes complémentaires et son repli. Le pipeline crée une unité de travail pour chacun des 106 critères, puis vérifie qu’aucune unité n’a été omise.

Le parcours normal est : axe-core et règles déterministes → contrôles navigateur via Obscura/CDP → MyIA pour les questions sémantiques ou visuelles → prédiction IA de dernier recours quand les preuves restent insuffisantes. La fusion des sources conserve leur provenance et n’efface pas les contradictions.

### Registre des tests

Le registre décrit les 258 tests RGAA, pas seulement les 106 critères. Pour chaque test, il documente :

- un ou plusieurs mécanismes responsables ;
- une couverture `complete` ou `partial` ;
- les résultats que le mécanisme peut produire ;
- les preuves attendues et le parcours de repli ;
- les fixtures conformes et non conformes.

Un mécanisme partiel peut signaler un échec observé, mais son absence de violation ne suffit pas à conclure `pass`. Un critère actuellement classé manuel reçoit une estimation MyIA et conserve `review_required`.

### Preuves et fiabilité

Les composants alimentent une structure de preuves commune. Les contrôles Obscura couvrent les états navigateur nécessaires, notamment clavier/focus, reflow à 320 px, zoom, orientation, formulaires et interactions. Les évaluations IA s’appuient sur le contexte pertinent et les éléments observés plutôt que sur le seul intitulé du critère.

Chaque mécanisme nouveau fournit des fixtures pass/fail. Les verdicts IA sont évalués sur un corpus annoté ; les faux conformes et faux échecs sont suivis séparément. Le niveau de confiance est calibré sur cette évaluation.

## Disponibilité et erreurs

La cible de 100 % s’applique aux audits terminés avec les services requis disponibles. Si une page ne charge pas, si le navigateur échoue ou si MyIA ne répond pas pour une unité qui en dépend, l’audit est signalé comme incomplet/échoué et ne prétend pas avoir atteint 100 %. Aucun verdict de remplacement n’est inventé silencieusement.

Un audit terminé peut produire un verdict `model_estimate` de faible confiance lorsque la preuve ne permet pas de conclure. Le rapport indique alors clairement qu’il s’agit d’une prédiction, et non d’une vérification de conformité.

## Revue humaine

Une revue peut confirmer ou remplacer `verified_status`, avec auteur, date et motif. Elle ne remplace ni ne supprime le verdict automatique. Pour le premier périmètre, les événements de revue sont conservés dans les artefacts JSON d’audit ; l’authentification multi-utilisateur et un service de persistance centralisé sont hors périmètre.

## Mesures et rapport

Le rapport présente trois mesures distinctes :

1. **Couverture des verdicts automatiques** : critères-page avec `automated_verdict` / critères-page attendus. Cible : 100 % pour un audit terminé. Pour quatre pages, le dénominateur est 424.
2. **Couverture des contrôles avec preuve** : tests dont le mécanisme a réellement exécuté le contrôle et fourni une preuve / tests prévus. Cette mesure peut être inférieure à 100 %.
3. **Conformité vérifiée** : critères tranchés par des preuves suffisantes ou une revue humaine. Les estimations IA seules n’augmentent pas le taux de conformité vérifiée.

Les pourcentages multi-pages sont calculés à partir des compteurs agrégés non arrondis, et non par moyenne de taux de page déjà arrondis. Les pages affichent chacune leurs propres compteurs. Le libellé générique « Couverture moteur » est remplacé par ces mesures explicites.

Chaque ligne de critère affiche verdict automatique, fondement, preuve, confiance, statut vérifié et besoin de revue. Les prédictions à preuve faible restent visibles et filtrables.

## Critères d’acceptation

- Un audit terminé de chaque page contient exactement 106 entrées de critère et un `automated_verdict` renseigné par entrée.
- Le registre associe chacun des 258 tests à un mécanisme, à un type de couverture et à un repli explicite.
- Un invariant détecte les critères ou tests sans routage avant qu’un audit soit déclaré complet.
- Les critères partiellement couverts ne deviennent jamais conformes sur le seul silence d’un mécanisme partiel.
- Les critères qui exigent un jugement reçoivent une estimation automatique et conservent un indicateur de revue humaine.
- Une indisponibilité d’un service requis empêche de déclarer un audit complet à 100 %.
- Le rapport distingue les trois mesures et n’inclut pas les prédictions sans preuve dans la conformité vérifiée.
- Une revue humaine est historisée sans effacer le verdict automatique initial.
- Toute nouvelle sonde comporte une fixture pass et une fixture fail ; les évaluations IA sont comparées au corpus annoté.

## Hors périmètre et limites

- Garantir que toutes les prédictions automatiques subjectives soient exactes ; la conception garantit leur production et leur traçabilité, pas leur infaillibilité.
- Déclarer la conformité RGAA officielle à partir des seules prédictions IA.
- Modifier un site externe ou publier des corrections de contenu sans accès à son code ou à son système d’administration.
- Ajouter une authentification, une interface de gestion d’équipe ou un stockage centralisé des revues humaines.

## Impacts probables dans le dépôt

- `rgaa-core` : registre test-par-test, types de verdict automatique, preuve, confiance et événement de revue.
- `rgaa-orchestrator` : exécution effective du plan, garantie d’une unité par critère, collecte et fusion des preuves, repli IA.
- `rgaa-obscura` / `rgaa-rules` / `rgaa-agent` / `rgaa-holo` : mécanismes complémentaires, extraction de preuves et estimation IA pour tous les critères.
- `rgaa-report` / `scripts/rgaa-report-html.py` : trois métriques et affichage séparé de la prédiction et du statut vérifié.
- `rgaa-test-corpus` : fixtures de mécanismes et jeux annotés pour mesurer les erreurs des prédictions.

La conception ne préjuge pas encore de la décomposition en PR ni de l’ordre d’implémentation. Ces décisions seront détaillées dans le plan après validation de cette spécification.
