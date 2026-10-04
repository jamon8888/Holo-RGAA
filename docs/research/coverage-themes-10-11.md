# Faisabilité des contrôles — thèmes 10 (présentation) et 11 (formulaires)

Ticket #250 (carte parente #247) · Date : 2026-10-04 · Branche : `research/coverage-themes-10-11`
Aucun code de production n'est écrit ici. Tous les chemins sont relatifs à `rgaa-rs/` sauf mention contraire.

## 1. Vérification à HEAD : quels critères n'ont réellement aucun contrôle ?

Sources relues : `crates/rgaa-core/data/rgaa-4.1.2/axe_mapping.json` (entrées par critère), `axe_rules.json` (tags `RGAA-x.y.z` d'axe-core 4.13), `crates/rgaa-rules/src/gap_fix.rs` (`GapFixRules::snippets()`), `crates/rgaa-linter/src/rules.rs`, et `docs/specs/deterministic-mechanisms-axe-cannot-supply.md` (#202).

Résultat : la liste de la carte est **exacte**, avec deux nuances.

| Critère | État dans `axe_mapping.json` | Sonde gap-fix | rgaa-linter | Verdict à HEAD |
|---|---|---|---|---|
| 10.3, 10.10, 11.3, 11.7, 11.8, 11.9, 11.10 | aucune entrée | non | non | **aucun contrôle** |
| 10.7, 10.12, 10.13, 11.5*, 11.6, 11.11, 11.12 | entrée `partial` avec `axe_rules: []` | non (sauf 11.5) | non | **aucun contrôle** (l'entrée vide est la valeur par défaut « partiel », pas un mécanisme) |
| 10.4, 10.11 | `meta-viewport`, `partial` | non (la sonde de reflux `scrollWidth`/320 a été retirée, test `the_unsound_reflow_snippet_is_not_on_the_audit_path`) | non | **partiel** : ne détecte que le blocage du zoom |
| 11.2 | `label-content-name-mismatch`, `partial` | non | non | **partiel** (étiquette contredite par le nom accessible) |
| 11.13 | `autocomplete-valid`, `partial` | non | non | **partiel** : valide la valeur présente, pas l'absence |
| 10.1 | `partial` | oui (balises/attributs de présentation, images espaceurs) | non | **sonde partielle** : tests 1 et 2 couverts, test 3 (espaces) non |
| 11.5 | `partial` | oui (radio/checkbox de même `name` sans `fieldset`/`group`) | non | **sonde partielle** |

Nuances à reporter sur la carte :
- Le linter (`rgaa-linter`) ne porte que 4 règles (`img-alt`, `form-label`, `button-name`, `link-name`), toutes sur 1.1, 6.1 et 11.1. Aucun critère 10.x/11.x listé n'y est couvert.
- axe-core tague `button-name`, `input-button-name` et `aria-command-name` `RGAA-11.9.1`, mais le dépôt les range sous 11.1 (et 7.1). 11.9 est donc **sans entrée** dans le mapping alors qu'axe en porte le tag. Ces règles ne vérifient que la **présence** d'un nom, pas sa pertinence : le mapping actuel est défendable, mais le test 11.9.1 « intitulé pertinent » reste sans contrôle.
- Règles axe natives **non mappées** utiles à ce périmètre : `avoid-inline-spacing` (pour 10.12). `target-size` n'a pas de pendant RGAA 4.1.2 (critère WCAG 2.2, hors référentiel). Rien d'autre dans `axe_rules.json` ne porte un tag `RGAA-10.*` ou `RGAA-11.*` hors règles déjà mappées (`aria-hidden-*` → 10.8, `link-in-text-block` → 10.6, `meta-viewport` → 10.4, `form-field-multiple-labels` → 11.2, `autocomplete-valid` → 11.13).

\* 11.5 est listé « sonde partielle à réévaluer » dans la carte ; il a une sonde, voir tableau.

## 2. Légende

**Mécanismes** : `axe-natif` (règle axe-core existante non mappée) · `axe-perso` (règle axe personnalisée) · `sonde-statique` (JS DOM/CSSOM, contrat gap-fix, `GapFixRules::snippets()`) · `sonde-comportementale` (interaction : Tab, survol, redimensionnement, injection de style, via le pont navigateur — « mode C » de #194) · `linter` (source, `rgaa-linter`) · `pixels` (capture et comparaison d'image) · `humain` (aucun contrôle déterministe réaliste).

**Règle de verdict** (héritée de #199/#202) : une sonde partielle qui ne trouve rien ne produit **aucun** `Pass` critère ; elle ne peut qu'échouer ou laisser le critère à `NeedsReview`. Toute estimation ci-dessous suppose ce contrat.

**Effort relatif** : S (< 1 j, une sonde statique), M (quelques jours, sonde ou règle axe testée sur fixtures), L (interaction navigateur + fixtures + calibrage), XL (nouvelle infrastructure).

**Décidabilité** : D = décidable, P = partiellement (détection d'applicabilité ou de violation sûre seulement), N = non décidable (jugement humain).

## 3. Tableau critère × test RGAA × mécanisme

Les numéros de test suivent `criteres.json` (une entrée = un test, qui peut regrouper des sous-conditions).

### Thème 10 — Présentation de l'information

| Critère | Test RGAA | D/P/N | Mécanisme réaliste | Preuve produite | Risque faux positif / négatif | Effort |
|---|---|---|---|---|---|---|
| **10.1** | 10.1.1 balises de présentation absentes | D | sonde-statique **déjà là** | nombre d'éléments `font, center, big…` | FP faible ; FN : balises ajoutées via CSS `content`/composants web fermés (shadow DOM non traversé) | S (étendre au shadow DOM) |
| 10.1 | 10.1.2 attributs de présentation absents | D | sonde-statique **déjà là** | nombre d'attributs `align, bgcolor…` | **FP réel** : `align`/`valign` sont admis sur certains éléments par des CMS, `background` sur `<body>` hérité ; `cellpadding` dans les e-mails. À restreindre aux éléments où l'attribut est obsolète | S |
| 10.1 | 10.1.3 espaces non utilisées pour mise en forme | P | sonde-statique (regex sur nœuds texte : lettres séparées par des espaces `"B O N J O U R"`, suites de ≥ 3 espaces ou `&nbsp;` consécutifs hors `<pre>`) ; sinon humain | extrait de texte fautif | FP moyen (acronymes épelés légitimes, code, `white-space: pre`) → résultat `NeedsReview`, jamais `Fail` seul | S–M |
| **10.3** | 10.3.1 information compréhensible sans CSS | N | humain ; **aide** : sonde comportementale qui désactive les feuilles de style (`document.styleSheets[i].disabled = true`, retrait des `style=`) puis compare l'ordre du texte et les contenus devenus invisibles/superposés | deux extractions de texte (`innerText`) avant/après, delta d'ordre | Diff d'ordre bruité (menus déroulants dépendants de CSS) → n'établit pas la conformité, ne sert qu'à l'échantillonnage pour revue | L (pour un gain faible) ; recommandation : **aucun contrôle automatique**, pas de verdict |
| **10.4** | 10.4.1 pas de perte d'information à 200 % de texte | P | sonde-comportementale : agrandir le texte (`font-size` racine ×2 ou `Emulation.setPageScaleFactor`/zoom texte), détecter contenu tronqué (`overflow:hidden` + `scrollWidth > clientWidth` / `scrollHeight > clientHeight`) et éléments se chevauchant (`getBoundingClientRect`) | liste d'éléments tronqués avec rectangles avant/après | FP moyen (`text-overflow: ellipsis` voulu, carrousels) ; FN : perte par masquage CSS non détectée. Zoom texte navigateur ≠ zoom page : choisir et documenter | L |
| 10.4 | 10.4.2 agrandissement possible pour tout le texte | D (en partie) | **axe-natif déjà mappé** (`meta-viewport` : `user-scalable=no`, `maximum-scale<2`) + sonde-statique CSSOM : tailles de police en `px` dans des `@media`/`!important` n'empêchent pas le zoom navigateur, donc peu utile ; ajouter détection de `text-size-adjust: none` | `content` du meta viewport | FN : agrandissement bloqué par JS (écouteurs `wheel`/`gesturestart`) non vu | S (déjà fait pour l'essentiel) |
| **10.7** | 10.7.1 focus visible | D (avec `cantTell`) | sonde-comportementale **ACT `oj04fd`** : pour chaque élément de l'ordre de tabulation, `focus()` puis diff du style calculé (`outline`, `box-shadow`, `border`, `background`, `text-decoration`) ; spec déjà rédigée dans #202 (mécanisme 2) | par élément : propriétés modifiées au focus (ou « aucune ») | FP : indicateur dessiné par pseudo-élément, image de fond ou canvas → doit sortir `NeedsReview` (`cantTell`), jamais `Fail` ; `:focus-visible` : forcer via vrai événement clavier (CDP `Input.dispatchKeyEvent` Tab), pas `element.focus()` seul | M–L |
| 10.7 | 10.7.2–3 (variantes) | P | idem + pixels optionnels (capture avant/après sur la boîte de l'élément, seuil de différence) pour résoudre les cas `cantTell` | deux captures + taux de pixels différents | Pixels : sensibilité aux animations/transitions → attendre fin de transition | L (en plus) |
| **10.10** | 10.10.1–4 info non donnée par forme/taille/position seule | N | humain. Détection d'applicabilité seulement possible par heuristique de langage (« voir le texte en rouge », « ci-dessus », « à droite ») : sonde-statique de regex FR, sortie `NeedsReview` | phrase déclencheuse | FP élevé, FN élevé (la plupart des cas n'ont aucune phrase) | S pour l'heuristique ; **valeur faible**, recommandation : aucun |
| **10.11** | 10.11.1 pas de défilement horizontal à 320 px | D (avec « cas particuliers ») | sonde-comportementale : `Emulation.setDeviceMetricsOverride` 320×(≥256), puis `document.documentElement.scrollWidth > clientWidth` et enumération des éléments débordants ; spec #202 mécanisme 3 / mode C #194. Cas particuliers (tableaux, images, cartes, code) : exclure par rôle et marquer `NeedsReview` | `scrollWidth`/`clientWidth`, premiers éléments dépassant le bord droit | FP moyen (cas particuliers WCAG 1.4.10 : tableaux de données, cartes, éditeurs) ; FN faible. **Prérequis** : le viewport doit réellement changer ; la sonde supprimée comparait à 320 sans redimensionner et produisait un FP quasi systématique | M–L |
| 10.11 | 10.11.2 pas de défilement vertical à 256 px de hauteur (sens vertical) | P | idem à 256 px de hauteur, uniquement pour contenu en écriture verticale (`writing-mode: vertical-*`) ; applicabilité déterministe (sinon `NotApplicable`) | idem | Application rare ; FP faible | S après 10.11.1 |
| **10.12** | 10.12.1 espacement du texte redéfinissable | D (en deux niveaux) | **Niveau 1 : axe-natif non mappé `avoid-inline-spacing`** (échoue si `style=` impose `!important` sur `line-height`, `letter-spacing`, `word-spacing` : l'utilisateur ne peut plus redéfinir) — mappable immédiatement, aussi équivalent aux règles ACT sur ces trois propriétés (identifiants à reconfirmer sur le site ACT, non re-téléchargés ici). **Niveau 2 : sonde-comportementale** : injecter la feuille WCAG 1.4.12 (`line-height:1.5; p{margin-bottom:2em} letter-spacing:.12em; word-spacing:.16em`, `!important`) puis détecter troncature/chevauchement comme pour 10.4 | niveau 1 : nœuds fautifs ; niveau 2 : éléments tronqués | Niveau 1 : FN (n'attrape pas les feuilles externes `!important`), FP faible. Niveau 2 : mêmes FP que 10.4 | **S** (niveau 1) ; M (niveau 2, partage la logique de 10.4) |
| **10.13** | 10.13.1 contenu additionnel masquable sans déplacer le focus/pointeur (Échap) | P | sonde-comportementale : déclencher survol/focus sur les candidats (éléments avec `aria-describedby`, `[role=tooltip]`, `title`, `:hover`/`:focus` révélateurs repérés dans le CSSOM — la logique de 10.14 existe déjà), constater l'apparition (élément devenu visible), envoyer Échap, vérifier la disparition | séquence événements + visibilité avant/après | FP/FN moyens : tout dépend du repérage des candidats ; contenu piloté par JS non repérable par CSSOM ; `title` natif géré par le navigateur (hors de la page) | L |
| 10.13 | 10.13.2 contenu apparu au survol, lui-même survolable | P | sonde-comportementale : après apparition, déplacer le pointeur sur le contenu additionnel (CDP `Input.dispatchMouseEvent`) et vérifier qu'il reste visible | idem | idem ; trajectoire du pointeur (passage par une zone morte) change le résultat | L |
| 10.13 | 10.13.3 persistance jusqu'au retrait du pointeur/focus | P | sonde-comportementale : attendre un délai fixe (> 5 s) et vérifier que le contenu est toujours visible | visibilité à t+5 s | FP si un temporisateur masque légitimement (cas autorisé : action utilisateur) | L |
| 10.13 | 10.13.4–6 | N | humain (cas particuliers, pertinence) | — | — | — |

### Thème 11 — Formulaires

| Critère | Test RGAA | D/P/N | Mécanisme réaliste | Preuve produite | Risque faux positif / négatif | Effort |
|---|---|---|---|---|---|---|
| **11.2** | 11.2.1–4 étiquette pertinente | N | humain/IA ; **détection d'applicabilité et heuristiques sûres** possibles en sonde-statique : étiquette vide ou ≡ placeholder, étiquette générique (« champ », « input », « texte »), étiquette identique pour des champs différents d'un même formulaire | texte de l'étiquette, champs concernés | FP moyen sur liste noire de mots ; ne conclut jamais `Pass` | S–M |
| 11.2 | 11.2.5 intitulé visible contenu dans le nom accessible (étiquette dans le nom, 2.5.3) | D | **axe-natif déjà mappé** (`label-content-name-mismatch`, expérimentale dans axe : vérifier qu'elle est activée) | nœud + nom accessible vs texte visible | FP faible, FN : ne couvre pas tous les types de champ | S (activation/qualification) |
| 11.2 | 11.2.6 bouton adjacent fournissant l'étiquette visible | P | sonde-statique : `<input>`/`<button>` voisin immédiat d'un champ sans `<label>` ; sortie `NeedsReview` | paire champ/bouton | FP/FN moyens | M |
| **11.3** | 11.3.1–2 cohérence des étiquettes d'une même fonction (page et ensemble de pages) | N | page seule : sonde-statique regroupant les champs par `autocomplete`/`name`/`type` et listant les étiquettes divergentes → `NeedsReview` ; **ensemble de pages** : agrégation orchestrateur sur l'audit multi-pages (hors sonde de page) | groupes et étiquettes divergentes | FP élevé si regroupement par `name` (noms génériques) ; plus fiable avec `autocomplete` | M (page) ; L (multi-pages) |
| **11.5** | 11.5.1 regroupement des champs de même nature | P | sonde-statique **déjà là** (radio/checkbox de même `name`) | liste des groupes non enclos | FN : champs de même nature hors radio/checkbox (adresse, date en trois champs) non vus ; FP : groupe d'un seul champ exclu, ok. **Réévaluation** : conserver `partial`, ajouter l'heuristique « ≥ 2 champs `autocomplete` partageant un préfixe (`address-*`, `bday-*`, `cc-*`) hors `fieldset` » | S |
| **11.6** | 11.6.1 chaque regroupement a une légende | D | sonde-statique : tout `fieldset` doit avoir `legend` enfant direct non vide ; tout `[role=group]`/`[role=radiogroup]` doit avoir `aria-label`/`aria-labelledby` résolvant du texte non vide. `fieldset`+`legend` est trivial, la partie ARIA demande de résoudre `aria-labelledby` (références) | nœud + raison | FP faible ; FN : `legend` masqué visuellement mais présent (acceptable). Axe n'a **aucune** règle dédiée : à ne pas attendre d'un correctif amont. **Candidat n° 1** du thème | **S** |
| **11.7** | 11.7.1 légende pertinente | N | humain/IA sur les légendes que 11.6 a extraites (pas de nouveau mécanisme) ; heuristique : légende vide/générique | texte de légende | — | S (pré-filtre) |
| **11.8** | 11.8.1 items de même nature regroupés par `optgroup` | N | sonde-statique d'applicabilité : `select` avec ≥ N options (ex. > 12) sans `optgroup` → `NeedsReview` (jamais `Fail`) | `select`, nombre d'options | FP/FN élevés (savoir si les items sont « de même nature » est humain) | S |
| 11.8 | 11.8.2 `optgroup` a un attribut `label` | D | sonde-statique : tout `optgroup` sans `label` non vide → `Fail` sûr | nœud | FP quasi nul (c'est une erreur de validité HTML) | **S** |
| 11.8 | 11.8.3 `label` d'`optgroup` pertinent | N | humain | — | — | — |
| **11.9** | 11.9.1 intitulé de bouton pertinent | N (P pour cas évidents) | axe fournit déjà la **présence** d'un nom (11.1) ; sonde-statique de pertinence : nom ≡ liste de génériques FR/EN (« cliquez ici », « ok », « bouton », « button », « envoyer » ne l'est pas toujours), nom ≡ identifiant technique (`btn_3`), nom unique pour des actions distinctes ; calcul du nom accessible via axe (`axe.commons.text.accessibleText`) | nom calculé + source | FP moyen sur liste noire ; ne peut pas conclure `Pass` | S–M |
| 11.9 | 11.9.2 étiquette dans le nom accessible (boutons) | D | **axe-natif** `label-content-name-mismatch` (déjà mappé sous 11.2/6.1.5) à ré-étiqueter aussi sous 11.9 ; deuxième condition (`value` de `input[type=submit]`) à confirmer | idem | FP faible | S |
| **11.10** | 11.10.1 indication de champ obligatoire avant validation | P | sonde-statique : champ avec `required`/`aria-required=true` → vérifier présence d'un indicateur visible (astérisque, « obligatoire », « required ») dans le `label`/texte lié ; ou à l'inverse champ sans attribut mais étiqueté « * » | champ + texte d'étiquette | FP moyen : légende globale « * champs obligatoires » ailleurs sur la page ; NeedsReview par défaut | M |
| 11.10 | 11.10.2 `required` ⇒ indication visible dans l'étiquette/passage associé | D (en partie) | idem 11.10.1 (même sonde, test complémentaire) | idem | idem | M |
| 11.10 | 11.10.3–4 message d'erreur d'absence de saisie identifie le champ / `aria-invalid` | P | sonde-comportementale : soumettre le formulaire vide (**à ne faire que sur formulaires réputés sûrs**, recherche/inscription fictive ; voir risque) et lire `aria-invalid`, `:invalid`, `validationMessage`, zones `role=alert` | messages + attributs | **Risque effet de bord** : soumission réelle d'un formulaire de commande, de suppression ou de newsletter. Utiliser `form.noValidate=false` + `checkValidity()`/`reportValidity()` sans `submit()` pour limiter à la validation native | L |
| 11.10 | 11.10.5–7 type/format de donnée indiqué | P | sonde-statique : champ avec `pattern`/`type=email|tel|date` ⇒ présence d'un format décrit (`placeholder`, `title`, `aria-describedby`) ; détecter le format décrit uniquement par placeholder (est-ce suffisant ? humain) | champ + descripteurs | FP moyen | M |
| 11.10 | autres tests (21 au total, 4 automatisables selon `automatable_criteres.json`) | N | humain | — | — | — |
| **11.11** | 11.11.1 suggestion du type et du format après erreur | P | sonde-comportementale (identique 11.10.3) : après `reportValidity()`, vérifier que le message natif ou `aria-describedby` contient un format/exemple ; sonde-statique de pré-filtre : `pattern` présent sans `title` | message d'erreur | Dépend du texte du navigateur (non fiable, localisé) ; message natif d'`input type=email` « inclure un @ » suffit-il ? humain | L |
| 11.11 | 11.11.2 exemples de valeurs attendues | P | sonde-statique : `pattern`/`type=tel|date|email` sans exemple (`placeholder` ou `aria-describedby` ou texte lié) → `NeedsReview` | champ | FP moyen | M |
| **11.12** | 11.12.1 données modifiables/annulables/vérifiables/confirmées | P | **applicabilité** déterministe par sonde-statique : formulaires `method=post` contenant un champ `password`, `iban`, `card`, un `button` « supprimer/payer/commander/valider » (regex FR/EN), ou un `action` contenant `delete|checkout|payment` → appliquer 11.12 ; résolution humaine (présence d'un récapitulatif ou d'une étape de confirmation) | formulaire + indices d'applicabilité | FP/FN élevés pour l'applicabilité par regex ; sert surtout à **éliminer** les critères `NotApplicable` (la plupart des pages, tests inapplicables) | M |
| 11.12 | 11.12.2 récupération ou demande de confirmation explicite | P | sonde-statique : présence d'une case `confirm`, d'un `dialog`/`confirm()` (non détectable statiquement), d'une page d'étape — tout hors de portée statique | — | FN élevé : confirmation par `window.confirm` ou étape serveur invisible | L (peu de valeur) |
| **11.13** | 11.13.1 `autocomplete` présent sur champs concernant l'utilisateur | P | **sonde-statique** : champ `input` (type `text|email|tel|password|url|date`, hors `hidden`/`search`) dont `name`/`id`/`type`/étiquette/`placeholder` correspond à un jeu de motifs (nom, prénom, email, tel, adresse, code postal, ville, pays, date de naissance, organisation, identifiant, mot de passe) **et** sans `autocomplete` → violation ; complète `autocomplete-valid` (axe, déjà mappé) qui valide la valeur présente. Jeu de motifs : liste WCAG 1.3.5 « Input Purposes ». Alternative d'ordre sûr : `type=email|tel|password` sans `autocomplete` | champ + motif reconnu | FP moyen (champ « nom » d'un produit ≠ nom utilisateur) → rester sur `type=email|tel|password|url` et `name` strict en `Fail`, le reste en `NeedsReview` ; FN : champs non reconnus | **S–M** |
| 11.13 | 11.13.2–3 valeur valide / pertinente | D / P | **axe-natif déjà mappé** `autocomplete-valid` (validité) ; pertinence (valeur cohérente avec l'étiquette) : sonde-statique comparant motif de l'étiquette et jeton `autocomplete` → `NeedsReview` en cas de désaccord | jeton + étiquette | FP moyen | S (comparaison) |

## 4. Synthèse par mécanisme (ordre de rentabilité)

| Priorité | Mécanisme | Critères | Effort | Motif |
|---|---|---|---|---|
| 1 | **axe-natif non mappé** | 10.12 (`avoid-inline-spacing`) ; ré-étiquetage de `label-content-name-mismatch` sous 11.9 | S | Aucun code de sonde : une ligne de mapping et un fixture. Risque faible. |
| 2 | **sonde-statique** déterministe (violation sûre) | 11.6, 11.8.2, 10.1.3 (partiel), 11.13.1 (motifs stricts) | S chacune | Contrat gap-fix existant, partiel/fail-closed, pas de nouvelle infrastructure. |
| 3 | **sonde-statique** d'applicabilité/heuristique → `NeedsReview` | 11.2, 11.3 (page), 11.8.1, 11.9.1, 11.10.1–2, 11.10.5–7, 11.11.2, 11.12 | S–M | Ne tranche pas ; réduit le nombre de critères envoyés à l'IA et supprime des `NotApplicable`. |
| 4 | **sonde-comportementale** | 10.7 (ACT `oj04fd`), 10.11, 10.4 puis 10.12 (niveau 2), 10.13 | M–L | Le gain le plus visible côté critères, mais exige le pont navigateur (#194 mode C) et un calibrage sur fixtures ; mutualiser « mesurer la troncature/le débordement » entre 10.4, 10.11, 10.12. |
| 5 | **comportementale avec effet de bord** | 11.10.3–4, 11.11.1 | L | Soumission de formulaires : limiter à `checkValidity()`/`reportValidity()`, jamais `submit()`. |
| 6 | **pixels** | complément de 10.7 pour les `cantTell` | L | Optionnel ; bruité par les transitions. |
| 7 | **linter** | 11.6, 11.8.2, 10.1 (source) | S–M | Les règles statiques de 11.6/11.8.2/11.13 se prêtent bien à une règle `rgaa-linter` ; mais le contrat du linter (« ne jamais deviner ») exclut tout ce qui dépend d'un composant ou d'un attribut calculé. |
| 8 | **humain** | 10.3, 10.10, 11.7, 11.8.1/3, 11.2.1–4 (pertinence), 11.12.2 | — | Sauf pré-filtres cités, aucune décision déterministe réaliste. |

## 5. Réévaluation des sondes existantes (10.1, 11.5)

- **10.1** : la sonde couvre 10.1.1 et 10.1.2. Risque connu de faux positif sur 10.1.2 : certains attributs (`align`, `valign`, `background`) ont des usages tolérés hors balisage de présentation (e-mails HTML, `<td align>` produits par un CMS). Recommandation : conserver la sonde, la garder `partial`, et distinguer dans la preuve les balises (sûres) des attributs (probables). Ajouter 10.1.3 en `NeedsReview` seulement.
- **11.5** : la sonde ne traite que radio/checkbox de même `name`. Les « champs de même nature » du test 11.5.1 incluent aussi adresse, date en plusieurs champs, etc. Elle reste correcte comme détecteur de violation, jamais comme preuve de conformité (déjà `partial`). Extension envisageable : groupement par préfixe `autocomplete`.

## 6. Limites de cette recherche

- Je n'ai pas exécuté de sonde ni mesuré de taux de faux positifs : les risques sont des estimations par analyse de code et de spécification, à calibrer sur fixtures avant implémentation.
- Les identifiants des règles ACT autres que `oj04fd` (cité dans le dépôt, `docs/specs/deterministic-mechanisms-axe-cannot-supply.md`) ne sont pas re-vérifiés sur le site ACT.
- Le comportement exact de `label-content-name-mismatch` (règle axe marquée expérimentale) et d'`avoid-inline-spacing` est établi d'après leurs entrées dans `axe_rules.json` (axe-core 4.13), pas d'un essai sur page réelle.
- Les chiffres « tests automatisables / total » proviennent de `automatable_criteres.json` et sont ceux du dépôt, non ceux de la DINUM.

## Sources

- `crates/rgaa-core/data/rgaa-4.1.2/{criteres,automatable_criteres,axe_mapping,axe_rules}.json`
- `crates/rgaa-rules/src/gap_fix.rs` (snippets 10.1, 10.2, 10.14, 11.1, 11.4, 11.5 ; test absence de snippet 10.11)
- `crates/rgaa-linter/src/rules.rs`
- `docs/specs/deterministic-mechanisms-axe-cannot-supply.md` (#202, mécanismes 2, 3, 4, 5)
- Référentiel RGAA 4.1.2 : https://accessibilite.numerique.gouv.fr/methode/criteres-et-tests/
- WCAG 2.1 SC 1.3.5, 1.4.4, 1.4.10, 1.4.12, 1.4.13, 2.4.7, 2.5.3 (correspondances 11.13, 10.4, 10.11, 10.12, 10.13, 10.7, 11.2/11.9)
