# 0001 — Une alternative d'image de données restitue la série complète

- Statut : accepté (confirmé par la 2e passe du lot D, 2026-10-13)
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

## Révision du 2026-10-13 (soir) — confirmé, et la règle est plus simple que prévu

La 1re passe du lot D souffrait d'un défaut de protocole, pas d'un désaccord de
doctrine : sur ses 39 cartes, **toutes** celles portant une correction écrite
étaient « non pertinente » et **toutes** celles sans commentaire étaient
« pertinente », sans exception (phi = −1,000). Le verdict n'apportait rien de
plus que le fait d'avoir écrit. Preuve : l'ancre `D037`, dont l'alternative est
factuellement fausse, était passée en « pertinente » sans un mot.

Les 39 mêmes cartes ont été re-soumises (ordre et identifiants changés,
réponses précédentes inaccessibles) avec **une justification obligatoire sur
chaque carte, « pertinente » comprise**. Résultats :

| | 1re passe | 2e passe |
|---|---|---|
| ancres | 3/4 (`D037` échouée) | **4/4** |
| axe « complétude » | 14/18 | **17/18** |
| axe « nature du média annoncée » | 8/14 | 7/14 |
| axe « légende de la galerie » | 1/3 | 0/3 |

Les **quatre** écarts qui avaient fait repasser cet ADR en provisoire se sont
tous retournés, avec un motif précis à chaque fois :

- `D007` (titre seul, valeurs déjà dans le texte voisin) → non pertinente,
  « manque la répartition par type d'énergie ». **Le cas décisif est reproduit**
  (il reproduit `C004`).
- `D011` → « manque les membres et leurs rôles » ; `D009` → « manque trois
  jalons et dates » ; `D006` → « ajouter au début Répartition de l'énergie
  consommée en 2024 ».

La préférence de la 2e passe n'est pas un choix post hoc : le changement de
protocole a été décidé pour corriger un défaut mesuré *avant* de voir les
nouveaux verdicts, et le contrôle pré-établi (les ancres) passe de 3/4 à 4/4.

### Ce qui change dans la décision

Les deux conventions « série complète » et « nature du média annoncée » étaient
une mauvaise découpe. L'étiqueteur a énoncé son critère lui-même, dans les
13 justifications de ses verdicts « pertinente » : **cohérence et complétude
entre l'image, le contexte de la page et l'alternative**. « contexte » y
apparaît 10 fois, « image » 9, « complétude » 7, « cohérence » 5.

Ce critère unique explique les trois axes :

- **images de données** : la complétude porte sur les grandeurs de l'image ;
- **photos** : l'alternative doit reprendre ce que la page dit de l'image
  (légende de galerie, raison d'être), et une description fidèle mais hors
  contexte est rejetée — « alternative textuelle ne couvre pas le contexte de
  la page juste la description de l'image » ;
- **préfixe « Photo : »** : incident, pas un critère. L'axe reste à 50 %, et
  `D019` comme `D031` ont été rejetées *bien que* portant l'annonce. Cette
  convention est **abandonnée**.

### Ce qui est infirmé dans la version précédente de cet ADR

La clause « elle ne reprend pas le contenu rédactionnel du texte voisin » est
**fausse**, et c'était l'inverse : la reprise de la légende est *due*. Les trois
cartes qui la portent sont jugées pertinentes en 2e passe (0/3 conformes à
l'ancienne règle). `CONTEXT.md` remplace donc « Reprise du contexte », qui
désignait un défaut, par **Cohérence avec le contexte**, qui est une exigence,
et introduit **Description seule** pour nommer le défaut réel.

### Limites qui restent

- Cohérence test-retest entre les deux passes : 29/39 = 74 %, IC95
  [59 % ; 85 %]. C'est bas, mais les bascules ne sont pas du bruit : elles vont
  toutes dans le sens d'une exigence accrue, et la passe la plus exigeante est
  celle dont les ancres sont bonnes.
- Aucun modèle n'a encore été mesuré contre ces étiquettes-ci. Les 79 % de
  `swift-1.5-27b` et les 68 % de `gemma4:e2b` l'ont été contre le lot C, dont le
  phi est de −0,94 : ces deux chiffres sont à refaire.
- **Un second étiqueteur reste requis** avant d'appliquer ce critère au code.
  Un critère énoncé par une seule personne reste une pratique individuelle.

## Mesure du 2026-10-13 (nuit) — ce qu'un modèle atteint sur ce critère

`swift-1.5-27b` (API MyIA), lot D 2e passe, 39 cartes, température 0. La consigne
donnée au modèle énonce le critère de cet ADR ; trois versions ont été essayées.

| | v4 | v5 | v5 (2e passage) |
|---|---|---|---|
| modèle vs humain | 31/39 = 79 % | 32/39 = 82 % | **33/39 = 85 %**, kappa 0,68 |
| faux « pertinent » | 7 | 6 | **5** |
| complétude (images de données) | 17/18 | 17/18 | **17/18 = 94 %** |
| photos et logo | 7/14 | 8/14 | **9/14 = 64 %** |
| ancres | 4/4 | 4/4 | **4/4** |

Repères : réponse constante 26/39 = 67 % ; cohérence test-retest de l'étiqueteur
entre ses deux passes 29/39 = 74 %.

### Le gain de consigne n'est pas démontré

Deux passages de la **même** consigne v5 à température 0 donnent 32/39 puis
33/39 : **38/39 verdicts identiques**, donc une carte de bruit. Le gain v4 → v5
est de 1 à 2 cartes, soit l'ordre de grandeur de ce bruit, et la v4 n'a été
mesurée qu'une fois. On ne peut donc pas conclure que la v5 fonctionne : au
mieux elle ne dégrade rien et la direction est plausible.

### L'erreur n'est pas dans l'agrégation : inutile de découper en deux appels

La v5 fait émettre séparément `complet_vis_a_vis_de_l_image` et
`coherent_avec_le_contexte` avant le verdict. Le **ET mécanique** de ces deux
réponses, recalculé hors du modèle, donne **exactement le même résultat** que le
verdict énoncé : 33/39, kappa 0,68, 5 faux « pertinent », 1 faux « non
pertinent ». Le modèle agrège donc correctement ses propres réponses.

Conséquence : scinder l'évaluation en deux appels et combiner par un `ET` en
Rust ne gagnerait rien. L'écart est dans la **condition B elle-même** — sur les
5 faux « pertinent » restants, le modèle répond `B = oui` alors que l'étiqueteur
juge le contexte absent. C'est un désaccord de jugement, pas un défaut de
docilité.

### Verdict d'automatisation

5 faux « pertinent » sur 26 alternatives rejetées par l'humain : **19 % des
mauvaises alternatives seraient validées**. Le `pass` automatique est exclu, et
la production d'un `pass` même sous revue reste imprudente. Le niveau atteint
est **Assisté** au sens de `CONTEXT.md` : le modèle peut remonter des `review`
et motiver son signalement (sa détection de l'incomplétude chiffrée est à 94 %),
jamais conclure seul à la conformité.

### Réserve sur un des désaccords

Sur `D031`, l'alternative « Logo de l'Atelier Lumière » est jugée non pertinente
par l'étiqueteur, qui demande d'y ajouter « créateur de luminaires depuis
2015 ». C'est une alternative que la plupart des auditeurs tiendraient pour
correcte, et l'ajout demandé relève de l'argumentaire de la page. Le modèle
donne ici la réponse conventionnelle. Ce cas illustre pourquoi le second
étiqueteur reste requis : il n'est pas établi que le critère de cet ADR soit
celui de la profession plutôt que celui d'un auditeur.
