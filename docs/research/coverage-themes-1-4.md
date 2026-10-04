# Faisabilité des contrôles RGAA 4.1.2 — thèmes 1 à 4 (images, cadres, couleurs, multimédia)

Ticket : jamon8888/Holo-RGAA#248 (carte : #247). Recherche uniquement, aucun code de production.
Base : `master` à `93dbd3f`. Date : 2026-10-04.

## 1. Méthode et sources

Sources primaires uniquement, toutes dans le dépôt :

- Tests officiels : `rgaa-rs/crates/rgaa-core/data/rgaa-4.1.2/criteres.json` (libellés RGAA 4.1.2).
- Mapping axe : `.../axe_mapping.json` (champ `coverage` `complete` / `partial`) et `.../axe_rules.json` (règles axe 4.13 et leurs étiquettes `RGAA-x.y.z`).
- Sondes JS : `rgaa-rs/crates/rgaa-rules/src/gap_fix.rs` (`GapFixRules::snippets()`, `COMPLETE_COVERAGE`).
- Linter source : `rgaa-rs/crates/rgaa-linter/src/rules.rs`.
- Capacités navigateur : `rgaa-rs/crates/rgaa-browser-tools/src/session.rs` (`eval_js`, `press_key`, `get_tab_order`, `get_a11y_tree`, `screenshot`, `run_gap_fix_batch`).
- Classification interne `automatable_criteres.json`, `rgaa-agent/src/criteria_defs.rs`.

Je n'ai exécuté aucun audit : les colonnes « risque » et « effort » sont des estimations d'ingénierie, pas des mesures. Les seuils chiffrés (ex. 80 caractères) sont des conventions de sonde à valider, pas des exigences du RGAA.

Vocabulaire des mécanismes : **axe natif non mappé** ; **axe personnalisé** (`axe.configure`, aucune infrastructure ne l'utilise aujourd'hui dans le dépôt : en pratique équivalent à une sonde JS, avec en plus le format de résultat axe) ; **sonde JS statique** (DOM/CSSOM, contrat gap-fix `{pass, details, nodes}`) ; **sonde comportementale** (Tab, temps, lecture, états forcés) ; **linter** (`rgaa-linter`, source) ; **pixels** (capture + OCR / échantillonnage) ; **humain**.

## 2. Vérification : quels critères listés n'ont vraiment aucun contrôle à HEAD

Constat de l'existant pour les thèmes 1–4 :

- Règles axe mappées dans le thème 1 : 1.1 (6 règles, `complete`), 1.2, 1.5, 1.6 (`image-alt`…), 1.9 (sonde gap-fix). Thème 2 : 2.1. Thème 3 : 3.2, 3.3. Thème 4 : 4.3 (`video-caption`), 4.10 (`no-autoplay-audio`, `partial`).
- Sondes gap-fix dans ces thèmes : 1.1, 1.2, 2.1, 3.2, 1.9. Aucune pour 1.3, 1.4, 1.7, 1.8, 2.2, 3.1, 3.3, 4.x.
- `rgaa-linter` : 4 règles seulement (`img-alt` → 1.1, `form-label`/`button-name` → 11.1, `link-name` → 6.1). Aucune règle des critères étudiés.

| Critère | Entrée dans `axe_mapping.json` | Verdict de vérification |
|---|---|---|
| 1.3, 1.4, 1.7, 3.1, 2.2, 4.2, 4.4, 4.6, 4.9 | absente | **Confirmé : aucun contrôle.** |
| 1.8 | `partial`, `axe_rules: []` (note : règle inexistante `image-text`) | **Confirmé : aucun contrôle** (entrée vide). |
| 4.1, 4.5, 4.7, 4.8, 4.13 | `partial`, `axe_rules: []` (règles inexistantes `audio-description`, `video-description`) | **Confirmé : aucun contrôle.** |
| 4.11, 4.12 | `partial`, `axe_rules: []` (règles inexistantes `keyboard`, `keyboard-trap`) | **Confirmé : aucun contrôle.** |
| 4.10 | `partial`, `no-autoplay-audio` | Couverture partielle confirmée (voir §3). |
| **3.3** | **`complete`, `color-contrast`** | **Écart avec la carte : la carte le dit « sans contrôle », le HEAD le déclare *complet*.** Voir ci-dessous. |

### Constats dérivés (à traiter hors de ce ticket)

1. **3.3 est un faux « complet ».** Le RGAA 3.3 porte sur les composants d'interface et éléments graphiques (3:1), pas sur le texte. `color-contrast` (étiquetée `RGAA-3.2.1` dans `axe_rules.json`) ne mesure que le texte et est déjà mappée sur 3.2. Comme `coverage = complete`, le silence d'axe produit un `Pass` de 3.3 sans qu'aucun composant n'ait été examiné (les valeurs `complete` sont initialisées à `Pass` par `AxeMapper`, voir les tests de `axe_mapper.rs`). Risque de **faux positif de conformité** (faux négatif d'audit). Reclasser 3.3 en `partial` ou `[]` est une correction de donnée, pas de code.
2. **2.2 a une règle axe, mais mappée ailleurs.** `frame-title-unique` porte l'étiquette `RGAA-2.2.1` mais `axe_mapping.json` l'attribue à 2.1. Un doublon de titre fait donc échouer 2.1 au lieu de 2.2.
3. **Entrées « partial » à règles inexistantes** (1.8, 4.1, 4.5, 4.7, 4.8, 4.11–4.13) : les noms `image-text`, `audio-description`, `video-description`, `keyboard`, `keyboard-trap` ne sont pas des règles de `axe_rules.json`. Les entrées sont des espaces réservés sans effet.
4. **`automatable_criteres.json` est trop optimiste** pour 3.1, 4.5, 4.9, 4.11–4.13 (classés « FullyAutomatable ») : les libellés officiels exigent de juger si une information est « porteuse », si un contenu est « pertinent » ou si une alternative est « nécessaire ». `rgaa-agent/src/criteria_defs.rs:683` le reconnaît pour 3.1 (« must SEE the page »). Ne pas s'y fier pour décider du mécanisme.

## 3. Tableau critère × test × mécanisme

Légende effort : **XS** données seules ; **S** sonde JS < 50 lignes ; **M** sonde plus état/interaction ; **L** pixels, états forcés ou gros corpus de faux positifs. Une sonde ne conclut `Pass`/`Fail` que là où indiqué ; sinon elle produit des **preuves de candidats** pour l'humain ou l'agent Holo (statut « à vérifier »), conformément à la règle de #202 : une sonde partielle muette ne vaut pas `Pass`.

### Thème 1 — Images

| Critère | Test(s) RGAA | Décidable de façon déterministe ? | Mécanisme réaliste | Preuve produite | Risque FP / FN | Effort |
|---|---|---|---|---|---|---|
| **1.3** pertinence de l'alternative | 1.3.1–1.3.7 (alt/title/aria-label/aria-labelledby pertinents, par type d'image) | **Non pour la pertinence.** Oui pour des **défauts manifestes** : alt = nom de fichier ou extension, alt générique (« image », « photo », « spacer »), alt = `src`, alt/title identiques à un nom de classe | Sonde JS statique (heuristique « suspect » → à vérifier). Variante : règle `rgaa-linter` sur la source (même heuristique, mais le linter n'a pas le DOM rendu) | Liste de nœuds (sélecteur, valeur du nom accessible, motif détecté) | FP moyen (alt « logo » légitime, « photo » dans un nom propre) ; FN élevé (alt plausible mais faux) : **ne peut jamais valider** | S |
| 1.3 | 1.3.8 contenu alternatif d'un `<canvas>` restitué par les TA | Partiellement : le `<canvas>` a des enfants et un rôle/nom exposé dans l'arbre d'accessibilité | Sonde JS + `get_a11y_tree` | Nœud canvas, enfants, rôle exposé | FP faible ; la « bonne restitution » reste humaine | S |
| 1.3 | 1.3.9 alternative **courte et concise** | Oui, sur un **seuil de longueur** (convention de sonde, ex. > 80 caractères ou > N phrases) | Sonde JS statique | Texte, longueur | FP élevé (alt long légitime pour un graphique, normalement à traiter via 1.7) | XS |
| **1.4** CAPTCHA / image-test | 1.4.1–1.4.7 (même grille que 1.3 pour CAPTCHA) | **Non** pour la nature/fonction. **Oui pour la détection** de ce qui est un CAPTCHA : `iframe[src*=recaptcha/hcaptcha/turnstile]`, id/classe/`src`/`alt` contenant « captcha » | Sonde JS statique de **découverte** + humain pour la décision | Éléments candidats ; sert à passer le critère de « non applicable » (aucun CAPTCHA) à « à vérifier » | FN de la découverte (CAPTCHA maison sans mot-clé) ; la conclusion « NA » doit rester prudente | S |
| **1.7** description détaillée pertinente | 1.7.1–1.7.6 (`longdesc`, passage lié par `aria-describedby`, lien/bouton adjacent, texte adjacent) | **Non** pour la pertinence. **Oui** pour l'**existence et la résolution du lien** : `aria-describedby` pointe vers un id présent et non vide ; `longdesc` joignable (HTTP 200) ; lien adjacent présent | Sonde JS statique (réutilise le parcours `aria-describedby` de la sonde 1.9) + fetch HEAD des URL `longdesc` | Pour chaque image : ids résolus, texte du passage, statut HTTP | FP faible sur « référence cassée » (vrai défaut) ; la pertinence reste humaine/Holo ; `longdesc` est obsolète en HTML5 (mapping 1.6 note déjà `longdesc` invalide) | S–M |
| **1.8** image-texte à remplacer par du texte | 1.8.1–1.8.6 (img, `input type=image`, object, embed, canvas, svg) | **Non** au sens strict (« si possible », « hors cas particuliers » : logos, mécanisme de remplacement). On peut **détecter les candidats** | **Pixels** : capture de l'élément + OCR, seuil de couverture texte ; complément statique : SVG sans `<text>` mais avec des `<path>` nombreux pour le test 1.8.6, `alt` long et multi-mots sur une image large | Capture, texte OCR, ratio, bbox | **FP élevé** (logos, photos de panneaux, infographies, texte légitimement image) ; FN sur texte stylisé | L |

### Thème 2 — Cadres

| Critère | Test(s) | Décidable ? | Mécanisme | Preuve | Risque FP / FN | Effort |
|---|---|---|---|---|---|---|
| **2.2** titre de cadre pertinent | 2.2.1 | **Non** pour la pertinence. **Oui** pour les défauts évidents : titre en doublon (`frame-title-unique`, étiquetée `RGAA-2.2.1`), titre générique (« iframe », « frame », « sans titre »), titre = URL/nom de fichier | **Axe natif déjà calculé** : déplacer `frame-title-unique` de 2.1 vers 2.2 (`partial`) ; compléter par sonde JS statique pour les titres génériques | Nœuds `iframe` en échec, valeurs de titre | FP faible pour les doublons (iframes jumeaux identiques, ex. deux vidéos : à traiter avec prudence) ; FN élevé (titre unique mais faux) | XS (donnée) + S |

### Thème 3 — Couleurs

| Critère | Test(s) | Décidable ? | Mécanisme | Preuve | Risque FP / FN | Effort |
|---|---|---|---|---|---|---|
| **3.1** information par la couleur seule | 3.1.1–3.1.6 | **Non en général** : il faut savoir quelle couleur porte une information. **Oui pour des motifs précis** : champs `required`/`aria-invalid` ou erreurs signalés seulement par `color`/`border-color` (aucun texte, icône, `aria-*` associé) ; textes « le champ en rouge » (3.1.2, recherche de mots-clés de couleur dans le texte visible) | Sonde JS statique (CSSOM + DOM) pour ces deux motifs. Le reste (graphiques, légendes, médias) : **humain / agent visuel**. Note : `link-in-text-block` existe mais est déjà mappée sur 10.6 (étiquette `RGAA-10.6.1`) | Éléments, règles CSS, absence d'indice non chromatique | FP moyen (indice porté par un pseudo-élément, une icône en CSS) ; FN très élevé : **ne couvre qu'une fraction minime** des 6 tests | M |
| **3.3** contraste des composants d'interface et éléments graphiques | 3.3.1 composant/état vs fond ; 3.3.2–3.3.3 éléments graphiques ; 3.3.4 mécanisme alternatif | **Oui en partie** (mesure de ratio 3:1) mais **le périmètre dépend de la sémantique** (quelles couleurs sont « nécessaires à la compréhension ») et du **fond contigu** | Deux voies : (a) sonde JS/CSSOM : bordure/fond/icône des `button`, `input`, `select`, `[role=*]` vs fond hérité ; états `:hover/:focus/:active` via **sonde comportementale** (forcer les pseudo-états par CDP). (b) **Pixels** : échantillonner de part et d'autre du contour de l'élément, ce qui gère fonds image et dégradés mieux que le CSS. 3.3.4 : humain | Couples de couleurs, ratio, état, bbox | FP élevés avec (a) (fond image, opacité, ombre) ; (b) plus robuste mais coûteux ; FN si la bordure est absente et que le fond seul distingue le composant | L |
| 3.3 (prérequis) | — | — | **Corriger le mapping** : `complete` → `partial` (voir §2). Sans cela, tout audit déclare 3.3 conforme faute de mesure | — | Sinon faux « conforme » systématique | XS |

### Thème 4 — Multimédia

Hypothèse commune : un inventaire des médias est nécessaire. Sonde JS statique unique « inventaire média » : `video`, `audio`, `object`, `embed`, `canvas`, `iframe` vers hôtes de lecteurs connus (YouTube, Vimeo, Dailymotion, PeerTube…), avec attributs `controls`, `autoplay`, `muted`, `<track kind>`, et texte adjacent. Les sondes ci-dessous la réutilisent ; son coût (S) est mutualisé. **Limite structurelle** : le contenu d'un `iframe` inter-origine est opaque (pas de pistes, pas de contrôles visibles). Seuls l'URL et les paramètres (ex. `autoplay=1`) sont observables.

| Critère | Test(s) | Décidable ? | Mécanisme | Preuve | Risque FP / FN | Effort |
|---|---|---|---|---|---|---|
| **4.1** transcription / audiodescription | 4.1.1 audio seul, 4.1.2 vidéo seule, 4.1.3 synchronisé | **Non** : « si nécessaire » (le média a-t-il une piste audio utile ? est-il pré-enregistré ?). **Oui** pour détecter une alternative **présente** : lien/bouton adjacent dont le texte évoque « transcription », « audiodescription », `<track kind="descriptions">` | Sonde JS statique (inventaire + recherche d'alternative adjacente) ; absence → « à vérifier », jamais `Fail` automatique | Média, alternative trouvée (texte, href) ou absence | FP élevé si `Fail` direct (vidéo décorative muette, direct) ; FN : alternative présente mais non pertinente | M |
| **4.2** transcription / AD pertinentes | 4.2.1–4.2.3 | **Non** | **Humain** (ou agent visuel après extraction de la transcription) | — | — | — |
| **4.4** sous-titres pertinents | 4.4.1 | **Non** pour la pertinence. Pré-contrôle possible : piste `<track kind="captions\|subtitles">` existante, VTT joignable et non vide/chronologiquement valide | **Humain** pour la décision ; sonde JS statique + fetch VTT en préalable (preuves) | URL VTT, nombre de cues | FP faible sur « VTT vide » | S |
| **4.5** audiodescription synchronisée | 4.5.1 vidéo seule, 4.5.2 synchronisé | **Non** pour la nécessité ; **oui** pour détecter `<track kind="descriptions">`, piste audio alternative, lien « version audiodescrite » | Sonde JS statique (même inventaire) ; limiter à « à vérifier / alternative trouvée » | Pistes, liens | FP élevé en `Fail` direct ; FN pour lecteurs inter-origine | M |
| **4.6** AD pertinente | 4.6.1–4.6.2 | **Non** | **Humain** | — | — | — |
| **4.7** média clairement identifiable | 4.7.1 | **Non** pour la clarté. Oui pour un **pré-filtre** : média sans titre/`aria-label`, sans légende (`figcaption`) ni texte adjacent | Sonde JS statique (inventaire) → candidats ; décision humaine | Média, texte adjacent trouvé | FP moyen (identification par le contexte de page) | S |
| **4.8** alternative à un média non temporel | 4.8.1 lien/bouton adjacent ; 4.8.2 alternative accessible | **Partiel** : détecter les médias non temporels (`object`, `embed`, `canvas` interactif, `applet`) et l'existence d'un lien/bouton adjacent ; l'accessibilité de la page cible = relancer l'audit dessus (hors périmètre) | Sonde JS statique | Média, lien adjacent résolu | FN : alternative dans la page non signalée ; FP : `canvas` décoratif | M |
| **4.9** alternative pertinente | 4.9.1 | **Non** (« même contenu et fonctionnalités similaires ») | **Humain** | — | — | — |
| **4.10** son déclenché automatiquement | 4.10.1 | **Oui, en grande partie**, par observation. Aujourd'hui `no-autoplay-audio` ne couvre que `<audio>/<video>` natifs > 3 s ; il manque `<embed>`, `<object>`, `<bgsound>`, iframes avec `autoplay=1`, et audio déclenché par JS (`new Audio().play()`, Web Audio) | **Sonde comportementale** : injecter avant chargement un crochet sur `HTMLMediaElement.prototype.play` et `AudioContext`, charger, attendre ~4 s, relever tout média non muet, `paused=false`, `volume>0` et vérifier l'existence d'un contrôle (attribut `controls`, bouton pause/muet accessible). Les iframes de lecteurs : analyse d'URL (`autoplay=1`, `mute=0`) | Horodatage, `src`, `muted`, `currentTime`, contrôle trouvé | FP : politique d'autoplay du navigateur sans geste (le navigateur de test doit être lancé en `--autoplay-policy=no-user-gesture-required`, sinon FN massif) ; FN : lecture déclenchée après interaction (hors critère) | M |
| **4.11** média temporel contrôlable au clavier / pointeur | 4.11.1 contrôles présents ; 4.11.2 accessibles ; 4.11.3 activables | **Oui pour les lecteurs HTML5** par comportement : `controls` présent ou boutons personnalisés ; ces boutons sont dans l'ordre de tabulation (`get_tab_order`) ; Espace/Entrée change `paused` (`press_key` + `eval_js`). Opaque pour iframes inter-origine | **Sonde comportementale** (Tab + touche + lecture de l'état) + statique pour la détection de `div onclick` non focalisables | Ordre de tabulation, état avant/après, éléments non focalisables | FP : lecteurs qui n'attachent le focus qu'après interaction ; FN : contrôles natifs du navigateur (shadow DOM) | L |
| **4.12** média non temporel contrôlable au clavier | 4.12.1 accessible ; 4.12.2 activable | **Partiel** : focalisabilité des `canvas/object/embed` interactifs et **absence de piège clavier** (Tab répété, détection d'un focus qui ne progresse plus). Nécessité de la fonctionnalité = jugement | **Sonde comportementale** (séquence de Tab) | Chemin de focus, piège détecté ou non | FN : fonctionnalité souris uniquement non détectable sans modèle d'interaction ; média rare en pratique (priorité basse) | M |
| **4.13** compatibilité avec les technologies d'assistance | 4.13.1 nom/rôle/valeur/états exposés ; 4.13.2 alternative adjacente | **Partiel** : pour chaque contrôle d'un lecteur, nom+rôle dans l'arbre d'accessibilité (`get_a11y_tree`) ; **réutiliser les violations axe déjà calculées** (`button-name`, `aria-*`) en les rattachant à 4.13 lorsque le nœud est dans un conteneur de média | **Axe natif non mappé par portée** (ré-attribution de nœuds au conteneur média ; demande un changement du mapper, pas des données) + sonde JS statique sur l'arbre | Nœud, règle axe, sous-arbre média | FP : contrôle décoratif dans le conteneur ; FN : états dynamiques invisibles à un instantané | M |

## 4. Synthèse par mécanisme (pour la carte #247)

| Mécanisme | Critères |
|---|---|
| Donnée seule (mapping JSON) | **3.3** (rétrograder en `partial`) ; **2.2** (rattacher `frame-title-unique`) ; retirer les noms de règles inexistants de 1.8, 4.1, 4.5, 4.7, 4.8, 4.11–4.13 |
| Axe natif non mappé / ré-attribution | 2.2 (déjà calculé) ; 4.13 (ré-attribution par portée) |
| Axe personnalisé | Aucun cas où il l'emporte sur une sonde JS ; pas d'infrastructure existante |
| Sonde JS statique | 1.3 (défauts manifestes, longueur), 1.4 (découverte), 1.7 (références), 2.2 (titres génériques), 3.1 (motifs couleur seule), inventaire média puis 4.1, 4.4, 4.5, 4.7, 4.8 |
| Sonde comportementale | 4.10 (crochet `play`, attente 4 s), 4.11, 4.12 (Tab, touches), 3.3 (états forcés) |
| `rgaa-linter` | 1.3 seulement (défauts d'`alt` manifestes, dédoublonnage avec la sonde). Les 4 règles actuelles ne couvrent aucun critère des thèmes 1–4 hors 1.1 |
| Pixels | 1.8 (OCR), 3.3 (échantillonnage de contours) |
| Humain (aucun contrôle déterministe utile) | 1.4 (décision), 4.2, 4.6, 4.9, pertinence dans 1.3/1.7/4.4, nature de l'information dans 3.1 |

### Ordre de valeur / effort suggéré

1. Corrections de données (XS) : 3.3 → `partial`, `frame-title-unique` → 2.2, nettoyage des noms de règles fantômes. **Corrige un faux « conforme » existant (3.3).**
2. 4.10 comportemental (M) : seul critère du thème 4 décidable bout en bout, extension d'une règle partielle existante.
3. Inventaire média statique (S) puis 4.1/4.5/4.7/4.8 en mode « à vérifier » (M chacun mais mutualisé).
4. Sondes de défauts manifestes 1.3 / 2.2 / 1.7 (S).
5. 4.11/4.12 comportemental, 4.13 par ré-attribution (M–L).
6. 3.3 mesure réelle, 1.8 OCR, 3.1 (L, rendement faible, FP élevés) en dernier.

### Règle de verdict transversale

Les trois défauts à éviter : (1) conclure `Pass` quand une sonde partielle est muette (déjà interdit par #202 via `covers_whole_criterion`) ; (2) conclure `Fail` quand le critère contient « si nécessaire » ou « hors cas particuliers » sans preuve que la condition de nécessité est remplie ; (3) étendre une sonde en la déclarant `complete`. Toute sonde ajoutée ici doit donc rester **hors** de `COMPLETE_COVERAGE` et produire des preuves « à vérifier » tant que la pertinence n'est pas décidable.
