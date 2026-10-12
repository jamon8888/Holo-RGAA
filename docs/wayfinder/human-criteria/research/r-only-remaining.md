# Recherche : outillage des tests « R seul » restants (formulaires, navigation, langue, scripts)

Ticket : [#346](https://github.com/jamon8888/Holo-RGAA/issues/346) — carte [#330](https://github.com/jamon8888/Holo-RGAA/issues/330). Date des vérifications : 2026-10-12.

Question : pour 5.1, 7.4, 8.7, 11.3, 11.10, 11.11, 11.12, 12.10, 12.11, 13.1 (test 4) et 13.6, quelle technique et quelle bibliothèque open source atteignent le meilleur niveau d'automatisation (Automatique / Assisté / Irréductible, voir `CONTEXT.md` de la branche `docs/human-criteria-automation`) ?

## Légende de fiabilité

- **[E]** établi par une source primaire citée (URL). Les chiffres de crates.io viennent de l'API `https://crates.io/api/v1/crates/<nom>` interrogée le 2026-10-12.
- **[I]** inférence de ma part à partir de [E] ; à confirmer.
- **[?]** inconnu ou non vérifié. Ne rien en déduire.
- « **À vérifier par fixture** » : capacité d'Obscura jamais mesurée par cette carte. Les faits Obscura 0.2.4 déjà établis par la carte sont repris tels quels de ses Notes ([#330](https://github.com/jamon8888/Holo-RGAA/issues/330)) : navigation Tab native absente (`keydown` livré), `Emulation.setScriptExecutionDisabled` sans effet, domaine `DOMDebugger` absent, géométrie réelle via `DOM.getBoxModel`, `getComputedStyle().direction` vide. Je n'ai exécuté aucun test Obscura pour ce ticket : ce qui suit sur Obscura est de la lecture de documents et de code du dépôt, pas une mesure.

## Résumé des conclusions

| Test | Niveau atteignable | En une phrase |
|---|---|---|
| 5.1.1 | **Automatique** (sous réserve du classement données/mise en forme) | Détection de la complexité par la grille d'en-têtes + présence de `caption` / `aria-describedby` ; pas de modèle. |
| 7.4.1 | Assisté | Un changement de contexte observé sur `change`/`focus` hors bouton/lien est un candidat `fail` ; l'avertissement textuel est un jugement. |
| 8.7.1 | Assisté | `lingua` (Apache-2.0) sur des blocs de texte vs `lang` hérité ; exceptions RGAA (noms propres, mots du dictionnaire) = jugement. |
| 11.3.1 / 11.3.2 | Assisté | Regroupement par `autocomplete`/`name` + comparaison de libellés ; juge texte seul pour la cohérence ; 11.3.2 exige l'agrégation inter-pages. |
| 11.10.1 à 11.10.7 | Assisté (aucun Automatique, aucun Irréductible) | Inventaire `required`/`aria-required`/`aria-invalid` déterministe ; indications et messages = juge texte seul ; interaction sans jamais soumettre. |
| 11.11.1 / 11.11.2 | Assisté | Capture des messages d'erreur + juge texte seul « type/format suggéré ? exemple donné ? ». |
| 11.12.1 / 11.12.2 | Assisté pour l'applicabilité et la preuve visible ; **Irréductible** pour ce qui se passe après validation | On ne soumet jamais un formulaire financier/juridique. |
| 12.10.1 | Assisté | Inventaire des écouteurs clavier à touche imprimable (surcharge de `addEventListener`) ; la sonde actuelle (`accesskey`) vise à côté. |
| 12.11.1 | Assisté | Révélation par survol/focus puis test d'atteignabilité au clavier ; dépend de capacités Obscura non mesurées. |
| 13.1.4 | **Irréductible** | La limite de temps de session côté serveur n'est pas observable depuis la page. |
| 13.6.1 | Assisté | Détection par dictionnaire/heuristique, pertinence par juge texte seul ; aucune bibliothèque de détection d'art ASCII trouvée. |

Constat transversal : un seul test sur ces 20 (5.1.1) atteint Automatique ; un seul (13.1.4) est Irréductible, plus la branche « après validation » de 11.12. Tous les autres sont Assistés, cohérent avec la règle de la carte : un verdict de modèle reste `review` tant qu'il n'est pas calibré (calibration : carte [#183](https://github.com/jamon8888/Holo-RGAA/issues/183), non redécidée ici).

## Cadre : modèle et bibliothèques

- Cible modèle de la carte : un modèle **servi par API** (Qwen 3.8 ou Holo 4 selon les Notes de [#330](https://github.com/jamon8888/Holo-RGAA/issues/330)) ; le local (Ollama via `rgaa-holo`) est une option. Je n'ai vérifié **aucune** propriété de ces modèles (capacités, coût, accord avec des experts) : [?].
- Les recherches amont n'ont trouvé aucune étude d'accord expert (français/RGAA) pour des juges LLM ([#333](https://github.com/jamon8888/Holo-RGAA/issues/333), résumé dans les Notes de #330). Je n'en ai pas cherché de nouvelle : tout verdict de modèle ci-dessous est donc non calibré [?] et reste Assisté.
- Un « juge texte seul » signifie : le modèle reçoit quelques chaînes (libellés, messages, alternatives) extraites par une sonde et répond par un indice structuré. Il n'a besoin ni d'image ni de page entière. Aucune nouvelle dépendance n'est nécessaire côté Rust pour l'appeler (le dépôt a déjà `rgaa-holo`) [I] ; des clients génériques existent (`genai` 0.6.5, MIT OR Apache-2.0 ; `async-openai` 0.42.2, MIT ; `rig-core` 0.44.0, MIT — crates.io [E]) mais je ne les recommande pas sans besoin.
- Pour mémoire, la carte amont [#336](https://github.com/jamon8888/Holo-RGAA/issues/336) a conclu qu'IBM Equal Access (Apache-2.0) donne des indices sur 13.6 et qu'aucun moteur ne conclut sur 11.12 ; je ne l'ai pas revérifié.

## Capacités Obscura à vérifier par fixture

Ces capacités conditionnent plusieurs lignes ci-dessous. Aucune n'est mesurée ici [?] :

1. Script exécuté avant ceux de la page (`Page.addScriptToEvaluateOnNewDocument` ou équivalent) pour surcharger `addEventListener` (7.4, 12.10, 12.11). Le dépôt n'utilise aujourd'hui que `Runtime.evaluate` et `Input.dispatchKeyEvent` dans `rgaa-obscura` (recherche textuelle dans `rgaa-rs/crates/rgaa-obscura/src/`).
2. API de validation de contraintes : `validity`, `checkValidity()`, `reportValidity()`, `validationMessage` (11.10, 11.11).
3. `getComputedStyle(el, '::after').content` pour les indicateurs « * » ajoutés en CSS (11.10.2).
4. `Input.dispatchMouseEvent` et l'appariement de `:hover` (12.11) ; `MutationObserver` (12.10, 12.11).
5. Attributs de grille de tableau (`rowSpan`, `colSpan`, `cellIndex`, `scope`) (5.1).
6. Interception réseau ou blocage des requêtes et navigations pendant une sonde, pour garantir qu'on ne soumet jamais (7.4, 11.10, 11.12).
7. Lecture des cookies (`Network.getCookies`) pour l'inventaire de 13.1.4 (la livraison v0.2.1 mentionne `Storage.clearCookies`, pas la lecture : [`docs/wayfinder/research/obscura-delta-v020-v022.md`](https://github.com/jamon8888/Holo-RGAA/blob/master/docs/wayfinder/research/obscura-delta-v020-v022.md)).

## État actuel du dépôt (lecture de code, pas mesure)

Les sondes JS de `rgaa-rs/crates/rgaa-rules/src/plan_snippets.rs` et les entrées de `rgaa-rs/crates/rgaa-core/data/rgaa-4.1.2/mechanisms.toml` (sur `master`) existent déjà pour 5.1, 7.4, 8.7, 11.3, 11.11, 12.10, 12.11, 13.1. Constats de lecture :

- 5.1 : la sonde traite `rowspan`/`colspan` ou plusieurs lignes de `thead` comme « complexe » et accepte `aria-label`, `aria-labelledby` ou un titre précédent contenant « résumé » comme description. C'est plus laxiste que la méthodologie RGAA (voir 5.1.1).
- 7.4 : la sonde ne lit que les attributs inline `onchange`/`onfocus` ; elle ne voit pas les écouteurs posés par `addEventListener`.
- 8.7 : la sonde cherche une poignée de mots (`the`, `with`, `bonjour`…) ; elle émet `review` dans tous les cas.
- 12.10 : la sonde inventorie `[accesskey]`. Or le glossaire RGAA précise que les `accesskey` « nécessitent déjà l'usage de touches de modification » (voir 12.10.1) : la sonde vise donc le mauvais objet.
- 13.1 : sonde `review` sur des indices (`meta refresh`, mots, scripts), sans lien avec le test 4.

## Résultats test par test

### 5.1.1 — résumé des tableaux de données complexes

**Ce que demande le RGAA [E]** ([critères et tests](https://accessibilite.numerique.gouv.fr/methode/criteres-et-tests/), test 5.1.1 ; [glossaire](https://accessibilite.numerique.gouv.fr/methode/glossaire/), « Tableau de données complexe ») : un tableau de données (`<table>` ou `role="table"`) est complexe quand il contient des en-têtes qui ne sont pas répartis uniquement sur la première ligne et/ou la première colonne, ou dont la portée n'est pas valable pour toute la ligne/colonne. Le résumé doit être présent dans `<caption>`, dans l'attribut `summary` (HTML antérieur à HTML 5) ou dans un passage lié par `aria-describedby`. La *pertinence* du résumé relève du test 5.2.1, pas de 5.1.1.

**Fait complémentaire [E]** : l'attribut `summary` de `table` est obsolète en HTML ([HTML Standard, obsolete features](https://html.spec.whatwg.org/multipage/obsolete.html)) ; le RGAA ne l'accepte que pour les versions antérieures à HTML 5.

**Technique [I]** : sonde DOM dans la page (pas de bibliothèque nécessaire) qui (1) écarte `role="presentation"`/`none`, (2) construit la grille de cellules et classe les `th` selon leur position et leur `scope`/`headers`/`rowspan`/`colspan`, (3) déclare « complexe » selon la définition ci-dessus, (4) vérifie la présence de `caption` non vide, de `aria-describedby` pointant vers un texte non vide, ou de `summary` (le dernier signalé comme obsolète). Tout tableau dont la nature (données ou mise en forme) ne peut être tranchée par les signaux DOM (aucun `th`, aucune `caption`) part en `review`, pas en `pass`.

**Bibliothèque** : aucune requise dans la page. Côté Rust, si l'on voulait rejouer l'analyse hors navigateur : `html5ever` 0.40.1 (MIT OR Apache-2.0) ou `scraper` 0.27.0 (ISC) [E, crates.io] ; ISC et MIT/Apache sont compatibles avec un binaire MIT [I]. Non nécessaire : le DOM rendu est déjà disponible.

**Obscura** : lecture du DOM standard ; attributs de cellule à vérifier par fixture (capacité 5). Les fixtures `5.1-complex-table-summary-pass/fail` sont déjà déclarées dans `mechanisms.toml`.

**Niveau : Automatique** [I] (fail et pass déterministes avec preuve : nœud, grille d'en-têtes, mécanismes absents), à condition que le cas « non classé » reste `review`. Risque principal : les faux négatifs de classification du « complexe » (tableau déclaré simple à tort → faux `pass`). Une fixture de classification doit couvrir `scope`, `headers`, `rowspan` d'en-tête, `th` en milieu de ligne.

### 7.4.1 — script qui initie un changement de contexte

**Ce que demande le RGAA [E]** (test 7.4.1) : un script qui initie un changement de contexte doit soit avertir l'utilisateur par un texte avant déclenchement, soit être initié par un bouton explicite, soit par un lien explicite. WCAG 2.1 3.2.1 (On Focus) et 3.2.2 (On Input). Le changement de contexte couvre : agent utilisateur, espace de restitution, focus, contenu qui modifie la signification de la page ; un changement de contenu n'en est pas toujours un ([glossaire RGAA](https://accessibilite.numerique.gouv.fr/methode/glossaire/), « Changement de contexte » ; [WCAG 2.1, définition](https://www.w3.org/TR/WCAG21/)).

**Règle ACT [E]** : la page ACT officielle indique « No ACT Rules available for this criterion yet » pour 3.2.1 et 3.2.2 ([ACT rules](https://www.w3.org/WAI/standards-guidelines/act/rules/)). Aucun test de référence W3C à réutiliser en fixture.

**Technique [I]** : (a) préchargement d'une surcharge de `addEventListener` pour inventorier les écouteurs `change`/`input`/`focus` sur des contrôles qui ne sont ni bouton ni lien (fait établi par la carte : `DOMDebugger` absent sous 0.2.4, donc l'inventaire passe par la surcharge, voir [#337](https://github.com/jamon8888/Holo-RGAA/issues/337)) ; (b) déclenchement d'un `change` sur un `select`/champ et observation d'un effet de contexte : navigation, `window.open`, déplacement de `document.activeElement`, soumission (bloquée) ; (c) extraction du texte voisin pour juger l'avertissement.

**Bibliothèque** : aucune (JS injecté + JSON côté Rust).

**Obscura** : surcharge au préchargement = capacité 1 ; blocage des soumissions/navigations = capacité 6 ; à vérifier par fixture. Les `change` synthétiques via `dispatchEvent` sont du JS standard mais leur effet réel n'est pas mesuré [?].

**Niveau : Assisté** [I]. Le déclencheur observé hors bouton/lien est un candidat `fail` à haute précision, mais seul le texte d'avertissement peut le sauver, et son existence/qualité est un jugement (juge texte seul sur le contexte du contrôle). Pas de `pass` sans revue : l'absence d'effet observé après un jeu limité d'événements ne prouve pas l'absence de script (sauts de chemin, handlers conditionnels).

### 8.7.1 — changement de langue indiqué dans le code

**Ce que demande le RGAA [E]** (test 8.7.1) : tout passage en langue étrangère doit porter `lang`/`xml:lang` sur lui-même ou un parent, sauf : noms propres, mots d'origine étrangère présents dans le dictionnaire de la langue du document, mots d'usage courant à prononciation compréhensible. Cas particuliers « non applicable » : nom propre, nom commun du dictionnaire officiel, terme saisi par l'utilisateur et rappelé, **passage dont la langue ne peut pas être déterminée**, langue morte/imaginaire. Note 2 : un nom commun étranger absent du dictionnaire et passé dans le langage commun (ex. « newsletter ») est applicable seulement si l'absence de `lang` peut provoquer une incompréhension.

**Règles ACT [E]** : « HTML element language subtag matches language » (`off6ek`, *proposed*, [page de la règle](https://www.w3.org/WAI/standards-guidelines/act/rules/off6ek/proposed/)) vérifie qu'un `lang` **existant** correspond à la langue du texte, et précise qu'elle « ne vérifie pas quand des attributs `lang` manquent ». Elle ne couvre donc pas 8.7 (passage sans `lang`) mais 8.8 (pertinence du code). Aucune règle ACT n'existe pour les `lang` manquants ; la page ACT liste pour 3.1.2 « Element with lang attribute has valid language tag » et `off6ek` seulement.

**Bibliothèques Rust de détection de langue [E, crates.io + README `lingua`]** :

| Crate | Version | Licence | Remarque |
|---|---|---|---|
| `lingua` | 1.8.0 | Apache-2.0 | n-grammes de 1 à 5, règles + Naive Bayes, hors ligne, détection multi-langues dans un texte mélangé (qualifiée « expérimentale » par l'auteur), modèles en crates séparées (`lingua-french-language-model` 1.3.0, Apache-2.0) ; toutes langues ≈ 300 Mo de dépendances, sous-ensemble par features ; modèles chargés à la demande, quelques dizaines de Mo en mémoire |
| `whatlang` | 0.18.0 | MIT | rapide, trigrammes |
| `whichlang` | 0.1.1 | MIT | le plus rapide selon l'auteur de `lingua` |
| `cld2` | 1.0.2 | Unlicense | liaison C++, dernière mise à jour 2017-11-28 |
| `langram` | 0.12.0 | MIT OR Apache-2.0 | non évaluée |

**Mesures publiées [E, mais auto-déclarées]** : rapports de précision du dépôt `lingua-rs` pour le français, sur 75 langues candidates, corpus de presse Wortschatz (Leipzig), 1000 mots / 1000 paires de mots / 1000 phrases ([README](https://github.com/pemistahl/lingua-rs/blob/main/README.md), [accuracy-reports](https://github.com/pemistahl/lingua-rs/tree/main/accuracy-reports)) :

| Détecteur | Mots seuls (8 car. en moyenne) | Paires (17 car.) | Phrases (112 car.) | Moyenne |
|---|---|---|---|---|
| `lingua` (haute précision) | 73,70 % | 94,30 % | 99,20 % | 89,07 % |
| `whichlang` | 36,80 % | 70,90 % | 99,20 % | 68,97 % |
| `whatlang` | 35,00 % | 58,90 % | 97,60 % | 63,83 % |

Réserves : mesures de l'auteur de `lingua`, sur du texte de presse, non sur des textes d'interface web (libellés courts, mélange de noms de produits) ; je ne les ai pas reproduites ; la licence du corpus d'entraînement n'est pas vérifiée [?]. Elles indiquent seulement que sous ~20 caractères la détection est peu fiable, ce qui rejoint le cas « langue indéterminable = non applicable » du RGAA [I].

**Technique [I]** : sonde JS qui extrait des blocs de texte significatifs (nœuds feuilles, texte visible, hors `script`/`style`/`code`/`pre`) avec leur langue effective héritée (`closest('[lang]')`, repli sur `<html lang>`), puis `lingua` côté Rust sur chaque bloc au-dessus d'un seuil de longueur, avec `with_minimum_relative_distance` pour rejeter les cas ambigus (option documentée dans le README). Bloc dont la langue détectée ≠ langue effective, avec forte confiance → candidat. Pré-filtre déterministe complémentaire : `unicode-script` 0.5.8 (MIT OR Apache-2.0) pour repérer un passage entièrement dans une autre écriture (cyrillique, arabe, CJK…) sur une page en écriture latine [I].

**Obscura** : lecture de `lang` et du texte = DOM standard, rien d'exotique ; à vérifier par fixture (déjà prévu : `8.7-language-change-pass/fail`).

**Niveau : Assisté** [I]. La machine peut proposer des passages candidats avec preuve (texte, langue détectée, confiance, `lang` effectif), mais les exceptions RGAA (noms propres, mots du dictionnaire, usage courant) sont un jugement ; un juge texte seul peut aider à les trier (non calibré). Un `fail` Automatique pour de longs passages à haute confiance serait envisageable après calibration sur un corpus de vérité terrain (carte #183) : **non établi** par cette recherche. Aucun `pass` sans revue : l'absence de candidat ne prouve pas l'absence de changement de langue (faux négatifs sur textes courts).

### 11.3.1 et 11.3.2 — étiquettes cohérentes

**Ce que demande le RGAA [E]** (tests 11.3.1 et 11.3.2 ; glossaire « Étiquettes cohérentes ») : les champs de même fonction répétés dans une page (11.3.1) ou un ensemble de pages (11.3.2) doivent avoir des étiquettes permettant de comprendre qu'il s'agit de saisies de même nature. WCAG 3.2.4 Consistent Identification. Aucune règle ACT pour 3.2.4 ([ACT rules](https://www.w3.org/WAI/standards-guidelines/act/rules/)).

**Technique [I]** : (1) regroupement des champs par clé de fonction : jeton `autocomplete` (les finalités sont définies dans WCAG 2.1 §7 « Input Purposes for User Interface Components », [spécification](https://www.w3.org/TR/WCAG21/)), à défaut `name`/`type` ; (2) normalisation des libellés (casse, accents, ponctuation) ; (3) écart de libellés au sein d'un groupe → candidat ; (4) juge texte seul : « ces libellés désignent-ils la même information ? » ; (5) pour 11.3.2, agréger ces observations sur l'échantillon de pages. La sonde actuelle (`11.3` dans `plan_snippets.rs`) fait déjà (1)-(3) en exact-match et émet `fail` sur toute divergence : une divergence de libellé n'est pas forcément une incohérence (« Adresse » vs « Adresse de livraison »), donc ce `fail` devrait redevenir `review` [I].

**Bibliothèques [E, crates.io]** : `unicode-normalization` 0.1.25 (MIT OR Apache-2.0) pour la normalisation ; `strsim` 0.11.1 (MIT) pour la distance entre libellés (indice, pas verdict). Aucune bibliothèque de « même fonction » trouvée ; non cherché au-delà de crates.io.

**Cross-pages** : `rgaa-orchestrator/src/site_comparison.rs` agrège déjà des observations par page pour des critères d'ensemble de pages et « ne traite jamais un manque d'observation comme preuve de Pass » (en-tête du module). 11.3.2 demanderait de nouveaux champs d'observation (libellé et clé de fonction par champ) [I].

**Obscura** : lecture de `field.labels`, `autocomplete`, `aria-label` en DOM standard ; à vérifier par fixture.

**Niveau : Assisté** pour 11.3.1 et 11.3.2 [I]. Le regroupement par clé forte est déterministe, mais la « même fonction » sans `autocomplete` et la cohérence sémantique sont des jugements.

### 11.10.1 à 11.10.7 — contrôle de saisie pertinent

**Ce que demande le RGAA [E]** (tests 11.10.1 à 11.10.7 ; glossaire « Indication de champ obligatoire », « Contrôle de saisie ») :

- 11.10.1 : tout champ obligatoire a, avant validation, soit une indication visible qui l'identifie nommément, soit `required`/`aria-required="true"`.
- 11.10.2 : tout champ avec `required`/`aria-required="true"` a une indication visible dans son étiquette ou dans le passage de texte associé. Une indication non textuelle (icône, « * ») doit être expliquée avant sa première utilisation.
- 11.10.3 / 11.10.6 : message d'erreur (absence de saisie / format) visible qui identifie le champ, ou champ en `aria-invalid="true"`.
- 11.10.4 / 11.10.7 : champ en `aria-invalid="true"` → message d'erreur (11.10.4) ou instruction de format (11.10.7) visible dans l'étiquette ou le passage lié.
- 11.10.5 : instruction/indication du type et du format obligatoire visible avant validation.
- Cas particuliers [E] : 11.10.1 et 11.10.2 sont non applicables si le formulaire n'a qu'un champ ou s'il indique visiblement (dans l'étiquette ou la légende) les champs *optionnels*. Si tous les champs sont obligatoires, ils restent applicables.

**Règle ACT [E]** : « Error message describes invalid form field value » (`36b590`, *proposed*, [page](https://www.w3.org/WAI/standards-guidelines/act/rules/36b590/proposed/)) mappe sur WCAG 3.3.1 et teste que les messages d'erreur identifient la cause ou la correction ; utilisable comme jeu de fixtures pour 11.10.3/11.10.6 [I]. Aucune règle ACT n'est listée pour 3.3.2 (Labels or Instructions).

**Technique [I]** :

- Inventaire déterministe : champs `required`/`aria-required`, `pattern`, `type` contraignant, `maxlength`, `min`/`max`, présence de `aria-invalid`.
- Indication visible (11.10.2) : texte de l'étiquette (`field.labels`), texte des éléments pointés par `aria-describedby`/`aria-labelledby`, contenu de `::after` du libellé (capacité 3), puis motif sur « * », « obligatoire », « requis », « required », « (facultatif) » pour détecter les cas particuliers. Un « * » seul impose de retrouver son explication en amont (ordre de code) : jugement.
- Provocation d'erreur (11.10.3/4/6/7) : `checkValidity()`/`reportValidity()` et lecture de `validationMessage` ; pour la validation JS maison, déclenchement d'événements `invalid`/`submit` **avec blocage réseau et navigation**. **Jamais `form.submit()` ni `requestSubmit()`** : une soumission à un formulaire de commande ou de suppression est un effet de bord inacceptable. Que `dispatchEvent(new Event('submit'))` ne soumette pas réellement la page n'est pas vérifié ici [?] : il faut une fixture avec compteur de requêtes.
- Juges texte seul : « l'indication nomme-t-elle le champ ? », « l'instruction donne-t-elle le type/format (ex. JJ/MM/AAAA) ? », « le message d'erreur identifie-t-il le champ concerné ? ». Chaque juge reçoit des chaînes courtes ; sa fiabilité est inconnue [?].

**Bibliothèque** : aucune au-delà du JS de page ; juge via `rgaa-holo`.

**Obscura** : `validity`/`checkValidity`/`validationMessage` = capacité 2 ; pseudo-éléments = capacité 3 ; blocage des requêtes = capacité 6 ; tout à vérifier par fixture. Aucune mention de `checkValidity`/`reportValidity` dans les documents ni le code `rgaa-obscura` du dépôt [E par recherche textuelle].

**Niveau par test [I]** :

| Test | Niveau | Justification |
|---|---|---|
| 11.10.1 | Assisté | `pass` déterministe pour les champs porteurs de `required`/`aria-required` (condition 2), mais les champs obligatoires **sans** attribut ni indication sont indétectables sans soumettre : le périmètre est inconnu, donc pas de `pass` global sans revue. |
| 11.10.2 | Assisté | Le `fail` candidat (aucune indication dans étiquette/passage, pas de marquage « facultatif ») est précis ; `fail` Automatique envisageable après fixtures (pseudo-éléments CSS, légende commune, explication de « * »). |
| 11.10.3 | Assisté | `aria-invalid="true"` après erreur = preuve déterministe ; message nommant le champ = juge texte. Nécessite de provoquer l'erreur sans soumettre. |
| 11.10.4 | Assisté | Énumération des champs `aria-invalid` et du texte associé déterministe ; pertinence du message = juge texte. |
| 11.10.5 | Assisté | Détection de champs à format imposé (`pattern`, `type`) déterministe ; présence d'une indication de format visible = juge texte. |
| 11.10.6 | Assisté | Idem 11.10.3 pour les erreurs de format. |
| 11.10.7 | Assisté | Idem 11.10.4 pour l'instruction de format. |

Aucun test de 11.10 n'est Irréductible : chacun admet au moins un indice machine utile [I]. Aucun n'atteint Automatique avant calibration.

### 11.11.1 et 11.11.2 — suggestions pour corriger les erreurs

**Ce que demande le RGAA [E]** (tests 11.11.1 et 11.11.2 ; WCAG 3.3.3) : pour chaque message d'erreur, les types et formats attendus (11.11.1) et des exemples de valeurs (11.11.2) sont suggérés « si nécessaire ». **Note technique RGAA [E]** : les messages natifs HTML5 (type `email`…) et l'attribut `pattern` avec `title` « valident le critère ».

**Technique [I]** : même mécanique de provocation que 11.10 ; collecte du texte des messages (`validationMessage`, nœuds `aria-describedby`, `role="alert"`, texte voisin) ; pré-classement déterministe : champ à contrainte native (`type` email/url/number/date, `pattern` + `title`) sans `novalidate` ni `invalid` intercepté → message natif présumé conforme selon la note RGAA ; juge texte seul pour les messages personnalisés (« un type ou un format est-il suggéré ? », « un exemple est-il donné ? »). La sonde actuelle (`11.11` dans `plan_snippets.rs`) lit l'état d'invalidité courant sans provoquer d'erreur ; elle compte donc surtout des champs déjà invalides au chargement [I, lecture de code].

**Obscura** : capacités 2 et 6 ; à vérifier par fixture.

**Niveau : Assisté** pour les deux [I]. Le « si nécessaire » et la pertinence de la suggestion dépendent de la règle métier ; la note RGAA sur les messages natifs donne un `pass` plausible mais seulement pour les champs réellement validés par le navigateur, ce que le moteur d'Obscura doit prouver (fixture).

### 11.12.1 et 11.12.2 — modification, confirmation ou récupération des données

**Ce que demande le RGAA [E]** (tests 11.12.1 et 11.12.2 ; glossaire « Modifier ou annuler les données et les actions effectués » ; WCAG 3.3.4) : pour les formulaires qui modifient/suppriment des données ou ont des conséquences financières/juridiques : annulation ou modification possible après validation, ou vérification/correction avant validation d'un formulaire multi-étapes, ou confirmation explicite (case à cocher ou étape supplémentaire) (11.12.1) ; récupération des données supprimées/modifiées ou demande de confirmation explicite (11.12.2). La note du glossaire exige que la page indique la durée et la procédure d'annulation.

**Technique [I]** :

1. Applicabilité (Assistée) : heuristique DOM (méthode `post`, champs `password`/`iban`/carte, boutons « payer », « supprimer », « valider la commande ») + juge texte seul sur le texte du bouton et du contexte.
2. Preuve visible sans rien soumettre : case à cocher de confirmation présente dans le formulaire, indicateurs d'étapes (`aria-current="step"`, texte « étape 2/3 »), mention « annuler », « rétractation », « récapitulatif » (juge texte seul).
3. Ce qui se passe **après** la validation (annulation effective, récupération, étape de confirmation suivante) ne s'observe qu'en validant réellement. Valider un formulaire financier, juridique ou de suppression est un effet de bord que ce projet ne doit pas produire : cette branche est **Irréductible** pour la machine [I]. Un agent de computer use ne change pas ce constat, il aurait à agir irréversiblement.

**Bibliothèque** : aucune.

**Obscura** : lecture DOM seulement ; les étapes multiples nécessiteraient de naviguer dans un tunnel, hors périmètre sûr.

**Niveau : Assisté** pour l'applicabilité et la preuve visible ; **Irréductible** pour la vérification de l'après-validation, avec cette justification écrite [I]. Statut par défaut `review`.

### 12.10.1 — raccourcis clavier à touche unique

**Ce que demande le RGAA [E]** (test 12.10.1 ; WCAG 2.1.4) : chaque raccourci à une seule touche imprimable doit pouvoir être désactivé, ou reconfiguré avec une touche de modification, ou n'être actif que lorsque le composant a le focus. **Le glossaire RGAA précise que les `accesskey` HTML « nécessitent déjà l'usage de touches de modification »** ([glossaire](https://accessibilite.numerique.gouv.fr/methode/glossaire/), « Raccourci clavier ») : l'inventaire `[accesskey]` de la sonde actuelle ne cible donc pas des raccourcis à touche unique [E+I]. (Le glossaire écrit « critère 12.1 » dans cette note ; je lis une coquille pour 12.10, ce que je ne peux pas confirmer [?].)

**Règle ACT [E]** : « No keyboard shortcut uses only printable characters » (`ffbc54`, *proposed*, *semi-auto*, [page](https://www.w3.org/WAI/standards-guidelines/act/rules/ffbc54/proposed/)). Applicabilité : événement clavier dont `key` est un caractère imprimable, `getModifierState` faux pour tous les modificateurs valides, et qui **change le contenu** du document HTML. Attendu : un jeu d'instruments clairement étiquetés pour bloquer ces événements (désactiver/remapper), ou l'`event target` est un widget sémantique héritant (le focus). Hypothèses de la règle : un moyen de désactivation hors page peut rendre un échec non-échec du critère ; une réactivation par minuterie rend le `pass` trompeur.

**Technique [I]** : préchargement d'une surcharge de `addEventListener` (et lecture des attributs `onkeydown`/`onkeypress`/`onkeyup` inline) pour inventorier les écouteurs `key*` ; classement par cible : posés sur `document`/`window`/`body` → portée globale ; posés sur un widget → actif seulement avec focus (condition 3 du RGAA, pass pour ce raccourci) ; puis envoi d'une touche imprimable sans modificateur (`Input.dispatchKeyEvent`, déjà utilisé par `rgaa-obscura`) avec `MutationObserver` pour détecter un changement de contenu (jugement de la règle ACT). La présence d'un mécanisme de désactivation/remappage (paramètres, aide « raccourcis ») est un jugement : texte voisin/pages d'aide, juge texte seul.

**Bibliothèque** : aucune.

**Obscura** : fait établi par la carte : `keydown` est livré ; surcharge au préchargement et `MutationObserver` = capacités 1 et 4, à vérifier par fixture. `DOMDebugger` absent : on ne peut pas lister les écouteurs par CDP.

**Niveau : Assisté** [I]. Un écouteur global à touche imprimable qui modifie le contenu est un candidat `fail` à preuve forte ; la décision dépend de l'existence d'un mécanisme de désactivation (jugement) et la détection ne voit que ce que l'inventaire a capté (bibliothèques, shadow DOM, iframes : [?]). Zéro écouteur global inventorié suggère « non applicable », pas `pass` (le critère s'applique aux raccourcis existants). Le `pass` n'est jamais rendu sans revue.

### 12.11.1 — contenus additionnels atteignables au clavier

**Ce que demande le RGAA [E]** (test 12.11.1 et notes techniques ; WCAG 2.1.1) : pour les contenus qui apparaissent au survol, à la prise de focus ou à l'activation d'un composant, ceux qui contiennent des composants interactifs (ex. info-bulle avec un lien) doivent être atteignables au clavier. Aucune règle ACT pour 1.4.13 (« No ACT Rules available ») ; pour 2.1.1 la page ACT liste seulement « Iframe with interactive elements is not excluded from tab-order » et « Scrollable content can be reached with sequential focus navigation » ([ACT rules](https://www.w3.org/WAI/standards-guidelines/act/rules/)), qui ne couvrent pas ce test.

**Technique [I]** : (1) inventorier les déclencheurs (écouteurs `mouseover`/`mouseenter`/`focus`/`click`, `aria-haspopup`, `aria-expanded`, `title`, `role=tooltip`) ; (2) révéler le contenu : `Input.dispatchMouseEvent` (mouvement) pour le survol, `focus()` pour le focus, clic pour l'activation ; (3) mesurer les nouveaux nœuds visibles (`MutationObserver` + `DOM.getBoxModel`, géométrie réelle disponible sous Obscura 0.2.4 d'après la carte) ; (4) vérifier que les descendants interactifs sont focalisables (`tabindex` ≥ 0 ou élément natif) et qu'un déclencheur *au clavier* existe (le déclencheur est lui-même focalisable). Candidat `fail` : contenu révélé seulement par un déclencheur non focalisable et contenant un composant interactif.

**Bibliothèque** : aucune.

**Obscura** : fait établi : navigation Tab native absente (`keydown` livré). Le survol réel (appariement `:hover` après `Input.dispatchMouseEvent`) n'est pas documenté dans le dépôt [?] : à vérifier par fixture (capacité 4). Un survol simulé par `dispatchEvent(mouseover)` ne déclenche pas les styles `:hover` CSS [I, comportement standard des navigateurs, non mesuré sous Obscura].

**Niveau : Assisté** [I]. Les révélations purement CSS (`:hover`) et le « si nécessaire » (le contenu est-il indispensable ?) limitent la détection. Pas de `pass` sans revue.

### 13.1.4 — limite de temps de session

**Ce que demande le RGAA [E]** (test 13.1.4 ; WCAG 2.2.1) : chaque procédé limitant le temps d'une session doit permettre de supprimer ou d'augmenter la limite, ou la limite est d'au moins vingt heures. Cas particulier : limite essentielle → non applicable. Aucune règle ACT pour ce test (les deux règles ACT 2.2.1 de la page concernent `meta refresh`).

**Technique [I]** : seule la partie client est observable : minuteries (`setTimeout` > N s avec message d'expiration), durées de vie des cookies (`Expires`/`Max-Age`), texte « votre session expirera » avec compte à rebours. L'inventaire de ces indices est possible (cookies : capacité 7). Mais : (1) une session avec cookie de session (sans `Expires`) n'a aucune limite lisible côté client ; (2) la limite d'inactivité côté serveur n'est pas observable ; (3) tester l'absence de limite à vingt heures impose d'attendre plus de vingt heures dans une session authentifiée ; (4) l'authentification est hors du périmètre de l'audit (identifiants).

**Bibliothèque** : aucune.

**Niveau : Irréductible** [I], avec cette justification : le fait à établir (durée de la session côté serveur, existence d'un mécanisme de prolongation) n'est pas exposé à la page et ne s'obtient qu'en attendant ou par déclaration de l'exploitant. Au-delà de l'inventaire d'indices (non concluant), rien ne s'automatise utilement ; le test reste à la charge d'un auditeur (entretien avec l'exploitant ou test authentifié). Les tests 13.1.1 à 13.1.3 (hors périmètre de ce ticket) sont traités ailleurs.

### 13.6.1 — alternative pertinente des contenus cryptiques

**Ce que demande le RGAA [E]** (test 13.6.1 ; WCAG 1.1.1) : pour chaque contenu cryptique (art ASCII, émoticône, syntaxe cryptique), le contenu de l'attribut `title` est pertinent, ou la définition du contexte adjacent est pertinente. La technique WCAG H86 couvre « emojis, emoticons, ASCII art, and leetspeak » ([H86](https://www.w3.org/WAI/WCAG21/Techniques/html/H86)). Le texte RGAA que j'ai lu ne dit pas si un emoji Unicode est une « émoticône » : [?].

**Technique [I]** :

1. Détection (recall inconnu) : pour les émoticônes typographiques, dictionnaire (voir bibliothèques) ; pour l'art ASCII, heuristique (bloc multi-lignes en `<pre>`/police fixe, densité de ponctuation et de symboles, faible ratio de lettres) ; pour la syntaxe cryptique/leetspeak, aucune méthode déterministe trouvée.
2. Alternative : `title` de l'élément, texte adjacent (voisinage DOM).
3. Pertinence : juge texte seul sur (contenu cryptique, alternative) ; pour les émoticônes du dictionnaire, la signification de référence peut servir d'entrée au juge.

**Bibliothèques [E, crates.io + READMEs]** : `emojicon` 0.5.0 (MIT) retrouve un emoji par émoticône (`get_by_emoticon("B-)")`) et par nom, données reprises de `emoticon` (wooorm) et `gemoji` (GitHub) — licences de ces jeux de données non vérifiées [?] ([README](https://github.com/mominul/emojicon)) ; `emoticond` 1.0.2 (MIT OR Apache-2.0) est un moteur de recherche de kaomoji (~176 000 kaomoji selon son README), peu utilisé (33 téléchargements), d'intérêt limité ici [I]. `emojis` 0.9.0 ((MIT OR Apache-2.0) AND Unicode-3.0) donne les métadonnées des emoji Unicode, pas des émoticônes. Une recherche crates.io (« ascii art detection », « emoticon ») n'a trouvé **aucune** bibliothèque de détection d'art ASCII ; la recherche n'est pas exhaustive [?].

**Obscura** : DOM standard, rien de particulier ; à vérifier par fixture. Si l'art ASCII devait être jugé visuellement (rendu en image), cela relève de la chaîne VLM de [#333](https://github.com/jamon8888/Holo-RGAA/issues/333) et de la capture de page [#335](https://github.com/jamon8888/Holo-RGAA/issues/335) : non étudié ici.

**Niveau : Assisté** [I]. La détection est lacunaire (recall inconnu) et la pertinence est un jugement ; une émoticône du dictionnaire sans `title` ni définition adjacente est un candidat `fail`, mais « définition donnée par le contexte » est vague. Pas de `pass` sans revue.

## Tableau de synthèse

| Test | Technique | Bibliothèque / licence | Support Obscura | Niveau | Fiabilité |
|---|---|---|---|---|---|
| 5.1.1 | Grille d'en-têtes + présence `caption`/`aria-describedby`/`summary` | aucune (DOM) ; en repli Rust `html5ever` 0.40.1 MIT OR Apache-2.0 | DOM standard ; à vérifier par fixture | Automatique | [I] |
| 7.4.1 | Surcharge `addEventListener` + `change` synthétique + juge texte | aucune | préchargement et blocage de soumission : à vérifier par fixture | Assisté | [I] |
| 8.7.1 | Blocs de texte + `lang` effectif + détection de langue | `lingua` 1.8.0 Apache-2.0 (+ `unicode-script` 0.5.8 MIT OR Apache-2.0) | DOM standard ; à vérifier par fixture | Assisté | [E] bibliothèque, [I] niveau |
| 11.3.1 | Regroupement `autocomplete`/`name` + comparaison de libellés + juge texte | `unicode-normalization` 0.1.25, `strsim` 0.11.1 (MIT) | `field.labels` : à vérifier par fixture | Assisté | [I] |
| 11.3.2 | Idem + agrégation inter-pages | idem | idem | Assisté | [I] |
| 11.10.1 | Inventaire `required`/`aria-required` (+ indications visibles) | aucune | à vérifier par fixture | Assisté | [I] |
| 11.10.2 | Texte d'étiquette/`aria-describedby`/`::after` | aucune | `::after` : à vérifier par fixture | Assisté | [I] |
| 11.10.3 | Provocation d'erreur sans soumission + `aria-invalid` + juge | aucune | validation de contraintes et blocage réseau : à vérifier par fixture | Assisté | [I] |
| 11.10.4 | Idem, champs `aria-invalid` | aucune | idem | Assisté | [I] |
| 11.10.5 | Champs à format imposé + indication visible (juge) | aucune | idem | Assisté | [I] |
| 11.10.6 | Messages de format après erreur (juge) | aucune | idem | Assisté | [I] |
| 11.10.7 | Champs `aria-invalid` + instruction de format (juge) | aucune | idem | Assisté | [I] |
| 11.11.1 | Messages d'erreur + juge « type/format suggéré » | aucune | idem | Assisté | [I] |
| 11.11.2 | Messages d'erreur + juge « exemple donné » | aucune | idem | Assisté | [I] |
| 11.12.1 | Applicabilité heuristique + preuve visible (confirmation, étapes) | aucune | DOM ; tunnel multi-étapes non parcouru | Assisté ; après validation : Irréductible | [I] |
| 11.12.2 | Idem (récupération, confirmation) | aucune | idem | Assisté ; après validation : Irréductible | [I] |
| 12.10.1 | Écouteurs clavier imprimables + touche synthétique + mutation (règle ACT `ffbc54`, proposée) | aucune | `keydown` livré (carte) ; préchargement/`MutationObserver` : à vérifier par fixture | Assisté | [E] règle, [I] niveau |
| 12.11.1 | Révélation survol/focus/clic + focalisabilité des descendants | aucune | survol réel : à vérifier par fixture ; pas de Tab natif (carte) | Assisté | [I] |
| 13.1.4 | Inventaire d'indices client (minuteries, cookies, texte) — non concluant | aucune | lecture des cookies : à vérifier par fixture | Irréductible | [I] |
| 13.6.1 | Dictionnaire d'émoticônes + heuristique d'art ASCII + juge texte | `emojicon` 0.5.0 MIT (données : licence non vérifiée) | DOM ; à vérifier par fixture | Assisté | [I] |

## Points ouverts et suites

1. **Fixtures Obscura** : mesurer les sept capacités listées plus haut avant d'implémenter 7.4, 11.10, 11.11, 12.10, 12.11. Si le préchargement de script n'existe pas, 7.4 et 12.10 se rabattent sur les attributs inline et l'inventaire reste partiel [I].
2. **Divergences à trancher** avec la spec : la sonde 5.1 est plus laxiste que le RGAA ; la sonde 12.10 cible `accesskey` ; la sonde 11.3 émet `fail` sur une simple divergence de libellé.
3. **Calibration** : aucun juge texte de ce document n'est calibré ; la promotion éventuelle d'un test Assisté vers Automatique (8.7 `fail` sur longs passages, 11.10.2 `fail`) dépend de [#183](https://github.com/jamon8888/Holo-RGAA/issues/183).
4. **Non couvert** : 13.1.1 à 13.1.3 (autres tests de 13.1), 1.4 (CAPTCHA).

## Sources

- RGAA 4.1.2, critères et tests : https://accessibilite.numerique.gouv.fr/methode/criteres-et-tests/ — glossaire : https://accessibilite.numerique.gouv.fr/methode/glossaire/
- WCAG 2.1 : https://www.w3.org/TR/WCAG21/ — technique H86 : https://www.w3.org/WAI/WCAG21/Techniques/html/H86
- Règles ACT (liste) : https://www.w3.org/WAI/standards-guidelines/act/rules/ — `ffbc54` : https://www.w3.org/WAI/standards-guidelines/act/rules/ffbc54/proposed/ — `off6ek` : https://www.w3.org/WAI/standards-guidelines/act/rules/off6ek/proposed/ — `36b590` : https://www.w3.org/WAI/standards-guidelines/act/rules/36b590/proposed/
- HTML Standard, fonctionnalités obsolètes : https://html.spec.whatwg.org/multipage/obsolete.html
- `lingua` : https://crates.io/crates/lingua — https://github.com/pemistahl/lingua-rs — rapports : https://github.com/pemistahl/lingua-rs/tree/main/accuracy-reports
- crates.io : https://crates.io/crates/whatlang, https://crates.io/crates/whichlang, https://crates.io/crates/cld2, https://crates.io/crates/langram, https://crates.io/crates/unicode-script, https://crates.io/crates/unicode-normalization, https://crates.io/crates/strsim, https://crates.io/crates/html5ever, https://crates.io/crates/scraper, https://crates.io/crates/emojicon, https://crates.io/crates/emoticond, https://crates.io/crates/emojis, https://crates.io/crates/lingua-french-language-model
- `emojicon` : https://github.com/mominul/emojicon
- Recherches amont : [#249](https://github.com/jamon8888/Holo-RGAA/issues/249), [#250](https://github.com/jamon8888/Holo-RGAA/issues/250), [#251](https://github.com/jamon8888/Holo-RGAA/issues/251), [#333](https://github.com/jamon8888/Holo-RGAA/issues/333), [#336](https://github.com/jamon8888/Holo-RGAA/issues/336), [#337](https://github.com/jamon8888/Holo-RGAA/issues/337) ; carte [#330](https://github.com/jamon8888/Holo-RGAA/issues/330).
- Dépôt (branche `master`) : `rgaa-rs/crates/rgaa-rules/src/plan_snippets.rs`, `rgaa-rs/crates/rgaa-core/data/rgaa-4.1.2/mechanisms.toml`, `rgaa-rs/crates/rgaa-orchestrator/src/site_comparison.rs`, `rgaa-rs/crates/rgaa-obscura/src/`.
