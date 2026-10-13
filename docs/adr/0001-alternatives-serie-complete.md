# 0001 — Une alternative d'image de données restitue la série complète

- Statut : **provisoire — le cas décisif est contesté par le lot D (2026-10-13)**
- Date : 2026-10-13
- Contexte d'origine : ticket #339 (grilling pertinence), lot C + re-vérification à l'aveugle

## Contexte

Pour mesurer si un modèle peut rendre un verdict sur la pertinence d'une
alternative textuelle (test 1.3.1 / 1.3.9), nous avons fait étiqueter par un
auditeur 41 images de synthèse, puis comparé le modèle à cet étiquetage.

Le modèle est tombé à 30/38 (79 %, kappa 0,56). En analysant les huit
désaccords, nous avons constaté que l'étiquetage appliquait deux exigences
absentes de la définition écrite de l'« alternative pertinente ». Une
re-vérification à l'aveugle de 16 cartes (les 8 contestées mêlées à 8
concordantes, identifiants et ordre changés) a tranché la question de savoir
s'il s'agissait de glissements ponctuels : **4 des 6 cartes contestées
re-tranchées sont restées identiques**, pour une cohérence test-retest globale
de 10/12.

Ce n'est donc pas du bruit d'étiquetage. Les deux exigences sont stables :

1. l'alternative d'un histogramme ou d'une courbe doit porter **toutes** les
   catégories et leurs valeurs, pas seulement la tête du classement ;
2. l'alternative annonce la nature du média (« Photo », « Logo de… »).

Le cas décisif est une carte construite exprès avec **les valeurs déjà
présentes dans le texte voisin** : l'auditeur demande malgré tout qu'elles
figurent dans l'alternative.

## Décision

Une alternative d'**image de données** n'est pertinente que si elle restitue la
série complète, y compris lorsque les mêmes valeurs figurent dans le texte
voisin. L'annonce de la nature du média fait également partie de la
pertinence.

## Alternatives écartées

- **S'en tenir à la lecture stricte de 1.3.9** (« courte et concise ») et
  traiter les étiquettes contraires comme des erreurs. Écarté : la
  re-vérification à l'aveugle montre une préférence stable, pas des erreurs.
  Trancher dans l'autre sens revenait à mesurer le modèle contre notre lecture
  du référentiel plutôt que contre le jugement de l'auditeur.
- **Faire dépendre la règle du texte voisin** (série complète seulement si les
  valeurs ne sont pas déjà dans la page). Écarté : c'est précisément le cas où
  l'auditeur a maintenu son verdict, et la règle deviendrait non testable sans
  analyse du texte environnant.

## Conséquences

- Tension assumée avec le test 1.3.9 : une alternative conforme à cet ADR peut
  être longue. Nous considérons que 1.3.9 impose de ne rien ajouter d'inutile,
  non d'amputer la donnée. Un auditeur externe peut ne pas partager cette
  lecture ; c'est le motif d'être de cet ADR.
- L'étiquetage du lot B, antérieur à cette règle, est inutilisable en l'état.
- Le score de 30/38 reste le chiffre publié pour le lot C : il a été mesuré
  contre l'étiquetage réel, et le recalculer à 32/38 après coup reviendrait à
  retenir la passe la plus flatteuse.
- Un lot D doit être construit autour des deux cas désormais identifiés comme
  seuls litigieux : images de données et annonce de la nature du média.
- La cohérence test-retest de l'auditeur est de 10/12 (83 %, IC95
  [55 % ; 95 %]). Elle plafonne ce qu'un modèle peut atteindre : les 79 % du
  modèle ne s'en distinguent pas sur un échantillon de cette taille.

## Révision du 2026-10-13 — le lot D ne confirme pas cette décision

Le lot D (39 cartes, construit pour isoler les trois conventions) a été étiqueté
le jour même. Il **contredit le cas décisif ci-dessus** :

- `D007` « Diagramme en anneau de l'origine de l'électricité en 2024 », avec les
  valeurs déjà dans le texte voisin, a été jugé **pertinent**. C'est exactement
  la situation de `C004`, jugée non pertinente. Les deux verdicts sont
  incompatibles.
- `D011` « Organigramme de l'association » et `D009` « Les grandes étapes du
  projet depuis 2019 » ont été jugés **pertinents**, alors que les mêmes
  alternatives étaient non pertinentes au lot C.
- Inversement, les cartes « titre seul » de l'histogramme et du tableau de
  tarifs (`D003`, `D015`) portent une demande d'ajout des chiffres — mais sans
  verdict coché, donc non comptées.

La décision reste donc **non reproduite**. Trois défauts de la passe
l'empêchent de trancher dans un sens ou dans l'autre :

1. **10 cartes sur 39 portent une correction écrite mais aucun verdict.** Ce
   sont systématiquement les cartes à rejeter, ce qui biaise tous les taux vers
   le haut. La feuille a été corrigée pour rendre ce cas visible et bloquer
   l'export.
2. **Une ancre a échoué.** `D037` « La marche est le mode de transport le plus
   utilisé » est factuellement faux (marche 13 %, voiture 48 %) et a été jugé
   pertinent. Les ancres du lot C étaient à 6/6 ; cette passe est donc moins
   fiable que la précédente, et aucun de ses taux ne doit être cité seul.
3. Les cellules du plan 2×2 sur la nature du média tombent à n = 1 à 3, trop
   peu pour conclure.

Un point est en revanche établi, et il **infirme** la règle écrite ici : la
**reprise de la légende de la galerie** est demandée, pas rejetée. Cinq
commentaires la réclament explicitement, et 2 des 3 cartes qui la portent ont
été jugées pertinentes. La clause « elle ne recopie pas le contenu rédactionnel
du texte voisin » est donc fausse au moins pour la légende d'une galerie
photo.

Prochaine étape : faire trancher les 10 cartes manquantes, puis reprendre ce
calcul. Ne pas durcir cet ADR, ni l'appliquer au code, avant qu'un second
étiqueteur ait été mesuré.
