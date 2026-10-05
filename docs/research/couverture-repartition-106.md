# Répartition de l’analyse des 106 critères RGAA 4.1.2 (258 tests)

> Date : 2026-10-05. Données : `couverture-repartition-106.json` (même dossier). Source des critères : `criteres.json` du dépôt, identique au PDF DINUM.

**Légende — moteur principal** : **A** = axe-core (verdict direct) · **D** = règle déterministe maison (DOM/CSS/HTTP, Playwright, crawler multi-pages, linter) · **H** = Holo IA (vision + sémantique) · **M** = validation humaine obligatoire (`needs_review`).

Les colonnes *Déterministe* et *Holo* décrivent le **rôle secondaire** : chaque critère est analysé par plusieurs couches, le moteur principal porte le verdict.

| Critère | Tests | Principal | Axe-core | Déterministe | Holo IA | Reste humain |
|---|---|---|---|---|---|---|
| 1.1 Chaque image porteuse d’information a-t-elle une alternative textuelle… | 8 | **A** | image-alt, input-image-alt, area-alt, role-img-alt, svg-img-alt, object-alt | alt présent sur <img>/role=img/<area>/<input image>/<object>/<svg>/<canvas> | pertinence si alt vide douteux |  |
| 1.2 Chaque image de décoration est-elle correctement ignorée par les techn… | 6 | **A** | image-redundant-alt, presentation-role-conflict | alt="" / aria-hidden / role=presentation sur images décoratives | décide si une image est réellement décorative |  |
| 1.3 Pour chaque image porteuse d’information ayant une alternative textuel… | 9 | **H** | image-alt | extraction alt + contexte (parent, légende, lien) | alt pertinent vs contenu de l’image (vision) |  |
| 1.4 Pour chaque image utilisée comme CAPTCHA ou comme image-test, ayant un… | 7 | **H** |  | détection CAPTCHA (iframe recaptcha/hcaptcha, noms/classes) | alt CAPTCHA identifie nature/fonction sans livrer la solution |  |
| 1.5 Pour chaque image utilisée comme CAPTCHA, une solution d’accès alterna… | 2 | **H** |  | détection CAPTCHA + présence d’une solution alternative (audio, question logique) | alternative équivalente en finalité |  |
| 1.6 Chaque image porteuse d’information a-t-elle, si nécessaire, une descr… | 10 | **D** |  | longdesc, aria-describedby, lien/bloc adjacent, <details> après l’image | besoin d’une description détaillée (graphique, schéma) |  |
| 1.7 Pour chaque image porteuse d’information ayant une description détaill… | 6 | **H** |  | résolution de la cible de la description détaillée | pertinence de la description vs image |  |
| 1.8 Chaque image texte porteuse d’information, en l’absence d’un mécanisme… | 6 | **H** |  | détection images contenant du texte (<img>, SVG, canvas) vs texte stylable possible | OCR + décision « texte image remplaçable par du texte » |  |
| 1.9 Chaque légende d’image est-elle, si nécessaire, correctement reliée à… | 5 | **D** |  | <figure>/<figcaption>, role=figure, aria-labelledby/describedby | légende réellement associée à l’image |  |
| 2.1 Chaque cadre a-t-il un titre de cadre ?… | 1 | **A** | frame-title, frame-title-unique | title non vide sur iframe/frame |  |  |
| 2.2 Pour chaque cadre ayant un titre de cadre, ce titre de cadre est-il pe… | 1 | **H** |  | extraction title + src/contenu du cadre | titre pertinent vs contenu du cadre |  |
| 3.1 Dans chaque page web, l’information ne doit pas être donnée uniquement… | 6 | **H** | link-in-text-block | styles calculés (couleur seule, liens sans soulignement, champs requis en rouge) | vision : l’information survit-elle en niveaux de gris |  |
| 3.2 Dans chaque page web, le contraste entre la couleur du texte et la cou… | 5 | **A** | color-contrast | styles calculés des cas « incomplete » (dégradés, images de fond, pseudo-éléments) | contraste de texte sur image de fond (vision) |  |
| 3.3 Dans chaque page web, les couleurs utilisées dans les composants d’int… | 4 | **D** |  | couleurs calculées bordures/icônes/focus, ratio ≥ 3:1 | éléments graphiques porteurs d’information (vision) |  |
| 4.1 Chaque média temporel pré-enregistré a-t-il, si nécessaire, une transc… | 3 | **D** |  | détection média (<video>,<audio>,<object>, embeds YouTube/Vimeo) + lien transcription adjacent | pertinence du contexte | visionnage/écoute si pas de transcription |
| 4.2 Pour chaque média temporel pré-enregistré ayant une transcription text… | 3 | **M** |  | présence du bloc de transcription / audiodescription | pré-tri : transcription vs contenu textuel du média | vérifier fidélité au contenu |
| 4.3 Chaque média temporel synchronisé pré-enregistré a-t-il, si nécessaire… | 2 | **A** | video-caption, audio-caption | <track kind=captions|subtitles> présent, WebVTT valide | sous-titres des players embarqués (vision frame) |  |
| 4.4 Pour chaque média temporel synchronisé pré-enregistré ayant des sous-t… | 1 | **M** |  | présence piste de sous-titres | alignement texte/frames échantillonnées | synchronisation et pertinence à l’écoute |
| 4.5 Chaque média temporel pré-enregistré a-t-il, si nécessaire, une audiod… | 2 | **D** |  | <track kind=descriptions>, piste audio AD, version alternative | vision : scènes riches en information visuelle |  |
| 4.6 Pour chaque média temporel pré-enregistré ayant une audiodescription s… | 2 | **M** |  | présence audiodescription | pré-tri | pertinence et synchro à l’écoute |
| 4.7 Chaque média temporel est-il clairement identifiable (hors cas particu… | 1 | **D** |  | média identifié par légende/titre adjacent | pertinence de l’identification |  |
| 4.8 Chaque média non temporel a-t-il, si nécessaire, une alternative (hors… | 2 | **D** |  | alternative au média non temporel (<object>, <canvas>, SVG animé) : lien/texte | pertinence de l’alternative |  |
| 4.9 Pour chaque média non temporel ayant une alternative, cette alternativ… | 1 | **H** |  | alternative résolue | pertinence vs média |  |
| 4.10 Chaque son déclenché automatiquement est-il contrôlable par l’utilisat… | 1 | **A** | no-autoplay-audio | autoplay sans muted, durée > 3 s, contrôle de volume/pause |  |  |
| 4.11 La consultation de chaque média temporel est-elle, si nécessaire, cont… | 3 | **D** |  | controls, boutons lecture/pause/stop/volume atteignables au clavier (Playwright) |  |  |
| 4.12 La consultation de chaque média non temporel est-elle contrôlable par… | 2 | **D** |  | média non temporel contrôlable clavier/pointeur (Playwright) |  |  |
| 4.13 Chaque média temporel et non temporel est-il compatible avec les techn… | 2 | **D** |  | attributs ARIA des lecteurs (nom, rôle, état) via arbre d’accessibilité | compatibilité AT sur players custom |  |
| 5.1 Chaque tableau de données complexe a-t-il un résumé ?… | 1 | **D** |  | détection tableau de données complexe (colspan/rowspan imbriqués, > 2 niveaux d’en-têtes) + résumé | confirme la complexité |  |
| 5.2 Pour chaque tableau de données complexe ayant un résumé, celui-ci est-… | 1 | **H** |  | extraction du résumé | pertinence vs tableau |  |
| 5.3 Pour chaque tableau de mise en forme, le contenu linéarisé reste-t-il… | 1 | **H** |  | table role=presentation/layout détectée | linéarisation compréhensible |  |
| 5.4 Pour chaque tableau de données ayant un titre, le titre est-il correct… | 1 | **A** | table-fake-caption | <caption>, aria-labelledby, figcaption rattaché au tableau |  |  |
| 5.5 Pour chaque tableau de données ayant un titre, celui-ci est-il pertine… | 1 | **H** |  | extraction du titre | pertinence vs tableau |  |
| 5.6 Pour chaque tableau de données, chaque en-tête de colonne et chaque en… | 4 | **A** | th-has-data-cells, scope-attr-valid, td-has-header | <th> sur en-têtes de colonne/ligne | en-tête visuellement identifié mais balisé en <td> |  |
| 5.7 Pour chaque tableau de données, la technique appropriée permettant d’a… | 5 | **A** | td-headers-attr, th-has-data-cells, scope-attr-valid, td-has-header | scope/headers/id corrects, ARIA columnheader/rowheader |  |  |
| 5.8 Chaque tableau de mise en forme ne doit pas utiliser d’éléments propre… | 1 | **D** |  | tableau de mise en forme sans th/caption/summary/headers/scope |  |  |
| 6.1 Chaque lien est-il explicite (hors cas particuliers) ?… | 5 | **H** | identical-links-same-purpose | intitulé + contexte (aria-label, title, parent, cellule) via arbre AX | lien explicite hors contexte, liens identiques mêmes cibles |  |
| 6.2 Dans chaque page web, chaque lien a-t-il un intitulé ?… | 1 | **A** | link-name | nom accessible non vide (a, role=link) |  |  |
| 7.1 Chaque script est-il, si nécessaire, compatible avec les technologies… | 3 | **H** | aria-allowed-attr, aria-valid-attr, aria-valid-attr-value, aria-required-attr, aria-roles, aria-hidden-focus, nested-interactive | widgets JS : rôle/nom/état dans l’arbre AX avant/après interaction | widget compris par une AT (état annoncé) |  |
| 7.2 Pour chaque script ayant une alternative, cette alternative est-elle p… | 2 | **H** |  | script ayant une alternative (noscript, lien) | pertinence de l’alternative |  |
| 7.3 Chaque script est-il contrôlable par le clavier et par tout dispositif… | 2 | **D** | scrollable-region-focusable, tabindex, nested-interactive, frame-focusable-content | Playwright : tabulation, Entrée/Espace sur chaque handler, parité clavier/pointeur | widgets exotiques (drag & drop) |  |
| 7.4 Pour chaque script qui initie un changement de contexte, l’utilisateur… | 1 | **D** |  | Playwright : focus/saisie dans chaque champ → URL, nouvelle fenêtre, focus déplacé ; avertissement présent | l’avertissement est-il compréhensible |  |
| 7.5 Dans chaque page web, les messages de statut sont-ils correctement res… | 3 | **M** | aria-allowed-role | déclenche action, observe régions role=status|alert|log, aria-live, présence du message dans l’AX tree | message visible ⇔ annoncé | vérification lecteur d’écran réelle (NVDA/VO) |
| 8.1 Chaque page web est-elle définie par un type de document ?… | 3 | **D** |  | doctype dans le HTML source (HTTP brut) |  |  |
| 8.2 Pour chaque page web, le code source généré est-il valide selon le typ… | 1 | **D** | duplicate-id-aria | validateur Nu Html Checker sur le DOM généré (hors règles hors-périmètre RGAA) |  |  |
| 8.3 Dans chaque page web, la langue par défaut est-elle présente ?… | 1 | **A** | html-has-lang | lang sur <html> |  |  |
| 8.4 Pour chaque page web ayant une langue par défaut, le code de langue es… | 1 | **A** | html-lang-valid, html-xml-lang-mismatch | code BCP47 valide | langue détectée du contenu = langue déclarée |  |
| 8.5 Chaque page web a-t-elle un titre de page ?… | 1 | **A** | document-title | <title> non vide |  |  |
| 8.6 Pour chaque page web ayant un titre de page, ce titre est-il pertinent… | 1 | **H** |  | title + h1 + contenu | pertinence et unicité du titre |  |
| 8.7 Dans chaque page web, chaque changement de langue est-il indiqué dans… | 1 | **D** |  | détection de langue par segment (cld/lingua) vs lang ancêtre | passages courts/ambigus |  |
| 8.8 Dans chaque page web, le code de langue de chaque changement de langue… | 1 | **A** | valid-lang | codes lang des éléments | pertinence de la langue annoncée |  |
| 8.9 Dans chaque page web, les balises ne doivent pas être utilisées unique… | 1 | **D** |  | balises de présentation (b, i, font, center, blink…), tableaux de mise en forme, <br> décoratifs |  |  |
| 8.10 Dans chaque page web, les changements du sens de lecture sont-ils sign… | 2 | **D** |  | dir/CSS direction, bdo, détection de scripts RTL | changement de sens bien signalé |  |
| 9.1 Dans chaque page web, l’information est-elle structurée par l’utilisat… | 3 | **A** | heading-order, empty-heading, page-has-heading-one | role=heading + aria-level | hiérarchie pertinente vs maquette visuelle (vision) |  |
| 9.2 Dans chaque page web, la structure du document est-elle cohérente (hor… | 1 | **H** | landmark-one-main, region, landmark-no-duplicate-banner | présence header/nav/main/footer/aside | structure cohérente avec la page rendue |  |
| 9.3 Dans chaque page web, chaque liste est-elle correctement structurée ?… | 3 | **A** | list, listitem, definition-list, dlitem | faux listes (tirets, <br>, paragraphes consécutifs) | confirme les faux positifs |  |
| 9.4 Dans chaque page web, chaque citation est-elle correctement indiquée ?… | 2 | **D** |  | <blockquote>/<q> vs guillemets/retraits stylés | détecte citations non balisées (sémantique) |  |
| 10.1 Dans le site web, des feuilles de styles sont-elles utilisées pour con… | 3 | **D** |  | HTML : attributs de présentation (align, bgcolor, width…), <style> inline, feuilles CSS liées |  |  |
| 10.2 Dans chaque page web, le contenu visible porteur d’information reste-t… | 1 | **D** |  | Playwright : CSS désactivée → comparaison texte visible vs texte rendu | disparition de contenu informatif (diff vision) |  |
| 10.3 Dans chaque page web, l’information reste-t-elle compréhensible lorsqu… | 1 | **H** |  | ordre DOM linéarisé sans CSS | ordre compréhensible |  |
| 10.4 Dans chaque page web, le texte reste-t-il lisible lorsque la taille de… | 2 | **D** | meta-viewport | Playwright : zoom texte 200 %, détection clipping/chevauchement/overflow | vision : texte lisible |  |
| 10.5 Dans chaque page web, les déclarations CSS de couleurs de fond d’éléme… | 3 | **D** |  | analyse CSS : color et background-color toujours déclarées ensemble |  |  |
| 10.6 Dans chaque page web, chaque lien dont la nature n’est pas évidente es… | 1 | **A** | link-in-text-block | distinction visuelle des liens (couleur ≥ 3:1, soulignement) | nature du lien non évidente |  |
| 10.7 Dans chaque page web, pour chaque élément recevant le focus, la prise… | 1 | **D** | focus-order-semantics | Playwright : tab sur chaque élément focusable, diff de capture avant/après | indicateur de focus perceptible (vision) |  |
| 10.8 Pour chaque page web, les contenus cachés ont-ils vocation à être igno… | 1 | **A** | aria-hidden-focus, hidden-content | CSS display:none/visibility/hidden/aria-hidden vs intention (texte caché pour AT) | contenu masqué à tort |  |
| 10.9 Dans chaque page web, l’information ne doit pas être donnée uniquement… | 4 | **H** |  | styles calculés (forme/taille/position) | information portée uniquement par la forme/position (vision) |  |
| 10.10 Dans chaque page web, l’information ne doit pas être donnée par la for… | 4 | **H** |  | styles calculés | information portée uniquement par la forme/taille/position, sans alternative |  |
| 10.11 Pour chaque page web, les contenus peuvent-ils être présentés sans per… | 2 | **D** | meta-viewport, meta-viewport-large | Playwright : viewport 320×256 px, scrollWidth/scrollHeight, 400 % zoom | perte d’information (vision) |  |
| 10.12 Dans chaque page web, les propriétés d’espacement du texte peuvent-ell… | 1 | **D** | avoid-inline-spacing | Playwright : injection line-height 1.5 / letter-spacing .12em / word-spacing .16em / paragraph 2em, détection clipping |  |  |
| 10.13 Dans chaque page web, les contenus additionnels apparaissant à la pris… | 3 | **D** |  | Playwright : hover/focus, Échap, survol du contenu additionnel, persistance | contenu additionnel pertinent |  |
| 10.14 Dans chaque page web, les contenus additionnels apparaissant via les s… | 2 | **D** |  | CSS :hover/:focus générant du contenu (content, display) + test clavier |  |  |
| 11.1 Chaque champ de formulaire a-t-il une étiquette ?… | 3 | **A** | label, select-name, input-button-name, form-field-multiple-labels, aria-input-field-name | label/for, aria-label, aria-labelledby, title (pas seul) |  |  |
| 11.2 Chaque étiquette associée à un champ de formulaire est-elle pertinente… | 6 | **H** | label-content-name-mismatch, label-title-only | nom accessible vs libellé visible | pertinence de l’étiquette |  |
| 11.3 Dans chaque formulaire, chaque étiquette associée à un champ de formul… | 2 | **D** |  | comparaison des étiquettes de champs de même fonction à travers le formulaire | même fonction ⇒ même sens |  |
| 11.4 Dans chaque formulaire, chaque étiquette de champ et son champ associé… | 3 | **D** |  | Playwright : bounding boxes label ↔ champ (adjacents, ordre)  |  |  |
| 11.5 Dans chaque formulaire, les champs de même nature sont-ils regroupés,… | 1 | **D** |  | <fieldset>, role=group, aria-labelledby | regroupement nécessaire (radios/checkboxes/champs adresse) |  |
| 11.6 Dans chaque formulaire, chaque regroupement de champs de même nature a… | 1 | **D** |  | <legend> / aria-label sur chaque regroupement |  |  |
| 11.7 Dans chaque formulaire, chaque légende associée à un regroupement de c… | 1 | **H** |  | extraction légende + champs du groupe | légende pertinente |  |
| 11.8 Dans chaque formulaire, les items de même nature d’une liste de choix… | 3 | **D** |  | <optgroup label> pour listes longues | regroupement logique |  |
| 11.9 Dans chaque formulaire, l’intitulé de chaque bouton est-il pertinent (… | 2 | **A** | button-name, input-button-name | nom accessible des boutons | intitulé pertinent |  |
| 11.10 Dans chaque formulaire, le contrôle de saisie est-il utilisé de manièr… | 7 | **H** |  | required/aria-required/pattern, indications de format, messages | contrôle de saisie pertinent (formats, champs obligatoires) |  |
| 11.11 Dans chaque formulaire, le contrôle de saisie est-il accompagné, si né… | 2 | **H** |  | erreurs de saisie : aria-describedby, aria-invalid, role=alert | suggestion de correction utile |  |
| 11.12 Pour chaque formulaire qui modifie ou supprime des données, ou qui tr… | 2 | **M** |  | détection formulaire transmettant données (méthode POST, paiement, compte) | présence d’une étape confirmation/annulation/vérification | test réel du parcours (écriture en base) |
| 11.13 La finalité d’un champ de saisie peut-elle être déduite pour faciliter… | 1 | **A** | autocomplete-valid | attribut autocomplete valide, champs de finalité connue (nom, email, adresse…) | finalité déduite du libellé |  |
| 12.1 Chaque ensemble de pages dispose-t-il de deux systèmes de navigation d… | 1 | **D** |  | crawler multi-pages : nav + plan du site + moteur de recherche (≥ 2 systèmes) |  |  |
| 12.2 Dans chaque ensemble de pages, le menu et les barres de navigation son… | 1 | **D** |  | crawler : position/ordre du menu et de la barre de navigation identiques sur les pages de l’ensemble |  |  |
| 12.3 La page « plan du site » est-elle pertinente ?… | 3 | **H** |  | plan du site détecté et analysé | pertinence/exhaustivité du plan du site |  |
| 12.4 Dans chaque ensemble de pages, la page « plan du site » est-elle acces… | 3 | **D** |  | crawler : lien « plan du site » présent sur chaque page, même place |  |  |
| 12.5 Dans chaque ensemble de pages, le moteur de recherche est-il atteignab… | 3 | **D** |  | crawler : champ de recherche accessible de la même manière sur toutes les pages |  |  |
| 12.6 Les zones de regroupement de contenus présentes dans plusieurs pages w… | 1 | **A** | landmark-one-main, region, landmark-unique, landmark-banner-is-top-level | landmarks et regroupements communs aux pages, entre pages |  |  |
| 12.7 Dans chaque page web, un lien d’évitement ou d’accès rapide à la zone… | 2 | **A** | bypass, skip-link | Playwright : 1er Tab → lien d’évitement, cible existante et visible au focus |  |  |
| 12.8 Dans chaque page web, l’ordre de tabulation est-il cohérent ?… | 2 | **D** | focus-order-semantics | Playwright : séquence de tabulation vs ordre visuel (coordonnées) | ordre cohérent (vision) |  |
| 12.9 Dans chaque page web, la navigation ne doit pas contenir de piège au c… | 1 | **D** |  | Playwright : tabulation N fois, détection de piège (focus bloqué hors modale) |  |  |
| 12.10 Dans chaque page web, les raccourcis clavier n’utilisant qu’une seule… | 1 | **D** | accesskeys | analyse JS : keydown/keypress sans modificateur ; raccourcis désactivables |  |  |
| 12.11 Dans chaque page web, les contenus additionnels apparaissant au survol… | 1 | **D** |  | Playwright : contenu additionnel au hover/focus atteignable au clavier (ordre DOM) |  |  |
| 13.1 Pour chaque page web, l’utilisateur a-t-il le contrôle de chaque limit… | 4 | **M** | meta-refresh, meta-refresh-no-exceptions | meta refresh, setTimeout de redirection/déconnexion, compteurs de session |  | cas de limite de temps applicative (session, panier) |
| 13.2 Dans chaque page web, l’ouverture d’une nouvelle fenêtre ne doit pas ê… | 1 | **D** |  | target=_blank, window.open, popups à l’ouverture sans action utilisateur ; Playwright popup events |  |  |
| 13.3 Dans chaque page web, chaque document bureautique en téléchargement po… | 1 | **D** |  | liens vers .pdf/.doc(x)/.odt/.xls(x)/.ppt(x)/.epub (extension + Content-Type) ; PDF : tags, titre, langue (veraPDF) | version accessible disponible/signalée |  |
| 13.4 Pour chaque document bureautique ayant une version accessible, cette v… | 1 | **M** |  | détection de la version accessible proposée | comparaison texte version accessible vs original | équivalence d’information |
| 13.5 Dans chaque page web, chaque contenu cryptique (art ASCII, émoticône,… | 1 | **D** |  | regex ASCII-art, émoticônes, notations cryptiques | détecte les cas non couverts par regex |  |
| 13.6 Dans chaque page web, pour chaque contenu cryptique (art ASCII, émotic… | 1 | **H** |  | alternative des contenus cryptiques | pertinence de l’alternative |  |
| 13.7 Dans chaque page web, les changements brusques de luminosité ou les ef… | 3 | **M** |  | animations CSS/JS rapides ; frames vidéo échantillonnées (3 flashs/s) | analyse de luminance par frames | confirmation d’un flash réel |
| 13.8 Dans chaque page web, chaque contenu en mouvement ou clignotant est-il… | 2 | **A** | blink, marquee | CSS animation > 5 s, <video autoplay loop>, GIF animé ; bouton pause/stop | contrôle atteignable |  |
| 13.9 Dans chaque page web, le contenu proposé est-il consultable quelle que… | 1 | **A** | css-orientation-lock | Playwright : portrait ↔ paysage, contenu/fonctions identiques |  |  |
| 13.10 Dans chaque page web, les fonctionnalités utilisables ou disponibles a… | 2 | **D** |  | JS : pointer events multi-touch/path/drag, touch-action ; alternative à geste simple | alternative réellement fonctionnelle |  |
| 13.11 Dans chaque page web, les actions déclenchées au moyen d’un dispositif… | 1 | **D** | target-size | JS : mousedown/pointerdown/touchstart sans up ; annulation possible |  |  |
| 13.12 Dans chaque page web, les fonctionnalités qui impliquent un mouvement… | 3 | **D** |  | JS : devicemotion/deviceorientation/accelerometer ; alternative UI | alternative équivalente |  |
