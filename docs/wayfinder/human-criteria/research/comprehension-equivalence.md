# Recherche : outillage des tests de compréhension et d'équivalence

Ticket : [#337](https://github.com/jamon8888/Holo-RGAA/issues/337), carte [#330](https://github.com/jamon8888/Holo-RGAA/issues/330).
Date : 2026-10-12. Vocabulaire (Test RGAA, Automatique, Assisté, Irréductible) : `CONTEXT.md`.

**Question** : quelle technique, à l'état de l'art, outille les tests 5.3, 7.2, 7.5, 9.4, 10.3, 12.3 test 1, 13.10 à 13.12, et jusqu'à quel niveau ?

**Réponse courte** : aucun de ces tests ne peut rendre `pass` seul. Tous relèvent d'un jugement d'équivalence ou de compréhension : « contenus et fonctionnalités similaires », « compréhensible », « représentatif », « message de statut », « citation », « hors cas particuliers ». La machine apporte beaucoup sur l'inventaire et sur la détection d'indices d'échec. Tous les tests sont donc classés **Assisté**. Trois exceptions partielles sont à valider par fixture : la sous-condition `role="presentation"` de 5.3, l'inapplicabilité (NA) de 13.12, et la détection d'action sur l'événement descendant en 13.11. Aucun test n'est classé Irréductible, mais 9.4 et 12.3.1 n'apportent qu'un indice faible. Côté Obscura, la lecture du code source à `2bda71a` (et au tag `v0.2.4`) **infirme trois hypothèses du ticket** :

- `Emulation.setScriptExecutionDisabled` est sans effet ;
- `DOMDebugger.getEventListeners` est rejeté ;
- l'arbre AX d'Obscura ne connaît ni `status` ni `log` ni `progressbar`, ni les propriétés `live`/`atomic`.

Il faut donc passer par d'autres voies.

Légende des sources : **[S]** établi par la source citée · **[I]** inférence de l'auteur · **[?]** inconnu / non vérifié.

---

## 1. Faits établis sur Obscura (lecture du code source)

Sources lues : `h4ckf0r0day/obscura` à `main` = `2bda71a5` (commit du 2026-10-10), recoupé pour les points 1 à 5 au tag `v0.2.4` (dernière release, 2026-10-04). Le dépôt épingle encore la v0.2.2 dans `ci.yml` et `rgaa-audit.yml` ; **le comportement en v0.2.2 n'est pas vérifié**. Licence d'Obscura : Apache-2.0 [S] (API GitHub du dépôt).

| # | Fait | Conséquence | Source |
|---|---|---|---|
| 1 | `Emulation.*` : toute méthode non listée tombe dans `_ => Ok(json!({}))`. `setScriptExecutionDisabled` n'est pas implémentée : elle est **acquittée et sans effet**. | 7.2 : on ne peut pas obtenir le rendu « JS désactivé » par CDP sous Obscura. | [S] [emulation.rs L176](https://github.com/h4ckf0r0day/obscura/blob/2bda71a557247bd2d6fabf01fa66eb4b07dcb2e8/crates/obscura-cdp/src/domains/emulation.rs#L176) |
| 2 | Le domaine `DOMDebugger` est absent du routeur : la réponse est `Err("Unknown domain")`. `DeviceOrientation` est absent lui aussi. Les domaines `CSS` et `Overlay` sont « acceptés mais no-op ». | 13.10 à 13.12 : pas d'inventaire des écouteurs par CDP, pas d'émulation d'orientation. 10.3 : `CSS.*` ne permet pas de désactiver les styles. | [S] [dispatch.rs L798-L831](https://github.com/h4ckf0r0day/obscura/blob/2bda71a557247bd2d6fabf01fa66eb4b07dcb2e8/crates/obscura-cdp/src/dispatch.rs#L798-L831) |
| 3 | `Input.dispatchTouchEvent` renvoie `Ok({})` sans rien faire. `Input.dispatchMouseEvent` est implémenté. | 13.10 : aucune simulation multipoint par CDP. 13.11 : la souris permet de séparer appui et relâchement. | [S] [input.rs L147, L514](https://github.com/h4ckf0r0day/obscura/blob/2bda71a557247bd2d6fabf01fa66eb4b07dcb2e8/crates/obscura-cdp/src/domains/input.rs#L514) |
| 4 | `Accessibility.getFullAXTree` : `map_role` ne reconnaît qu'une liste fermée de rôles explicites. `status`, `log`, `progressbar`, `marquee` et `timer` deviennent `generic` ; seul `alert` est conservé. Aucune occurrence de `live` dans `accessibility.rs`. | 7.5 : il faut lire les attributs `role`/`aria-live`/`aria-atomic` **dans le DOM**, pas dans l'arbre AX d'Obscura. | [S] [accessibility.rs L182-L224](https://github.com/h4ckf0r0day/obscura/blob/2bda71a557247bd2d6fabf01fa66eb4b07dcb2e8/crates/obscura-cdp/src/domains/accessibility.rs#L182-L224) |
| 5 | `Page.addScriptToEvaluateOnNewDocument` est implémenté (y compris `runImmediately` et l'injection dans les cadres). | Voie de remplacement pour inventorier les écouteurs : surcharger `EventTarget.prototype.addEventListener` avant les scripts de la page. **L'ordre d'exécution réel par rapport aux scripts de la page reste à vérifier par fixture.** | [S] [page.rs L1539](https://github.com/h4ckf0r0day/obscura/blob/2bda71a557247bd2d6fabf01fa66eb4b07dcb2e8/crates/obscura-cdp/src/domains/page.rs#L1539) |
| 6 | `DOMSnapshot.captureSnapshot` **synthétise** les boîtes : une pile verticale, sans vraie géométrie. | 10.3 : ne pas utiliser `DOMSnapshot` pour l'ordre visuel. | [S] [domsnapshot.rs L1-L15](https://github.com/h4ckf0r0day/obscura/blob/2bda71a557247bd2d6fabf01fa66eb4b07dcb2e8/crates/obscura-cdp/src/domains/domsnapshot.rs#L1-L15) |
| 7 | La vraie géométrie (`getBoundingClientRect`, `elementFromPoint`) n'existe qu'avec la feature `render` (`obscura-render`), qui est **désactivée par défaut**. `DOM.getBoxModel` délègue à `getBoundingClientRect`. | 10.3 : l'ordre visuel n'est mesurable qu'avec l'artefact « render », jamais avec `-no-render`. Je n'ai pas vérifié que l'artefact `obscura-x86_64-linux.tar.gz` utilisé en CI est bien un build `render` **[?]**. | [S] [obscura-js/Cargo.toml](https://github.com/h4ckf0r0day/obscura/blob/2bda71a557247bd2d6fabf01fa66eb4b07dcb2e8/crates/obscura-js/Cargo.toml), [obscura-render/src/lib.rs](https://github.com/h4ckf0r0day/obscura/blob/2bda71a557247bd2d6fabf01fa66eb4b07dcb2e8/crates/obscura-render/src/lib.rs), [dom.rs L296](https://github.com/h4ckf0r0day/obscura/blob/2bda71a557247bd2d6fabf01fa66eb4b07dcb2e8/crates/obscura-cdp/src/domains/dom.rs#L296) |
| 8 | Faits **fournis par la carte, non revérifiés ici** : `MutationObserver` honore les filtres d'attributs depuis la 0.2.4 ; navigation Tab native absente ; `getComputedStyle().direction` vide. | — | carte #330 |

Non vérifié **[?]** : la recherche de code GitHub ne trouve ni `DeviceMotionEvent`, ni le constructeur `TouchEvent`, ni le support de `CSSStyleSheet.disabled` dans Obscura. Cette recherche indexe mal le dépôt : elle ne trouve pas non plus `MutationObserver`, qui existe pourtant. **Il faut donc vérifier par fixture**, pas conclure à l'absence.

Pour comparaison, Chrome expose bien tout cela [S] ([browser_protocol.json](https://github.com/ChromeDevTools/devtools-protocol/blob/master/json/browser_protocol.json)) :

- [`DOMDebugger.getEventListeners`](https://chromedevtools.github.io/devtools-protocol/tot/DOMDebugger/#method-getEventListeners), avec `depth: -1` et `pierce`. Champs renvoyés : `type`, `useCapture`, `passive`, `once`, `scriptId`, `lineNumber`, `handler`, `backendNodeId`.
- [`Emulation.setScriptExecutionDisabled`](https://chromedevtools.github.io/devtools-protocol/tot/Emulation/#method-setScriptExecutionDisabled), stable.
- [`Emulation.setSensorOverrideEnabled`](https://chromedevtools.github.io/devtools-protocol/tot/Emulation/#method-setSensorOverrideEnabled), expérimental. Capteurs : `accelerometer`, `gyroscope`, `absolute-orientation`, etc.
- [`DeviceOrientation.setDeviceOrientationOverride`](https://chromedevtools.github.io/devtools-protocol/tot/DeviceOrientation/).
- [`DOMSnapshot.captureSnapshot`](https://chromedevtools.github.io/devtools-protocol/tot/DOMSnapshot/#method-captureSnapshot), avec `includePaintOrder` et `includeDOMRects`.
- Côté arbre AX, les propriétés `live`, `atomic`, `relevant` ([`AXPropertyName`](https://chromedevtools.github.io/devtools-protocol/tot/Accessibility/#type-AXPropertyName)).

**[I]** Un repli Chromium ponctuel, pour les seuls tests qui en ont besoin, est donc techniquement plus riche. Il sort cependant des préférences de la carte (Obscura comme substrat) et n'est pas évalué ici.

## 2. État de l'art transversal

- **Règles ACT** : la liste W3C n'a **aucune règle** pour 1.3.2 (ordre), 4.1.3 (messages de statut), 2.5.1, 2.5.2 et 2.4.5. Pour 2.5.4, elle en compte **deux, au statut « proposed »** : [7677a9](https://www.w3.org/WAI/standards-guidelines/act/rules/7677a9/proposed/) (alternative d'interface) et [c249d5](https://www.w3.org/WAI/standards-guidelines/act/rules/c249d5/proposed/) (désactivation). Leur seule implémentation déclarée est UsableNet AQA, un outil semi-automatique propriétaire, donc exclu. [S] [liste des règles ACT](https://www.w3.org/WAI/standards-guidelines/act/rules/)
- **Ordre visuel / ordre DOM** : l'audit Lighthouse « Visual order on the page follows DOM order » est classé **contrôle manuel** [S] ([Chrome for Developers](https://developer.chrome.com/docs/lighthouse/accessibility/visual-order-follows-dom)). La procédure de la technique WCAG C27 consiste à examiner visuellement puis à comparer au DOM [S] ([C27](https://www.w3.org/WAI/WCAG22/Techniques/css/C27)). **Je n'ai trouvé aucun outil open source qui conclue automatiquement sur l'ordre de lecture.** [?] Recherche non exhaustive.
- **Messages de statut** : [`@guidepup/virtual-screen-reader`](https://github.com/guidepup/virtual-screen-reader) (MIT [S]) simule un lecteur d'écran sur un DOM.
  - Son module [`getLiveSpokenPhrase.ts`](https://github.com/guidepup/virtual-screen-reader/blob/main/src/getLiveSpokenPhrase.ts) calcule les annonces des régions live à partir d'enregistrements `MutationObserver`. Il traite `aria-live`, `aria-atomic`, `aria-relevant` et les rôles implicites (`alert`, `log`, `status`, `timer`, `marquee`). `aria-busy` est noté « TODO » [S].
  - Le README précise que le paquet complète les tests avec un vrai lecteur d'écran sans les remplacer [S].
  - [`@guidepup/guidepup`](https://github.com/guidepup/guidepup) (MIT [S]) pilote NVDA ou VoiceOver réels, donc Windows ou macOS **[I]**. Il est hors du pipeline Linux headless.
- **Tableaux de mise en forme** : deux heuristiques « données vs mise en forme » existent.
  - axe-core : [`lib/commons/table/is-data-table.js`](https://github.com/dequelabs/axe-core/blob/develop/lib/commons/table/is-data-table.js), licence MPL-2.0 [S], déjà utilisé par le projet (embarquement dans le dépôt décidé par la carte #330).
  - Firefox : [`HTMLTableAccessible::IsProbablyLayoutTable`](https://github.com/mozilla-firefox/firefox/blob/main/accessible/html/HTMLTableAccessible.cpp), MPL-2.0, à prendre comme référence d'algorithme seulement.
- **Écouteurs et délégation** : depuis React 17, les gestionnaires sont attachés au conteneur racine (`rootNode.addEventListener()`) [S] ([blog React 17](https://legacy.reactjs.org/blog/2020/10/20/react-v17.html)). **[I]** Un inventaire d'écouteurs, par CDP ou par surcharge, attribue donc souvent `pointer*`/`touch*` à la racine et non au composant. Il signale qu'un écouteur existe, sans dire quelle fonctionnalité il sert.

---

## 3. Par test RGAA

La méthodologie officielle de chaque test est citée depuis le dépôt DINUM [`DISIC/accessibilite.numerique.gouv.fr`](https://github.com/DISIC/accessibilite.numerique.gouv.fr/tree/main/src/rgaa/criteres), fichiers `src/rgaa/criteres/<critère>/tests/<n>.md`.

### 5.3.1 — linéarisation d'un tableau de mise en forme

- **Exigence** [S] ([5.3/tests/1.md](https://github.com/DISIC/accessibilite.numerique.gouv.fr/blob/main/src/rgaa/criteres/5.3/tests/1.md)) : retrouver les tableaux de mise en forme, puis vérifier (a) que l'ordre d'accès aux cellules est cohérent avec le contenu et (b) que `<table>` porte `role="presentation"`.
- **Technique** :
  1. Qualifier chaque `<table>` comme table de données ou de mise en forme, avec l'heuristique axe `isDataTable`.
  2. Vérifier la sous-condition (b) de façon déterministe.
  3. Produire le texte linéarisé, cellules dans l'ordre du DOM, en Rust natif.
  4. Le soumettre au modèle local avec la capture de la table : « l'ordre de lecture reste-t-il cohérent ? ».
- **Bibliothèque** : axe-core (MPL-2.0, déjà utilisé par le projet, compatible avec une distribution MIT tant que les fichiers MPL ne sont pas modifiés, ou que leurs modifications restent publiées sous MPL [S] [FAQ MPL](https://www.mozilla.org/en-US/MPL/2.0/FAQ/)).
- **Obscura** :
  - axe tourne déjà sous Obscura [I] (projet).
  - L'arbre AX d'Obscura n'applique pas d'heuristique « table de mise en forme » [S] (fait 4 : `table` explicite seulement). On ne peut pas s'en servir pour qualifier.
- **Niveau : Assisté.**
  - La sous-condition (b) serait Automatique **si** la qualification « mise en forme » l'était. Or c'est une heuristique, et une table de données sans `<th>` peut être classée à tort [I]. Proposition : `fail` candidat en `review`.
  - (a) est un jugement de compréhension.

### 7.2.1 — alternative à un script

- **Exigence** [S] ([7.2/tests/1.md](https://github.com/DISIC/accessibilite.numerique.gouv.fr/blob/main/src/rgaa/criteres/7.2/tests/1.md)) : retrouver les alternatives (`<noscript>`, page alternative, alternative dans la page), désactiver JavaScript, vérifier « contenus et fonctionnalités similaires ». Le glossaire précise que le moyen d'accéder à l'alternative doit être fourni par le site [S] ([alternative-a-script.md](https://github.com/DISIC/accessibilite.numerique.gouv.fr/blob/main/src/rgaa/glossaire/alternative-a-script.md)).
- **Technique** :
  1. Construire la vue « JS désactivé » **sans Obscura** : reparser le HTML brut de la réponse HTTP avec `html5ever` réglé sur `scripting_enabled: false`. Le contenu de `<noscript>` est alors un vrai sous-arbre. [S] [docs.rs html5ever 0.29.1 `TreeBuilderOpts`](https://docs.rs/html5ever/0.29.1/html5ever/tree_builder/struct.TreeBuilderOpts.html). `html5ever` 0.29.1 est déjà dans `Cargo.lock` sous licence MIT OR Apache-2.0 [S].
  2. Comparer avec le DOM rendu sous Obscura (JS actif) : texte visible, liens (`href`), contrôles de formulaire et `action`, titres.
  3. Le diff structuré et les deux captures vont au modèle local pour un jugement « similaire ».
  - Variante à vérifier par fixture : rendre la vue sans JS dans Obscura en bloquant les scripts via le domaine `Fetch` et en retirant les scripts inline.
- **Obscura** : `setScriptExecutionDisabled` est sans effet [S] (fait 1). **Ne pas l'utiliser** : un rendu « JS désactivé » obtenu ainsi serait en réalité un rendu JS actif, donc un faux `pass`.
- **Niveau : Assisté.**
  - Le diff est déterministe ; l'équivalence fonctionnelle ne l'est pas.
  - Indice d'échec fort [I] : une alternative déclarée (`<noscript>`) vide ou réduite à « activez JavaScript ».

### 7.2.2 — alternative d'un élément non textuel mis à jour par script

- **Exigence** [S] ([7.2/tests/2.md](https://github.com/DISIC/accessibilite.numerique.gouv.fr/blob/main/src/rgaa/criteres/7.2/tests/2.md)) : l'alternative est mise à jour et reste pertinente.
- **Technique** :
  1. Poser un `MutationObserver` avec `attributeFilter: ['src','srcset','alt','aria-label','aria-labelledby','title']` sur les éléments non textuels.
  2. Relever les couples (changement de `src`, changement ou non de l'alternative).
  3. Juger la pertinence avec le modèle local, en lui fournissant la nouvelle image et la nouvelle alternative.
- **Obscura** : filtres d'attributs honorés depuis la 0.2.4 (fait fourni par la carte). Les déclencheurs (carrousel, minuterie) restent à provoquer par interaction ou attente.
- **Niveau : Assisté.** Un `src` modifié avec un `alt` inchangé est un `fail` candidat [I]. Ce n'est pas un échec certain : une alternative générique peut rester pertinente.

### 7.5.1, 7.5.2, 7.5.3 — messages de statut

- **Exigence** [S] ([7.5/tests/1.md](https://github.com/DISIC/accessibilite.numerique.gouv.fr/blob/main/src/rgaa/criteres/7.5/tests/1.md)) : retrouver les messages qui valent message de statut, déterminer leur nature (réussite ou état → `status` ; erreur ou suggestion → `alert` ; progression → `log`/`progressbar`/`status`), vérifier le rôle ou l'équivalent `aria-live` + `aria-atomic`. Les tests 2 et 3 renvoient à la même méthodologie.
  - Définition RGAA [S] ([message-de-statut.md](https://github.com/DISIC/accessibilite.numerique.gouv.fr/blob/main/src/rgaa/glossaire/message-de-statut.md)) : pas de changement de contexte, pas de déplacement du focus.
  - WCAG 4.1.3 précise que le critère n'oblige pas à créer des messages : il s'applique à ceux qui existent [S] ([Understanding 4.1.3](https://www.w3.org/WAI/WCAG22/Understanding/status-messages.html)).
  - ARIA : la région live devrait exister, vide, le plus tôt possible avant la mise à jour [S] ([WAI-ARIA, `aria-live`](https://w3c.github.io/aria/#aria-live), éditeur 1.3).
- **Technique** :
  1. Observer `childList`, `characterData` et `subtree` via un `MutationObserver` posé avant les interactions.
  2. Provoquer les actions : soumission de formulaire vide, soumission valide sur fixture, boutons, ajout au panier, etc.
  3. Pour chaque texte inséré ou modifié **sans déplacement du focus**, calculer la région live ancêtre depuis les **attributs DOM** (`role`, `aria-live`, `aria-atomic`), avec les équivalences listées dans la note technique RGAA 7.5.
  4. Vérifier que la région existait **avant** la mutation.
  5. Faire classer la nature du message (réussite / erreur / progression) par le modèle local.
  - Le calcul « ce qui serait annoncé » peut s'inspirer de `getLiveSpokenPhrase.ts` (Guidepup, MIT). Sa réimplémentation en JS injecté ou en Rust est **[I]** faisable ; l'embarquer tel quel suppose un bundle JS **[?]**.
- **Obscura** :
  - L'arbre AX ignore `status`/`log`/`progressbar` et n'expose pas `live` [S] (fait 4) : **lire le DOM**.
  - `MutationObserver` est disponible (carte).
  - L'enchaînement des interactions reste dépendant des événements synthétiques ; la navigation Tab native est absente (carte).
- **Niveau : Assisté.**
  - L'inventaire exhaustif des messages de statut demande d'explorer toutes les interactions, ce qui est impossible en général [I]. Un `pass` automatique est donc exclu.
  - Le classement de la nature d'un message est un jugement (modèle → `review`).
  - Indices d'échec forts [I] :
    - texte d'erreur inséré hors de toute région live ;
    - région live créée en même temps que son contenu ;
    - `role="alert"` sur un message de réussite.

### 9.4.1 et 9.4.2 — citations courtes et blocs de citation

- **Exigence** [S] ([9.4/tests/1.md](https://github.com/DISIC/accessibilite.numerique.gouv.fr/blob/main/src/rgaa/criteres/9.4/tests/1.md), [9.4/tests/2.md](https://github.com/DISIC/accessibilite.numerique.gouv.fr/blob/main/src/rgaa/criteres/9.4/tests/2.md)) : retrouver les citations courtes (ou en ligne) et les blocs de citation ; vérifier `<q>` ou `<blockquote>`.
- **Technique** :
  1. Générer des candidats en Rust natif :
     - segments entre « », “ ” ou " " suivis ou précédés d'une attribution (tiret, verbe de parole, nom propre) ;
     - paragraphes en retrait ou en italique avec une source ;
     - `<blockquote>` utilisé pour l'indentation (usage inverse).
  2. Faire classer chaque candidat par le modèle local : citation ou non, courte ou bloc.
  3. La sonde existante `probe-9-4` (`mechanisms.toml`, sortie `review`) couvre déjà la partie balisage.
- **Bibliothèque** : **aucune bibliothèque open source spécialisée trouvée** [?]. Recherche non exhaustive ; je n'en connais pas.
- **Obscura** : DOM et texte suffisent ; aucun besoin propre au navigateur.
- **Niveau : Assisté, valeur faible.** Une citation sans aucune marque typographique est indétectable autrement que par la lecture [I]. La présence de `<q>`/`<blockquote>` ne prouve rien sur les citations qui ne sont pas balisées. Ce niveau est proche d'Irréductible ; je le garde en Assisté parce que les candidats typographiques sont un vrai indice d'échec.

### 10.3.1 — information compréhensible sans feuilles de styles

- **Exigence** [S] ([10.3/tests/1.md](https://github.com/DISIC/accessibilite.numerique.gouv.fr/blob/main/src/rgaa/criteres/10.3/tests/1.md)) : désactiver les CSS, vérifier que l'ordre d'implémentation ne pose pas de problème de compréhension. « Compréhensible » signifie « lisible (ordre logique) et cohérent » [S] ([glossaire](https://github.com/DISIC/accessibilite.numerique.gouv.fr/blob/main/src/rgaa/glossaire/comprehensible-ordre-de-lecture.md)). Les CSS incluent les styles externes, embarqués et en ligne [S] ([feuille-de-style.md](https://github.com/DISIC/accessibilite.numerique.gouv.fr/blob/main/src/rgaa/glossaire/feuille-de-style.md)).
- **Technique** :
  1. **Linéarisation « sans CSS »** calculée depuis le DOM rendu, en Rust natif : texte et alternatives dans l'ordre du DOM, en excluant `head`, `script`, `style`, `template`, `[hidden]`, et en **incluant** le contenu masqué par CSS. **[I]** C'est ce qu'un auditeur voit styles coupés.
  2. **Indices de réordonnancement** :
     - éléments dont l'ordre visuel (haut→bas, puis début→fin de ligne, mesuré avec `getBoundingClientRect`) inverse l'ordre du DOM ;
     - propriétés `order`, `flex-direction: *-reverse`, placement explicite en grille, `position: absolute/fixed`, `float` ;
     - contenu porteur d'information injecté par `::before`/`::after` ou en image de fond, perdu sans CSS.
  3. Fournir au modèle local et à l'auditeur la linéarisation, la liste des inversions et la capture de la page.
- **Bibliothèque** : **aucun outil open source concluant trouvé**. Lighthouse : manuel [S] ; C27 : manuel [S].
- **Obscura** :
  - Pas de désactivation via le domaine `CSS` (no-op [S], fait 2).
  - Vraie géométrie uniquement avec le build `render` [S] (fait 7).
  - `DOMSnapshot` est synthétique [S] (fait 6).
  - Couper les styles en JS (`styleSheets[i].disabled`, retrait des `style`) : **à vérifier par fixture** [?].
  - La linéarisation « sans CSS » ne demande pas de couper les styles : elle se déduit du DOM [I].
- **Niveau : Assisté.** L'inversion d'ordre est un indice mesurable ; la compréhensibilité ne l'est pas.

### 12.3.1 — plan du site représentatif de l'architecture

- **Exigence** [S] ([12.3/tests/1.md](https://github.com/DISIC/accessibilite.numerique.gouv.fr/blob/main/src/rgaa/criteres/12.3/tests/1.md)) : le plan est « représentatif de l'architecture générale ». Note : toutes les pages ne doivent pas y figurer si elles sont atteignables depuis une rubrique.
  - Le glossaire exige qu'à partir du plan du site l'utilisateur puisse atteindre l'ensemble des pages [S] ([page-plan-du-site.md](https://github.com/DISIC/accessibilite.numerique.gouv.fr/blob/main/src/rgaa/glossaire/page-plan-du-site.md)).
  - WCAG 2.4.5 : aucune règle ACT [S].
- **Technique** :
  1. Localiser le plan (déjà fait par `site-12-4-sitemap-access`).
  2. Extraire son arbre de liens (listes imbriquées).
  3. Le confronter au graphe du crawl `spider` (crate `spider` 2.53.9, MIT [S] [crates.io](https://crates.io/crates/spider)) et, s'il existe, à `sitemap.xml` :
     - couverture des rubriques de la navigation principale ;
     - **atteignabilité** : part des pages crawlées atteignables par parcours en largeur depuis les liens du plan ;
     - profondeur.
  4. Faire juger « représentatif » par le modèle local et l'auditeur.
- **Obscura** : non requis au-delà du rendu de la page du plan ; c'est le crawl qui compte.
- **Niveau : Assisté, valeur moyenne.** Une atteignabilité faible est un indice d'échec, mais le crawl lui-même est incomplet [I]. « Représentatif » est un jugement.
- 12.3.2 (liens fonctionnels), hors ticket : vérifiable par statut HTTP **[I]**, donc candidat Automatique.

### 13.10.1 et 13.10.2 — gestes complexes (multipoint, trajectoire)

- **Exigence** [S] ([13.10/tests/1.md](https://github.com/DISIC/accessibilite.numerique.gouv.fr/blob/main/src/rgaa/criteres/13.10/tests/1.md), [tests/2.md](https://github.com/DISIC/accessibilite.numerique.gouv.fr/blob/main/src/rgaa/criteres/13.10/tests/2.md)) : chaque fonctionnalité multipoint ou à trajectoire doit rester disponible par un contact unique. Cas particuliers : gestes du navigateur ou du système, et gestes essentiels.
  - WCAG : le glisser-déposer (seuls le départ et l'arrivée comptent) **n'est pas** un geste à trajectoire ; échec F105 [S] ([Understanding 2.5.1](https://www.w3.org/WAI/WCAG22/Understanding/pointer-gestures.html)).
- **Technique** :
  1. Inventaire par **surcharge** injectée tôt (`addScriptToEvaluateOnNewDocument`) de `EventTarget.prototype.addEventListener`, plus les propriétés `on*` : types `touchstart/move/end`, `pointerdown/move/up`, `gesture*`, `wheel`, avec la cible et la pile d'appel (fichier, ligne).
  2. Lire `touch-action` dans les styles calculés.
  3. Signaux multipoint : lecture de `touches.length`, suivi de plusieurs `pointerId`.
  4. Signaux trajectoire : `pointermove`/`touchmove` avec calcul de direction, bibliothèques de gestes présentes [I].
  5. Chercher les alternatives candidates : boutons « précédent/suivant », « + / − » à proximité.
  6. Le modèle local et l'auditeur jugent l'équivalence.
- **Obscura** :
  - `DOMDebugger` rejeté [S] (fait 2) → surcharge JS obligatoire ; ordre d'injection **à vérifier par fixture**.
  - `dispatchTouchEvent` no-op [S] (fait 3) → **aucune simulation multipoint** par CDP. Des `TouchEvent` synthétiques en JS restent **à vérifier par fixture**.
  - Délégation React → attribution au composant imprécise [S]/[I].
- **Niveau : Assisté.**
  - Aucun écouteur `touch*`/`pointer*`/`gesture*` nulle part (y compris `window`, `document` et les cadres) → **NA candidat**, à valider par fixture. Ce verdict ne serait Automatique que si l'inventaire est prouvé complet [I]. Je ne le prouve pas ici.

### 13.11.1 — annulation d'une action au pointeur

- **Exigence** [S] ([13.11/tests/1.md](https://github.com/DISIC/accessibilite.numerique.gouv.fr/blob/main/src/rgaa/criteres/13.11/tests/1.md)) : l'action est déclenchée au relâchement, ou déclenchée à l'appui puis annulée au relâchement, ou un mécanisme d'abandon ou d'annulation existe. NA si l'appui est essentiel, par exemple un émulateur de clavier.
  - WCAG : No Down-Event / Abort or Undo / Up Reversal / Essential, échec F101 [S] ([Understanding 2.5.2](https://www.w3.org/WAI/WCAG22/Understanding/pointer-cancellation.html)).
- **Technique** :
  1. Inventorier les écouteurs `mousedown`/`pointerdown`/`touchstart` (même surcharge qu'en 13.10).
  2. **Test comportemental** : pour chaque élément cliquable, envoyer `Input.dispatchMouseEvent` `mousePressed` **seul**.
  3. Observer pendant quelques centaines de millisecondes : mutations, navigation, requêtes réseau, ouverture de dialogue.
  4. Envoyer `mouseReleased` **hors de la cible**, puis observer si l'effet est annulé (Up Reversal).
- **Obscura** : `dispatchMouseEvent` est implémenté [S] (fait 3). La fidélité de la séquence `pointerdown`/`mousedown` produite reste **à vérifier par fixture**.
- **Niveau : Assisté.**
  - « Effet à l'appui, non annulé au relâchement » est un `fail` candidat fort [I].
  - Il reste en `review` à cause de la branche « mécanisme d'annulation » (dialogue, retour arrière) et de l'exception « essentiel », qui relèvent du jugement.
  - Proposition à arbitrer (grilling) : Automatique `fail` si l'effet à l'appui est une **navigation** ou une **soumission** sans confirmation.

### 13.12.1, 13.12.2, 13.12.3 — mouvement de l'appareil

- **Exigence** [S] ([13.12/tests/1.md](https://github.com/DISIC/accessibilite.numerique.gouv.fr/blob/main/src/rgaa/criteres/13.12/tests/1.md), [2.md](https://github.com/DISIC/accessibilite.numerique.gouv.fr/blob/main/src/rgaa/criteres/13.12/tests/2.md), [3.md](https://github.com/DISIC/accessibilite.numerique.gouv.fr/blob/main/src/rgaa/criteres/13.12/tests/3.md)) :
  - test 1 : alternative en composants d'interface pour le mouvement de l'appareil ;
  - test 2 : idem pour un geste en direction de l'appareil ;
  - test 3 : désactivation possible de la détection.
  - WCAG 2.5.4 inclut les gestes face à une caméra ; techniques G213 et F106 [S] ([Understanding 2.5.4](https://www.w3.org/WAI/WCAG22/Understanding/motion-actuation.html)).
  - Règles ACT 7677a9 et c249d5 (proposed) : applicabilité = documents ayant des écouteurs `devicemotion`/`deviceorientation`. Elles **supposent** que le mouvement n'est ni essentiel ni accessibility-supported [S].
- **Technique** :
  1. Inventorier par surcharge : écouteurs `devicemotion`/`deviceorientation`/`deviceorientationabsolute`, `window.ondevicemotion`, constructeurs Generic Sensor (`Accelerometer`, `Gyroscope`…), et pour le test 2 `navigator.mediaDevices.getUserMedia({video})` [I].
  2. Si l'applicabilité est établie, déclencher un événement synthétique, `dispatchEvent(new DeviceMotionEvent(...))` en JS, puis observer les mutations.
  3. Chercher une commande d'interface équivalente et un interrupteur de désactivation ; rejouer l'événement après activation de l'interrupteur (test 3).
- **Obscura** : `DeviceOrientation` est un domaine inconnu, `Emulation.setSensorOverride*` n'est pas implémentée (catch-all no-op) [S] (faits 1 et 2). L'existence de `DeviceMotionEvent` dans le JS d'Obscura est **à vérifier par fixture** [?].
- **Niveau : Assisté**, avec **NA candidat Automatique** : aucune API de mouvement, de capteur ou de caméra n'est touchée, et l'inventaire est validé par fixture. C'est l'applicabilité même des règles ACT [S]/[I].
  - Tests 1 et 2 applicables : Assisté (équivalence fonctionnelle).
  - Test 3 : Assisté (trouver l'interrupteur et rejouer l'événement).

---

## 4. Tableau de synthèse

| Test RGAA | Technique principale | Bibliothèque (licence) | Support Obscura | Niveau |
|---|---|---|---|---|
| 5.3.1 | Qualification par heuristique `isDataTable`, `role=presentation` déterministe, linéarisation des cellules + modèle local | axe-core (MPL-2.0, déjà utilisé) | axe : oui. Arbre AX : pas d'heuristique [S] | **Assisté** |
| 7.2.1 | Diff HTML brut (`html5ever`, `scripting_enabled=false`) vs DOM rendu + modèle local | html5ever 0.29.1 (MIT/Apache-2.0) | `setScriptExecutionDisabled` **no-op** [S] ; vue sans JS dans Obscura via `Fetch` : à vérifier par fixture | **Assisté** |
| 7.2.2 | `MutationObserver` (`src`/`alt`…) + modèle local | — | filtres d'attributs OK depuis 0.2.4 (carte) | **Assisté** |
| 7.5.1–3 | Interactions + `MutationObserver` + région live calculée **depuis le DOM** + classement de la nature par le modèle local | logique inspirée de `getLiveSpokenPhrase` (Guidepup VSR, MIT) | arbre AX sans `status`/`log`/`live` [S] → DOM ; MO OK | **Assisté** |
| 9.4.1–2 | Candidats typographiques + modèle local | aucune trouvée | DOM suffit | **Assisté** (faible) |
| 10.3.1 | Linéarisation du DOM « sans CSS » + inversions ordre visuel/DOM + modèle local | aucune concluante (Lighthouse/C27 manuels [S]) | géométrie réelle seulement en build `render` [S] ; `CSS.*` no-op ; coupure des styles en JS : à vérifier par fixture | **Assisté** |
| 12.3.1 | Arbre du plan vs graphe de crawl : couverture, atteignabilité, profondeur + modèle local | spider 2.53.9 (MIT, déjà présent) | non critique | **Assisté** (moyen) |
| 13.10.1–2 | Inventaire par surcharge de `addEventListener` + `touch-action` + alternatives candidates | — | `DOMDebugger` **rejeté** [S] ; `dispatchTouchEvent` **no-op** [S] ; injection précoce : à vérifier par fixture | **Assisté** (NA candidat) |
| 13.11.1 | `mousePressed` seul puis relâchement hors cible, observation des effets | — | `dispatchMouseEvent` implémenté [S] ; fidélité `pointerdown` : à vérifier par fixture | **Assisté** |
| 13.12.1–3 | Inventaire mouvement / capteurs / caméra + événement synthétique + interrupteur | — | `DeviceOrientation` inconnu, capteurs no-op [S] ; `DeviceMotionEvent` : à vérifier par fixture | **Assisté** (NA candidat Automatique) |

**Licences** : toutes les bibliothèques retenues sont MIT, Apache-2.0 ou MPL-2.0.

- MIT et Apache-2.0 sont compatibles avec une distribution sous MIT.
- MPL-2.0 (axe-core) est compatible avec une œuvre plus large sous MIT si les fichiers MPL restent sous MPL [S] FAQ MPL.
- Aucune dépendance AGPL n'est proposée.
- Firefox (MPL-2.0) sert de référence d'algorithme, pas de dépendance.
- Guidepup (MIT) pilote un vrai lecteur d'écran et sort du pipeline headless Linux [I].

## 5. Fixtures à écrire pour lever les « à vérifier »

1. `addScriptToEvaluateOnNewDocument` s'exécute-t-il **avant** un `<script>` inline en tête de document, et dans les `iframe` ? (13.10 à 13.12)
2. `new DeviceMotionEvent('devicemotion', …)` et `new TouchEvent(...)` existent-ils dans le JS d'Obscura, et un écouteur les reçoit-il ? (13.10, 13.12)
3. `Input.dispatchMouseEvent mousePressed` déclenche-t-il `pointerdown` puis `mousedown`, sans `click` tant que `mouseReleased` n'est pas envoyé ? (13.11)
4. `document.styleSheets[i].disabled = true` modifie-t-il le style calculé et la géométrie dans le build `render` ? (10.3)
5. L'artefact `obscura-x86_64-linux.tar.gz` de la CI est-il un build `render` (`getBoundingClientRect` non nul et cohérent) ? (10.3)
6. Le domaine `Fetch` permet-il de bloquer les scripts pour produire une vue sans JS rendue ? (7.2.1)
7. Le comportement des faits 1 à 5 est-il identique en v0.2.2, la version épinglée ? Non vérifié.

## 6. Ce qui reste inconnu

- Fiabilité des verdicts du modèle local sur ces jugements : non mesurée. Elle relève de la calibration (carte #183). Tant que ce n'est pas fait, le niveau reste Assisté par construction (carte #330, Notes).
- Existence d'un outil open source qui conclut sur l'ordre de lecture ou sur les messages de statut : **aucun trouvé** ; la recherche n'est pas exhaustive.
- Comportement d'Obscura en v0.2.2 : seuls `main@2bda71a` et `v0.2.4` ont été lus.
