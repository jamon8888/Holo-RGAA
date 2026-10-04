# Faisabilité des contrôles — thèmes 5 à 9 (tableaux, scripts, éléments obligatoires, structure)

Ticket : #249 (carte parente : #247). Recherche uniquement, aucun code de production.
Base : `master` @ `e7e866f`. Date : 2026-10-04.

## 1. Méthode et sources

Sources primaires relues pour ce document :

- Tests officiels RGAA 4.1.2 : `rgaa-rs/crates/rgaa-core/data/rgaa-4.1.2/criteres.json` (thèmes 5, 7, 8, 9).
- Mapping axe → RGAA : `.../axe_mapping.json` (champs `axe_rules`, `coverage`, `provenance`).
- Catalogue des règles axe embarqué : `.../axe_rules.json` (105 règles, tags `RGAA-x.y.z` fournis par axe).
- Snippets gap-fix : `rgaa-rs/crates/rgaa-rules/src/gap_fix.rs` (clés insérées : 1.1 1.2 2.1 3.2 6.1 8.3 8.5 10.1 10.2 10.14 11.1 11.4 11.5 12.7 1.9).
- Linter source : `rgaa-rs/crates/rgaa-linter/src/rules.rs` (4 règles seulement : `img-alt`→1.1, `form-label`/`button-name`→11.1, `link-name`→6.1).
- Outils comportementaux existants : `rgaa-browser-tools` (`tab_order`, `press_key`, `click`, `type_input`, `eval_js`, `a11y_tree`) et `rgaa-obscura` (`guided`, exécuteur d'étapes guidées ; moteur `obscura scrape --eval` pour le gap-fix).

Contrat gap-fix à respecter (`GapFixRules::parse_results`, `gap_fix.rs`) : un snippet renvoie `{pass, details, nodes}`.
Un `pass:false` produit toujours un `Fail` avec preuve. Un `pass:true` n'est retenu que si le critère figure dans `COMPLETE_COVERAGE` ; sinon il est ignoré (« un mécanisme partiel qui ne trouve rien ne prouve rien »).
Conséquence directe pour ce document : toute sonde à couverture partielle ne peut produire que des échecs, jamais de validation.

Échelle d'effort : **S** (≤ 1 jour, un snippet ou une ligne de mapping), **M** (2-4 jours, détection + corpus de tests), **L** (> 4 jours, infrastructure nouvelle ou comportementale).
Risques : FP = faux positif (échec à tort), FN = faux négatif (défaut manqué).

## 2. Vérification à HEAD : quels critères n'ont réellement aucun contrôle ?

Résultat : les 15 critères « sans contrôle réel » de la carte sont **tous confirmés sans contrôle** à HEAD. Les 6 « couverture partielle » le sont bien (au moins une règle axe réelle).

| Critère | Entrée `axe_mapping.json` | Gap-fix | Linter | Verdict à HEAD |
|---|---|---|---|---|
| 5.1 | `[]` partial (note : règle inexistante `table-header`) | non | non | aucun contrôle |
| 5.2 | absente | non | non | aucun contrôle |
| 5.3 | absente | non | non | aucun contrôle |
| 5.5 | absente | non | non | aucun contrôle |
| 5.8 | `[]` partial (`layout-table` inexistante) | non | non | aucun contrôle |
| 7.2 | absente | non | non | aucun contrôle |
| 7.4 | `[]` partial (`on-focus`, `on-input` inexistantes) | non | non | aucun contrôle |
| 7.5 | absente | non | non | aucun contrôle |
| 8.1 | `[]` partial (`doctype` inexistante) | non | non | aucun contrôle |
| 8.6 | absente | non | non | aucun contrôle |
| 8.7 | `[]` partial (`lang` inexistante) | non | non | aucun contrôle |
| 8.9 | `[]` partial (`layout-table`, `deprecated-element` inexistantes) | non | non | aucun contrôle |
| 8.10 | `[]` partial (`focus-order`, `meaningful-sequence` inexistantes) | non | non | aucun contrôle |
| 9.2 | absente | non | non | aucun contrôle (voir 5.2 ci-dessous : `region` est tagué 9.2 par axe mais mappé sur 9.1) |
| 9.4 | `[]` partial (`blockquote` inexistante) | non | non | aucun contrôle |
| 5.4 | `table-fake-caption`, `table-duplicate-name` partial | non | non | partiel réel |
| 7.1 | 16 règles ARIA, partial | non | non | partiel réel |
| 7.3 | `scrollable-region-focusable`, `server-side-image-map`, `focus-order-semantics`, partial | non | non | partiel réel |
| 8.2 | `html-has-lang`, `html-lang-valid`, `duplicate-id-aria`, partial | non | non | partiel réel (mais voir anomalies) |
| 8.4 | `html-xml-lang-mismatch`, partial | non | non | partiel réel |
| 8.8 | `valid-lang`, partial | non | non | partiel réel |

Précision utile : les entrées `[]` + `partial` datent de la génération du 2026-08-24 ; leur `provenance.notes` indique « Invalid rules found », c'est-à-dire que les règles supposées n'existent pas dans axe. Elles ressemblent à un contrôle mais n'en sont pas.

### Anomalies de mapping découvertes (gain gratuit, effort S, sans code de sonde)

Relevées en croisant `axe_mapping.json` et les tags `RGAA-x.y.z` de `axe_rules.json` :

1. `html-lang-valid` (tag axe `RGAA-8.4.1`) est mappé sur 8.2 (avec `html-has-lang`) et non sur 8.4. 8.4 ne porte que `html-xml-lang-mismatch` (tag axe `RGAA-8.3.1`). À réaffecter : `html-lang-valid` → 8.4.
2. `region` (tag axe `RGAA-9.2.1`) est mappé sur 9.1, 12.1, 12.4, 12.6 mais pas sur 9.2. `landmark-one-main` est dans le même cas. 9.2 reste donc sans aucun contrôle alors qu'axe en fournit.
3. `table-duplicate-name` (tag axe `RGAA-5.2.1`, résumé identique au titre) est mappé sur 5.4 mais l'étiquette axe le rattache à 5.2.
4. `document-title` est mappé sur 13.3 et 13.4 seulement ; 8.5 repose uniquement sur le snippet gap-fix (qui ne décide que la présence).
5. `label-content-name-mismatch` n'est mappé que sur 11.2 ; le test 7.1.3 (« le nom accessible contient l'intitulé visible ») le réclame aussi.
6. Écart de version : le catalogue `axe_rules.json` porte des `help_url` axe 4.13 alors que l'injection fait `AXE_CORE_CDN = .../axe-core/4.9.1/axe.min.js` (`rgaa-obscura/src/lib.rs`). Toute règle « non encore mappée » citée ci-dessous doit être vérifiée comme existante dans la version réellement injectée avant mapping (notamment `duplicate-id`, `duplicate-id-active`, dépréciées dans les versions récentes).

Règles axe natives présentes dans le catalogue mais non mappées sur aucun critère (extraction : 33 règles, dont celles utiles ici) : `aria-allowed-role`, `aria-dialog-name`, `aria-tooltip-name`, `aria-treeitem-name`, `aria-text`, `presentation-role-conflict`, `empty-heading`, `empty-table-header`, `scope-attr-valid`, `tabindex`, `accesskeys`, `landmark-banner-is-top-level`, `landmark-contentinfo-is-top-level`, `landmark-main-is-top-level`, `landmark-complementary-is-top-level`, `landmark-no-duplicate-banner`, `landmark-no-duplicate-contentinfo`, `landmark-no-duplicate-main`, `landmark-unique`, `page-has-heading-one`, `duplicate-id`, `duplicate-id-active`, `summary-name`, `identical-links-same-purpose`.

## 3. Tableau principal — critère × test RGAA × mécanisme

Légende des mécanismes : **AXE-N** règle axe native non mappée ; **AXE-C** règle axe personnalisée ; **JS-S** sonde JS statique (DOM/CSSOM, contrat gap-fix, échec uniquement tant que le critère n'est pas en `COMPLETE_COVERAGE`) ; **COMP** sonde comportementale (interaction, temps, observateurs) ; **LINT** `rgaa-linter` (source) ; **PIX** pixels / capture ; **HUM** aucun mécanisme déterministe réaliste.

### Thème 5 — Tableaux

| Critère | Test RGAA | Décidable déterministe ? | Mécanisme réaliste | Preuve produite | Risque FP / FN | Effort |
|---|---|---|---|---|---|---|
| 5.1 | 1. Tableau complexe : résumé disponible ? | Partiellement. « Complexe » se détecte par heuristique (en-têtes sur plusieurs niveaux : `th[colspan>1]`, `th[rowspan>1]`, `scope=colgroup/rowgroup`, `headers`, plus d'une ligne d'en-tête). Le résumé se détecte par présence (`aria-describedby` résolu, `figcaption`, `details` dans `caption`) ; la note technique du critère reconnaît elle-même que ces méthodes sont mal supportées. | JS-S (candidat « tableau complexe sans résumé ») | selector du tableau, indices de complexité relevés, absence des 3 formes de résumé | FP moyen-élevé (la définition de « complexe » est floue) ; FN moyen. À remonter comme à vérifier, pas comme échec ferme. | S |
| 5.2 | 1. Résumé pertinent ? | Non pour la pertinence. Une seule partie est mécanique : résumé identique au titre. | AXE-N : `table-duplicate-name` (à re-mapper sur 5.2 selon son tag axe `RGAA-5.2.1`) ; le reste HUM | nœud axe + texte dupliqué | FP faible ; FN très élevé (pertinence invérifiable) | S (re-mapping) |
| 5.3 | 1. Tableau de mise en forme : contenu linéarisé compréhensible + `role="presentation"` | Seule la 2ᵉ condition (`role="presentation"`) est décidable, et seulement si l'on sait qu'un tableau est « de mise en forme ». La compréhension du contenu linéarisé est humaine. | JS-S (tableau sans `th`, sans `caption`, sans `headers`/`scope`, sans `thead`, sans rôle → candidat mise en forme sans `role=presentation|none`) ; linéarisation = HUM (PIX éventuel : capture avec CSS tabulaire neutralisé, comparaison visuelle par un humain ou le LLM) | selector, structure relevée | FP moyen (un tableau de données sans `th` est aussi signalé, il échoue alors 5.6) ; FN élevé sur la linéarisation | S (rôle) / L (linéarisation) |
| 5.4 | 1. Titre associé correctement ? | Partiellement. axe couvre déjà le faux titre en cellule fusionnée (`table-fake-caption`) et le doublon. Mécanique supplémentaire : `caption` vide, `aria-labelledby`/`aria-describedby` dont l'`id` ne se résout pas, titre visuel (`strong`/`p`/`h*` immédiatement avant `table`) sans `caption` ni `aria-labelledby`. | Déjà : AXE (partiel). Ajout : JS-S (idref cassé, `caption` vide) ; faux titre extérieur au tableau = heuristique faible | nœud axe ; pour le JS : id cassé ou caption vide | FP faible (idref cassé, caption vide), moyen (titre adjacent) ; FN moyen | S |
| 5.5 | 1. Titre pertinent ? | Non pour la pertinence ; mécanique : titre vide ou générique (« tableau », « table », « sans titre », identique entre tableaux de la page). | JS-S (échec sur titre vide/générique/dupliqué) + HUM pour le reste | texte du `caption`, liste de motifs génériques | FP faible ; FN très élevé | S |
| 5.6 | 1-4. En-têtes de colonnes/lignes déclarés | Déjà couvert (`td-headers-attr`, tag axe 5.7.4 — en réalité 5.6 est mappé `complete` sur cette seule règle) | **hors périmètre #249** mais à relire : le mapping `complete` repose sur une règle de validité d'attribut `headers`, pas sur la présence de `th` ; les 4 tests 5.6 ne sont pas tous décidés | — | — | — |
| 5.8 | 1. Tableau de mise en forme : pas de `summary` non vide, `caption`, `th`, `thead`, `tfoot`, rôles d'en-tête. 2. Pas d'`headers`, `scope`, `axis` sur les `td`. | **Oui**, dès que le tableau est identifiable comme de mise en forme : `role="presentation"`/`none` explicite est un discriminant sûr. Un tableau de mise en forme sans rôle ne se distingue pas fiablement d'un tableau de données mal construit. | JS-S : tableau à `role=presentation|none` contenant `th`, `caption`, `thead`, `tfoot`, `summary` non vide, `[scope]`, `[headers]`, `[axis]`, `[role=rowheader|columnheader]` | selector + liste des éléments interdits trouvés | FP faible ; FN élevé (tableaux de mise en forme sans rôle) | S |

### Thème 7 — Scripts

| Critère | Test RGAA | Décidable déterministe ? | Mécanisme réaliste | Preuve produite | Risque FP / FN | Effort |
|---|---|---|---|---|---|---|
| 7.1 | 1-2. Nom/rôle/valeur/états exposés ; rendu correct par les AT | Partiellement : validité ARIA déjà couverte (16 règles). Non encore mappées, directement pertinentes : `aria-allowed-role`, `aria-dialog-name`, `aria-treeitem-name`, `aria-tooltip-name`, `presentation-role-conflict`. Comportement des widgets personnalisés (états mis à jour après interaction) : non couvert. | AXE-N (5 règles à ajouter au mapping) ; changements d'état : COMP (basculer un `aria-expanded`/`aria-selected` via `click`, relire `a11y_tree`) | nœud axe ; pour COMP : arbre d'accessibilité avant/après | FP faible pour AXE-N (certaines sont « best-practice ») ; COMP : FP moyen, FN élevé | S (mapping) / L (COMP) |
| 7.1 | 3. Nom pertinent, nom contient l'intitulé visible, rôle pertinent | Le nom contenu dans l'intitulé visible est décidable. | AXE-N : `label-content-name-mismatch` (déjà sur 11.2, à ajouter à 7.1) | nœud axe | FP moyen (composants icônes, texte visuel stylé) | S |
| 7.2 | 1. `noscript` / page sans JS équivalente | Seule la **présence** d'un `noscript` est mécanique ; l'équivalence de contenu est humaine. | COMP : chargement avec JavaScript désactivé (CDP `Emulation.setScriptExecutionDisabled`) puis comparaison texte/liens/fonctions avec la version scriptée → indice, jamais un verdict ; reste HUM | deux snapshots texte + différentiel | FP/FN élevés (SPA vides sans JS, rendu serveur) ; à traiter comme « à vérifier » | L |
| 7.2 | 2. Alternative d'un élément non textuel mis à jour par script | Non sans observateur de mutations. | COMP (MutationObserver sur `img[src]`, `object`, `canvas` + suivi de `alt`/`aria-label` après interaction) ou HUM | journal des mutations | FN élevé (il faut déclencher la bonne interaction) | L |
| 7.3 | 1. Élément à gestionnaire d'événement accessible clavier et pointeur | **Oui** pour le cas classique : élément non sémantique (`div`, `span`) portant un écouteur `click` sans être focusable ni avoir de rôle. | COMP/CDP : `DOMDebugger.getEventListeners` (ou hook `addEventListener` injecté avant chargement via `Page.addScriptToEvaluateOnNewDocument`) croisé avec focusabilité et rôle. Déjà partiel côté axe (`scrollable-region-focusable`, `server-side-image-map`). La disponibilité de ces API CDP dans le moteur obscura reste **à vérifier** (non éprouvée ici). | élément, type d'écouteur, absence de `tabindex`/rôle | FP moyen (délégation d'événements sur un ancêtre, alternative accessible présente ailleurs, « hors cas particuliers ») ; FN moyen | M |
| 7.3 | 2. Le script ne supprime pas le focus | Oui en partie : `[onfocus*="blur"]` en statique ; appel à `.blur()` détecté par hook sur `HTMLElement.prototype.blur`. Parcours complet : `tab_order` + `press_key` en vérifiant que `document.activeElement` reste l'élément ciblé. | JS-S (attributs `onfocus`/`blur`) + COMP (parcours Tab) | séquence d'éléments focalisés, élément perdant le focus | FP faible ; FN moyen (focus perdu uniquement sur interactions précises) | S (statique) / M (COMP) |
| 7.4 | 1. Changement de contexte initié par script : avertissement ou contrôle | Partiellement, par observation : un changement de contexte sur `change`/`input`/`focus` d'un `select`, `radio`, `checkbox`, champ (navigation, ouverture de fenêtre, déplacement du focus, soumission). Le caractère « averti par un texte » reste humain. | COMP : hook `addEventListener`/`onchange` avant chargement, puis `type_input`/`click` sur contrôles ciblés en observant `location`, `window.open`, `document.activeElement`. S'appuie sur l'exécuteur `guided` d'`rgaa-obscura`. | journal d'événements + URL avant/après | FP moyen (avertissement présent), FN élevé (parcours non exhaustif, formulaires à soumission AJAX) | L |
| 7.5 | 1-3. Messages de statut : `role=status`, `alert`, `log`/`progressbar` | Pas de règle axe correspondante. Un message de statut est un contenu apparaissant sans déplacement du focus : détectable seulement par observation. | COMP : MutationObserver global autour d'une action (soumission de formulaire, ajout au panier) puis vérification que le nœud ajouté est dans une région `status/alert/log` ou `aria-live`. Indices statiques faibles (classes `.alert`, `.toast`, `.error` sans rôle). | nœud inséré, ancêtre live ou non | FP élevé (tout nœud inséré n'est pas un message de statut), FN élevé (il faut déclencher l'action) ; ne produire que « à vérifier » | L |

### Thème 8 — Éléments obligatoires

| Critère | Test RGAA | Décidable déterministe ? | Mécanisme réaliste | Preuve produite | Risque FP / FN | Effort |
|---|---|---|---|---|---|---|
| 8.1 | 1. Doctype présent. 2. Doctype valide. 3. Doctype avant `<html>`. | **Oui**. Test 1 et 2 : `document.doctype` (nom `html`, identifiants publics vides pour HTML5, ou liste blanche des doctypes HTML 4.01/XHTML 1.x). Test 3 : un doctype placé après du contenu est ignoré par l'analyseur, d'où `document.doctype === null` et `document.compatMode === 'BackCompat'`, donc le test 1 échoue de toute façon ; pour le distinguer il faut le source brut (préfixe de la réponse HTTP). | JS-S (`document.doctype`, `compatMode`) ; source brut optionnel (spider/fetch) pour attribuer le message à 8.1.3. Si la fidélité de `document.doctype` dans obscura est confirmée, le critère peut rejoindre `COMPLETE_COVERAGE`. | nom/identifiants du doctype, `compatMode` | FP/FN très faibles ; seul risque : fidélité du moteur | S |
| 8.2 | 1. Code généré valide (balises, imbrication, fermetures, `id` uniques, attributs non doublés) | Oui pour `id` uniques : règles natives `duplicate-id`, `duplicate-id-active` (existent dans le catalogue, à vérifier dans la version 4.9.1 injectée ; `duplicate-id-aria` est déjà mappée). Validité générale : exige un validateur HTML (Nu Html Checker) sur la sérialisation du DOM rendu. Un simple compte d'erreurs `html5ever` ne vaut pas validation. | AXE-N (`duplicate-id*`) ; validateur externe sur DOM sérialisé : hors taxonomie, dépendance lourde (JVM) | nœuds axe ; rapport du validateur (ligne/colonne) | FP moyen avec un validateur (attributs `data-*`/ARIA frameworks, composants Web), FN élevé sans validateur | S (`duplicate-id*`) / L (validateur) |
| 8.3 | 1. Langue par défaut présente | Déjà couvert (`html-has-lang` complete + gap-fix 8.3) | — | — | — | — |
| 8.4 | 1. Code de langue valide **et pertinent** | Validité : `html-lang-valid` existe mais est mal rangé (anomalie 1 ci-dessus). Pertinence (langue déclarée = langue réelle) : détection de langue sur le texte visible comparée à `html[lang]`. | AXE-N (re-mapping `html-lang-valid` sur 8.4) + JS-S/Rust (détecteur de langue côté Rust, p. ex. un crate de détection n-grammes, sur texte de ≥ N mots) | `lang` déclaré, langue détectée, score, extrait | FP moyen (pages multilingues, texte court, contenu dominé par des noms propres) ; émettre « à vérifier » en dessous d'un seuil de confiance | S (mapping) / M (pertinence) |
| 8.5 | 1. Titre de page présent | Déjà couvert par gap-fix (présence seule) ; la pertinence est 8.6 | — | — | — | — |
| 8.6 | 1. Titre pertinent | Non pour la pertinence. Signaux mécaniques : titre générique (« Accueil » partout, « Untitled », nom de fichier ou URL, « Page »), titre identique sur toutes les pages d'un crawl, titre sans lien avec le `h1`. | JS-S (titre générique ou URL) ; comparaison inter-pages via `rgaa-spider` (même titre sur ≥ N pages distinctes) ; reste HUM | titre, motif, nombre de pages partageant le titre | FP moyen (sites à titre court volontaire) ; FN très élevé | S (statique) / M (inter-pages) |
| 8.7 | 1. Changement de langue indiqué (hors noms propres, etc.) | Partiellement : détection de blocs de texte dans une autre langue que celle du contexte (`lang` hérité) sans `lang` local. Le « hors cas particuliers » du test (noms propres, termes techniques, citations) n'est pas décidable. | JS-S + détecteur de langue (Rust) sur éléments de texte de longueur significative | élément, langue héritée, langue détectée, score | FP élevé (anglicismes, marques, extraits de code), FN élevé (textes courts) ; seulement « à vérifier » | M |
| 8.8 | 1. Code de langue de chaque changement valide et pertinent | Validité : `valid-lang` déjà mappée. Pertinence : même infrastructure que 8.7 (attribut `lang` local vs langue détectée). | Déjà AXE (partiel) + réutilisation de la sonde de détection de 8.7 | idem 8.7 | idem 8.7 | S (une fois 8.7 faite) |
| 8.9 | 1. Balises (hors `div`, `span`, `table`) non détournées à des fins de présentation | Partiellement. Cas mécaniques : éléments présentationnels obsolètes (`font`, `center`, `big`, `strike`, `tt`, `basefont`, `blink`, `marquee`) ; suites de `br` utilisées comme espacement ; `p` vide ou ne contenant que `&nbsp;` ; `blockquote` utilisé comme retrait (sans contenu citant). | JS-S (éléments obsolètes, `br` consécutifs, `p` vide) ; LINT possible pour `font`/`center` dans les gabarits sources | selector + élément fautif | FP faible pour les éléments obsolètes, moyen pour `br`/`p` vides ; FN élevé (usage détourné non repérable sans sémantique) | S |
| 8.10 | 1. Texte de sens de lecture différent dans une balise avec `dir`. 2. `dir` valide et pertinent | **Oui** pour l'essentiel : détecter des caractères à classe bidi forte RTL (hébreu, arabe, syriaque, thaana) dans un élément dont la direction calculée (`getComputedStyle(el).direction`) est `ltr` sans `dir` explicite ; valeur de `dir` hors `ltr`/`rtl` ; `dir="rtl"` posé sur du texte fortement LTR. | JS-S (CSSOM + plages Unicode) | élément, extrait, direction calculée, caractères RTL | FP faible (un seul nom arabe isolé est un défaut au sens du test) ; FN faible | S |

### Thème 9 — Structuration de l'information

| Critère | Test RGAA | Décidable déterministe ? | Mécanisme réaliste | Preuve produite | Risque FP / FN | Effort |
|---|---|---|---|---|---|---|
| 9.2 | 1. Structure cohérente : `header`, `nav`, `main`, `footer` ; `nav` réservé à la navigation | Partiellement. Présence/unicité des repères : natif. Absence de `banner`/`contentinfo` : pas de règle axe, sonde simple. `nav` réservé à la navigation : heuristique (un `nav` sans lien). | AXE-N : re-mapper `region`, `landmark-one-main` sur 9.2 ; mapper `landmark-banner-is-top-level`, `landmark-contentinfo-is-top-level`, `landmark-main-is-top-level`, `landmark-no-duplicate-banner/contentinfo/main`, `landmark-unique`. JS-S : présence d'un `banner` et d'un `contentinfo` de premier niveau. | nœuds axe ; liste des repères trouvés | FP moyen (« hors cas particuliers » : pages mono-fonction, pop-up, iframe) ; FN moyen | S |
| 9.4 | 1. Citation courte avec `q`. 2. Bloc de citation avec `blockquote` | Pas de règle axe. Détection de citations non balisées par motifs (« … », guillemets typographiques, tirets de dialogue) hors `q`/`blockquote`/`code` : heuristique faible. | JS-S d'indices uniquement (jamais d'échec ferme) ; reste HUM | extrait entre guillemets, parent | FP très élevé (titres, mise à distance, interface), FN élevé (citations sans guillemets) | S |

## 4. Synthèse — rapport effort / gain

Classement par rapport gain/effort (ordre de mise en œuvre conseillé) :

1. **Corrections de mapping uniquement (S, aucun code de sonde)** : `html-lang-valid` → 8.4 ; `region` et `landmark-one-main` → 9.2 ; `table-duplicate-name` → 5.2 ; `label-content-name-mismatch` → 7.1 ; règles ARIA absentes sur 7.1 (`aria-allowed-role`, `aria-dialog-name`, `aria-treeitem-name`, `aria-tooltip-name`, `presentation-role-conflict`) ; `duplicate-id`/`duplicate-id-active` → 8.2 (après vérification de version). Cela fait passer 9.2 et 5.2 de « aucun contrôle » à « partiel réel » sans rien écrire d'autre.
2. **Sondes JS statiques à faible risque (S)** : 8.1 (doctype), 8.10 (bidi), 5.8 (tableau `role=presentation` avec éléments de données), 8.9 (éléments obsolètes), 5.5 (titre vide/générique), 8.6 (titre générique), 5.3 (rôle manquant sur tableau de mise en forme probable). 8.1 pourrait atteindre une couverture complète.
3. **Détection de langue (M)** : mutualise 8.4 (pertinence), 8.7 et 8.8. Un seul détecteur, trois critères ; sortie « à vérifier » seulement.
4. **Sondes comportementales (L)** : 7.3 (écouteurs sans focusabilité) a le meilleur rendement du lot ; 7.4 et 7.2 apportent peu de preuve fiable ; 7.5 est surtout un indice. À conditionner à la validation que le moteur obscura expose les API CDP nécessaires.
5. **Pas de mécanisme déterministe réaliste** : pertinence de 5.2, 5.5, 8.6, 9.1.2 (hors périmètre), linéarisation de 5.3, équivalence de 7.2, citations 9.4. Ces tests relèvent de l'évaluation humaine ou de l'agent LLM déjà en place (`rgaa-agent`, `VISUAL_CRITERIA` contient 5.3).

Aucun critère du lot ne se prête à un contrôle « pixels » comme mécanisme principal ; la capture n'intervient qu'en appui humain (linéarisation 5.3).

## 5. Points d'attention transversaux

- **Pass/Fail** : tant qu'un critère n'est pas dans `COMPLETE_COVERAGE`, toute sonde ne peut produire que des échecs (contrat `parse_results`). Pour 5.1, 8.4 (pertinence), 8.7, 9.4, 7.5, une sortie « à vérifier / needs-review » serait plus honnête qu'un `Fail`. Le contrat actuel n'a pas de statut intermédiaire côté snippet : c'est une décision de conception à trancher avant d'implémenter ces sondes.
- **Fidélité du moteur** : les snippets s'exécutent dans `obscura scrape --eval`. Je n'ai pas vérifié que ce moteur implémente `document.doctype`, `getComputedStyle().direction`, `DOMDebugger`, ni `Emulation.setScriptExecutionDisabled`. À tester avant de promettre ces sondes.
- **Linter source** : le scanner (`scan.rs`) ne connaît que des éléments et attributs, et se retire dès qu'une valeur est dynamique. Il convient à 8.9 (éléments obsolètes) et, marginalement, 5.8 ; il ne convient pas aux contrôles dépendant du rendu (langue détectée, direction calculée, structure du DOM final).
- **Hors périmètre signalé** : 5.6 est `complete` alors que sa règle (`td-headers-attr`) ne décide pas les 4 tests ; à rouvrir dans la recherche du thème qui le concerne.
