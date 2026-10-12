# axe-core : dernière version, licence, règles nouvelles et expérimentales

Ticket : [#332](https://github.com/jamon8888/Holo-RGAA/issues/332) — carte parente [#330](https://github.com/jamon8888/Holo-RGAA/issues/330).
Recherche menée le 2026-10-12. Ne mesure **pas** la dérive des verdicts (tâche d'implémentation).

## Réponse courte

- **Version à embarquer : axe-core 4.14.0** (publiée le 2026-10-05, tag npm `latest`).
- **Licence : MPL-2.0**, copyleft *par fichier*. Embarquer `axe.min.js` tel quel dans un binaire distribué sous MIT est permis (« Larger Work »), à condition de : garder l'en-tête de copyright du fichier, livrer le texte de la MPL-2.0, indiquer où obtenir la forme source, et reproduire les avis MIT/ISC des composants tiers regroupés dans axe (`LICENSE-3RD-PARTY.txt`).
- **Règles 4.9.1 → 4.14.0** : +2 (`aria-tab-name`, `summary-name`), aucune supprimée, aucune renommée ; 103 → 105 règles.
- **Expérimentales** : 6 des 7 le restent. **`label-content-name-mismatch` n'est plus expérimentale depuis 4.14.0 et s'exécute par défaut.**
- **Fait nouveau majeur** : depuis 4.11.0, axe porte lui-même des étiquettes `RGAAv4` / `RGAA-x.y.z` sur 67 règles — une table de correspondance RGAA maintenue en amont, comparable à notre registre.

## Méthode

Faits extraits des sources primaires :

- registre npm `https://registry.npmjs.org/axe-core` (`dist-tags`, dates de publication) ;
- archives npm `axe-core-4.9.1.tgz`, `axe-core-4.13.0.tgz`, `axe-core-4.14.0.tgz` (`https://registry.npmjs.org/axe-core/-/axe-core-<v>.tgz`) : fichiers `LICENSE`, `LICENSE-3RD-PARTY.txt`, `package.json`, `axe.min.js`, `sri-history.json` ;
- liste des règles obtenue en **exécutant** `axe.js` de chaque archive sous Node 24 (`require(...)`, puis `axe._audit.rules` et `axe.getRules()`) — mesure locale, reproductible ;
- `CHANGELOG.md`, `doc/API.md`, `doc/rule-descriptions.md` au tag `v4.14.0` du dépôt `dequelabs/axe-core` ;
- recherche d'issues GitHub `dequelabs/axe-core` (`gh search issues`, `gh issue list --label experimental`) le 2026-10-12.

Légende : **[source]** = établi par la source citée ; **[mesure]** = mesuré localement sur l'archive npm ; **[inférence]** = raisonnement de l'auteur, non vérifié ; **non vérifié** = inconnu.

## 1. Dernière version stable

| Version | Publication npm | Remarque |
|---|---|---|
| 4.9.1 | 2024-05-06 | injectée aujourd'hui depuis cdnjs (`rgaa-obscura/src/lib.rs:39`, `rgaa-obscura/src/native.rs:15`) |
| 4.10.0 | 2024-07-29 | ajoute `summary-name` |
| 4.11.0 | 2025-10-09 | ajoute les étiquettes RGAA |
| 4.12.0 | 2026-06-01 | ajoute `aria-tab-name` |
| 4.13.0 | 2026-08-05 | version de `rgaa-rs/package-lock.json` et du catalogue `axe_rules.json` [mesure : étiquettes identiques à 4.13.0] |
| **4.14.0** | **2026-10-05** | `dist-tags.latest` = `4.14.0` |

- [source] `dist-tags` : `{"latest": "4.14.0", "next": "4.14.0-canary.44f5b6c"}` — https://registry.npmjs.org/axe-core
- [source] Release GitHub `v4.14.0`, publiée le 2026-10-05 — https://github.com/dequelabs/axe-core/releases/tag/v4.14.0
- [source] Empreinte SRI publiée par Deque pour `axe.min.js` 4.14.0 : `sha256-IMCf4Veoo0ow4kGqofza3mV3NPCKs3ns++t9RcxG6Hg=` (`sri-history.json` de l'archive npm). [mesure] Le `sha256sum` du fichier extrait (`20c09fe157a8a34a30e241aaa1fcdade657734f08ab379ecfbeb7d45cc46e878`) correspond. Taille : 652 140 octets (contre 555 038 pour 4.9.1).

## 2. Licence et obligations dans un binaire MIT

Faits [source : `LICENSE` et `package.json` de l'archive 4.14.0, identiques au dépôt https://github.com/dequelabs/axe-core/blob/v4.14.0/LICENSE] :

- `package.json` : `"license": "MPL-2.0"`. `LICENSE` : texte intégral de la Mozilla Public License 2.0.
- En-tête de `axe.min.js` : « Copyright (c) 2015 - 2026 Deque Systems, Inc. », avis MPL-2.0 (Exhibit A), et « This entire copyright notice must appear in every copy of this file you distribute ».
- L'en-tête **ne porte pas** l'avis Exhibit B « Incompatible With Secondary Licenses ».
- `LICENSE-3RD-PARTY.txt` : axe regroupe du code sous **MIT** (colorjs.io, core-js-pure, css-selector-parser, doT.js, emoji-regex, es6-iterator, es6-promise, event-emitter, is-promise, lru-queue, typedarray, weakmap-polyfill) et sous **ISC** (d, es5-ext). [mesure] `axe.min.js` ne contient que l'avis Deque : les avis tiers ne sont **pas** dans le fichier minifié.

Clauses de la MPL-2.0 qui s'appliquent [source : `LICENSE`] :

- §1.6 : « Executable Form » = toute forme autre que la forme source ; §1.7 : « Larger Work » = combinaison avec d'autres fichiers qui ne sont pas couverts.
- §3.3 : un Larger Work peut être distribué **sous les termes de son choix** si les obligations MPL sont respectées pour les fichiers couverts → **le binaire `rgaa-rs` reste sous MIT**.
- §3.2(a) : distribuer la forme exécutable impose de rendre la forme source disponible et d'**indiquer comment l'obtenir**.
- §3.1 / §3.4 : la forme source reste sous MPL-2.0 ; interdiction de retirer ou d'altérer les avis de licence et de copyright.

Ce qu'il faut faire [inférence, pas un avis juridique] :

1. Garder `axe.min.js` **non modifié**, en-tête compris. Toute modification du fichier reste sous MPL-2.0 et doit être publiée sous cette forme.
2. Un fichier minifié n'est probablement pas « la forme préférée pour le modifier » : il faut le traiter comme une forme exécutable (§3.2). Deux options suffisent : (a) committer aussi `axe.js` (non minifié, même archive), ou (b) documenter dans un fichier de licence l'URL de l'archive npm et du tag `v4.14.0`.
3. Ajouter à côté du fichier embarqué : `LICENSE` (MPL-2.0) et `LICENSE-3RD-PARTY.txt` d'axe, et les référencer depuis les avis tiers du binaire. L'inclusion via `include_str!` place le fichier dans le binaire : les avis doivent donc accompagner aussi les **archives de release** (`release.yml`, `install.sh`), pas seulement le dépôt.
4. Compatibilité : MPL-2.0 sans Exhibit B est compatible avec une distribution d'ensemble sous MIT. Seul le fichier d'axe reste sous MPL-2.0 ; le reste du code n'est pas contaminé (copyleft par fichier). **Aucun obstacle de licence** à l'embarquement.

## 3. Règles ajoutées, supprimées ou renommées entre 4.9.1 et 4.14.0

[mesure] `axe._audit.rules` : 4.9.1 = **103** règles ; 4.13.0 = 105 ; 4.14.0 = **105**. Aucune règle supprimée, aucune renommée, aucune différence d'identifiant entre 4.13.0 et 4.14.0.

| Règle | Arrivée | Étiquettes 4.14.0 | Par défaut | Source |
|---|---|---|---|---|
| `summary-name` | 4.10.0 | `wcag2a`, `wcag412`, `section508.22.a`, `TT6.a`, `EN-9.4.1.2` — **pas d'étiquette RGAA** | activée | CHANGELOG 4.10.0, « summary elements must have an accessible name » (#4511, closes #4510) |
| `aria-tab-name` | 4.12.0 | `wcag412`, `ACT`, **`RGAA-7.1.1`** | activée | CHANGELOG 4.12.0, « check that aria-tab have an accessible name » (#5001, closes #4842) |

- CHANGELOG : https://github.com/dequelabs/axe-core/blob/v4.14.0/CHANGELOG.md
- `aria-tab-name` manque à l'exécution en 4.9.1 (constat du ticket) : **confirmé** [mesure], la règle n'existe qu'à partir de 4.12.0.

Autres changements de métadonnées qui touchent le registre :

- **4.11.0** — « add RGAA tags to rules (#4862) » et « tap: some best practice rules map to RGAA (#4895) » [source : CHANGELOG]. [mesure] Les 67 règles étiquetées `RGAAv4` en 4.14.0 n'en avaient aucune en 4.9.1.
- **4.11.1** — « rgaa: adjust mapping of aria-hidden-* and valid-lang (#4935) » [source : CHANGELOG].
- **4.12.0** — `landmark-complementary-is-top-level` passe en `deprecated` (#4992) [source : CHANGELOG ; mesure]. Le registre ne la cite pas.
- **4.12.0** — `axe.getRules()` renvoie désormais un champ `enabled` (#5118) [source : CHANGELOG]. [mesure] En 4.14.0, `enabled: false` pour 15 règles : les 6 expérimentales, les 5 `deprecated` (`aria-roledescription`, `audio-caption`, `duplicate-id`, `duplicate-id-active`, `landmark-complementary-is-top-level`), plus `color-contrast-enhanced`, `identical-links-same-purpose`, `meta-refresh-no-exceptions` et `target-size`.
- **4.14.0** — `label-content-name-mismatch` activée par défaut (#5421) ; `target-size` reçoit les étiquettes `EN-301-549v4` (#5361) [source : CHANGELOG].
- Correctifs 4.14.0 qui touchent des règles citées par le registre : `p-as-heading` ignore les `<p role="heading">` (#5348, faux positif #5283) ; `scrollable-region-focusable` renvoie *needs review* au lieu d'un échec pour un contenu en `tabindex` négatif (#5364) ; `color-contrast` traite en *incomplete* les champs vides à faible contraste (#5359) ; résolution des `@import` contre la bonne feuille (#5269, closes #5268), utile à `css-orientation-lock` [source : CHANGELOG].

## 4. Les 7 règles expérimentales citées par le registre

Mécanisme d'activation [source : `doc/API.md` v4.14.0, https://github.com/dequelabs/axe-core/blob/v4.14.0/doc/API.md] :

- « The default operation for axe.run is to run all rules except for rules with the "experimental" tag. » Le tableau des étiquettes décrit `experimental` comme « Cutting-edge rules, disabled by default ».
- Activation une par une : `axe.run(document, { rules: { 'p-as-heading': { enabled: true } } })` — c'est l'exemple 3 d'API.md, présenté comme « enable two additional experimental rules ». Compatible avec la décision de cadrage « une par une ».
- [source : code local] `rgaa-obscura` appelle `axe.run(document)` / `axe.run()` **sans options** (`native.rs:337`, `cdp_pool.rs:339`), donc aucune règle expérimentale ne tourne aujourd'hui. Les deux chemins ne gardent que `results.violations` : les résultats *incomplete* (« needs review ») sont jetés.

| Règle | Expérimentale en 4.14.0 ? | Étiquette RGAA d'axe | Registre (`mechanisms.toml`) | Résultats possibles (rule-descriptions) | Issues ouvertes pertinentes |
|---|---|---|---|---|---|
| `css-orientation-lock` | **oui** | `RGAA-13.9.1` | `axe-13-9` | failure, needs review | [#5268](https://github.com/dequelabs/axe-core/issues/5268) : `@import` chargé depuis une mauvaise URL → résultat faussé sans bruit. Le CHANGELOG 4.14.0 dit le corriger (#5269), l'issue reste ouverte au 2026-10-12. |
| `focus-order-semantics` | **oui** | `RGAA-12.8.1` (best-practice) | `axe-7-3` | failure | [#3863](https://github.com/dequelabs/axe-core/issues/3863) « Question about focus-order-semantics logic » (label `experimental`, ouverte depuis 2023). Faux positif `role="article"` corrigé (#3905, fermée). |
| `hidden-content` | **oui** | aucune (`best-practice`, `review-item`) | `axe-10-8` | failure, needs review | Pas d'issue ouverte au titre de la règle trouvée. |
| `label-content-name-mismatch` | **non — activée par défaut depuis 4.14.0** (#5421, épopée #5226 fermée) | `RGAA-6.1.5` | `axe-11-2` | failure, needs review | [#5458](https://github.com/dequelabs/axe-core/issues/5458) / [#5459](https://github.com/dequelabs/axe-core/issues/5459) : textes de blocs adjacents collés sans espace (risque de faux positif) ; [#4261](https://github.com/dequelabs/axe-core/issues/4261) (faux négatif, contenu `sr-only`) ; [#5339](https://github.com/dequelabs/axe-core/issues/5339) (faux négatif, technique F96). Six correctifs dans 4.14.0 (#5345, #5355, #5360, #5419, #5437…). |
| `p-as-heading` | **oui** | `RGAA-9.1.3` | `axe-9-1` | failure, needs review | Aucune issue ouverte au titre de la règle ; faux positif `<p role="heading">` corrigé en 4.14.0 (#5283 → #5348). |
| `table-fake-caption` | **oui** | `RGAA-5.4.1` | `axe-5-4` | failure | Aucune issue au titre de la règle, ouverte ou fermée. |
| `td-has-header` | **oui** | `RGAA-5.7.4` | `axe-5-7` | failure | [#2028](https://github.com/dequelabs/axe-core/issues/2028) : `headers=""` vide sur un `<th>` non détecté (faux négatif, ouverte depuis 2020). |

Sources : `doc/rule-descriptions.md` v4.14.0, section « Experimental Rules » (https://github.com/dequelabs/axe-core/blob/v4.14.0/doc/rule-descriptions.md) ; étiquettes [mesure] ; issues : recherche `"<règle> in:title"` sur dequelabs/axe-core.

Issues transverses ouvertes (2026-10-08/09, non triées par Deque), qui touchent plusieurs de ces règles : sélecteurs qui comparent les attributs à la casse près, par exemple `scope="COL"` ou `aria-hidden="TRUE"` ([#5450](https://github.com/dequelabs/axe-core/issues/5450), [#5453](https://github.com/dequelabs/axe-core/issues/5453), [#5454](https://github.com/dequelabs/axe-core/issues/5454), [#5456](https://github.com/dequelabs/axe-core/issues/5456) pour `td-has-header`/`scope-attr-valid`). Ce sont des faux négatifs (règle qui ne s'applique pas), pas des faux positifs.

Limite : la recherche porte sur les titres d'issues. Une issue qui signale un faux positif sans nommer la règle dans son titre a pu échapper. L'exhaustivité est **non vérifiée**.

## 5. Règles 4.14.0 applicables à nos tests RGAA et non mappées

[mesure] 72 identifiants de règles distincts sont cités dans `mechanisms.toml` ; tous existent en 4.14.0. 33 règles de 4.14.0 ne sont citées nulle part. **Aucune n'a d'étiquette RGAA chez axe** : toutes les règles étiquetées RGAA par Deque sont déjà dans le registre.

Les 33 règles non citées se répartissent ainsi :

- **désactivées par défaut** (`deprecated`, AAA ou WCAG 2.2 hors RGAA 4.1.2) : `aria-roledescription`, `audio-caption`, `duplicate-id`, `duplicate-id-active`, `landmark-complementary-is-top-level`, `color-contrast-enhanced`, `identical-links-same-purpose`, `meta-refresh-no-exceptions`, `target-size` ;
- **WCAG A/AA sans étiquette RGAA** : `summary-name` (4.1.2), `aria-tooltip-name` (4.1.2), `aria-braille-equivalent` (4.1.2), `avoid-inline-spacing` (1.4.12) ;
- **best-practice** : `accesskeys`, `aria-allowed-role`, `aria-dialog-name`, `aria-text`, `aria-treeitem-name`, `empty-heading`, `empty-table-header`, `frame-tested`, `landmark-banner-is-top-level`, `landmark-contentinfo-is-top-level`, `landmark-main-is-top-level`, `landmark-no-duplicate-banner`, `landmark-no-duplicate-contentinfo`, `landmark-no-duplicate-main`, `landmark-unique`, `meta-viewport-large`, `page-has-heading-one`, `presentation-role-conflict`, `scope-attr-valid`, `tabindex`.

Candidates à un rattachement, **[inférence]** à valider test par test contre le texte RGAA 4.1.2 (rien de ceci n'est affirmé par Deque) :

| Règle | Test RGAA plausible | Pourquoi |
|---|---|---|
| `summary-name` | 7.1 (composant dont le nom est restitué) ou 11.9 (intitulé de bouton) | `<summary>` est un contrôle interactif sans nom ; nouvelle depuis 4.10.0. |
| `aria-tooltip-name`, `aria-dialog-name`, `aria-treeitem-name` | 7.1.1 | Même nature que `aria-tab-name`, que Deque rattache à `RGAA-7.1.1`. |
| `avoid-inline-spacing` | 10.12 (propriétés d'espacement du texte) | SC WCAG 1.4.12, qui correspond au critère 10.12. |
| `empty-heading`, `page-has-heading-one` | 9.1 | Structure des titres ; Deque les laisse en best-practice. |
| `empty-table-header`, `scope-attr-valid` | 5.6 / 5.7 | En-têtes de tableaux de données. |
| `landmark-*` | 12.6 | Zones de regroupement ; le registre ne cite que `landmark-one-main` et `region`. |
| `tabindex` | 12.8 | Ordre de tabulation (`tabindex` > 0). |

Écarts entre le rattachement du registre et l'étiquette RGAA d'axe [mesure, non jugés ici — à trancher par un ticket d'implémentation] : `focus-order-semantics` (axe 12.8.1 / registre `axe-7-3`) ; `label-content-name-mismatch` (6.1.5 / `axe-11-2`) ; `server-side-image-map` (1.1.4 / `axe-7-3`) ; `aria-hidden-body` (10.8.1 / `axe-7-1`) ; `aria-required-children` et `aria-required-parent` (9.3.1 / `axe-7-1`) ; `aria-command-name` (11.9.1 / `axe-7-1`) ; `aria-toggle-field-name` (7.1.1 / `axe-11-1`) ; `frame-focusable-content` (7.3.2 / `axe-2-1`) ; `table-duplicate-name` (5.2.1 / `axe-5-4`). Plusieurs critères peuvent légitimement reposer sur la même règle : un écart n'est pas forcément une erreur.

Dette de provenance : beaucoup d'entrées de `mechanisms.toml` et `rgaa-data/src/validate.rs:52` citent encore « axe-core 4.9.1 rule-descriptions.md », alors que `axe_rules.json` est en 4.13 (`help_url` en `/4.13/`, `label-content-name-mismatch` encore `experimental`). Une mise à jour en 4.14.0 doit régénérer `axe_rules.json`.

## 6. À vérifier par fixture sous Obscura

Rien de ce qui suit n'a été exécuté sous Obscura. Ce sont des hypothèses **[inférence]** à confirmer avec le plancher de validation (fixture conforme + non conforme) :

1. **`axe.min.js` 4.14.0 se charge et `axe.run()` aboutit sous Obscura.** Le fichier grossit de ~97 Ko ; la version 4.13.0 a activé par défaut le support des `ElementInternals` (#5284), et le dépôt a déjà un réglage à ce sujet (`rgaa-obscura/src/config.rs:127`).
2. **`label-content-name-mismatch` tourne désormais par défaut** : de nouvelles violations apparaîtront sans opt-in. Vérifier le verdict sur des liens et boutons dont le texte visible est réparti sur plusieurs blocs (#5458/#5459).
3. **`css-orientation-lock`** lit le CSSOM (`@media (orientation)` + `transform`) : vérifier que les feuilles externes et `@import` sont visibles sous Obscura, sinon la règle se tait (faux négatif silencieux, cf. #5268).
4. **`p-as-heading`** repose sur `getComputedStyle` (graisse, italique, taille) : vérifier ces valeurs sous Obscura, puisque `getComputedStyle().direction` y est déjà vide (fait établi sur la carte #330).
5. **`focus-order-semantics`** : vérifier qu'elle ne dépend pas d'une navigation Tab réelle (absente sous Obscura).
6. **`hidden-content`** renvoie surtout du *needs review* (étiquette `review-item`) : avec le code actuel qui jette `incomplete`, son activation n'aurait probablement **aucun effet visible**. Vérifier le partage failure / incomplete.
7. **`td-has-header`, `table-fake-caption`, `aria-tab-name`, `summary-name`** sont des règles de DOM et d'arbre d'accessibilité : comportement attendu identique à Chrome, à confirmer.
8. **Règles qui passent en *needs review*** (`scrollable-region-focusable` #5364, `color-contrast` #5359) : moins de `violations` et plus d'`incomplete`. Avec le traitement actuel, certaines non-conformités disparaîtraient du rapport au lieu d'être signalées à revoir.

## 7. Inconnu / non vérifié

- Disponibilité de 4.14.0 sur cdnjs : non vérifié (sans objet si le fichier est embarqué).
- Exhaustivité des faux positifs connus : la recherche porte sur les titres d'issues seulement.
- Calendrier de sortie du statut expérimental des 6 règles restantes : aucune feuille de route publique trouvée, **non vérifié**.
- Lecture juridique de la MPL-2.0 : interprétation d'ingénieur, pas un avis juridique.
