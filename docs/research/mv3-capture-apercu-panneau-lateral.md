# Extension MV3 : capture, aperçu en direct, panneau latéral (issue #492, carte #490)

Sources : uniquement developer.chrome.com (consultées le 2026-11-23). Les faits non confirmés par une source primaire sont listés en fin de document, ne pas les tenir pour acquis.

## 1. Capture

| Fait | Source |
|---|---|
| `tabs.captureVisibleTab(windowId?, options?)` capture la zone **visible** de l'onglet actif d'une fenêtre (Chrome 88+). Aucune capture pleine page ni d'une autre fenêtre n'est documentée. | reference/api/tabs |
| Permission : `<all_urls>` ou `activeTab`. Avec `activeTab` seul, pages sensibles capturables (`chrome:`, pages d'autres extensions, `data:`). Fichiers : accès fichier requis. | reference/api/tabs |
| Quota : `MAX_CAPTURE_VISIBLE_TAB_CALLS_PER_SECOND` = 2 (Chrome 92+) ; appel « coûteux ». Impact : 106 critères x captures doivent être sérialisés/limités. | reference/api/tabs |
| `chrome.debugger` exige la permission `"debugger"` ; expose un sous-ensemble de domaines CDP (Accessibility, CSS, DOM, Network, Page, Runtime, Storage, Target...). `onDetach` si DevTools s'ouvre sur l'onglet (`canceled_by_user`). Iframes hors-processus = cibles distinctes ; sessions plates depuis Chrome 125. | reference/api/debugger |
| Permissions `debugger` : avertissements « Access the page debugger backend. » et « Read and change all your data on all websites. » | reference/permissions-list |

Conséquence : `captureVisibleTab` = viewport seul, simple, sans avertissement avec `activeTab`. Pleine page ou éléments encadrés hors viewport imposent soit un défilement + assemblage (limité à 2 appels/s), soit `chrome.debugger` (avertissements lourds, conflit DevTools). Encadrer un élément se fait par injection DOM/CSS (section 2) puis capture visible.

## 2. Injection éphémère (aperçu)

| Fait | Source |
|---|---|
| `scripting.insertCSS` (chaîne `css` ou `files`) / `removeCSS` : le retrait exige `css`, `files` et `origin` identiques à l'insertion. `origin` : `AUTHOR` (défaut) ou `USER`. | reference/api/scripting |
| `executeScript` : `files` ou `func`, jamais une chaîne. Monde `ISOLATED` (défaut) ou `MAIN`. Cibles : `allFrames` ou `frameIds` (exclusifs), `documentIds` (Chrome 106+). | reference/api/scripting |
| Permissions : `scripting` + `host_permissions` ou `activeTab`. | reference/api/scripting |
| `activeTab` : accès temporaire après geste utilisateur (clic action, menu contextuel, raccourci, omnibox) ; perdu à la navigation vers un autre site ou fermeture ; aucun avertissement ; inaccessible sur pages restreintes (`chrome://`). | concepts/activeTab |
| Content scripts en monde isolé : CSP d'extension (pas d'`eval`, pas de script externe). Monde `MAIN` : la CSP de la page s'applique. | concepts/content-scripts |
| Iframes `about:`, `data:`, `blob:`, `filesystem:` : `match_origin_as_fallback` (chemin `*` requis) ; `about:blank` : `match_about_blank`. | concepts/content-scripts |
| `chrome.dom.openOrClosedShadowRoot(el)` renvoie la racine shadow ouverte ou fermée, ou `null`. | reference/api/dom |
| Motifs d'hôte : schémas `http`, `https`, `file` (accès fichier manuel). | concepts/match-patterns |

Conséquence : l'aperçu en CSS (`insertCSS`, `removeCSS` symétrique) et en DOM via `executeScript` avec `func` embarquée est faisable sans code distant. Shadow DOM fermé accessible via `chrome.dom.openOrClosedShadowRoot` ; iframes via `allFrames`/`frameIds`.

## 3. Panneau latéral

- Permission `"sidePanel"` (aucun avertissement) ; clé `side_panel.default_path` ; Chrome 114+. `open()` : Chrome 116+, uniquement en réponse à un geste utilisateur. `getLayout()` 140+, `close()`/`onOpened` 141+, `onClosed` 142+.
- `setOptions` global ou par `tabId` ; `setPanelBehavior({openPanelOnActionClick:true})` ouvre au clic sur l'icône.
- Source : reference/api/sidePanel ; avertissements : reference/permissions-list.

## 4. Publication (Chrome Web Store)

| Fait | Source |
|---|---|
| Pas de code distant : `<script>` externe, `eval` d'une chaîne distante et interpréteur de commandes distantes interdits ; exceptions pour les API Debugger et User Scripts (usage conforme à leur objet) ; code en iframes/pages sandbox exempté mais fonctionnalité discernable. Données externes sans logique : autorisées. | webstore/program-policies/mv3-requirements |
| Revue plus longue : hôtes larges (`<all_urls>`, `https://*/*`), `tabs`, `downloads`, `cookies`/`webRequest` avec hôtes ; chaque permission doit être justifiée ; code non obfusqué. | webstore/review-process |
| `activeTab`, `scripting`, `sidePanel` : pas d'avertissement. `tabs` : « Read your browsing history. » | reference/permissions-list ; concepts/permission-warnings |

Conséquence : du CSS/JS **généré par l'IA** et appliqué à l'exécution est de la logique distante s'il est exécuté comme script (interdit). Le traiter comme **donnée** (propositions structurées : sélecteur + propriétés CSS validées par un schéma, interprétées par du code embarqué) est le chemin conforme ; l'injection de CSS brut reçu d'un serveur n'est pas tranchée par la politique (voir lacunes).

## 5. Verdict

Faisable tel quel : capture du viewport (`activeTab` + `scripting`), aperçu CSS/DOM éphémère via code embarqué, panneau latéral, sans avertissement d'installation.

Compromis : (a) pleine page / éléments hors viewport : défilement + assemblage à 2 captures/s, ou `debugger` (avertissements lourds, revue plus lente, conflit DevTools) ; (b) `activeTab` n'est valable que pour l'onglet invoqué et jusqu'à navigation inter-sites : un audit multi-pages automatique exige des hôtes larges (revue approfondie) ; (c) aperçu IA : propositions en données structurées, pas en script distant.

## Lacunes (non confirmées par les sources consultées)

- Bandeau « started debugging this browser » et conflits avec un autre débogueur : absents de la page debugger consultée.
- Effet de la CSP de la page sur le CSS injecté par `insertCSS` : non documenté dans les pages lues (la page manifest-csp renvoie 404).
- `captureVisibleTab` sous `activeTab` : confirmé par la page tabs, mais la page activeTab ne le cite pas.
- Capture pleine page, `Page.captureScreenshot` via `debugger`, statut CWS des CSS distants : non couverts.
- Pages inaccessibles (`chrome://`, Chrome Web Store) : non confirmé hors mention `activeTab`.
