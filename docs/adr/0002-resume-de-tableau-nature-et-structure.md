# 0002 — Un résumé de tableau porte la nature et la structure, pas le contenu des cellules

- Statut : accepté, précisé par l'ADR 0003 (les en-têtes y sont nommés)
- Date : 2026-10-18
- Contexte d'origine : lot R (famille « résumé et libellé »), carte #330, ticket #354

## Contexte

Le lot R fait juger 40 textes courts sur cinq tests (5.2.1, 5.5.1, 8.6.1,
11.7.1, 11.8.3). Sur les **huit cartes du test 5.2.1**, l'auditeur a rejeté les
huit, aux deux lectures, avec à chaque fois la même raison écrite : « il manque
le détail des cellules du tableau ».

Quatre de ces huit résumés décrivent pourtant exactement la nature et la
structure du tableau, par exemple « Taux d'imposition par tranche de revenu,
selon que le foyer est une personne seule, un couple ou un couple avec
enfants. »

Le glossaire RGAA 4.1.2 définit le résumé mot pour mot ainsi :

> « Un résumé est un passage de texte associé à un tableau de données complexe.
> Il permet de donner des informations sur la nature et la structure du tableau
> afin d'en faciliter l'utilisation par les utilisateurs de technologies
> d'assistance par exemple. »

Le critère exigé par l'auditeur est donc **plus strict que le référentiel**.

## Décision

Nous retenons la définition du référentiel. Un résumé est pertinent s'il
explique la nature et la structure du tableau ; il n'a pas à restituer les
valeurs des cellules.

Un résumé reste non pertinent lorsqu'il est générique (« Tableau de données »),
lorsqu'il énonce une consigne de lecture, lorsqu'il donne un avis plutôt qu'une
description, ou lorsqu'il est faux au regard du tableau.

## Conséquences

- Les quatre cartes concernées (r06, r09, r10, r25) ont été étiquetées selon un
  critère que nous n'adoptons pas. **Elles ne sont pas corrigées à la main** :
  les corriger après coup alignerait l'étiquetage sur la réponse connue.
- Deux cartes (r04, r10) portent sur un emploi du temps dont les en-têtes
  tiennent dans la première ligne et la première colonne : ce n'est pas un
  tableau complexe, donc le test 5.2.1 ne s'y applique pas. Erreur de
  construction du lot.
- Le test 5.2.1 du lot R est donc à refaire sur des cartes neuves, avec la
  définition rappelée dans la feuille d'étiquetage. Les 32 cartes des quatre
  autres tests ne sont pas touchées par cette décision.
- Toute consigne donnée à un modèle sur 5.2.1 doit porter cette définition.

## Ce que cette décision ne tranche pas

La cohérence test-retest du lot R est de 12/21 = 57 %, bien en deçà des 93 %
du lot G. Cette instabilité est un problème distinct, qui touche les cinq
tests et reste ouvert.

## Alternative écartée

Conserver le critère de l'auditeur comme règle maison. Écartée : l'outil
signalerait alors des non-conformités qu'un audit RGAA opposable ne retiendrait
pas, sans que la différence soit visible dans le rapport.
