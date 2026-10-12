# Moteurs d'accessibilité open source complémentaires à axe

Ticket : [#336](https://github.com/jamon8888/Holo-RGAA/issues/336). Carte parente : [#330](https://github.com/jamon8888/Holo-RGAA/issues/330).
Recherche menée le 2026-10-12, uniquement sur sources primaires : dépôts officiels, fichiers `LICENSE`, code source aux tags cités, données du W3C.

**Convention.** Chaque fait porte sa source. Un raisonnement de notre part est marqué **(inférence)**. Ce qui n'a pas été établi est marqué **non vérifié**. La correspondance « règle du moteur → test RGAA » n'est publiée par aucun des trois moteurs WCAG (voir §1). Toute correspondance RGAA donnée ici est donc une **inférence** : elle passe par le critère WCAG que le moteur déclare et par les références WCAG de chaque critère RGAA (`rgaa-rs/crates/rgaa-core/data/rgaa-4.1.2/criteres.json`, champ `references`).

## 0. Réponse courte

1. **Pour le périmètre « humain », aucun moteur open source ne rend un verdict `pass` qu'axe ne rendrait pas.** Quand Alfa, IBM Equal Access, QualWeb ou Asqatasun traitent un test de pertinence ou d'équivalence, ils produisent une **question** ou un résultat « à vérifier » (`cantTell`, `Potential`, `NEED_MORE_INFO`), jamais une conclusion. Ce qu'ils apportent relève donc au mieux du niveau **Assisté** : un ciblage d'éléments, une heuristique ou une question structurée.
2. **Pour la moitié de la liste « règles axe expérimentales », la meilleure réponse est de passer à la dernière version d'axe, sans ajouter de moteur.** Axe 4.14.0 contient `aria-tab-name`, étiquetée `RGAA-7.1.1` et absente de 4.9.1. Il active aussi `label-content-name-mismatch` par défaut. Le §3 donne les sources.
3. **Alfa (MIT) est le seul candidat dont la forme correspond à notre modèle Assisté.** Ses 19 règles à questions déclarent leur applicabilité, puis posent une question typée : booléen, nœud ou couleur. Un « oracle » branché par l'intégrateur y répond (§2.1). Certaines questions sont **factuelles** : « la vidéo a-t-elle du son ? », « dure-t-elle plus de 3 s ? ». Le sidecar `ffmpeg` déjà validé pourrait y répondre de façon déterministe **(inférence)**. C'est l'unique piste trouvée qui pourrait faire passer un test du niveau Assisté au niveau **Automatique** (thème 4, sons automatiques).
4. **IBM Equal Access (Apache-2.0)** apporte les heuristiques « Potential » les plus ciblées sur des tests de notre périmètre. On y trouve les citations non balisées (9.4), le tableau de mise en forme non linéarisable (5.3), l'art ASCII et les émoticônes (13.6), la couleur seule (3.1) et les caractéristiques sensorielles (10.10). Pour 10.10, la liste de mots est **en anglais**. Il s'injecte exactement comme axe : un script `ace.js` évalué dans la page.
5. **QualWeb (ISC)** recoupe largement axe, Alfa et IBM. Il n'apporte presque rien de propre au périmètre.
6. **Asqatasun (AGPL-3.0)** est le seul moteur à suivre RGAA 4.1.2. Il est maintenu sur GitLab. Pour les tests de notre périmètre, il rend un « pré-qualifié » (`NEED_MORE_INFO`) **sans verdict** (exemple : 5.3.1, §2.4). Il exige Java 17, Selenium et Firefox, et il ne peut pas tourner sous Obscura. Sa licence ne l'admet qu'en service séparé. Pour nous, il sert de **référence de comparaison**, pas de brique.
7. **Le catalogue ACT du W3C** compte 94 règles, dont 57 au statut « proposed » et 7 dépréciées. Il sert de **banc de fixtures** (cas d'essai passed/failed publiés) et de carte de couverture par outil. Ce n'est pas un moteur. Plusieurs règles ACT de notre périmètre n'ont **aucune** implémentation déclarée par axe, Alfa, IBM ou QualWeb (§2.5). Ce sont des « Irréductible » de fait côté moteurs.

## 1. Tableau par moteur

| Moteur | Version vérifiée | Licence (fichier LICENSE) | Compatible MIT ? | Forme d'intégration sous Obscura | Maintenance | Correspondance RGAA publiée |
|---|---|---|---|---|---|---|
| axe-core (référence) | 4.14.0 (2026-10-05) | MPL-2.0 ([API dépôt](https://github.com/dequelabs/axe-core)) | oui, déjà utilisé | script injecté par `Runtime.evaluate` (en place, 4.9.1) | active | **oui** : étiquettes `RGAAv4` et `RGAA-x.y.z` ajoutées par [#4862](https://github.com/dequelabs/axe-core/pull/4862) (commit du 2025-09-05) |
| Siteimprove **Alfa** | v0.119.0 (dernier tag) ; `alfa-rules` 0.119.0 | MIT ([LICENSE.md](https://github.com/Siteimprove/alfa/blob/main/LICENSE.md)) | oui | sérialiseur DOM+CSSOM injecté (`Native.fromNode`), puis audit en JS, dans la page ou dans un processus Node isolé | active (dernier push 2026-10-09) | **non** : aucune occurrence de « rgaa » dans le dépôt ([recherche de code GitHub](https://github.com/search?q=repo%3ASiteimprove%2Falfa+rgaa&type=code), 0 résultat) |
| IBM **Equal Access** (`accessibility-checker-engine`) | release 4.0.34 (2026-09-08) | Apache-2.0 ([LICENSE](https://github.com/IBMa/equal-access/blob/main-4.x/LICENSE)) | oui (licence permissive ; conserver LICENSE et NOTICE en redistribution) | `ace.js` injecté dans la page, puis `new ace.Checker().check(document, [...])` ([README-NPM](https://github.com/IBMa/equal-access/blob/main-4.x/accessibility-checker-engine/README-NPM.md)) | active (dernier push 2026-09-28) | **non** (0 résultat « rgaa » dans le dépôt) |
| **QualWeb** | `@qualweb/act-rules` 0.8.6 ; dernier commit 2026-09-26 | ISC ([LICENSE](https://github.com/qualweb/qualweb/blob/main/LICENSE)) | oui | bundles injectés par `addScriptTag` (`qw-page`, `util`, `locale`, puis le module), pilotés par Puppeteer ou Playwright ([QualwebPage.object.ts](https://github.com/qualweb/qualweb/blob/main/packages/core/src/lib/QualwebPage.object.ts)) | active | **non** (0 résultat « rgaa ») |
| **Asqatasun** | 6.0.0-rc.6 (2026-05-06, [releases GitLab](https://gitlab.com/asqatasun/Asqatasun/-/releases)) ; CHANGELOG jusqu'à rc.7 (2026-06-19) | **AGPL-3.0** ([LICENSE](https://gitlab.com/asqatasun/Asqatasun/-/blob/master/LICENSE)) | **non** : admissible **en service séparé seulement** | serveur Java 17 / Spring Boot avec son propre navigateur (Selenium, Firefox ESR, geckodriver) ([pom.xml](https://gitlab.com/asqatasun/Asqatasun/-/blob/master/pom.xml), [CHANGELOG](https://gitlab.com/asqatasun/Asqatasun/-/blob/master/CHANGELOG.md)) ; **pas de mode « injecter dans Obscura »** | active sur GitLab (dernier commit 2026-07-03). Le miroir GitHub est figé depuis 2023-02-22 | **oui, natif** : module `rules-rgaa4.1.2` |
| Règles **ACT** du W3C | données `wcag-act-rules`, branche `main` (push 2026-10-08) | W3C Document License ([LICENSE.md](https://github.com/w3c/wcag-act-rules/blob/main/LICENSE.md)) | sans objet : ce sont des spécifications et des fixtures, pas du code | aucune : catalogue + cas d'essai HTML | active | non (correspondance WCAG uniquement, [wcag-mapping.json](https://github.com/w3c/wcag-act-rules/blob/main/wcag-mapping.json)) |

**Rust natif.** Une recherche rapide sur crates.io (`q=wcag`) ne remonte que des crates jeunes et peu téléchargées, par exemple `auditmysite`, `haptic-css` ou `ssg-a11y`. **Non évaluées.** Aucune n'a d'implémentation ACT déclarée au W3C ([liste des implémentations](https://github.com/w3c/wcag-act-rules/blob/main/_data/wcag-act-rules/act-implementations.yml)). Aucun moteur Rust mature n'a été trouvé. Cette conclusion n'est pas exhaustive.

## 2. Détail par moteur

### 2.1 Siteimprove Alfa (MIT)

- **Nature.** Moteur conforme au format ACT. Il produit de l'EARL en JSON-LD et tourne « dans tout environnement JavaScript compatible ECMAScript 2022 » ([README](https://github.com/Siteimprove/alfa#requirements)).
- **Entrée sous Obscura.** `packages/alfa-dom/src/native.ts` est écrit pour être « injecté dans une page de navigateur […] via un outil d'automatisation ». Il sérialise le DOM, les feuilles de style (`sheet.cssRules`) et les boîtes (`getBoundingClientRect`) en JSON ([native.ts@v0.119.0](https://github.com/Siteimprove/alfa/blob/v0.119.0/packages/alfa-dom/src/native.ts)). Alfa calcule ensuite sa propre cascade CSS à partir des feuilles sérialisées. **(Inférence)** : il dépend donc moins de `getComputedStyle`, qui est incomplet sous Obscura (`direction` vide, d'après la carte #330). En revanche, il dépend d'un CSSOM `cssRules` complet. **À vérifier sous Obscura.**
- **Règles.** `rules.ts` importe 89 règles actives, `experimental.ts` en ajoute 8 (`ER8, ER87, R82, R101, R109, R114, R115, R117`) et `deprecated.ts` en liste 5 ([rules.ts](https://github.com/Siteimprove/alfa/blob/v0.119.0/packages/alfa-rules/src/rules.ts), [experimental.ts](https://github.com/Siteimprove/alfa/blob/v0.119.0/packages/alfa-rules/src/experimental.ts)). Les intitulés figurent sur le [hub Alfa](https://alfa.siteimprove.com/rules).
- **Questions (semi-automatique).** Les questions sont définies dans [question.ts@v0.119.0](https://github.com/Siteimprove/alfa/blob/v0.119.0/packages/alfa-rules/src/common/act/question.ts). Les règles qui en posent sont : R15, R22–R27, R29–R33, R35, R37–R39, R41, R48–R50, R55, R65, R66, R69, R81, R87, et les expérimentales R82, R101, R109, R114, R115, R117 et ER87. Un « oracle » fourni par l'intégrateur répond ; sinon le résultat est `cantTell` ([README, section Examples](https://github.com/Siteimprove/alfa#examples)). Questions pertinentes pour nous :
  - **Factuelles** (un outil pourrait y répondre) : `has-audio` (« la `<video>` a-t-elle de l'audio ? »), `is-above-duration-threshold` (« plus de 3 secondes ? »), `is-below-audio-duration-threshold`, `is-playing`, `is-audio-streaming` et `is-video-streaming`. Elles servent R48, R49 et R50 (son automatique) et les règles média.
  - **De jugement** (Assisté au mieux) : `is-title-descriptive` (R114), `is-heading-descriptive` (R115), `is-image-accessible-name-descriptive` (R117), `name-describes-purpose` (R39), `has-captions`, `has-description`, `track-describes-video`, `is-content-equivalent` (R55), `reference-equivalent-resources` (R15, R41, R81), `error-indicator-*` (R82) et `has-focus-indicator` (R65).
- **Validation externe.** Le rapport ACT du W3C publie deux implémentations Alfa 0.118.0, datées du 13 juillet 2026. En mode automatique, Alfa est « complete » sur 27 règles approuvées et 3 proposées. En mode semi-automatique, il l'est sur 29 règles approuvées et 25 proposées ([alfa.json](https://github.com/w3c/wcag-act-rules/blob/main/_data/wcag-act-rules/implementations/alfa.json), [alfa-assisted.json](https://github.com/w3c/wcag-act-rules/blob/main/_data/wcag-act-rules/implementations/alfa-assisted.json)). Le gain du mode assisté porte presque entièrement sur le média, sur 3.3.1, sur les titres et intitulés descriptifs et sur la langue.

### 2.2 IBM Equal Access (Apache-2.0)

- **Intégration.** `ace.js` (build navigateur) s'injecte dans la page, comme axe. Le README-NPM montre l'injection par `page.addScriptTag` sur un navigateur connecté en CDP ([README-NPM](https://github.com/IBMa/equal-access/blob/main-4.x/accessibility-checker-engine/README-NPM.md)). C'est la forme la plus proche de notre intégration axe actuelle **(inférence)**.
- **Règles.** Le tag 4.0.34 contient 172 fichiers de règles ([src/v4/rules@4.0.34](https://github.com/IBMa/equal-access/tree/4.0.34/accessibility-checker-engine/src/v4/rules)). Selon notre comptage par script sur le code, 73 peuvent rendre `Potential` ou `Manual`, c'est-à-dire « à vérifier », et 9 rendent `Manual`. Les règles qui touchent notre périmètre, avec leur message exact dans le code :
  - `text_quoted_correctly` (WCAG 1.3.1, Potential) : « Quotations should be marked with <q> or <blockquote> elements ». **Inférence :** cible le test 9.4.1.
  - `blockquote_cite_exists` (1.3.1, Potential) : « Use <blockquote> only for quotations, not indentation ». **Inférence :** cible 9.4.2.
  - `table_layout_linearized` (1.3.1, RECOMMENDATION, Potential) : « Verify table is not being used to format text content in columns unless the table can be linearized ». **Inférence :** cible 5.3.1.
  - `style_color_misuse` (Potential) : « Verify color is not used as the only visual means of conveying information ». **Inférence :** cible 3.1.
  - `text_sensory_misuse` (1.3.3, Potential) : sa liste de mots est **en anglais** (`"top-left"`, `"round"`, `"right-click"`…), voir [text_sensory_misuse.ts](https://github.com/IBMa/equal-access/blob/4.0.34/accessibility-checker-engine/src/v4/rules/text_sensory_misuse.ts). **Inférence :** inopérant sur un site français sans traduction de la liste.
  - `asciiart_alt_exists` et `emoticons_alt_exists` (1.1.1, Potential). **Inférence :** cible 13.6, ce qui recoupe la sonde prévue par la spec amont.
  - `page_title_valid` (2.4.2, Fail) : échoue si le titre est vide ou ressemble à un nom de fichier. **Inférence :** indice pour 8.6.
  - `input_onchange_review`, `script_select_review`, `form_submit_review` et `form_interaction_review` (3.2.2 / 3.2.1, Potential), sur le changement de contexte. Ils sont **hors périmètre direct**, mais utiles au thème 7 et au 13.2 **(inférence)**.
  - `text_block_heading` (1.3.1, Potential) : texte qui ressemble à un titre sans en avoir le balisage. **Inférence :** recoupe `p-as-heading` (9.1.3).
  - `element_orientation_unlocked` (orientation verrouillée par `transform`), `label_name_visible` (nom accessible contenant l'étiquette visible) et `aria_widget_labelled` (nom des composants à rôle ARIA de widget). **Inférence :** ils recoupent `css-orientation-lock`, `label-content-name-mismatch` et `aria-tab-name`. Que `aria_widget_labelled` couvre `role="tab"` est **non vérifié par fixture**.
- **Validation ACT.** Equal Access 3.1.42-rc.0 (rapport du 20 juillet 2026) est « complete » sur 21 règles approuvées et 2 proposées ([equal-access.json](https://github.com/w3c/wcag-act-rules/blob/main/_data/wcag-act-rules/implementations/equal-access.json)).

### 2.3 QualWeb (ISC)

- **Modules.** `act-rules` (68 fichiers de règles QW-ACT-R*), `wcag-techniques` (34 fichiers QW-WCAG-T*), `best-practices` (27 fichiers QW-BP*) et `cui-checks` ([packages](https://github.com/qualweb/qualweb/tree/main/packages), [README best-practices](https://github.com/qualweb/qualweb/blob/main/packages/best-practices/README.md), [README wcag-techniques](https://github.com/qualweb/qualweb/blob/main/packages/wcag-techniques/README.md)).
- **Apport propre au périmètre.** Il est mince. QW-BP6 (titre de plus de 64 caractères) et QW-BP7 (titre contenant de l'art ASCII) donnent des indices pour 8.6 et 13.6. QW-BP2 (alternative concise) donne un indice pour 1.3. Pour les médias, ses règles recouvrent celles d'Alfa (80f0bf, 4c31df et aaa1bf « complete » dans le rapport ACT).
- **Validation ACT.** QualWeb 3.0.0 (rapport du 14 juillet 2026) est « complete » sur 23 règles approuvées et 9 proposées. Il est « inconsistent » sur 15 règles proposées ([qualweb.json](https://github.com/w3c/wcag-act-rules/blob/main/_data/wcag-act-rules/implementations/qualweb.json)).

### 2.4 Asqatasun (AGPL-3.0)

- **Référentiel.** Le dépôt contient les modules `rules-rgaa4.0` et `rules-rgaa4.1.2` ([rules/](https://gitlab.com/asqatasun/Asqatasun/-/tree/master/rules)). Le module 4.1.2 contient 26 classes `Rgaa412Rule*` (1.1.4–1.1.8, 1.3.6, 1.4.1–1.4.7, 1.8.6, 4.8.x, 4.9.1, 4.12.x, 5.3.1, 5.6.4, 9.1.2, 10.4.x, 10.9.4, 10.10.4, 13.7.1) ; les autres tests réutilisent vraisemblablement des classes génériques (non vérifié). Le module 4.0 en contient environ 257. Le nombre de tests 4.1.2 réellement « décidés » est **non vérifié** : la doc par règle (`doc.asqatasun.org/v6/...`) répond 404 le 2026-10-12.
- **Exemple représentatif du périmètre.** [Rgaa412Rule050301.java](https://gitlab.com/asqatasun/Asqatasun/-/blob/master/rules/rules-rgaa4.1.2/src/main/java/org/asqatasun/rules/rgaa412/Rgaa412Rule050301.java) (5.3.1, linéarisation) rend `NEED_MORE_INFO` pour tout tableau de mise en forme. Il ne rend `FAILED` que si un tableau marqué « de présentation » n'a pas `role="presentation"`. La linéarisation elle-même n'est pas jugée.
- **Conclusion (inférence).** Pour les tests humains, Asqatasun propose un pré-tri par marqueurs, pas un verdict. Ce pré-tri est reproductible en JS sous Obscura sans dépendance AGPL. Le dépôt a d'ailleurs un plan de remplacement d'Asqatasun ([docs/superpowers/plans/2026-08-08-rgaa-rs-asqatasun-replacement.md](../../../superpowers/plans/2026-08-08-rgaa-rs-asqatasun-replacement.md)).

### 2.5 Catalogue ACT du W3C

- 94 entrées dans [wcag-mapping.json](https://github.com/w3c/wcag-act-rules/blob/main/wcag-mapping.json) : 37 approuvées et 57 proposées, dont 7 dépréciées.
- **Usage pour nous (inférence).** Chaque règle publie des cas d'essai HTML passed, failed et inapplicable (par exemple `https://www.w3.org/WAI/content-assets/wcag-act-rules/testcases/<id>/<hash>.html`, référencés dans les rapports d'implémentation). Ce sont des fixtures toutes prêtes pour le plancher de validation « fixture conforme + non conforme » exigé par la carte.
- **Règles ACT du périmètre sans aucune implémentation « consistent » chez axe, Alfa (auto ou assisté), IBM ou QualWeb** (rapports cités au §2.1–2.3) :
  - `0va7u6` « HTML images contain no text » (→ 1.8) ;
  - `qt1vmo` « Image accessible name is descriptive » (→ 1.3) ;
  - `9bd38c` « Content has alternative for visual reference » (→ 10.10) ;
  - `cc0f0a` « Form field label is descriptive » (→ 11.7) ;
  - `5effbb`, `aizyf1` et `fd3a94` (intitulés de lien descriptifs) ;
  - `7677a9` et `c249d5` (mouvement de l'appareil → 13.12) ;
  - `efbfc7` (contenu en mouvement → 13.8) ;
  - `80af7b`, `ebe86a` et `a1b64e` (piège clavier → 12.9) ;
  - `59br37` (texte agrandi rogné).

  Le préfixe « → » indique une correspondance RGAA **(inférence)**.
- **Attention.** Le rapport axe-core du W3C porte sur axe 4.10.2 et date du 2 novembre 2023 ([axe-core.json](https://github.com/w3c/wcag-act-rules/blob/main/_data/wcag-act-rules/implementations/axe-core.json)). Il ne reflète pas axe 4.14.

## 3. Les règles axe « expérimentales » de la carte : faut-il un autre moteur ?

Source pour axe : `lib/rules/*.json` au tag [v4.14.0](https://github.com/dequelabs/axe-core/tree/v4.14.0/lib/rules).

| Règle visée par la carte | État dans axe 4.14.0 | Équivalent ailleurs | Conclusion |
|---|---|---|---|
| `aria-tab-name` (7.1) | **présente**, non expérimentale, étiquetée `RGAA-7.1.1` ; ajoutée par le commit [0d4e4e70](https://github.com/dequelabs/axe-core/commit/0d4e4e70) (#5001). Absente de v4.11.4, présente dès v4.12.0 | IBM `aria_widget_labelled` (couverture de `tab` non vérifiée) | **Passer à axe ≥ 4.12** suffit |
| `label-content-name-mismatch` | **activée par défaut** ([#5421](https://github.com/dequelabs/axe-core/pull/5421), 2026-10-01), étiquetée `RGAA-6.1.5`. Elle était expérimentale dans 4.9.1 | Alfa R14 ; IBM `label_name_visible` | Passer à axe 4.14 ; Alfa et IBM peuvent servir de second avis |
| `css-orientation-lock` (13.9.1) | encore `experimental` | Alfa R44 et IBM `element_orientation_unlocked` : tous deux « complete » sur l'ACT `b33eff` ; axe 4.10.2 n'y est pas déclaré consistent | Un **second moteur** est utile pour valider avant activation **(inférence)** |
| `focus-order-semantics` (12.8.1) | `experimental`, `best-practice` | aucun équivalent trouvé | aucun apport |
| `hidden-content` | `experimental`, `review-item`, **sans étiquette RGAA** | aucun | aucun apport |
| `p-as-heading` (9.1.3) | `experimental` | IBM `text_block_heading` (Potential) | IBM n'apporte qu'un indice, pas plus fort qu'axe |
| `table-fake-caption` (5.4.1) | `experimental` | QualWeb QW-BP9 (« table sans en-têtes a une caption ») est voisin mais pas identique | aucun apport net |
| `td-has-header` (5.7.4) | `experimental` | Alfa R77 « Data cells are assigned at least one header cell » ; Alfa R46 et QualWeb (ACT `d0f69e`) | second avis possible **(inférence)** |

## 4. Conclusion par test RGAA du périmètre

Ce tableau dit **ce que les moteurs open source ajoutent à axe**. Ce n'est pas le niveau final du test : les sondes maison, les VLM et le sidecar sont traités par d'autres tickets de la carte. Les correspondances moteur → RGAA sont toutes des **inférences** (§0).

| Test RGAA | Apport au-delà d'axe | Moteur | Niveau que les moteurs permettent |
|---|---|---|---|
| 1.3 (pertinence de l'alternative) | nom de fichier utilisé comme alternative (R39), alternative trop longue (QW-BP2, IBM `media_alt_brief`), question structurée `is-image-accessible-name-descriptive` (R117, expérimentale) | Alfa, QualWeb, IBM | **Assisté** |
| 1.7 (description détaillée pertinente) | aucun | — | aucun apport |
| 1.8 (image texte) | aucun : ACT `0va7u6` n'est implémentée par personne | — | aucun apport (pixels/OCR : autre ticket) |
| 3.1 (information par la couleur seule) | `style_color_misuse` (Potential) | IBM | **Assisté** (indice) |
| 4.1 à 4.3, 4.6, 4.9 (pertinence des alternatives média) | applicabilité ACT et questions `has-captions`, `has-description`, `track-describes-video`, `transcript` | Alfa (assisté) | **Assisté** ; la pertinence reste humaine |
| 4.10 (son déclenché automatiquement) | R48, R49 et R50, avec des questions **factuelles** (`has-audio`, `is-above-duration-threshold`, `is-playing`) | Alfa | **Automatique possible** si le sidecar `ffmpeg` répond aux questions factuelles **(inférence, à prototyper)** ; sinon Assisté |
| 5.2 et 5.5 (résumé, titre de tableau pertinents) | aucun | — | aucun apport |
| 5.3 (linéarisation) | détection des tableaux de mise en forme (`table_layout_linearized`) ; Asqatasun ne fait que pré-qualifier | IBM | **Assisté** (ciblage seulement) |
| 7.2 (alternative à un script) | aucun | — | aucun apport |
| 7.5 (messages de statut) | aucun (pas de règle ACT) | — | aucun apport |
| 8.6 (titre de page pertinent) | titre en forme de nom de fichier (`page_title_valid`, Fail), titre > 64 car. (QW-BP6), question `is-title-descriptive` (R114, expérimentale) | IBM, QualWeb, Alfa | **Assisté** |
| 9.4 (citations) | `text_quoted_correctly`, `blockquote_cite_exists` (Potential) | IBM | **Assisté** (indice nouveau par rapport à axe) |
| 10.3 (compréhension sans CSS) | aucun direct | — | aucun apport |
| 10.10 (forme, taille, position) | `text_sensory_misuse`, **liste de mots anglaise** ; ACT `9bd38c` non implémentée | IBM | **Assisté** seulement après traduction de la liste ; en l'état, rien |
| 11.7 et 11.8.3 (étiquettes pertinentes) | aucun ; ACT `cc0f0a` non implémentée | — | aucun apport |
| 11.12 (données financières, juridiques) | aucun | — | aucun apport |
| 12.3 test 1, 13.1 test 4 | aucun | — | aucun apport |
| 13.6 (contenus cryptiques) | `asciiart_alt_exists`, `emoticons_alt_exists`, QW-BP7 | IBM, QualWeb | **Assisté** (heuristique existante, sinon la sonde de la spec) |
| 13.10 à 13.12 (gestes, annulation, mouvement) | aucune règle fiable (ACT `7677a9` et `c249d5` non implémentées) | — | aucun apport |

**Réponse honnête.** Pour la plupart des tests du §9, aucune bibliothèque open source compatible MIT ne fait mieux qu'axe, plus nos sondes et un modèle. Les moteurs complémentaires apportent du **ciblage** et des **questions bien formées**, pas des verdicts.

## 5. À vérifier sous Obscura (aucun point testé ici)

1. **Alfa `Native.fromNode`** : itère `document.styleSheets[*].cssRules` et appelle `getBoundingClientRect` et `Range.getBoundingClientRect`. Il faut vérifier que le CSSOM d'Obscura 0.2.2 expose `cssRules`, y compris pour les feuilles cross-origin (option `enforceAnonymousCrossOrigin`). Il faut aussi vérifier que les boîtes ne sont pas nulles. Obscura annonce un rendu natif depuis 0.2.0 ([obscura-delta-v020-v022.md](../../research/obscura-delta-v020-v022.md)).
2. **Taille et exécution d'un bundle Alfa dans la page.** Les packages Alfa sont des modules npm séparés ; aucun bundle navigateur officiel n'est vérifié. Deux options : soit sérialiser dans la page et auditer dans un processus Node isolé (JS en dernier recours, conforme à la carte), soit tout bundler et l'exécuter dans le V8 d'Obscura. Les deux sont **non vérifiées**.
3. **IBM `ace.js`** : chargement par `Runtime.evaluate`, temps d'exécution et dépendance aux styles calculés (règles de visibilité et de contraste). Les deux sont **non vérifiés**.
4. **QualWeb** : il suppose Puppeteer ou Playwright. Fonctionnement en `puppeteer.connect` sur le CDP d'Obscura : **non vérifié**. L'alternative est d'injecter ses bundles nous-mêmes.
5. **Questions média d'Alfa** : Obscura ne lit pas les médias (fait établi dans la carte #330). Les réponses `has-audio`, `is-above-duration-threshold` et `is-playing` doivent donc venir du sidecar `ffmpeg`, ou de l'état DOM `autoplay`/`muted`. Le branchement d'un oracle hors navigateur est **à prototyper**.
6. **Langue** : aucune règle textuelle d'IBM ni de QualWeb n'a été vérifiée sur du contenu français.

## 6. Ce qui reste inconnu

- Le nombre exact de tests RGAA 4.1.2 décidés par Asqatasun 6.0 : la documentation par règle répond 404.
- Si `aria_widget_labelled` d'IBM couvre `role="tab"` : il faut une fixture.
- Les performances (temps par page) d'Alfa et d'IBM sous Obscura.
- La conformité ACT d'axe 4.14 : le dernier rapport W3C date d'axe 4.10.2.
