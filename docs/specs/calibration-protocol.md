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

## 4. Seuil d'admission d'un modèle

| Condition | Seuil |
|---|---|
| ancres | **4/4** |
| accord | au-dessus du repère trivial, **intervalles de confiance disjoints** |
| faux « pertinent » | **≤ 10 %** des alternatives rejetées par l'humain |

Aucun modèle n'est épinglé : un seuil survit au changement de modèle et de passerelle.

État mesuré :

| Modèle | Ancres | Accord | Faux « pertinent » | Verdict |
|---|---|---|---|---|
| `swift-1.5-27b` (API) | 4/4 | 33/39 = 85 %, kappa 0,68 | **19 %** | admis en *Assisté*, échoue le seuil |
| `gemma4:e2b` (local) | — | 22/33 = 67 %, **sous son repère trivial de 70 %** | — | disqualifié |

Deux pistes mesurées et fermées :

- **Scinder l'évaluation en deux appels et agréger par un `ET` en Rust ne gagne rien.** Le ET
  mécanique des conditions émises par le modèle donne *exactement* le même résultat que son
  verdict — 33/39, kappa 0,68, 5 faux « pertinent », 1 faux « non pertinent », à la carte
  près. Le modèle agrège correctement ; l'écart est dans son jugement.
- **Durcir la consigne n'est pas démontrable à cette taille.** Deux passages de la même
  consigne à température 0 donnent 38/39 verdicts identiques, soit une carte de bruit —
  l'ordre de grandeur du gain observé entre deux rédactions. Nommer explicitement le piège,
  en majuscules, ne l'a pas supprimé.

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
| `build_lot.py` | génère un lot : cartes, ancres, clé de correction tenue hors de la feuille |
| `sheet.py` | feuille d'étiquetage HTML autonome : justification obligatoire, export bloqué tant qu'une carte est incomplète |
| `run_models.py` | exécute un modèle sur un lot. Backends `ollama` (local) et `openai`. **Le backend externe refuse de partir sans `--allow-external`** et ne reçoit la clé que par nom de variable d'environnement, jamais en ligne de commande |
| `score.py` | accord, kappa de Cohen, intervalles de Wilson, matrice de confusion, repère trivial, ancres, et **phi(justification, verdict)** — le contrôle de collinéarité du §2 |

## 8. État

| Famille de preuve | Tests | État |
|---|---|---|
| alternative d'image | 1.3.1–1.3.9 | **calibrée** — 33/39 = 85 %, kappa 0,68, ancres 4/4, repère trivial 67 % |
| description détaillée | 1.7.1–1.7.6 | à calibrer |
| transcription et sous-titres | 4.2.1, 4.4.1 | à calibrer |
| audiodescription | 4.6.1, 4.6.2, 4.9.1 | sans objet — ces tests sont *Irréductibles* ; la calibration ne porterait que sur la preuve d'inventaire |
| résumé et libellé | 5.2.1, 5.5.1, 8.6.1, 11.7.1, 11.8.3 | à calibrer |

**Réserve ouverte** : le critère de pertinence de
[`docs/adr/0001-alternatives-serie-complete.md`](../adr/0001-alternatives-serie-complete.md)
est celui d'un seul auditeur. Sur `D031`, « Logo de l'Atelier Lumière » est jugée non
pertinente, là où le modèle donne la réponse qu'un auditeur donnerait. Le second étiqueteur
ne bloque pas le niveau *Assisté* — il tient quel que soit l'arbitrage — mais il bloque
l'application du critère au code.
