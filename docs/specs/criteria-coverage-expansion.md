# Spec — extension de la couverture réelle des critères RGAA testés

> **Statut** : spec de décision, prête pour l'implémentation. Aucune décision n'est laissée ouverte, sauf les
> vérifications de capacités listées au §10 (elles se tranchent par une fixture, pas par une discussion).
> **Carte** : [Wayfinder Map] Étendre la couverture réelle des critères RGAA testés (contrôles déterministes) — jamon8888/Holo-RGAA#247.
> **Sources** : quatre recherches de faisabilité (`research/coverage-themes-1-4`, `-5-9`, `-10-11`, `-12-13`),
> prototype clavier (`prototype/keyboard-trap-probe`), décisions des tickets de la carte (§12).
> **Date** : 2026-10-04. État du dépôt de référence : `master` à `e7e866f`.

## 1. Objet et périmètre

« Étendre le nombre de critères testés » ne veut pas dire ajouter des lignes au catalogue : les 106 critères y sont déjà.
Cela veut dire **ajouter de vrais contrôles là où rien ne s'exécute**.

État mesuré à HEAD : **57 critères n'ont ni règle axe ni sonde gap-fix**, et **14** ont une couverture axe `partial`.
Plusieurs entrées `partial` de `axe_mapping.json` portent `axe_rules: []` : elles ressemblent à un contrôle mais n'en sont pas
(les « règles » citées sont des noms WCAG, pas des règles axe-core).

Cette spec fixe, pour chaque critère concerné : le mécanisme cible, le moteur, les issues possibles, la déclaration de couverture,
les fixtures exigées et le lot de livraison.

**Hors périmètre** (autres cartes ou efforts) :
- précédence des verdicts, budget LLM, batching, vérification par LLM : carte #180 ;
- gate de couverture, corpus de vérité terrain, banc d'essai de modèles, transport : carte #183 ;
- envoi de captures au juge LLM (multimodal) : brouillard de #180 ;
- RGAA 5, remédiation, rendu de rapport, TUI/CLI ;
- correction des couvertures déclarées à tort (3.3, 5.6, 12.1, 12.4) : issue dédiée *Corriger la couverture déclarée de 3.3, 5.6, 12.1 et 12.4* (#256) ;
  cette spec en dépend pour le lot 0 de chaque thème mais ne la remplace pas.

## 2. Principes de conception

1. **Pas de `Pass` sans preuve.** Un mécanisme qui ne décide pas tous les tests d'un critère ne peut jamais produire `pass`. Son silence laisse le critère au repli du pipeline.
2. **Trois issues par mécanisme** : `fail` (avec preuve), `pass` (réservé aux mécanismes déclarés `complete`), `review` (indice ciblé pour l'agent ou l'humain, sans verdict). C'est une extension du contrat gap-fix actuel `{pass, details, nodes}`.
3. **Deux niveaux de « couvert »**, comptés séparément :
   - **couvert décisif** : le mécanisme peut émettre `fail`, ou `pass` + `fail` s'il est `complete` ;
   - **orienté** : le mécanisme ne peut émettre que `review`. Il aide l'agent mais ne compte pas dans l'objectif de couverture.
4. **Humain par conception** : une liste explicite de critères (§9) n'est jamais automatisée. Leur statut `needs_review` est assumé et documenté ; ils sont exclus de l'objectif.
5. **Coût borné** : le déterministe d'abord, une seule évaluation JS par page par sonde, aucune boucle d'interaction coûteuse hors besoin prouvé.

## 3. Moteurs

| Moteur | Usage | Limites mesurées |
|---|---|---|
| **axe-core natif** (mapping de règles existantes) | première ligne : corrections de données, règles non mappées | version injectée **4.9.1** vs catalogue 4.13 : toute règle citée se vérifie dans 4.9.1 par une fixture (§10) |
| **Sonde JS** (Obscura par défaut) | DOM/CSSOM, évaluation unique par page | contrat à trois issues (§2) |
| **Sonde comportementale** | interaction (clavier, redimensionnement, survol) | sous Obscura : `keydown` livré mais **pas de navigation Tab native** ; redimensionnement à 320 px **fonctionne** ; `getComputedStyle().direction` **vide** |
| **Chrome** (facultatif) | seulement si une fixture échoue sous Obscura et réussit sous Chrome | binaire de plus ; voir §3.2 |
| **Analyse de médias** | 13.7 seulement | GIF, APNG, vidéos décodés hors rendu |
| **Niveau site** | critères d'« ensemble de pages » | §6 |

Exclus : **règles axe personnalisées** (aucun gain sur une sonde JS, aucune infrastructure) ; **pixels hors 13.7** ;
**`rgaa-linter`** reste une surface séparée (§3.3).

### 3.1 Règle de choix du moteur

Obscura par défaut. Une sonde ne passe sous Chrome que si elle exige la navigation Tab native ou un rendu réel, et elle le **prouve** :
une fixture qui échoue sous Obscura et réussit sous Chrome. Sans cette preuve, Chrome n'est pas admis « par précaution ».

### 3.2 Chrome facultatif

- Absent : les sondes qui l'exigent sortent `review` avec la raison `moteur absent`.
- Le rapport indique **en tête** quels critères n'ont pas été testés faute de Chrome (le résultat ne doit pas dépendre de la machine sans que l'utilisateur le voie).

### 3.3 `rgaa-linter`

Surface séparée (source statique, sans navigateur, une passe). On y ajoute une règle seulement quand un contrôle source reproduit
fidèlement un contrôle que le DOM couvre déjà. Ses règles **ne comptent jamais** dans la couverture d'un audit et ne sont jamais
citées comme verdict de conformité.

## 4. Plancher de validation d'un nouveau contrôle (ticket *plancher de validation*)

1. **Admission** : un contrôle compte comme « testé » s'il a une fixture conforme **et** une fixture non conforme, et si le mécanisme les classe correctement.
2. **Emplacement** : `rgaa-test-corpus/criteria/`, convention de nom existante `{critère}-{slug}-pass|fail`.
3. **Granularité** : un verdict par critère par défaut. Un mécanisme déclaré `complete` exige **une fixture par test RGAA** (la convention de nom portera le test dans le `slug`, ex. `11.6-t1-fieldset-legend-fail`).
4. **Sans fixture** : le mécanisme n'est pas branché sur le chemin d'audit ; le critère retombe dans le repli du pipeline. Un test de CI le vérifie à partir du registre (§5).
5. **Portée** : seulement les **nouveaux** mécanismes. Les 77 entrées axe et les 15 sondes gap-fix existantes migrent dans le registre comme « héritées », exemptées de fixtures.
6. **Exécution en CI** : le contrôle de présence des paires de fixtures est un contrôle de fichiers, sans navigateur, dans le job de tests habituel. Le test de **classification** tourne sous Obscura dans le job E2E existant.
7. **Échec** : un mécanisme qui classe mal sa fixture **bloque la PR**.

## 5. Registre unique des mécanismes

Un registre déclaratif remplace `axe_mapping.json` (couverture axe) et `COMPLETE_COVERAGE` de `gap_fix.rs`. Une entrée par mécanisme :

```toml
[[mechanism]]
id        = "probe-11-6-fieldset-legend"
criterion = "11.6"
kind      = "axe-native | js-static | js-behavioural | media-analysis | site"
engine    = "obscura"            # "chrome" seulement avec preuve (§3.1)
coverage  = "partial"            # "complete" : tous les tests du critère sont décidés
tests     = ["11.6.1"]           # tests RGAA couverts
outcomes  = ["fail", "review"]   # "pass" interdit si coverage = "partial"
fixtures  = ["11.6-t1-fieldset-legend-pass", "11.6-t1-fieldset-legend-fail"]
legacy    = false                # true : hérité, exempté de fixtures
```

Invariants vérifiés en CI : `pass ∈ outcomes ⇒ coverage = "complete"` ; `legacy = false ⇒ fixtures` présentes (une paire, ou une par test si `complete`) ;
les règles axe citées existent dans la version injectée ; aucun mécanisme n'est actif sans entrée de registre.

**Migration** : les 77 entrées axe et les 15 sondes existantes passent dans le registre avec `legacy = true`. Les entrées `partial` à `axe_rules: []`
sont **supprimées** (elles n'ont aucun mécanisme) ; leur critère retombe dans le repli du pipeline. La migration est le premier lot de chaque thème (lot 0, §8).

## 6. Niveau site (ticket *verdict de site pour les critères multi-pages*)

Critères concernés : tout critère dont l'énoncé RGAA dit « ensemble de pages » — aujourd'hui **12.1, 12.2, 12.4, 12.5**, plus **12.3** pour la page « plan du site ».
La liste se dresse par lecture de l'énoncé, pas à la main.

1. **Emplacement** : une liste de résultats de site sur `AuditResult`, calculée après toutes les pages. Les critères de site **disparaissent des résultats de page** et sont comptés **une fois** au niveau site. Le taux de conformité par page se calcule hors critères de site.
2. **Source** : le comparateur relit le DOM rendu de chaque page, conservé en mémoire le temps de la comparaison (au plus 7 pages), puis libéré. Pas d'empreinte.
3. **Verdict** : `fail` avec preuve, sinon `review`. Jamais `pass` (la place visuelle, la pertinence et la session échappent au déterministe).
4. **Échantillon** : pas de minimum global. Les critères de comparaison (12.2, 12.5…) sortent `review` « moins de deux pages comparables » quand il n'y a rien à comparer ; les contrôles sans comparaison (liens du plan du site, 12.3) s'évaluent à part.
5. **Ensemble comparé** : toutes les pages échantillonnées sauf l'accueil (profondeur 0) et les pages portant un `input[type=password]`. Chaque exclusion est nommée dans la preuve avec sa raison.
6. **12.3** : requêtes `HEAD` seulement, même domaine, `robots.txt` respecté, plafonnées (par exemple 50 liens), comptées à part du budget LLM. Le test 1 (pertinence du plan) reste humain.
7. **Agrégation** : `aggregate_site_compliance` consomme ces résultats de site au lieu des statuts par page pour ces critères ; un `fail` de site donne NC, un `review` donne « non testé ».

12.1 et 12.4 sont aujourd'hui `complete` via des règles de page seule (`landmark-one-main`, `region`) qui ne prouvent pas la propriété d'un ensemble de pages ;
ils passent en `partial` (issue dédiée, #256), puis relèvent du niveau site.

## 7. Spécification par critère

Légende. **Mécanisme** : `axe-N` règle axe native non mappée ou ré-attribuée ; `data` correction de `axe_mapping.json` sans code ; `js-S` sonde JS statique ;
`js-C` sonde comportementale ; `site` niveau site ; `media` analyse de médias ; `humain`. **Issues** : `F` = `fail`, `R` = `review`, `P` = `pass`.
**Couv.** : `partial` = fail seul, `orienté` = review seul, `complete` = candidat `pass` (exige une fixture par test).
**Moteur** : Obscura sauf mention. « (v) » = capacité à vérifier par fixture avant de s'engager (§10).

### 7.1 Thème 12 — Navigation  (lot B1)

| Critère | Mécanisme | Moteur | Issues | Couv. | Lot | Remarque |
|---|---|---|---|---|---|---|
| 12.1 | `data` (→ `partial`) puis `site` | — | F,R | partial | B1-L0, B1-L2 | `complete` à tort aujourd'hui (#256) |
| 12.2 | `site` : ordre relatif de la navigation dans le source (la « place » visuelle reste humaine) | Obscura | F,R | partial | B1-L2 | accueil et pages de connexion exclus |
| 12.3 | `site` : liens du plan du site en `HEAD` ; test 3 intitulé/titre de cible en `R` ; test 1 humain | Obscura | F,R | partial | B1-L2 | détection de la page plan fragile → `R` si introuvable |
| 12.4 | `data` (→ `partial`) puis `site` | — | F,R | partial | B1-L0, B1-L2 | « atteignable de manière identique » |
| 12.5 | `site` : moteur de recherche (`[role=search]`, `input[type=search]`) présence, place et ordre dans le source | Obscura | F,R | partial | B1-L2 | `not_applicable` si aucun moteur sur tout l'échantillon |
| 12.8 | `data` : `focus-order-semantics` (tag RGAA-12.8.1) rattachée à 12.8, `tabindex` | Obscura | F,R | partial | B1-L0 | verdict max « suspect » (`R`) ; parcours Tab réel reste hors Obscura |
| 12.9 | `js-C` : `keydown` Tab annulable sur chaque focusable, puis Échap (prototype) | Obscura | F,R | partial | B1-L1 | détecte les `Fail` seulement ; silence = `needs_review` ; pièges par refocus asynchrone non détectés |
| 12.10 | `js-S` : listeners clavier globaux (hook `addEventListener`) (v) | Obscura | R | orienté | B1-L3 | conformité non décidable |
| 12.11 | `js-C` : déclencheurs survol/focus (v : `Input.dispatchMouseEvent`) | Obscura | R | orienté | B1-L3 | « si nécessaire » interdit tout `F` direct |

### 7.2 Thème 13 — Consultation  (lot B1)

| Critère | Mécanisme | Moteur | Issues | Couv. | Lot | Remarque |
|---|---|---|---|---|---|---|
| 13.1 | `data` : `meta-refresh-no-exceptions` ; tests 1 et 3 en `js-S` (hooks `location.*`, `setTimeout`) (v) ; test 4 humain | Obscura | F,R | partial | B1-L0, B1-L1 | timers légitimes (carrousels) ≠ redirection |
| 13.2 | `js-C` : hook `window.open`, fenêtres ouvertes avant toute interaction (v) | Obscura | F,R | partial | B1-L1 | `target="_blank"` seul n'est pas un déclenchement |
| 13.6 | `js-S` : détection de contenus cryptiques (ASCII art, émoticônes) | Obscura | R | orienté | B1-L3 | pertinence humaine ; faux positifs élevés |
| 13.7 | `media` : décodage GIF/APNG/vidéo, luminance, comptage de flashs/s et surface ; CSS/JS animés → `R` avec alerte « animation détectée » | — | F,R | partial | B1-L3 | `partial` définitif : jamais `pass` |
| 13.8 | `js-S`/`js-C` : animations infinies ou > 5 s (`document.getAnimations()`) (v), carrousels, bouton de contrôle (heuristique) ; existant `blink`, `marquee` | Obscura | F,R | partial | B1-L1 | loaders/spinners = faux positifs : `R` |
| 13.9 | `js-C` : rendu en deux orientations (`Emulation.setDeviceMetricsOverride`, OK sous Obscura), diff du contenu textuel, `screen.orientation.lock` ; existant `css-orientation-lock` | Obscura | F,R | partial | B1-L1 | le contenu doit rester, pas la présentation |
| 13.10 | `js-S` : inventaire des widgets à geste | Obscura | R | orienté | B1-L3 | équivalence humaine |
| 13.11 | `js-S` : handlers `mousedown`/`pointerdown` portant l'action | Obscura | R | orienté | B1-L3 | délégation d'événements : faux négatifs |
| 13.12 | `js-S` : écoute des capteurs (`devicemotion`, `deviceorientation`, `Accelerometer`) | Obscura | R | orienté | B1-L3 | pas d'usage détecté → `not_applicable` plausible |

### 7.3 Thème 4 — Multimédia  (lot B2)

Mutualisation : **un inventaire média** (`video`, `audio`, `object`, `embed`, `canvas`, `iframe` vers lecteurs connus ; `controls`, `autoplay`, `muted`, `<track kind>`, texte adjacent) alimente 4.1, 4.4, 4.5, 4.7, 4.8. Contenu inter-origine opaque : seules l'URL et les paramètres sont observables.

| Critère | Mécanisme | Moteur | Issues | Couv. | Lot | Remarque |
|---|---|---|---|---|---|---|
| 4.1 | `js-S` inventaire + alternative adjacente (« transcription », `<track kind="descriptions">`) | Obscura | R | orienté | B2-L1 | jamais `F` direct (« si nécessaire ») |
| 4.2 | `humain` | — | R | — | — | §9 |
| 4.4 | `js-S` : piste existante, VTT joignable, non vide ; décision humaine | Obscura | F,R | partial | B2-L1 | `F` seulement sur VTT 404 ou vide |
| 4.5 | `js-S` inventaire (pistes de description, lien « version audiodescrite ») | Obscura | R | orienté | B2-L1 | |
| 4.6 | `humain` | — | R | — | — | §9 |
| 4.7 | `js-S` inventaire : média sans titre/`aria-label`/légende | Obscura | R | orienté | B2-L1 | |
| 4.8 | `js-S` : médias non temporels + lien/bouton adjacent | Obscura | R | orienté | B2-L1 | |
| 4.9 | `humain` | — | R | — | — | §9 |
| 4.10 | `js-C` : crochet sur `play`/`AudioContext` avant chargement, attente ~4 s, média non muet sans contrôle ; iframes : `autoplay=1`, `mute=0` dans l'URL ; existant `no-autoplay-audio` | Obscura (v) ; Chrome seulement si la politique d'autoplay l'impose | F,R | partial | B2-L2 | sans neutraliser la politique d'autoplay, faux négatifs massifs |
| 4.11 | `js-S` : contrôles personnalisés non focalisables ; `js-C` seulement si une fixture le justifie | Obscura ; Chrome si preuve | F,R | partial | B2-L2 | lecteurs qui ne donnent le focus qu'après interaction : `R` |
| 4.12 | `js-C` : même balayage que 12.9 sur les médias non temporels | Obscura | F,R | partial | B2-L2 | priorité basse (média rare) |
| 4.13 | `axe-N` : violations axe déjà calculées (`button-name`, `aria-*`) ré-attribuées au conteneur média (changement du mapper) | Obscura | F | partial | B2-L1 | |

### 7.4 Thèmes 5, 7, 8, 9 — Tableaux, scripts, éléments obligatoires, structure  (lot B3)

| Critère | Mécanisme | Moteur | Issues | Couv. | Lot | Remarque |
|---|---|---|---|---|---|---|
| 5.1 | `js-S` : tableau « complexe » (en-têtes multiples) sans résumé | Obscura | R | orienté | B3-L1 | « complexe » est flou : jamais `F` ferme |
| 5.2 | `data` : `table-duplicate-name` (tag RGAA-5.2.1) mappée sur 5.2 | Obscura | F | partial | B3-L0 | pertinence humaine |
| 5.3 | `js-S` : tableau de mise en forme probable sans `role=presentation\|none` | Obscura | R | orienté | B3-L1 | linéarisation humaine |
| 5.4 | `js-S` : `idref` cassé, `caption` vide ; existant `table-fake-caption`, `table-duplicate-name` | Obscura | F | partial | B3-L1 | |
| 5.5 | `js-S` : titre vide, générique ou dupliqué | Obscura | F,R | partial | B3-L1 | pertinence humaine |
| 5.8 | `js-S` : tableau `role=presentation\|none` contenant `th`, `caption`, `thead`, `[scope]`, `[headers]`… | Obscura | F | partial | B3-L1 | |
| 7.1 | `data` : `label-content-name-mismatch` et règles ARIA non mappées (`aria-allowed-role`, `aria-dialog-name`, `aria-treeitem-name`, `aria-tooltip-name`, `presentation-role-conflict`) | Obscura | F | partial | B3-L0 | certaines règles sont « best-practice » : vérifier leur tag |
| 7.2 | `js-C` : chargement sans JS (v : `Emulation.setScriptExecutionDisabled`) puis diff | Obscura | R | orienté | B3-L3 | faux positifs élevés sur SPA |
| 7.3 | `js-C` : hook `addEventListener` (v : `Page.addScriptToEvaluateOnNewDocument`) croisé avec focusabilité et rôle ; statique `[onfocus*="blur"]` | Obscura | F,R | partial | B3-L3 | meilleur rendement du thème 7 ; sinon abandonner |
| 7.4 | `js-C` : changement de contexte sur `change`/`input` | Obscura | R | orienté | B3-L3 | faible rendement |
| 7.5 | `humain` (statut `Manuel` conservé) | — | R | — | — | §9 |
| 8.1 | `js-S` : `document.doctype`, `compatMode` (fonctionne sous Obscura) | Obscura | F | partial | B3-L1 | candidat `complete` seulement avec le source brut (test 3) |
| 8.2 | `data` : `duplicate-id`, `duplicate-id-active` après vérification dans axe 4.9.1 ; existant `html-has-lang`, `duplicate-id-aria` | Obscura | F | partial | B3-L0 | validateur HTML complet exclu (JVM) |
| 8.4 | `data` : `html-lang-valid` → 8.4 ; pertinence par détection de langue | Obscura | F,R | partial | B3-L0, B3-L2 | |
| 8.6 | `js-S` : titre générique ou URL ; titre identique sur ≥ N pages (niveau site) | Obscura | R | orienté | B3-L1 | pertinence humaine |
| 8.7 | `js-S` + détecteur de langue (Rust) sur blocs de texte significatifs | Obscura | R | orienté | B3-L2 | anglicismes, marques : jamais `F` |
| 8.8 | existant `valid-lang` + même détecteur que 8.7 | Obscura | F,R | partial | B3-L2 | |
| 8.9 | `js-S` : éléments obsolètes (`font`, `center`, `big`…), `br` consécutifs, `p` vides | Obscura | F | partial | B3-L1 | |
| 8.10 | `js-S` : caractères RTL dans un élément sans `dir` (direction déduite des attributs `dir` hérités, `getComputedStyle().direction` étant vide sous Obscura) ; valeur de `dir` invalide | Obscura | F | partial | B3-L1 | |
| 9.2 | `data` : `region`, `landmark-one-main` et les règles `landmark-*` mappées sur 9.2 ; `js-S` : `banner` et `contentinfo` de premier niveau | Obscura | F | partial | B3-L0, B3-L1 | « hors cas particuliers » : pages mono-fonction |
| 9.4 | `humain` (indice facultatif en `R`) | — | R | — | — | §9 |

### 7.5 Thèmes 10 et 11 — Présentation, formulaires  (lot B4)

| Critère | Mécanisme | Moteur | Issues | Couv. | Lot | Remarque |
|---|---|---|---|---|---|---|
| 10.1 | existant : garder `partial`, distinguer balises (sûres) et attributs (probables, `align`/`valign`/`background` tolérés) ; ajouter 10.1.3 en `R` | Obscura | F,R | partial | B4-L1 | pas de `F` sur 10.1.3 |
| 10.3 | `humain` | — | R | — | — | §9 |
| 10.4 | `js-C` : texte ×2, contenu tronqué ou chevauchant ; existant `meta-viewport` | Obscura | F,R | partial | B4-L2 | mutualise la mesure de troncature avec 10.11 et 10.12 |
| 10.7 | `js-C` : ACT `oj04fd`, diff du style calculé au focus par élément de l'ordre de tabulation ; pseudo-élément/image de fond → `R` | Obscura ; Chrome si `:focus-visible` exige une vraie touche (preuve) | F,R | partial | B4-L2 | jamais `F` sur `cantTell` |
| 10.10 | `humain` | — | R | — | — | §9 |
| 10.11 | `js-C` : `Emulation.setDeviceMetricsOverride` 320 px, `scrollWidth > clientWidth`, énumération des débordements | Obscura (viewport vérifié) | F,R | partial | B4-L2 | cas particuliers (tableaux, cartes, code) → `R` ; la sonde retirée comparait sans redimensionner |
| 10.12 | `axe-N` : `avoid-inline-spacing` (niveau 1) ; niveau 2 `js-C` : feuille WCAG 1.4.12 et détection de troncature | Obscura | F,R | partial | B4-L0, B4-L2 | |
| 10.13 | `js-C` : survol/focus, Échap, survol du contenu, persistance (`Input.dispatchMouseEvent` (v)) | Obscura | F,R | partial | B4-L2 | dépend du repérage des candidats |
| 11.2 | existant `label-content-name-mismatch` ; `js-S` : étiquette vide ou ≡ placeholder, générique, dupliquée | Obscura | F,R | partial | B4-L1 | ne conclut jamais `P` |
| 11.3 | `js-S` par page : champs groupés par `autocomplete`, étiquettes divergentes | Obscura | R | orienté | B4-L1 | |
| 11.5 | existant ; extension : ≥ 2 champs `autocomplete` partageant un préfixe hors `fieldset` | Obscura | F | partial | B4-L1 | |
| 11.6 | `js-S` : tout `fieldset` a un `legend` enfant direct non vide ; tout `[role=group\|radiogroup]` a un nom non vide | Obscura | F,P,R | **complete (candidat)** | B4-L1 | un seul test (11.6.1) ; `complete` si la fixture par test passe |
| 11.7 | `humain` ; pré-filtre `R` : légende vide ou générique | Obscura | R | — | B4-L1 | §9 |
| 11.8 | `js-S` : `optgroup` sans `label` non vide (11.8.2, `F` sûr) ; `select` de > N options sans `optgroup` (11.8.1, `R`) | Obscura | F,R | partial | B4-L1 | 11.8.3 humain |
| 11.9 | `data` : `label-content-name-mismatch` rattachée aussi à 11.9 ; `js-S` : noms génériques ou techniques (`btn_3`) en `R` | Obscura | F,R | partial | B4-L0, B4-L1 | pertinence du nom humaine |
| 11.10 | `js-S` : champ `required` sans indicateur visible, format décrit seulement par `placeholder` (`R`) ; 11.10.3-4 `js-C` : `checkValidity()`/`reportValidity()` **jamais `submit()`** | Obscura | R | orienté | B4-L1, B4-L3 | formulaires de commande/suppression : risque d'effet de bord |
| 11.11 | `js-S` : `pattern`/`type` sans exemple (11.11.2) ; 11.11.1 `js-C` comme 11.10.3 | Obscura | R | orienté | B4-L1, B4-L3 | message natif localisé : non fiable |
| 11.12 | `js-S` : applicabilité (formulaire `post` avec `password`/`iban`/`card`, boutons « payer/supprimer ») ; résolution humaine | Obscura | R | orienté | B4-L1 | sert surtout à établir `not_applicable` |
| 11.13 | `js-S` : champ à finalité utilisateur sans `autocomplete` (`type=email\|tel\|password\|url` et `name` strict en `F`, le reste en `R`) ; existant `autocomplete-valid` | Obscura | F,R | partial | B4-L1 | jeu de motifs WCAG 1.3.5 |

### 7.6 Thèmes 1, 2, 3 — Images, cadres, couleurs  (lot B5)

| Critère | Mécanisme | Moteur | Issues | Couv. | Lot | Remarque |
|---|---|---|---|---|---|---|
| 1.3 | `js-S` : défauts manifestes (alt = nom de fichier, générique, longueur > seuil) ; `<canvas>` : enfants et rôle exposé | Obscura | F,R | partial | B5-L1 | ne valide jamais ; seuil de longueur = convention de sonde, à valider |
| 1.4 | `js-S` : découverte de CAPTCHA (`iframe[src*=recaptcha\|hcaptcha\|turnstile]`, mots-clés) | Obscura | R | orienté | B5-L1 | `not_applicable` seulement avec prudence |
| 1.7 | `js-S` : `aria-describedby` résolu et non vide ; `longdesc` joignable (`HEAD`) | Obscura | F,R | partial | B5-L1 | pertinence humaine |
| 1.8 | `humain` ; supprimer l'entrée `partial` vide | — | R | — | — | §9 (pixels exclus) |
| 2.2 | `data` : `frame-title-unique` (tag RGAA-2.2.1) mappée sur 2.2 ; `js-S` : titres génériques | Obscura | F,R | partial | B5-L0, B5-L1 | deux iframes jumeaux : prudence |
| 3.1 | `js-S` : information portée par la couleur seule (`required`/`aria-invalid` par `color`/`border-color` seul ; « le champ en rouge ») | Obscura | F,R | partial | B5-L1 | ne couvre qu'une fraction minime des 6 tests |
| 3.3 | `data` (→ `partial`, #256) ; `js-C` optionnel : contraste des bordures et états forcés des composants (`:hover`, `:focus`) | Obscura | R | partial | B5-L0, B5-L2 | pixels exclus ; faux positifs élevés → `R` |

## 8. Lots de livraison

**Ordre des thèmes** : par risque agrégé, B1 (thèmes 12–13) et B2 (thème 4) d'abord, puis B3, B4, B5.
**Risque** = exposition aux faux `Pass` (plancher d'honnêteté), puis gravité pour l'utilisateur (piège clavier, flashs, contrôle du média).
**Unité d'une PR** = un mécanisme (une sonde ou un inventaire partagé, ses critères, ses fixtures).

Chaque thème s'ouvre par son **lot 0** (données), suivi des sondes :

| Lot | Contenu | Règle d'admission |
|---|---|---|
| **L0 — données** | migration vers le registre ; corrections de mapping ; suppression des entrées vides (§5) ; `partial` des couvertures déclarées à tort (#256) | chaque correction garde sa fixture |
| **L1 — sondes statiques et ré-attributions** | `js-S`, `axe-N`, inventaires partagés | plancher §4 |
| **L2 — sondes comportementales** | redimensionnement, survol, balayage clavier, niveau site | après le prototype (fait) ; seulement la forme `keydown` annulable sous Obscura pour le clavier |
| **L3 — heuristiques `review` et rendement faible** | indices orientés, 13.7 média, 11.10.3-4 | construits seulement s'ils réutilisent un inventaire déjà bâti (ou 13.7) |

Les contrôles `review`-seuls (orientés) ne sont construits que s'ils réutilisent un inventaire déjà bâti pour un contrôle décisif ; sinon ils sortent de la carte.

| Lot | Thèmes | Livre |
|---|---|---|
| **B1** | 12, 13 | L0 : 12.8, 13.1, 12.1/12.4 `partial`, suppression des règles fantômes (12.2, 12.5, 12.9, 12.10, 12.11, 13.2, 13.7, 13.10–13.12). L1 : 12.9, 13.1 (tests 1 et 3), 13.2, 13.8, 13.9. L2 : niveau site (12.1, 12.2, 12.3, 12.4, 12.5). L3 : 12.10, 12.11, 13.6, 13.7, 13.10, 13.11, 13.12. |
| **B2** | 4 | L0 : suppression des règles fantômes (4.1, 4.5, 4.7, 4.8, 4.11–4.13). L1 : inventaire média (4.1, 4.4, 4.5, 4.7, 4.8), 4.13. L2 : 4.10, 4.11, 4.12. |
| **B3** | 5, 7, 8, 9 | L0 : 5.2, 7.1, 8.2, 8.4 (`html-lang-valid`), 9.2, 5.6 `partial`, règles fantômes. L1 : 8.1, 8.10, 5.8, 8.9, 5.5, 8.6, 5.3, 5.4, 5.1, 9.2. L2 : détecteur de langue (8.4, 8.7, 8.8). L3 : 7.3, 7.2, 7.4. |
| **B4** | 10, 11 | L0 : 10.12, 11.9, règles fantômes (10.7, 10.12, 10.13, 11.6, 11.11, 11.12). L1 : 11.6, 11.8, 11.13, 10.1.3, 11.5, 11.2, 11.9, 11.10, 11.12. L2 : 10.11, 10.4, 10.12 niveau 2, 10.7, 10.13. L3 : 11.10.3-4, 11.11.1. |
| **B5** | 1, 2, 3 | L0 : 3.3 `partial`, 2.2, règle fantôme 1.8. L1 : 1.3, 1.4, 1.7, 2.2, 3.1. L2 : 3.3 (optionnel). |

Les sondes comportementales (L2) ne démarrent qu'après la vérification de capacités du §10 concernée ; en attendant, le thème avance avec ses lots L0 et L1.

## 9. Critères humains par conception

Leur statut `needs_review` est assumé. Ils sont **exclus de l'objectif de couverture** et ne reçoivent aucun mécanisme de verdict :

- **Pertinence** : 4.2, 4.6, 4.9 ; pertinence dans 1.3, 1.7, 4.4, 5.2, 5.5, 8.6, 11.7, 11.8.3.
- **Décision de fond** : 1.4, 1.8, 3.1 (nature de l'information), 10.3, 10.10, 12.3 (test 1), 13.1 (test 4).
- **Équivalence ou compréhension** : 5.3 (linéarisation), 7.2 (équivalence), 13.6 (pertinence), 13.10–13.12 (alternative et désactivation).
- **Citations** : 9.4.
- **Messages de statut** : 7.5 (reste `Manuel`).

Une revue périodique de cette liste relève de la maintenance, pas de cette spec.

## 10. Vérifications de capacités avant de s'engager

Chacune se tranche par une fixture ; si elle échoue, le critère concerné reste sur son lot L0/L1 et la sonde est abandonnée ou passe sous Chrome (§3.1).

**Mesurées sous Obscura 0.2.2** (prototype et essais) :
- `document.doctype` : fonctionne (8.1).
- Redimensionnement à 320 px : fonctionne ; `matchMedia` et la largeur calculée suivent (10.11, 10.4, 13.9).
- Capture d'écran : fonctionne (non utilisée hors 13.7).
- `getComputedStyle().direction` : **vide** (8.10 passe par les attributs `dir`).
- Navigation Tab native : **absente**, `keydown` livré (12.9, 4.11, 4.12 en forme `keydown` annulable).

**Non vérifiées** (à tester par fixture avant le lot concerné) :
- `Page.addScriptToEvaluateOnNewDocument` et hook `addEventListener` (7.3, 12.10, 13.1, 13.2) ;
- `Emulation.setScriptExecutionDisabled` : commande acceptée, effet non mesuré (7.2) ;
- `Input.dispatchMouseEvent` (10.13, 12.11) ;
- `document.getAnimations()` (13.8) ;
- comportement réel de `video.play()` et politique d'autoplay (4.10) ;
- `DOMDebugger.getEventListeners` (7.3).

**Écart de version axe** : l'injection utilise 4.9.1, le catalogue vient de 4.13. Toute règle citée en `axe-N` ou `data` (notamment `duplicate-id`, `duplicate-id-active`, `avoid-inline-spacing`, `meta-refresh-no-exceptions`, `label-content-name-mismatch`) est confirmée dans 4.9.1 par une fixture, faute de quoi elle ne produit rien et la fixture `fail` échoue au plancher (§4).

**12.9 : câblage.** `rgaa-obscura` contient une sonde Tab (`run_igt_keyboard`) qui n'est convertie en verdict d'aucun critère (déduit de recherches textuelles). La sonde cible de 12.9 est le balayage `keydown` annulable du prototype, pas la règle « 5 Tab sur le même élément » (faux positif sur la sortie par Échap, indistinguable sous Obscura).

## 11. Conséquences sur le pipeline et les rapports

- Le taux de conformité par page se calcule hors critères de site ; l'audit compte chaque critère de site **une fois**.
- Un audit d'une seule page donne `review` « moins de deux pages comparables » pour les critères de comparaison.
- Le rapport indique en tête les critères non testés faute de Chrome (§3.2).
- La couverture se rapporte en deux compteurs : **décisif** et **orienté** (§2).

## 12. Traçabilité des décisions

| Décision | Ticket de la carte #247 |
|---|---|
| Plancher de validation (§4) | Grilling : plancher de validation d'un nouveau contrôle (fixtures et preuve) |
| Ordre de priorité et lots (§8, §9) | Grilling : ordre de priorité des critères à couvrir |
| Moteurs, contrat trois issues, registre, linter, pixels (§2, §3, §5) | Grilling : moteurs retenus pour les nouveaux contrôles |
| Niveau site (§6) | Grilling : verdict de site pour les critères multi-pages |
| Sonde clavier sous Obscura (§7.1, §10) | Prototype : sonde comportementale clavier (piège 12.9) sur une page de test |
| Faisabilité par critère (§7) | Recherches : faisabilité des contrôles, thèmes 1–4, 5–9, 10–11, 12–13 |
| Couvertures déclarées à tort (3.3, 5.6, 12.1, 12.4) | Corriger la couverture déclarée de 3.3, 5.6, 12.1 et 12.4 |
