# Protocole de calibration des tests Assistés

**Statut** : protocole opérationnel. Complète
[`human-criteria-automation.md`](human-criteria-automation.md) §2.5, §2.6 et §7.

**Source** : ticket [#354](https://github.com/jamon8888/Holo-RGAA/issues/354), méthode
établie par le prototype [#339](https://github.com/jamon8888/Holo-RGAA/issues/339).

**Vocabulaire** : `CONTEXT.md` — *Famille de preuve*, *Calibration*, *Assisté*.

## 1. Ce que la calibration mesure, et ce qu'elle ne mesure pas

Elle mesure **l'accord d'un modèle avec un étiquetage humain**, sur une famille de preuve,
avec un intervalle de confiance et des contrôles de qualité.

Elle ne mesure **pas** que la chaîne technique fonctionne. L'extraction de l'alternative et
le rendu de l'image diffèrent selon l'élément porteur, et une panne silencieuse à ce niveau
produit un étiquetage de données vides sans que rien ne le signale. Cette vérification est
**déterministe, à fixtures, en CI** — §5 — et non une campagne d'étiquetage. Une campagne
coûte des heures et ne tourne qu'une fois ; une fixture tourne à chaque commit.

Le défaut des 64 octets de `rgaa-holo` ([#353](https://github.com/jamon8888/Holo-RGAA/pull/353))
rejetait **toute** image réelle. Aucune calibration ne l'aurait trouvé ; une fixture l'attrape.

## 2. La règle non négociable : justification obligatoire partout

**Chaque carte reçoit un verdict ET une justification, « pertinente » comprise.**

Ce n'est pas une préférence de confort. Mesure faite lors de la première passe du lot D, où
la justification était facultative :

| | verdict « pertinente » | verdict « non pertinente » |
|---|---|---|
| commentaire saisi | 0 | 20 |
| aucun commentaire | 19 | 0 |

**phi = −1,000 sur 39 cartes.** Le bouton radio ne portait aucune information indépendante
du fait d'avoir écrit une correction : le signal réel était « ai-je pris la peine d'écrire ».
Preuve la plus nette : une ancre dont l'alternative est **factuellement fausse** était cochée
« pertinente », sans un mot.

Le lot C présente le même défaut (phi = −0,936), ce qui rend ses mesures inexploitables.

Effet du correctif, mêmes 39 cartes, ordre et identifiants changés :

| | 1re passe | 2e passe |
|---|---|---|
| ancres | 3/4 | **4/4** |
| conformité à la règle (images de données) | 14/18 | **17/18** |

Les quatre écarts qui avaient fait tomber l'ADR 0001 se sont **tous** retournés, chacun avec
un motif précis. La feuille d'étiquetage applique donc :

- justification d'au moins 8 caractères sur chaque carte ;
- carte incomplète signalée visuellement et comptée en tête de page ;
- **export refusé** tant qu'il en manque une, avec renvoi à la première.

**Remplir le champ ne suffit pas : la justification doit dire pourquoi.** Sur le lot « description
détaillée », 11 des 19 justifications favorables étaient la même phrase (« parfaitement cohérent très
bien décrit »). Le champ était rempli, donc le contrôle ci-dessus ne voyait rien, mais ces justifications
ne disent pas ce qui rend l'alternative suffisante : on ne peut pas vérifier qu'une complétude a été
contrôlée, et on ne peut pas en extraire un critère. `score.py` mesure donc la part de la justification
favorable la plus fréquente : 58 % sur ce lot, 15 % sur la passe propre de référence. Au-delà de 30 % la
passe est marquée suspecte (seuil indicatif, posé sur ces deux points seulement). La consigne de la feuille
demande, pour une carte favorable, **de citer ce qui a été vérifié** (une valeur, un élément, une relation).

## 3. Composition d'un lot

| Exigence | Valeur | Pourquoi |
|---|---|---|
| cartes | **≥ 36** | 39 cartes donnent IC95 [70 % ; 93 %] à 85 % d'accord. En dessous, l'intervalle ne dégage plus le repère trivial |
| équilibre des verdicts attendus | repère trivial **≤ 60 %** | le lot C, à 71 %, ne discriminait pas : un modèle répondant toujours « non pertinente » le battait. Le lot D, à 59 %, discrimine |
| ancres | **≥ 4**, dont **une factuellement fausse** | c'est l'ancre fausse qui a révélé le défaut de protocole |
| étiqueteurs | **2**, en aveugle l'un de l'autre | un critère énoncé par une seule personne reste une pratique individuelle |

Le **repère trivial** est le score du meilleur verdict constant. Il est calculé et publié
avec chaque mesure : un accord qui ne le dépasse pas ne vaut rien.

La **cohérence test-retest** de l'étiqueteur est mesurée sur un sous-ensemble re-soumis en
aveugle, identifiants et ordre changés. Elle plafonne ce qu'un modèle peut atteindre : sur la
famille calibrée elle vaut 74 %, pour 85 % d'accord du modèle.

### Une re-vérification n'est pas facultative

Sur deux familles et deux campagnes, **les glissements de l'étiqueteur sont du même type** : une
affirmation **vraie mais incomplète** est validée à la première passe, puis rejetée à la seconde avec le
motif de l'incomplétude.

| campagne | carte | 1re passe | 2e passe |
|---|---|---|---|
| alternatives d'image | « L'électricité vient surtout du nucléaire » (tête de classement seule) | pertinente | non — « Ajouter au début Répartition de Energie consommée en 2024 » |
| alternatives d'image | 3 autres cartes d'incomplétude (titre seul, jalons manquants) | pertinente | non — « manque les membres et leur rôles », « manque trois jalons et dates »… |
| descriptions détaillées | « La voiture est le mode de transport le plus utilisé » | pertinente — « vrai très cohérent » | non — « manque … transports en commun 27 %, … vélo 12 % » |

La première justification ne vérifie que l'**exactitude** ; la seconde vérifie aussi la **complétude**.
Aucun contrôle statique ne voit ce glissement, puisque champ rempli et ancres sont corrects : seule une
seconde lecture le trouve. Toute campagne comporte donc une re-vérification (`recheck.py`) d'au moins 12
cartes, dont **toutes celles où l'étiqueteur diverge de la règle**, et le verdict retenu pour une carte
qui bascule est celui de la seconde passe, justification à l'appui.

Un résultat voisin sur le lot « description détaillée » : sur 11 cartes non signalées, 11 sont stables.
Le glissement ne se produit donc pas au hasard, il se concentre sur les cartes qui le méritaient. Ce qui
rend la re-vérification **ciblée** efficace : 14 cartes ont suffi à le trouver.

### La re-vérification doit être équilibrée, sinon elle fabrique l'accord

Sur le lot R, la re-vérification comptait **17 cartes divergentes sur 21**. Sur une carte choisie
parce que l'étiqueteur s'écarte de la règle, tout changement de verdict ne peut aller que dans un
sens : vers la règle. Les 9 changements observés sont donc allés **9 fois sur 9** vers la prédiction,
et l'accord du modèle est passé de 67 % à 88 % par ce seul effet de composition. Ce n'était pas une
convergence, c'était le dispositif.

Une re-vérification comporte donc **au moins autant de cartes concordantes que de cartes
divergentes**. Le rapport publie les deux taux de stabilité séparément : sur les cartes signalées et
sur les autres. Un écart entre les deux se lit alors comme un fait, non comme un résultat.

### Un lot n'est utilisable que si l'étiqueteur est stable

La cohérence test-retest plafonne ce qu'un modèle peut atteindre : on ne mesure pas un modèle contre
une référence qui ne se reproduit pas. Mesures obtenues : **93 % sur le lot G**, **57 % sur le lot R**.
Sous **80 %**, le lot n'est pas admis comme référence ; on cherche d'abord pourquoi l'étiquetage varie
— consigne absente, cartes ambiguës, critère non arrêté — avant de mesurer quoi que ce soit.

### Une règle énoncée une seule fois n'est pas lue

Le lot T portait la définition du glossaire dans un encadré en tête de feuille. L'étiqueteur ne l'a
pas vue : il a rejeté les 8 cartes, **ancre positive comprise**, en appliquant son critère habituel.
Les mêmes 8 cartes, règle rappelée **sur chaque carte** juste au-dessus de la question (champ `rule`
de `sheet.py`), ont donné 8 verdicts conformes et 3 ancres sur 3.

Toute règle qu'une campagne impose est donc rappelée sur chaque carte, au point de décision. Un
en-tête de page ne vaut pas consigne.

### Une raison écrite doit s'appliquer aux cartes équivalentes

`g03` (prix inversés) est rejetée aux deux passes pour « manque le titre Piscine municipale », alors
que `g19`, qui manque le même élément, est validée aux deux passes. Les verdicts sont stables et
probablement justes — l'erreur réelle de `g03` est l'inversion des prix, jamais mentionnée — mais la
raison écrite ne distingue pas les deux cartes. **Extraire un critère des justifications ne vaut que si
la raison citée est appliquée de la même façon à toute carte équivalente** ; sinon on écrit dans la
consigne modèle une règle que l'étiqueteur lui-même n'applique pas. À vérifier avant chaque consigne.

## 4. Seuil d'admission d'un modèle

| Condition | Seuil |
|---|---|
| ancres | **4/4** |
| accord | au-dessus du repère trivial, **intervalles de confiance disjoints** |
| pouvoir discriminant | **le lot doit être mesuré avec un modèle faible de référence** (ici `gemma4:e2b`) : il doit y tomber nettement sous le modèle fort, intervalles disjoints. Sinon le lot est déclaré *non discriminant* et aucune admission n'y compte. Lot D2 : `swift` 85 % [70 ; 93] vs `gemma` 56 % [41 ; 71] — séparation à la limite (1 point de recouvrement). Lot G : 100 % vs 92 % — non discriminant |
| faux « pertinent » | **≤ 10 %** des éléments rejetés par l'humain — chacun est une non-conformité manquée |
| faux « non pertinent » | **≤ 25 %** des éléments acceptés par l'humain — chacun est du travail de revue inutile |

Aucun modèle n'est épinglé : un seuil survit au changement de modèle et de passerelle.

### Pourquoi un plafond sur les faux « non pertinent »

Le seuil ne portait d'abord que sur les faux « pertinent ». Il a **admis une consigne qui avait
dégradé le modèle** : sur le lot 5.2.1, `Opus` passe de 33/36 = 92 % (kappa 0,83) à 29/36 = 81 %
(kappa 0,59) entre deux rédactions, et se trouve admis par la seconde parce qu'il a cessé de faire
des faux « pertinent » — en rejetant 7 résumés sur 16 que l'auditeur accepte. Un modèle qui rejette
tout passait donc le seuil, exactement le travers que le repère trivial contrôle chez l'étiqueteur.

Les deux erreurs ne coûtent pas la même chose, d'où deux plafonds différents : un faux « pertinent »
laisse passer une non-conformité et fausse l'audit ; un faux « non pertinent » ajoute un élément
conforme à la file de revue humaine. Le second est donc toléré plus largement, mais pas sans borne :
au-delà du quart des éléments conformes renvoyés en revue, l'assistance ne fait plus gagner de temps.
Le plafond de 25 % est indicatif et à revoir dès qu'une campagne mesure le coût réel de la revue.

La condition « au-dessus du repère trivial, intervalles disjoints » figurait au protocole mais
**n'était pas implémentée** dans `score.py` : elle l'est depuis. À elle seule, elle refusait déjà la
seconde rédaction (IC modèle [65 % ; …] contre repère trivial [… ; 70 %]).

État mesuré :

| Modèle | Ancres | Accord | Faux « pertinent » | Verdict |
|---|---|---|---|---|
| `swift-1.5-27b` (API) | 4/4 | 33/39 = 85 %, kappa 0,68 | **19 %** | admis en *Assisté*, échoue le seuil |
| `gemma4:e2b` (local) | — | lot D2 : 22/39 = 56 %, IC95 [41 % ; 71 %], **sous son repère trivial de 67 %**, kappa 0,26, faux « pertinent » 17/26 = 65 % | — | insuffisant (figure antérieure 67 % vs 70 % tirée du lot C vicié, remplacée) |

Famille « résumé de tableau » — **provenance incertaine, à refaire avant de servir à une
admission.** L'en-tête annonçait « lot de 36 cartes, test-retest 94 %, repère trivial 58 % », mais
ces chiffres ne se raccordent pas au reste du document, alors qu'ils y sont arrivés par le même
commit :

- le §8 note que **28 des 36 cartes du lot T restent à étiqueter**. Un test-retest de 94 % sur
  36 cartes exige les 36 étiquetées deux fois ; il n'a donc pas pu être mesuré sur ce lot ;
- le §4 bis relève que `Opus` est d'accord avec la clé sur **8/8 sur le lot T**, pas sur 33/36.

L'écart de repère trivial, lui, ne prouve rien : `lots/resume_tableau.py` produit 36 cartes
équilibrées 18/18, soit 50 % par construction, et un repère mesuré sur l'étiquetage humain diffère
légitimement du repère conçu — le §8 écrit d'ailleurs 59 % pour un lot « résumé et libellé » conçu
à 50 %.

Les mesures sont conservées ci-dessous : elles ont été faites, et les deux erreurs qualitatives
qu'elles ont révélées — l'accord des juges sur les mêmes cartes, et l'effet de bord de l'exigence
d'effectif — tiennent indépendamment du lot. Mais **elles ne décident pas de l'admission d'un modèle
sur cette famille** tant que le lot qui les porte n'est pas identifié. À refaire sur le lot T une
fois ses 36 cartes étiquetées.

| Modèle | Consigne | Accord | Faux « pert. » | Faux « non pert. » | Verdict |
|---|---|---|---|---|---|
| `Opus` | règle du glossaire | 33/36 = 92 %, kappa 0,83 | 15 % | 0 % | refusé (faux « pertinent ») |
| `Opus` | + exigence d'effectif | 29/36 = 81 %, kappa 0,59 | 0 % | **44 %** | refusé (faux « non pertinent », et sous le repère trivial) |
| `gemma4:e2b` | règle du glossaire | 27/35 = 77 %, kappa 0,56 | 40 % | 0 % | refusé |

Les deux modèles se trompent **sur les mêmes quatre cartes** sous la première consigne, et toujours
dans le même sens : ils acceptent des résumés que l'auditeur refuse, jamais l'inverse. Une erreur
commune à tous les juges est invisible au tri par désaccord du §4 bis : c'est ce qui justifie
l'échantillon tiré au sort parmi les cartes d'accord.

Deux pistes mesurées et fermées :

- **Scinder l'évaluation en deux appels et agréger par un `ET` en Rust ne gagne rien.** Le ET
  mécanique des conditions émises par le modèle donne *exactement* le même résultat que son
  verdict — 33/39, kappa 0,68, 5 faux « pertinent », 1 faux « non pertinent », à la carte
  près. Le modèle agrège correctement ; l'écart est dans son jugement.
- **Durcir la consigne n'est pas démontrable à cette taille.** Deux passages de la même
  consigne à température 0 donnent 38/39 verdicts identiques, soit une carte de bruit —
  l'ordre de grandeur du gain observé entre deux rédactions. Nommer explicitement le piège,
  en majuscules, ne l'a pas supprimé.

## 4 bis. Un modèle juge ne devient jamais la référence

La tentation est d'étiqueter avec un modèle fort pour cesser d'étiqueter à la main. Elle est
écartée, et pour une raison mesurée : `Opus` est d'accord avec la clé des lots sur **56 cartes sur
56** (40/40 sur le lot R, 8/8 sur le lot T, 8/8 sur le lot T2). Or cette clé est écrite par la même
personne qui écrit les cartes. Prendre le modèle pour référence revient à prendre l'auteur du lot
pour référence : on mesurerait la compatibilité d'un modèle avec nos consignes, pas sa conformité au
RGAA. Les écarts trouvés jusqu'ici — l'exigence de complétude de l'ADR 0001, le critère de résumé de
l'ADR 0002 — sont précisément ceux que l'auteur et le modèle ne voyaient ni l'un ni l'autre.

Un modèle peut en revanche **réduire le travail humain**, selon quatre étapes :

1. **Un jeu humain gelé par famille.** Il reste la seule référence. Composition et contrôles : §3.
2. **Le juge est validé contre ce jeu**, au seuil du §4, pouvoir discriminant compris. Son taux
   d'erreur est publié. S'il échoue, il n'est pas promu — et aucun juge n'est promu aujourd'hui :
   le seul « admis » d'`Opus` porte sur 8 cartes faciles, IC95 [68 % ; 100 %].
3. **Le juge préétiquette les cartes nouvelles.** L'humain arbitre les cartes où les juges divergent,
   plus un échantillon tiré au sort de celles où ils s'accordent — sans quoi une erreur commune à
   tous les juges ne serait jamais vue.
4. **Les modèles évalués sont mesurés contre le jeu humain**, jamais contre le juge. Là où le juge a
   servi de filtre, le chiffre publié porte l'erreur du juge.

L'erreur se propage : un modèle en accord parfait avec un juge à 88 % est à 88 % de l'humain, et
l'écart réel entre deux modèles se noie dans le bruit du juge. C'est la raison de l'étape 4.

## 5. Fixtures d'extraction par élément porteur

Déterministes, en CI, **sans étiquetage humain**. Le critère 1.3 a sept porteurs, dont
l'extraction diffère :

| Test | Porteur | Où vit l'alternative |
|---|---|---|
| 1.3.1 | `<img>` | `alt` |
| 1.3.2 | `<area>` dans une `<map>` | `alt` de l'`<area>` |
| 1.3.3 | `<input type="image">` | `alt` |
| 1.3.4 | `<object type="image/*">` | contenu interne |
| 1.3.5 | `<embed type="image/*">` | `aria-label` |
| 1.3.6 | `<svg>` | `<title>` / `<desc>` |
| 1.3.7–1.3.8 | `<canvas>` | `aria-label` et contenu de replacement |

`rgaa-rs/crates/rgaa-obscura/tests/alt_carriers.rs` charge une fixture portant les sept, plus
un porteur **sans** alternative pour que l'absence se distingue d'un échec d'extraction. Deux
gardes y sont indispensables :

- `OBSCURA_ALLOW_PRIVATE_NETWORK=1` — sans quoi le garde SSRF d'Obscura rend un document vide
  de 39 octets pendant que `Page.loadEventFired` se déclenche et que `readyState` vaut
  `complete`. Le test passerait pour de mauvaises raisons ;
- une assertion sur le `<title>` de la fixture avant toute autre, pour prouver que la page a
  bien été chargée.

## 6. Budget

| | Par famille | Les cinq familles |
|---|---|---|
| cartes | ~40 | ~200 |
| étiquetage, un étiqueteur | ~1 h | ~5 h |
| **deux étiqueteurs** | ~2 h | **~10 h** |

Rythme observé : 39 cartes en environ une heure, justification obligatoire comprise.

Pour mémoire, l'option écartée d'une calibration **par test** aurait demandé ~810 cartes et
plus de 20 heures par étiqueteur, pour re-mesurer sept fois le même jugement : ce qui change
avec le porteur est l'extraction, traitée au §5.

## 7. Outillage

`tools/calibration/` — consolidé depuis le harnais jetable de #339.

| Fichier | Rôle |
|---|---|
| `lots/images.py` | neuf images de données **synthétiques**, générées par code : rien à commiter, aucune licence. Libellés uniques (un défaut du prototype fusionnait les libellés répétés en une catégorie) |
| `lots/description_detaillee.py`, `lots/resume_libelle.py` | un générateur par famille : cartes, ancres, clé de correction tenue hors de la feuille |
| `sheet.py` | feuille d'étiquetage HTML autonome : justification obligatoire, export bloqué tant qu'une carte est incomplète |
| `prompts/neutre.txt` | consigne modèle qui ne donne que le texte officiel du test, sans règle de pertinence |
| `recheck.py` | re-vérification à l'aveugle d'un sous-ensemble, et calcul de la cohérence test-retest de l'étiqueteur |
| `run_models.py` | exécute un modèle sur un lot. Backends `ollama` (local) et `openai`. **Le backend externe refuse de partir sans `--allow-external`** et ne reçoit la clé que par nom de variable d'environnement, jamais en ligne de commande |
| `score.py` | accord, kappa de Cohen, intervalles de Wilson, matrice de confusion, repère trivial, ancres, et **phi(justification, verdict)** — le contrôle de collinéarité du §2 |

## 8. État

| Famille de preuve | Tests | État | Lot |
|---|---|---|---|
| alternative d'image | 1.3.1–1.3.9 | **calibrée** — 33/39 = 85 %, kappa 0,68, ancres 4/4, repère trivial 67 % | (campagne #339) |
| description détaillée | 1.7.1–1.7.6 | **étiqueté, mesure non discriminante** — ancres 4/4, test-retest 13/14 = 93 % ; `swift` 39/39 = 100 %, `gemma4:e2b` 36/39 = 92 % (2 faux « pertinent » sur 21 = 10 %, admis). Un modèle de 2 Md passe : famille **non déclarée calibrée** tant qu'un lot plus difficile n'existe pas | `lots/description_detaillee.py` — 39 cartes, repère trivial 53,8 %, 4 ancres |
| résumé et libellé | 5.5.1, 8.6.1, 11.7.1, 11.8.3 | **étiqueté, non admis** — ancres 6/6, repère trivial 59 %, mais test-retest 12/21 = 57 % (seuil : 80 %). La tranche 5.2.1 en est retirée (ADR 0002) | `lots/resume_libelle.py` — **32 cartes** (4 tests), repère trivial 50 %, 5 ancres dont 3 factuellement fausses. `--avec-5-2-1-retire` reproduit le lot R d'origine à 40 cartes et 6 ancres, pour relire ces mesures |
| résumé de tableau | 5.2.1 | **en cours** — définition du glossaire rappelée sur chaque carte (ADR 0002). 8 premières cartes : étiqueteur 8/8 conforme, ancres 3/3, `Opus` 8/8, mais lot trop facile. Porté à 36 cartes avec des cas limites, **étiquetage des 28 restantes à faire**. Les mesures de modèles du §4 portent sur un lot antérieur déséquilibré et sont à refaire sur celui-ci | `lots/resume_tableau.py` — 36 cartes, repère trivial 50 %, 6 ancres, 9 tableaux complexes |
| transcription et sous-titres | 4.2.1, 4.4.1 | **bloquée** | — |
| audiodescription | 4.6.1, 4.6.2, 4.9.1 | **sans objet** | — |

**Pourquoi « transcription et sous-titres » est bloquée.** Le modèle qui juge ces tests ne lit pas
l'audio : il compare un texte publié à une transcription produite par Whisper. Calibrer cette chaîne
de bout en bout mêlerait deux erreurs — celle du transcripteur et celle du juge — sans moyen de les
séparer. Or **Whisper est un prérequis non résolu** (« à valider », `human-criteria-automation.md`
§5.3) : il n'est pas installé ici, et rien n'a été mesuré sur du français. L'étiqueteur devrait de plus
écouter chaque média, ce qui exige des clips de parole connue ; aucun synthétiseur vocal n'est
disponible non plus. À reprendre une fois Whisper validé.

**Pourquoi « audiodescription » est sans objet.** Ses trois tests sont *Irréductibles*
(`human-criteria-automation.md` §3, G3) : aucun modèle n'y rend de verdict, donc il n'y a rien à
calibrer. Seule la preuve d'inventaire (piste présente, marqueur lu) est produite, et elle est
déterministe — couverte par les fixtures du §5, pas par un étiquetage.

Il reste donc **deux campagnes réalisables**, pas quatre : environ 80 cartes, environ 2 h
d'étiquetage par étiqueteur au rythme observé (39 cartes en une heure, justification comprise).

**Consigne des deux campagnes.** Aucune règle de pertinence n'est donnée aux étiqueteurs : seule la
formulation officielle de chaque test figure dans la feuille. Le critère sera extrait des
justifications, comme il l'a été pour la famille « alternative d'image », puis une seconde consigne
modèle l'énoncera. `tools/calibration/prompts/neutre.txt` sert à la première mesure.

**Réserve ouverte** : le critère de pertinence de
[`docs/adr/0001-alternatives-serie-complete.md`](../adr/0001-alternatives-serie-complete.md)
est celui d'un seul auditeur. Sur `D031`, « Logo de l'Atelier Lumière » est jugée non
pertinente, là où le modèle donne la réponse qu'un auditeur donnerait. Le second étiqueteur
ne bloque pas le niveau *Assisté* — il tient quel que soit l'arbitrage — mais il bloque
l'application du critère au code.
