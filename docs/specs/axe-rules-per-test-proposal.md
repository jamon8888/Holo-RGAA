# Table de proposition — règle d'axe par test RGAA

Statut : **proposition à relire**, rien n'est écrit dans `mechanisms.toml`. Prérequis bloquant 6 de `human-criteria-automation.md`.

Pour chaque mécanisme `axe-native` d'un critère à plusieurs tests : l'énoncé de chaque test, les règles axe citées avec leur description, et un **rattachement proposé** sous réserve d'une relecture humaine. Un test sans règle reste *non couvert* ; la couverture `partial` du mécanisme ne change pas.

> Le rattachement est un jugement sur le texte des règles et des tests. Il n'est **pas** vérifié par exécution. Les cases « à trancher » sont celles où le texte ne suffit pas.


## Constat à vérifier avant toute chose : 1.1 est déclaré « complete » avec trois tests sans règle axe

`mechanisms.toml` déclare `axe-1-1` en couverture **complète**, et `test_routes.json` route les **huit** tests de 1.1 vers `axe-1-1` et `gapfix-1-1`. Or les six règles citées ne ciblent que `<img>`/`role=img` (1.1.1), `<area>` (1.1.2), `<input type=image>` (1.1.3), `<svg>` (1.1.5) et `<object>` (1.1.6). **Aucune ne cible** le test 1.1.4 (image réactive côté serveur), 1.1.7 (`<embed>`) ni 1.1.8 (`<canvas>`).

Si le silence d'axe vaut `Pass` pour 1.1 (c'est le sens de « complete »), une page dont seul un `<canvas>` ou un `<embed>` est dépourvu d'alternative ressortirait conforme sur 1.1. **Non vérifié par exécution** : la sonde `gapfix-1-1` (legacy) pourrait couvrir ces cas, je n'y ai trouvé aucune vérification d'alternative sur `embed`/`canvas` en lisant `gap_fix.rs`, mais je n'ai pas exécuté la sonde. À tester par fixture avant de statuer.


## 1.1 — mécanisme `axe-1-1` (complete)

| test | énoncé |
|---|---|
| 1.1.1 | Chaque image (balise `<img>` ou balise possédant l’attribut WAI-ARIA `role="img"`) porteuse d’information a-t-elle une alternative textuelle ? |
| 1.1.2 | Chaque zone d’une image réactive (balise `<area>`) porteuse d’information a-t-elle une alternative textuelle ? |
| 1.1.3 | Chaque bouton de type `image` (balise `<input>` avec l’attribut `type="image"`) a-t-il une alternative textuelle ? |
| 1.1.4 | Chaque zone cliquable d’une image réactive côté serveur est-elle doublée d’un mécanisme utilisable quel que soit le dispositif de pointage utilisé et permettant d’accéder |
| 1.1.5 | Chaque image vectorielle (balise `<svg>`) porteuse d’information, vérifie-t-elle ces conditions ? |
| 1.1.6 | Chaque image objet (balise `<object>` avec l’attribut `type="image/…"`) porteuse d’information, vérifie-t-elle une de ces conditions ? |
| 1.1.7 | Chaque image embarquée (balise `<embed>` avec l’attribut `type="image/…"`) porteuse d’information, vérifie-t-elle une de ces conditions ? |
| 1.1.8 | Chaque image bitmap (balise `<canvas>`) porteuse d’information, vérifie-t-elle une de ces conditions ? |

| règle axe | description | test(s) proposé(s) |
|---|---|---|
| `image-alt` | Ensure &lt;img&gt; elements have alternative text or a role of none or presentation | **à trancher** |
| `input-image-alt` | Ensure &lt;input type=&quot;image&quot;&gt; elements have alternative text | **à trancher** |
| `area-alt` | Ensure &lt;area&gt; elements of image maps have alternative text | **à trancher** |
| `object-alt` | Ensure &lt;object&gt; elements have alternative text | **à trancher** |
| `role-img-alt` | Ensure [role=&quot;img&quot;] and [role=&quot;image&quot;] elements have alternative text | **à trancher** |
| `svg-img-alt` | Ensure &lt;svg&gt; elements with an img, image, graphics-document or graphics-symbol role have  | **à trancher** |

## 1.2 — mécanisme `axe-1-2` (partial)

| test | énoncé |
|---|---|
| 1.2.1 | Chaque image (balise `<img>`) de décoration, sans légende, vérifie-t-elle une de ces conditions ? |
| 1.2.2 | Chaque zone non cliquable (balise `<area>` sans attribut `href`) de décoration, vérifie-t-elle une de ces conditions ? |
| 1.2.3 | Chaque image objet (balise `<object>` avec l’attribut `type="image/…"`) de décoration, sans légende, vérifie-t-elle ces conditions ? |
| 1.2.4 | Chaque image vectorielle (balise `<svg>`) de décoration, sans légende, vérifie-t-elle ces conditions ? |
| 1.2.5 | Chaque image bitmap (balise `<canvas>`) de décoration, sans légende, vérifie-t-elle ces conditions ? |
| 1.2.6 | Chaque image embarquée (balise `<embed>` avec l’attribut `type="image/…"`) de décoration, sans légende, vérifie-t-elle ces conditions ? |

| règle axe | description | test(s) proposé(s) |
|---|---|---|
| `image-alt` | Ensure &lt;img&gt; elements have alternative text or a role of none or presentation | **à trancher** |
| `image-redundant-alt` | Ensure image alternative is not repeated as text | **à trancher** |

## 3.2 — mécanisme `axe-3-2` (complete)

| test | énoncé |
|---|---|
| 3.2.1 | Dans chaque page web, le texte et le texte en image sans effet de graisse d’une taille restituée inférieure à 24px vérifient-ils une de ces conditions (hors cas particuli |
| 3.2.2 | Dans chaque page web, le texte et le texte en image en gras d’une taille restituée inférieure à 18,5px vérifient-ils une de ces conditions (hors cas particuliers) ? |
| 3.2.3 | Dans chaque page web, le texte et le texte en image sans effet de graisse d’une taille restituée supérieure ou égale à 24px vérifient-ils une de ces conditions (hors cas  |
| 3.2.4 | Dans chaque page web, le texte et le texte en image en gras d’une taille restituée supérieure ou égale à 18,5px vérifient-ils une de ces conditions (hors cas particuliers |
| 3.2.5 | Dans le mécanisme qui permet d’afficher un rapport de contraste conforme, le rapport de contraste entre le texte et la couleur d’arrière-plan est-il suffisamment élevé ? |

| règle axe | description | test(s) proposé(s) |
|---|---|---|
| `color-contrast` | Ensure the contrast between foreground and background colors meets WCAG 2 AA minimum contrast r | **à trancher** |

## 3.3 — mécanisme `axe-3-3` (partial)

| test | énoncé |
|---|---|
| 3.3.1 | Dans chaque page web, le rapport de contraste entre les couleurs d’un composant d’interface dans ses différents états et la couleur d’arrière-plan contiguë vérifie-t-il u |
| 3.3.2 | Dans chaque page web, le rapport de contraste des différentes couleurs composant un élément graphique, lorsqu’elles sont nécessaires à sa compréhension, et la couleur d’a |
| 3.3.3 | Dans chaque page web, le rapport de contraste des différentes couleurs contiguës entre elles d’un élément graphique, lorsqu’elles sont nécessaires à sa compréhension, vér |
| 3.3.4 | Dans le mécanisme qui permet d’afficher un rapport de contraste conforme, les couleurs du composant ou des éléments graphiques porteurs d’informations qui le composent, s |

| règle axe | description | test(s) proposé(s) |
|---|---|---|
| `color-contrast` | Ensure the contrast between foreground and background colors meets WCAG 2 AA minimum contrast r | **à trancher** |

## 4.3 — mécanisme `axe-4-3` (complete)

| test | énoncé |
|---|---|
| 4.3.1 | Chaque média temporel synchronisé pré-enregistré vérifie-t-il, si nécessaire, l’une de ces conditions (hors cas particuliers) ? |
| 4.3.2 | Pour chaque média temporel synchronisé pré-enregistré possédant des sous-titres synchronisés diffusés via une balise `<track>`, la balise `<track>` possède-t-elle un attr |

| règle axe | description | test(s) proposé(s) |
|---|---|---|
| `video-caption` | Ensure &lt;video&gt; elements have captions | **à trancher** |

## 5.6 — mécanisme `axe-5-6` (partial)

| test | énoncé |
|---|---|
| 5.6.1 | Pour chaque tableau de données, chaque en-tête de colonne s’appliquant à la totalité de la colonne vérifie-t-il une de ces conditions ? |
| 5.6.2 | Pour chaque tableau de données, chaque en-tête de ligne s’appliquant à la totalité de la ligne vérifie-t-il une de ces conditions ? |
| 5.6.3 | Pour chaque tableau de données, chaque en-tête ne s’appliquant pas à la totalité de la ligne ou de la colonne est-il structuré au moyen d’une balise `<th>` ? |
| 5.6.4 | Pour chaque tableau de données, chaque cellule associée à plusieurs en-têtes est-elle structurée au moyen d’une balise `<td>` ou `<th>` ? |

| règle axe | description | test(s) proposé(s) |
|---|---|---|
| `td-headers-attr` | Ensure that each cell in a table that uses the headers attribute refers only to other &lt;th&gt | **à trancher** |

## 5.7 — mécanisme `axe-5-7` (complete)

| test | énoncé |
|---|---|
| 5.7.1 | Pour chaque contenu de balise `<th>` s’appliquant à la totalité de la ligne ou de la colonne, la balise `<th>` respecte-t-elle une de ces conditions (hors cas particulier |
| 5.7.2 | Pour chaque contenu de balise `<th>` s’appliquant à la totalité de la ligne ou de la colonne et possédant un attribut `scope`, la balise `<th>` vérifie-t-elle une de ces  |
| 5.7.3 | Pour chaque contenu de balise `<th>` ne s’appliquant pas à la totalité de la ligne ou de la colonne, la balise `<th>` vérifie-t-elle ces conditions ? |
| 5.7.4 | Pour chaque contenu de balise `<td>` ou `<th>` associée à un ou plusieurs en-têtes possédant un attribut `id`, la balise vérifie-t-elle ces conditions ? |
| 5.7.5 | Pour chaque balise pourvue d’un attribut WAI-ARIA `role="rowheader"` ou `role="columnheader"` dont le contenu s’applique à la totalité de la ligne ou de la colonne, la ba |

| règle axe | description | test(s) proposé(s) |
|---|---|---|
| `td-headers-attr` | Ensure that each cell in a table that uses the headers attribute refers only to other &lt;th&gt | **à trancher** |
| `th-has-data-cells` | Ensure that &lt;th&gt; elements and elements with role=columnheader/rowheader have data cells t | **à trancher** |
| `td-has-header` | Ensure that each non-empty data cell in a &lt;table&gt; larger than 3 by 3 has one or more tabl | **à trancher** |

## 6.1 — mécanisme `axe-6-1` (partial)

| test | énoncé |
|---|---|
| 6.1.1 | Chaque lien texte vérifie-t-il une de ces conditions (hors cas particuliers) ? |
| 6.1.2 | Chaque lien image vérifie-t-il une de ces conditions (hors cas particuliers) ? |
| 6.1.3 | Chaque lien composite vérifie-t-il une de ces conditions (hors cas particuliers) ? |
| 6.1.4 | Chaque lien SVG vérifie-t-il une de ces conditions (hors cas particuliers) ? |
| 6.1.5 | Pour chaque lien ayant un intitulé visible, le nom accessible du lien contient-il au moins l’intitulé visible (hors cas particuliers) ? |

| règle axe | description | test(s) proposé(s) |
|---|---|---|
| `link-name` | Ensure links have discernible text | **à trancher** |

## 7.1 — mécanisme `axe-7-1` (partial)

| test | énoncé |
|---|---|
| 7.1.1 | Chaque script qui génère ou contrôle un composant d’interface vérifie-t-il, si nécessaire, une de ces conditions ? |
| 7.1.2 | Chaque script qui génère ou contrôle un composant d’interface respecte-t-il une de ces conditions ? |
| 7.1.3 | Chaque script qui génère ou contrôle un composant d’interface vérifie-t-il ces conditions (hors cas particuliers) ? |

| règle axe | description | test(s) proposé(s) |
|---|---|---|
| `aria-roles` | Ensure all elements with a role attribute use a valid value | **à trancher** |
| `aria-valid-attr` | Ensure attributes that begin with aria- are valid ARIA attributes | **à trancher** |
| `aria-valid-attr-value` | Ensure all ARIA attributes have valid values | **à trancher** |
| `aria-allowed-attr` | Ensure an element&apos;s role supports its ARIA attributes | **à trancher** |
| `aria-required-attr` | Ensure elements with ARIA roles have all required ARIA attributes | **à trancher** |
| `aria-required-children` | Ensure elements with an ARIA role that require child roles contain them | **à trancher** |
| `aria-required-parent` | Ensure elements with an ARIA role that require parent roles are contained by them | **à trancher** |
| `aria-prohibited-attr` | Ensure ARIA attributes are not prohibited for an element&apos;s role | **à trancher** |
| `aria-conditional-attr` | Ensure ARIA attributes are used as described in the specification of the element&apos;s role | **à trancher** |
| `aria-deprecated-role` | Ensure elements do not use deprecated roles | **à trancher** |
| `aria-hidden-body` | Ensure aria-hidden=&quot;true&quot; is not present on the document body. | **à trancher** |
| `nested-interactive` | Ensure interactive controls are not nested as they are not always announced by screen readers o | **à trancher** |
| `aria-command-name` | Ensure every ARIA button, link and menuitem has an accessible name | **à trancher** |
| `aria-meter-name` | Ensure every ARIA meter node has an accessible name | **à trancher** |
| `aria-progressbar-name` | Ensure every ARIA progressbar node has an accessible name | **à trancher** |
| `aria-tab-name` | Ensure every ARIA tab node has an accessible name | **à trancher** |

## 7.3 — mécanisme `axe-7-3` (partial)

| test | énoncé |
|---|---|
| 7.3.1 | Chaque élément possédant un gestionnaire d’événement contrôlé par un script vérifie-t-il une de ces conditions (hors cas particuliers) ? |
| 7.3.2 | Un script ne doit pas supprimer le focus d’un élément qui le reçoit. Cette règle est-elle respectée (hors cas particuliers) ? |

| règle axe | description | test(s) proposé(s) |
|---|---|---|
| `scrollable-region-focusable` | Ensure elements that have scrollable content are accessible by keyboard in Safari | **à trancher** |
| `server-side-image-map` | Ensure that server-side image maps are not used | **à trancher** |
| `focus-order-semantics` | Ensure elements in the focus order have a role appropriate for interactive content | **à trancher** |

## 9.1 — mécanisme `axe-9-1` (partial)

| test | énoncé |
|---|---|
| 9.1.1 | Dans chaque page web, la hiérarchie entre les titres (balise `<hx>` ou balise possédant un attribut WAI-ARIA `role="heading"` associé à un attribut WAI-ARIA `aria-level`) |
| 9.1.2 | Dans chaque page web, le contenu de chaque titre (balise `<hx>` ou balise possédant un attribut WAI-ARIA `role="heading"` associé à un attribut WAI-ARIA `aria-level`) est |
| 9.1.3 | Dans chaque page web, chaque passage de texte constituant un titre est-il structuré à l’aide d’une balise `<hx>` ou d’une balise possédant un attribut WAI-ARIA `role="hea |

| règle axe | description | test(s) proposé(s) |
|---|---|---|
| `heading-order` | Ensure the order of headings is semantically correct | **à trancher** |
| `landmark-one-main` | Ensure the document has a main landmark | **à trancher** |
| `region` | Ensure all page content is contained by landmarks | **à trancher** |
| `p-as-heading` | Ensure bold, italic text and font-size is not used to style &lt;p&gt; elements as a heading | **à trancher** |

## 9.3 — mécanisme `axe-9-3` (complete)

| test | énoncé |
|---|---|
| 9.3.1 | Dans chaque page web, les informations regroupées visuellement sous forme de liste non ordonnée vérifient-elles une de ces conditions ? |
| 9.3.2 | Dans chaque page web, les informations regroupées visuellement sous forme de liste ordonnée vérifient-elles une de ces conditions ? |
| 9.3.3 | Dans chaque page web, les informations regroupées sous forme de liste de description utilisent-elles les balises `<dl>` et `<dt>/<dd>` ? |

| règle axe | description | test(s) proposé(s) |
|---|---|---|
| `list` | Ensure that lists are structured correctly | **à trancher** |
| `listitem` | Ensure &lt;li&gt; elements are used semantically | **à trancher** |
| `definition-list` | Ensure &lt;dl&gt; elements are structured correctly | **à trancher** |
| `dlitem` | Ensure &lt;dt&gt; and &lt;dd&gt; elements are contained by a &lt;dl&gt; | **à trancher** |

## 10.4 — mécanisme `axe-10-4` (partial)

| test | énoncé |
|---|---|
| 10.4.1 | Dans chaque page web, l’augmentation de la taille des caractères jusqu’à 200 %, au moins, ne doit pas provoquer de perte d’information. Cette règle est-elle respectée sel |
| 10.4.2 | Dans chaque page web, l’augmentation de la taille des caractères jusqu’à 200 %, au moins, doit être possible pour l’ensemble du texte dans la page. Cette règle est-elle r |

| règle axe | description | test(s) proposé(s) |
|---|---|---|
| `meta-viewport` | Ensure &lt;meta name=&quot;viewport&quot;&gt; does not disable text scaling and zooming | **à trancher** |

## 10.11 — mécanisme `axe-10-11` (partial)

| test | énoncé |
|---|---|
| 10.11.1 | Pour chaque page web, lorsque le contenu dont le sens de lecture est horizontal est affiché dans une fenêtre réduite à une largeur de 320 px, l’ensemble des informations  |
| 10.11.2 | Pour chaque page web, lorsque le contenu dont le sens de lecture est vertical est affiché dans une fenêtre réduite à une hauteur de 256 px, l’ensemble des informations et |

| règle axe | description | test(s) proposé(s) |
|---|---|---|
| `meta-viewport` | Ensure &lt;meta name=&quot;viewport&quot;&gt; does not disable text scaling and zooming | **à trancher** |

## 11.1 — mécanisme `axe-11-1` (complete)

| test | énoncé |
|---|---|
| 11.1.1 | Chaque champ de formulaire vérifie-t-il une de ces conditions ? |
| 11.1.2 | Chaque champ de formulaire associé à une balise `<label>` ayant un attribut `for`, vérifie-t-il ces conditions ? |
| 11.1.3 | Chaque champ de formulaire ayant une étiquette dont le contenu n’est pas visible ou à proximité (masqué, `aria-label`) ou qui n’est pas accolé au champ (`aria-labelledby` |

| règle axe | description | test(s) proposé(s) |
|---|---|---|
| `label` | Ensure every form element has a label | **à trancher** |
| `label-title-only` | Ensure that every form element has a visible label and is not solely labeled using hidden label | **à trancher** |
| `input-image-alt` | Ensure &lt;input type=&quot;image&quot;&gt; elements have alternative text | **à trancher** |
| `button-name` | Ensure buttons have discernible text | **à trancher** |
| `input-button-name` | Ensure input buttons have discernible text | **à trancher** |
| `select-name` | Ensure select element has an accessible name | **à trancher** |
| `aria-input-field-name` | Ensure every ARIA input field has an accessible name | **à trancher** |
| `aria-toggle-field-name` | Ensure every ARIA toggle field has an accessible name | **à trancher** |
| `form-field-multiple-labels` | Ensure form field does not have multiple label elements | **à trancher** |

## 11.2 — mécanisme `axe-11-2` (partial)

| test | énoncé |
|---|---|
| 11.2.1 | Chaque balise `<label>` permet-elle de connaître la fonction exacte du champ de formulaire auquel elle est associée ? |
| 11.2.2 | Chaque attribut `title` permet-il de connaître la fonction exacte du champ de formulaire auquel il est associé ? |
| 11.2.3 | Chaque étiquette implémentée via l’attribut WAI-ARIA `aria-label` permet-elle de connaître la fonction exacte du champ de formulaire auquel elle est associée ? |
| 11.2.4 | Chaque passage de texte associé via l’attribut WAI-ARIA `aria-labelledby` permet-il de connaître la fonction exacte du champ de formulaire auquel il est associé ? |
| 11.2.5 | Chaque champ de formulaire ayant un intitulé visible vérifie-t-il ces conditions (hors cas particuliers) ? |
| 11.2.6 | Chaque bouton adjacent au champ de formulaire qui fournit une étiquette visible permet-il de connaître la fonction exacte du champ de formulaire auquel il est associé ? |

| règle axe | description | test(s) proposé(s) |
|---|---|---|
| `label-content-name-mismatch` | Ensure that elements labelled through their content must have their visible text as part of the | **à trancher** |

## 12.4 — mécanisme `axe-12-4` (partial)

| test | énoncé |
|---|---|
| 12.4.1 | Dans chaque ensemble de pages, la page « plan du site » est-elle accessible à partir d’une fonctionnalité identique ? |
| 12.4.2 | Dans chaque ensemble de pages, la fonctionnalité vers la page « plan du site » est-elle située à la même place dans la présentation ? |
| 12.4.3 | Dans chaque ensemble de pages, la fonctionnalité vers la page « plan du site » se présente-t-elle toujours dans le même ordre relatif dans le code source ? |

| règle axe | description | test(s) proposé(s) |
|---|---|---|
| `landmark-one-main` | Ensure the document has a main landmark | **à trancher** |
| `region` | Ensure all page content is contained by landmarks | **à trancher** |

## 12.7 — mécanisme `axe-12-7` (complete)

| test | énoncé |
|---|---|
| 12.7.1 | Dans chaque page web, un lien permet-il d’éviter la zone de contenu principal ou d’y accéder (hors cas particuliers) ? |
| 12.7.2 | Dans chaque ensemble de pages, le lien d’évitement ou d’accès rapide à la zone de contenu principal vérifie-t-il ces conditions (hors cas particuliers) ? |

| règle axe | description | test(s) proposé(s) |
|---|---|---|
| `bypass` | Ensure each page has at least one mechanism for a user to bypass navigation and jump straight t | **à trancher** |
| `skip-link` | Ensure all skip links have a focusable target | **à trancher** |

## 13.1 — mécanisme `axe-13-1` (partial)

| test | énoncé |
|---|---|
| 13.1.1 | Pour chaque page web, chaque procédé de rafraîchissement (balise `<object>`, balise `<embed>`, balise `<svg>`, balise `<canvas>`, balise `<meta>`) vérifie-t-il une de ces |
| 13.1.2 | Pour chaque page web, chaque procédé de redirection effectué via une balise `<meta>` est-il immédiat (hors cas particuliers) ? |
| 13.1.3 | Pour chaque page web, chaque procédé de redirection effectué via un script vérifie-t-il une de ces conditions (hors cas particuliers) ? |
| 13.1.4 | Pour chaque page web, chaque procédé limitant le temps d’une session vérifie-t-il une de ces conditions (hors cas particuliers) ? |

| règle axe | description | test(s) proposé(s) |
|---|---|---|
| `meta-refresh` | Ensure &lt;meta http-equiv=&quot;refresh&quot;&gt; is not used for delayed refresh | **à trancher** |

## 13.8 — mécanisme `axe-13-8` (partial)

| test | énoncé |
|---|---|
| 13.8.1 | Dans chaque page web, chaque contenu en mouvement déclenché automatiquement, vérifie-t-il une de ces conditions ? |
| 13.8.2 | Dans chaque page web, chaque contenu clignotant déclenché automatiquement, vérifie-t-il une de ces conditions ? |

| règle axe | description | test(s) proposé(s) |
|---|---|---|
| `blink` | Ensure &lt;blink&gt; elements are not used | **à trancher** |
| `marquee` | Ensure &lt;marquee&gt; elements are not used | **à trancher** |

## 11.9 — mécanisme `axe-11-9` (partial)

| test | énoncé |
|---|---|
| 11.9.1 | L’intitulé de chaque bouton vérifie-t-il ces conditions (hors cas particuliers) ? |
| 11.9.2 | Chaque bouton affichant un intitulé visible vérifie-t-il ces conditions (hors cas particuliers) ? |

| règle axe | description | test(s) proposé(s) |
|---|---|---|
| `button-name` | Ensure buttons have discernible text | **à trancher** |
| `input-button-name` | Ensure input buttons have discernible text | **à trancher** |
