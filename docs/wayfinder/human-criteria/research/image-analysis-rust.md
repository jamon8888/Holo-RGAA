# Recherche : analyse d'image en Rust pour les tests visuels (1.8, 3.1, 3.2, 3.3, 10.7, 10.10)

Ticket : [#335](https://github.com/jamon8888/Holo-RGAA/issues/335) — carte [#330](https://github.com/jamon8888/Holo-RGAA/issues/330).
Date de collecte : 2026-10-12. Vocabulaire : `CONTEXT.md` (Test RGAA, Niveau d'automatisation, Automatique / Assisté / Irréductible).

Conventions de ce document :

- **[source]** : fait établi par la source citée (URL à côté).
- **[inférence]** : raisonnement de l'auteur à partir des sources ; à confirmer.
- **[inconnu]** / **non vérifié** : pas pu établir ; à vérifier, en général par fixture sous Obscura.

Contrainte de cadrage reprise de #330 : **une capture par page, réutilisée** par la preuve et par toutes les analyses de pixels. Faits Obscura déjà établis (fournis par la carte) : `Page.captureScreenshot` et `Emulation.setDeviceMetricsOverride` fonctionnent ; `getComputedStyle().direction` est vide ; pas de navigation Tab native (un `keydown` est livré mais ne déplace pas le focus).

## 1. Réponse courte

| Test RGAA | Technique sur la capture | Bibliothèque (licence) | Niveau atteignable |
|---|---|---|---|
| 1.8.1 – 1.8.6 image texte | Recadrer chaque `img` / `input[type=image]` / `object` / `embed` / `canvas` / `svg` sans `<text>` dans la capture, **détecter** du texte (pas besoin de le transcrire) | `ocrs` 0.13.1 (MIT OR Apache-2.0, pur Rust) + poids CC-BY-SA-4.0 ; repli : `tesseract` 5.x en CLI (Apache-2.0) | **Assisté** (le « porteuse d'information », le « si possible » et les cas particuliers restent un jugement) |
| 3.1.1, 3.1.3, 3.1.4, 3.1.6 info par la couleur seule | Simulation de daltonisme (Viénot 1999 / Brettel 1997 / Machado 2009) de la capture + recherche de paires distinguables seulement par la teinte ; preuve = capture simulée côte à côte | Matrices publiées, implémentation maison (~quelques dizaines de lignes) sur `image` + `palette` ; aucune crate de référence trouvée | **Assisté** (indices + preuve) ; décider qu'une couleur « véhicule une information » n'est pas mesurable par pixels |
| 3.1.2 indication de couleur donnée par un texte | Lexique de couleurs dans le texte du DOM (pas de pixels) | — | **Assisté** |
| 3.1.5 média temporel | Hors capture (sidecar média, ticket thème 4) | — | **Irréductible** pour l'analyse d'image de page |
| 3.2.1 – 3.2.4 contraste texte (résidus axe) | Pour les cas `incomplete` d'axe `color-contrast` (image ou dégradé de fond, chevauchement, pseudo-élément…) et le texte en image : recadrer la boîte du texte, séparer pixels texte / fond, ratio WCAG | `image`, `imageproc` (MIT), `palette::Wcag21RelativeContrast` (MIT OR Apache-2.0) ; `ocrs` pour localiser le texte en image | **Assisté** (axe reste Automatique sur les cas qu'il conclut) |
| 3.2.5 / 3.3.4 mécanisme de contraste | Repérer et actionner le mécanisme n'est pas de l'analyse d'image | — | **Irréductible** ici (inventaire seulement) |
| 3.3.1 composant d'interface | Anneau de pixels intérieur/extérieur autour de la boîte DOM, couleurs dominantes, ratio 3:1 ; états via captures supplémentaires | `image`, `imageproc`, `palette` | **Assisté** (Automatique `fail` envisageable seulement pour une sous-classe à couleurs CSS unies — relève du computed style, pas des pixels) |
| 3.3.2 / 3.3.3 éléments graphiques | Étiquetage des régions de couleur (composantes connexes), ratio 3:1 entre régions contiguës | `imageproc::region_labelling` | **Assisté** (« nécessaires à sa compréhension » est un jugement) |
| 10.7.1 focus visible | Règle ACT `oj04fd` : comparer les pixels de la zone de l'élément avant / après `focus()` ; aucun pixel modifié ⇒ `fail` | `image` (diff pixel à pixel), `palette` pour un score type 2.4.13 | **Automatique pour `fail`** (si la fidélité de rendu d'Obscura est validée par fixture), **Assisté pour `pass`** |
| 10.10.1 texte | Lexique de références sensorielles (« à droite », « ci-dessous », « bouton rond »…) dans le DOM | — | **Assisté** |
| 10.10.2 – 10.10.4 images, médias | Aucune technique d'analyse d'image ne décide qu'une information passe « uniquement » par la forme, la taille ou la position | — | **Irréductible** au titre de l'analyse d'image (un VLM local pourrait proposer un indice ⇒ Assisté, hors périmètre de ce ticket) |

En une phrase : l'outillage Rust natif (`image` + `imageproc` + `palette` + `ocrs`) suffit pour **mesurer** (texte présent, ratios de contraste, pixels modifiés au focus, simulation de daltonisme) ; il ne suffit pour **conclure seul** qu'au 10.7 en `fail`, tout le reste reste Assisté ou Irréductible parce que les tests exigent un jugement de sens.

## 2. Briques disponibles (vérifiées sur crates.io / dépôts)

Métadonnées lues via l'API crates.io (`https://crates.io/api/v1/crates/<nom>`) le 2026-10-12.

| Crate | Version | Licence | Rôle | Remarques |
|---|---|---|---|---|
| [`image`](https://crates.io/crates/image) | 0.25.10 | MIT OR Apache-2.0 | décodage PNG, tampons de pixels, recadrage | Déjà présent en dépendance transitive dans `rgaa-rs/Cargo.lock` (0.25.10) [source : `Cargo.lock` de `origin/master`]. MSRV 1.88.0. |
| [`imageproc`](https://crates.io/crates/imageproc) | 0.27.0 | MIT | composantes connexes, contours, bords, morphologie, statistiques, transformée de distance, template matching | Modules listés sur [docs.rs/imageproc/0.27.0](https://docs.rs/imageproc/0.27.0/imageproc/) : `region_labelling`, `contours`, `edges`, `stats`, `morphology`, `distance_transform`, `template_matching`… Pas de k-means / quantification de couleur [source, même page]. MSRV 1.87.0. |
| [`palette`](https://crates.io/crates/palette) | 0.7.7 | MIT OR Apache-2.0 | espaces colorimétriques, sRGB ↔ linéaire, ratio WCAG | Trait [`Wcag21RelativeContrast`](https://docs.rs/palette/0.7.7/palette/color_difference/trait.Wcag21RelativeContrast.html) : `relative_luminance`, `relative_contrast`, `has_min_contrast_text` (4.5:1), `has_min_contrast_large_text` (3:1), `has_min_contrast_graphics` (3:1, SC 1.4.11) [source]. |
| [`ocrs`](https://crates.io/crates/ocrs) | 0.13.1 (publiée le 2026-09-13) | MIT OR Apache-2.0 | détection + reconnaissance de texte | Pur Rust : dépendances `rten`, `rten-imageproc`, `rten-tensor`, `rayon`, `anyhow`, `thiserror`, `wasm-bindgen` [source : API crates.io, dépendances de 0.13.1]. Pas de dépendance C. |
| [`rten`](https://crates.io/crates/rten) | 0.27.0 (ocrs 0.13.1 demande `^0.26.0`) | MIT OR Apache-2.0 | moteur d'inférence ONNX/RTen | MSRV 1.94.0 ; le toolchain du dépôt est épinglé à 1.98.1 (`rgaa-rs/rust-toolchain.toml`). |
| [`image-compare`](https://crates.io/crates/image-compare) | 0.5.0 | MIT | SSIM, RMS | Optionnel : un simple diff pixel à pixel suffit pour 10.7. |
| [`dssim-core`](https://crates.io/crates/dssim-core) | 3.5.1 | **AGPL-3.0** | SSIM multi-échelle | **Exclu** (AGPL non admissible hors service séparé). |
| [`tesseract`](https://crates.io/crates/tesseract) | 0.15.2 | MIT | liaison haut niveau | Lie `libtesseract` (C++). |
| [`leptess`](https://crates.io/crates/leptess) | 0.14.0 (dernière mise à jour 2023-02-21) | MIT | liaison Tesseract + Leptonica | Peu maintenue d'après la date crates.io. |
| [`tesseract-plumbing`](https://crates.io/crates/tesseract-plumbing) | 0.11.1 | MIT | enveloppe sûre de `tesseract-sys` | — |

Bibliothèques natives :

- **Tesseract** : licence Apache-2.0 ([API GitHub `tesseract-ocr/tesseract`](https://github.com/tesseract-ocr/tesseract), `spdx_id: Apache-2.0`) ; dernière release 5.5.3 publiée le 2026-07-24 ([releases](https://github.com/tesseract-ocr/tesseract/releases)). Modèles `tessdata_fast` : Apache-2.0 ([dépôt](https://github.com/tesseract-ocr/tessdata_fast)). Sur cette machine : `tesseract 5.5.0` avec `leptonica-1.86.0`, langues installées `eng` et `osd` seulement (`tesseract --list-langs`) ; la disponibilité du paquet `tesseract-ocr-fra` n'a **pas été vérifiée**.
- **Leptonica** (dépendance de Tesseract) : licence de type BSD à 2 clauses ([leptonica-license.txt](https://github.com/DanBloomberg/leptonica/blob/master/leptonica-license.txt)).
- La CLI produit du TSV avec une colonne `conf` par mot (`tesseract image - -l eng tsv`) et accepte `--psm 11` (« sparse text ») [source : [Command-Line-Usage](https://tesseract-ocr.github.io/tessdoc/Command-Line-Usage.html)].

Compatibilité avec une distribution sous MIT :

- `image`, `imageproc`, `palette`, `ocrs`, `rten`, `image-compare`, `tesseract*`, `leptess` : MIT ou MIT/Apache-2.0 → compatibles [source : licences crates.io].
- Tesseract (Apache-2.0) et Leptonica (BSD-2) en sidecar ou liés : compatibles ; Apache-2.0 impose de conserver les avis de licence / NOTICE lors de la redistribution des binaires [inférence à partir des termes usuels d'Apache-2.0 ; pas un avis juridique].
- **Poids des modèles `ocrs`** : publiés sur Hugging Face sous **CC-BY-SA-4.0** ([robertknight/ocrs](https://huggingface.co/robertknight/ocrs), `license: cc-by-sa-4.0`) ; entraînés sur HierText (CC-BY-SA 4.0) [source : [README ocrs-models](https://github.com/robertknight/ocrs-models)]. Le dépôt `ocrs-models` n'expose pas de licence détectée par GitHub (API licence : 404) [source]. [inférence] Télécharger les poids à l'exécution (comme le fait la CLI `ocrs`, cache `~/.cache/ocrs`) ne contamine pas le code MIT ; les **embarquer** dans une release imposerait l'attribution et le partage à l'identique **sur les poids** — à faire valider avant d'embarquer.
- `dssim-core` : AGPL-3.0 → exclu.

## 3. Par test

### 3.1 Critère 1.8 — image texte (tests 1.8.1 à 1.8.6)

Ce que le test demande : chaque image texte porteuse d'information, sans mécanisme de remplacement, doit **si possible** être remplacée par du texte stylé (hors cas particuliers) [source : `rgaa-rs/crates/rgaa-core/data/rgaa-4.1.2/criteres.json`].

Technique proposée [inférence] :

1. Inventaire DOM des candidats (le DOM le donne déjà) : `img` / `[role=img]` (1.8.1), `input[type=image]` (1.8.2), `object[type^=image/]` (1.8.3), `embed[type^=image/]` (1.8.4), `canvas` (1.8.5), `svg` dont le texte n'est pas entièrement en `<text>` (1.8.6 — en partie décidable par le DOM seul).
2. Recadrer chaque boîte dans **la** capture de page (pleine page, pour couvrir les éléments hors viewport).
3. Lancer la **détection** de mots d'`ocrs` (`OcrEngine::detect_words`, boîtes orientées) ; la reconnaissance (`recognize_text`) ne sert qu'à afficher le texte lu dans la preuve [source : [docs.rs/ocrs OcrEngine](https://docs.rs/ocrs/latest/ocrs/struct.OcrEngine.html)].
4. Statut `review` avec preuve (recadrage + boîtes de texte + transcription) quand du texte est détecté ; pas de conclusion quand rien n'est détecté (un faux négatif sur du texte stylisé est plausible).

`ocrs` vs `tesseract` :

- `ocrs` : pur Rust, aucune dépendance C, modèle téléchargé à la première exécution ; projet en « early preview », « Expect more errors than commercial OCR engines », alphabet latin seulement [source : [README ocrs](https://github.com/robertknight/ocrs)]. **L'alphabet par défaut est ASCII** (`DEFAULT_ALPHABET` dans [`ocrs/src/lib.rs`](https://github.com/robertknight/ocrs/blob/main/ocrs/src/lib.rs) : chiffres, ponctuation, `A-Z`, `a-z`) : pas de `é`, `à`, `ç` [source]. [inférence] Gênant pour transcrire du français, pas pour **détecter** la présence de texte, qui est ce dont 1.8 a besoin. Pas de score de confiance par caractère documenté [source : docs.rs, non trouvé].
- `tesseract` : mûr, multilingue (modèle `fra`), sortie TSV avec confiance par mot ; mais conçu pour des documents : « works best on images which have a DPI of at least 300 dpi », binarisation fragile sur fonds inégaux, mode `--psm 11` pour le texte épars [source : [ImproveQuality](https://github.com/tesseract-ocr/tessdoc/blob/main/ImproveQuality.md)]. Les bannières web (texte sur photo, faible résolution) sont précisément le cas défavorable [inférence].
- Recommandation [inférence] : `ocrs` en natif pour la détection (préférence Rust natif de la carte), `tesseract` CLI en sidecar optionnel quand on veut une transcription française fiable dans la preuve. Préférer la CLI (TSV sur stdout) aux liaisons FFI, pour éviter une dépendance de compilation (bindgen/clang + `libtesseract-dev`) — même logique que le sidecar `ffmpeg` validé.

Fiabilité / coût : **non mesurés**. Aucun chiffre de précision ni de performance n'est publié dans le README `ocrs` [source]. À mesurer sur fixtures (voir §4).

Niveau : **Assisté**. Justification : « porteuse d'information », « si possible », « mécanisme de remplacement » et les cas particuliers (logos, etc.) sont des jugements ; la machine fournit l'indice (texte détecté) et la preuve.

### 3.2 Critère 3.1 — information donnée uniquement par la couleur

- La technique WCAG associée se décline en échecs ciblés : F81 (champs obligatoires / en erreur signalés uniquement par la couleur), F73 (liens non visibles sans la couleur), F13 ; techniques G14, G182, G183 [source : [Understanding 1.4.1](https://www.w3.org/WAI/WCAG22/Understanding/use-of-color.html)].
- Simulation de daltonisme, algorithmes publiés :
  - Machado, Oliveira, Fernandes, « A Physiologically-based Model for Simulation of Color Vision Deficiency », IEEE TVCG 15(6), 2009 — matrices publiées pour protanomalie, deutéranomalie, tritanomalie, sévérités 0.0 à 1.0 par pas de 0.1 [source : [page des auteurs](https://www.inf.ufrgs.br/~oliveira/pubs_files/CVD_Simulation/CVD_Simulation.html)]. Aucune licence d'usage n'y est indiquée [source] ; les coefficients sont des données d'un article publié [inférence : réutilisation usuelle avec citation].
  - Comparatif DaltonLens : Brettel 1997, Viénot 1999 et Machado 2009 sont valables pour protanopie/deutéranopie ; **seul Brettel 1997** convient à la tritanopie ; toutes ces méthodes supposent du **RGB linéaire** (décodage sRGB obligatoire) [source : [DaltonLens, Review of open source CVD simulation](https://daltonlens.org/opensource-cvd-simulation/) — source secondaire, mais qui renvoie aux articles].
  - Crates Rust : la recherche crates.io « color blindness simulation » ne remonte que des crates marginales (ex. `color_blinder` 1.0.2, 58 téléchargements récents ; `bevy_color_blindness` 0.2.0) [source : API de recherche crates.io, 2026-10-12]. [inférence] Implémenter Viénot/Brettel directement (matrices 3×3 en RGB linéaire, `palette` pour la conversion sRGB ↔ linéaire) coûte moins que dépendre de ces crates.
- Technique proposée [inférence] : simuler protanopie / deutéranopie (Viénot) et tritanopie (Brettel) sur la capture ; pour les éléments candidats issus du DOM (liens dans le texte, champs `aria-invalid` / `required`, légendes de graphiques, pastilles d'état), mesurer l'écart de couleur (ΔE dans `palette`) entre éléments avant et après simulation ; un écart qui s'effondre après simulation sans autre indice DOM (graisse, soulignement, icône, texte) est un indice de `review`. La capture simulée côte à côte est la preuve montrée à l'auditeur.
- 3.1.2 (indication de couleur donnée par un texte, ex. « cliquez sur le bouton vert ») : détection lexicale dans le texte du DOM — pas d'analyse d'image.
- 3.1.5 (média temporel) : hors de la capture de page, relève du sidecar média (thème 4).

Niveau : **Assisté** pour 3.1.1, 3.1.2, 3.1.3, 3.1.4, 3.1.6 ; **Irréductible** au titre de l'analyse d'image pour 3.1.5. Justification : la question « la couleur véhicule-t-elle une information ? » n'est pas une mesure de pixels ; la simulation produit un indice et une preuve, pas un verdict. Les cas déterministes (lien en texte non souligné et de contraste < 3:1 avec le texte voisin, G183) relèvent d'axe `link-in-text-block`, qu'axe rattache à RGAA 10.6.1 et non à 3.1 [source : [rule-descriptions axe-core](https://github.com/dequelabs/axe-core/blob/develop/doc/rule-descriptions.md)].

### 3.3 Critère 3.2 — contraste du texte (résidus)

- axe `color-contrast` est déjà le mécanisme Automatique (`mechanisms.toml`, `coverage = "complete"`). Il renvoie `incomplete` dans des cas précisément visuels : `bgImage`, `bgGradient`, `imgNode`, `bgOverlap`, `elmPartiallyObscured`, `pseudoContent`, `fgAlpha`, `complexTextShadows`, `outsideViewport` [source : [`lib/checks/color/color-contrast.json`](https://github.com/dequelabs/axe-core/blob/develop/lib/checks/color/color-contrast.json)].
- Technique proposée [inférence] : pour chaque nœud `incomplete`, recadrer sa boîte dans la capture, séparer pixels de texte et de fond (masque de `ocrs::detect_text_pixels`, ou binarisation + morphologie `imageproc`), prendre la couleur de texte dominante et le **pire** fond adjacent (percentile défavorable plutôt que moyenne), calculer le ratio avec `palette::Wcag21RelativeContrast`. Même chaîne pour le « texte en image » (3.2.1–3.2.4 le couvrent explicitement) une fois le texte localisé par `ocrs`.
- Limite publiée : l'anticrénelage rend des pixels plus pâles que la couleur CSS (« Due to anti-aliasing… ») [source : [Understanding 1.4.11](https://www.w3.org/WAI/WCAG22/Understanding/non-text-contrast.html)] — d'où un biais vers des faux `fail` sur les petits textes [inférence].
- 3.2.5 (contraste dans le mécanisme de remplacement) : trouver et actionner le mécanisme n'est pas de l'analyse d'image.

Niveau : **Assisté** pour les résidus pixels de 3.2.1–3.2.4 (tant que non calibré, cf. carte #183) ; **Irréductible** ici pour 3.2.5.

### 3.4 Critère 3.3 — contraste des composants d'interface et éléments graphiques

- Exigence de référence : 3:1 contre les couleurs adjacentes ; exceptions : composants inactifs, apparence déterminée par l'agent utilisateur et non modifiée, présentation essentielle ; « computed values should not be rounded » ; techniques G174, G195, C40, G207, G209, échec F78 [source : [Understanding 1.4.11](https://www.w3.org/WAI/WCAG22/Understanding/non-text-contrast.html)].
- axe-core n'a pas de règle de contraste non textuel : seules `color-contrast` / `color-contrast-enhanced` concernent le contraste, et elles visent le texte (wcag143 / wcag146) [source : rule-descriptions axe-core ci-dessus]. Le dépôt a déjà `probe-3-3-graphic-contrast` (`js-static`, `outcomes = ["review"]`) [source : `mechanisms.toml`].
- Technique proposée [inférence] :
  - 3.3.1 : pour chaque contrôle (boîte DOM), comparer un anneau de pixels juste à l'intérieur de la limite visuelle et un anneau juste à l'extérieur (couleur d'arrière-plan contiguë) ; ratio 3:1 via `has_min_contrast_graphics`. Les états (focus, coché, survol) demandent des captures supplémentaires — en tension avec « une capture par page » (voir 10.7). Exceptions : `disabled` lisible dans le DOM ; « apparence non modifiée par l'auteur » difficile à établir [inconnu].
  - 3.3.2 / 3.3.3 : sur les `svg`, `canvas`, `img` de graphiques, étiqueter les régions de couleur (`imageproc::region_labelling`), puis calculer le ratio pour chaque paire de régions contiguës ; signaler les paires < 3:1 ; la preuve est la carte des paires.
  - 3.3.4 : mécanisme — hors analyse d'image.
- Niveau : **Assisté** pour 3.3.1–3.3.3 (« nécessaires à sa compréhension », « porteurs d'informations » et les exceptions sont des jugements) ; un `fail` **Automatique** n'est envisageable que pour une sous-classe étroite (contrôle actif, bordure et fond en couleurs CSS unies, sans image) — et il se calcule alors mieux par le style calculé que par les pixels [inférence]. **Irréductible** ici pour 3.3.4.

### 3.5 Critère 10.7 — prise de focus visible (test 10.7.1)

- Technique publiée : règle ACT [`oj04fd` « Element in sequential focus order has visible focus »](https://www.w3.org/WAI/standards-guidelines/act/rules/oj04fd/) : l'attente est qu'il existe « at least one device pixel inside the scrolling area of the viewport whose HSL color value is different when the element is focused from when it is not » ; la page liste des implémentations automatiques (Alfa, QualWeb, @accesslint/core) et semi-automatiques [source]. La règle note elle-même que ce seuil (un pixel) laisse passer des changements à peine perceptibles [source].
- Un score plus exigeant est publié par SC 2.4.13 Focus Appearance (AAA, WCAG 2.2) : aire au moins égale à un périmètre de 2 px CSS et contraste ≥ 3:1 « between the same pixels in the focused and unfocused states » [source : [Understanding 2.4.13](https://www.w3.org/WAI/WCAG22/Understanding/focus-appearance.html)]. RGAA 10.7 n'exige pas ce niveau ; il sert d'indice de confiance pour un `pass` proposé [inférence].
- axe-core n'a pas de règle de focus visible (aucune règle `focus-visible` dans rule-descriptions) [source ci-dessus]. Le dépôt a déjà `probe-10-7` (`js-static`, analyse des règles CSS `:focus` qui suppriment `outline`) [source : `plan_snippets.rs`, `mechanisms.toml`].
- Technique proposée [inférence] : pour chaque élément focalisable, (a) prendre le « avant » dans la capture de page (recadrage boîte + marge pour `outline-offset` / `box-shadow`), (b) `element.focus()` en JS (pas de Tab sous Obscura), vérifier `document.activeElement`, (c) capture **découpée** (`clip`) de la même zone, (d) diff pixel à pixel (`image`), puis aire et contraste des pixels modifiés (`palette`). Zéro pixel modifié ⇒ `fail` ; pixels modifiés ⇒ `review` avec avant/après et score 2.4.13 en indice.
- Tension avec le cadrage : la décision « une capture par page » ne suffit pas au 10.7 — il faut au minimum **une capture découpée par élément focalisable** (l'« avant » peut venir de la capture de page). Coût en temps **non mesuré**. À arbitrer sur la carte : exception explicite pour 10.7 (et pour les états de 3.3.1), ou échantillonnage.
- Risques propres à Obscura [inconnu] : (1) Obscura peint-il `outline`, `box-shadow`, et l'anneau de focus **natif** de l'agent utilisateur ? Si l'anneau natif n'est pas peint, un site qui garde le focus natif (conforme selon 10.7.1, première condition) sortirait en faux `fail`. (2) `:focus-visible` correspond-il après un `focus()` programmatique ? (3) Les transitions CSS sur le focus : faut-il attendre une frame / la fin de transition avant la capture ?

Niveau : **Automatique pour `fail`** (aucun pixel ne change, élément réellement focalisé, non masqué) **sous réserve** de validation par fixtures sous Obscura ; **Assisté** pour `pass` (« visible » reste un jugement au-delà d'un pixel modifié).

### 3.6 Critère 10.10 — information par la forme, la taille ou la position

- 10.10.1 (textes) : indice lexical dans le texte du DOM (références sensorielles : « ci-dessous », « à droite », « le bouton rond », « en rouge » — ce dernier relevant de 3.1.2) [inférence]. Pas d'analyse d'image.
- 10.10.2 (images), 10.10.3 (médias temporels), 10.10.4 (médias non temporels) : aucune bibliothèque ni technique publiée trouvée qui décide, à partir de pixels, qu'une information n'est transmise **que** par la forme, la taille ou la position [inconnu — non trouvé, pas démontré inexistant]. Un VLM local pourrait proposer un indice (ce serait Assisté), hors du périmètre de ce ticket (analyse d'image).

Niveau : **Assisté** pour 10.10.1 (indice lexical) ; **Irréductible** au titre de l'analyse d'image pour 10.10.2–10.10.4, justification : le test porte sur le sens, pas sur une propriété mesurable des pixels.

## 4. À vérifier par fixture sous Obscura

1. **Fidélité de capture** : la capture pleine page d'Obscura (rendu natif Rust, v0.2.x) restitue-t-elle couleurs, dégradés, images de fond, `box-shadow`, anticrénelage, polices web de façon comparable à un navigateur de référence ? Sans cela, aucun ratio mesuré sur pixels n'est opposable.
2. **10.7** : (a) `outline` auteur peint ; (b) anneau de focus **natif** peint ou non ; (c) `:focus` / `:focus-visible` après `element.focus()` ; (d) `document.activeElement` à jour ; (e) capture `clip` cohérente avec la capture pleine page (même DPR, même décalage de défilement) ; (f) temps par élément.
3. **Couple de fixtures 10.7** : `outline: none` sans remplacement (attendu `fail`) / focus auteur visible (attendu `review` avec score) / focus natif conservé (attendu : pas de `fail`) — le troisième tranche le risque de faux `fail`.
4. **1.8** : taux de détection `ocrs` sur bannières web françaises (texte sur photo, petites tailles, accents) vs images sans texte ; faux positifs sur photos contenant du texte incident (enseignes) ; temps CPU par image ; taille des poids téléchargés. Comparer avec `tesseract --psm 11` (paquet `fra` à installer).
5. **3.2 / 3.3** : écart entre ratio mesuré sur pixels et ratio calculé par axe sur les cas que axe conclut (fixture de calibration) ; effet de `setDeviceMetricsOverride` (`deviceScaleFactor` 1 vs 2) sur l'anticrénelage.
6. **3.1** : rendu des matrices Viénot/Brettel sur un jeu de couleurs de référence (comparer avec DaltonLens), avant de s'en servir comme preuve.

## 5. Ce qui reste inconnu

- Précision et coût de `ocrs` et de `tesseract` sur des captures web : **non mesurés** (aucun chiffre publié côté `ocrs`).
- Rendu du focus et fidélité colorimétrique d'Obscura : **non vérifiés**.
- Statut (proposée / approuvée) de la règle ACT `oj04fd` : **non vérifié**.
- Disponibilité du paquet Ubuntu `tesseract-ocr-fra` sur la cible d'installation : **non vérifiée**.
- Statut juridique précis de l'embarquement des poids CC-BY-SA-4.0 dans une release MIT : à faire valider.
