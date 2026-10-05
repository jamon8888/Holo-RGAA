# Critères RGAA : traités vs non testés (état du code au 2026-10-05)

**TRAITÉ** = le moteur principal du plan est réellement implémenté pour ce critère (règle axe mappée, gap-fix JS, ou évaluation agent Holo). Les mécanismes gap-fix ajoutés pour le plan sont *partiels* : ils prouvent des échecs, jamais une conformité. **NON TESTÉ** = le contrôle prévu n'existe pas encore ; seul un éventuel repli (indiqué) produit un verdict, sinon `needs_review`. **MANUEL** = humain par conception.

Totaux : {'TRAITÉ': 75, 'NON TESTÉ': 23, 'MANUEL': 8} (12.8 et 12.10 : règles axe `tabindex`/`accesskeys` non retenues, voir #263, #272, #279)

| Critère | Moteur prévu | Statut | Moteurs qui tournent aujourd’hui | Intitulé |
|---|---|---|---|---|
| 1.1 | AxeCore | TRAITÉ | axe(complete)+gap-fix+agent-Holo | Chaque image porteuse d’information a-t-elle une alternative textuelle ? |
| 1.2 | AxeCore | TRAITÉ | axe(partial)+gap-fix+agent-Holo | Chaque image de décoration est-elle correctement ignorée par les technologies d’ |
| 1.3 | Holo | TRAITÉ | agent-Holo | Pour chaque image porteuse d’information ayant une alternative textuelle, cette  |
| 1.4 | Holo | TRAITÉ | agent-Holo | Pour chaque image utilisée comme CAPTCHA ou comme image-test, ayant une alternat |
| 1.5 | Holo | TRAITÉ | agent-Holo | Pour chaque image utilisée comme CAPTCHA, une solution d’accès alternatif au con |
| 1.6 | Deterministic | NON TESTÉ (repli : agent-Holo) | agent-Holo | Chaque image porteuse d’information a-t-elle, si nécessaire, une description dét |
| 1.7 | Holo | TRAITÉ | agent-Holo | Pour chaque image porteuse d’information ayant une description détaillée, cette  |
| 1.8 | Holo | TRAITÉ | agent-Holo | Chaque image texte porteuse d’information, en l’absence d’un mécanisme de rempla |
| 1.9 | Deterministic | TRAITÉ | gap-fix+agent-Holo | Chaque légende d’image est-elle, si nécessaire, correctement reliée à l’image co |
| 2.1 | AxeCore | TRAITÉ | axe(complete)+gap-fix | Chaque cadre a-t-il un titre de cadre ? |
| 2.2 | Holo | TRAITÉ | agent-Holo | Pour chaque cadre ayant un titre de cadre, ce titre de cadre est-il pertinent ? |
| 3.1 | Holo | TRAITÉ | agent-Holo | Dans chaque page web, l’information ne doit pas être donnée uniquement par la co |
| 3.2 | AxeCore | TRAITÉ | axe(complete)+gap-fix | Dans chaque page web, le contraste entre la couleur du texte et la couleur de so |
| 3.3 | Deterministic | NON TESTÉ | — | Dans chaque page web, les couleurs utilisées dans les composants d’interface ou  |
| 4.1 | Deterministic | TRAITÉ | gap-fix+agent-Holo | Chaque média temporel pré-enregistré a-t-il, si nécessaire, une transcription te |
| 4.2 | Human | MANUEL (par conception) | agent-Holo | Pour chaque média temporel pré-enregistré ayant une transcription textuelle ou u |
| 4.3 | AxeCore | TRAITÉ | axe(complete)+agent-Holo | Chaque média temporel synchronisé pré-enregistré a-t-il, si nécessaire, des sous |
| 4.4 | Human | MANUEL (par conception) | agent-Holo | Pour chaque média temporel synchronisé pré-enregistré ayant des sous-titres sync |
| 4.5 | Deterministic | TRAITÉ | gap-fix | Chaque média temporel pré-enregistré a-t-il, si nécessaire, une audiodescription |
| 4.6 | Human | MANUEL (par conception) | agent-Holo | Pour chaque média temporel pré-enregistré ayant une audiodescription synchronisé |
| 4.7 | Deterministic | TRAITÉ | gap-fix+agent-Holo | Chaque média temporel est-il clairement identifiable (hors cas particuliers) ? |
| 4.8 | Deterministic | TRAITÉ | gap-fix+agent-Holo | Chaque média non temporel a-t-il, si nécessaire, une alternative (hors cas parti |
| 4.9 | Holo | TRAITÉ | agent-Holo | Pour chaque média non temporel ayant une alternative, cette alternative est-elle |
| 4.10 | AxeCore | TRAITÉ | axe(partial) | Chaque son déclenché automatiquement est-il contrôlable par l’utilisateur ? |
| 4.11 | Deterministic | TRAITÉ | gap-fix | La consultation de chaque média temporel est-elle, si nécessaire, contrôlable pa |
| 4.12 | Deterministic | NON TESTÉ | — | La consultation de chaque média non temporel est-elle contrôlable par le clavier |
| 4.13 | Deterministic | NON TESTÉ | — | Chaque média temporel et non temporel est-il compatible avec les technologies d’ |
| 5.1 | Deterministic | TRAITÉ | gap-fix | Chaque tableau de données complexe a-t-il un résumé ? |
| 5.2 | Holo | TRAITÉ | agent-Holo | Pour chaque tableau de données complexe ayant un résumé, celui-ci est-il pertine |
| 5.3 | Holo | TRAITÉ | agent-Holo | Pour chaque tableau de mise en forme, le contenu linéarisé reste-t-il compréhens |
| 5.4 | AxeCore | TRAITÉ | axe(partial) | Pour chaque tableau de données ayant un titre, le titre est-il correctement asso |
| 5.5 | Holo | TRAITÉ | agent-Holo | Pour chaque tableau de données ayant un titre, celui-ci est-il pertinent ? |
| 5.6 | AxeCore | TRAITÉ | axe(complete)+agent-Holo | Pour chaque tableau de données, chaque en-tête de colonne et chaque en-tête de l |
| 5.7 | AxeCore | TRAITÉ | axe(complete)+agent-Holo | Pour chaque tableau de données, la technique appropriée permettant d’associer ch |
| 5.8 | Deterministic | TRAITÉ | gap-fix+agent-Holo | Chaque tableau de mise en forme ne doit pas utiliser d’éléments propres aux  tab |
| 6.1 | Holo | TRAITÉ | axe(partial)+gap-fix+agent-Holo | Chaque lien est-il explicite (hors cas particuliers) ? |
| 6.2 | AxeCore | TRAITÉ | axe(complete) | Dans chaque page web, chaque lien a-t-il un intitulé ? |
| 7.1 | Holo | TRAITÉ | axe(partial)+agent-Holo | Chaque script est-il, si nécessaire, compatible avec les technologies d’assistan |
| 7.2 | Holo | TRAITÉ | agent-Holo | Pour chaque script ayant une alternative, cette alternative est-elle pertinente  |
| 7.3 | Deterministic | TRAITÉ | axe(partial) | Chaque script est-il contrôlable par le clavier et par tout dispositif de pointa |
| 7.4 | Deterministic | TRAITÉ | gap-fix+agent-Holo | Pour chaque script qui initie un changement de contexte, l’utilisateur est-il av |
| 7.5 | Human | MANUEL (par conception) | — | Dans chaque page web, les messages de statut sont-ils correctement restitués par |
| 8.1 | Deterministic | TRAITÉ | gap-fix | Chaque page web est-elle définie par un type de document ? |
| 8.2 | Deterministic | TRAITÉ | axe(partial)+agent-Holo | Pour chaque page web, le code source généré est-il valide selon le type de docum |
| 8.3 | AxeCore | TRAITÉ | axe(complete)+gap-fix+agent-Holo | Dans chaque page web, la langue par défaut est-elle présente ? |
| 8.4 | AxeCore | TRAITÉ | axe(partial)+agent-Holo | Pour chaque page web ayant une langue par défaut, le code de langue est-il perti |
| 8.5 | AxeCore | TRAITÉ | axe(complete)+gap-fix | Chaque page web a-t-elle un titre de page ? |
| 8.6 | Holo | TRAITÉ | agent-Holo | Pour chaque page web ayant un titre de page, ce titre est-il pertinent ? |
| 8.7 | Deterministic | NON TESTÉ (repli : agent-Holo) | agent-Holo | Dans chaque page web, chaque changement de langue est-il indiqué dans le code so |
| 8.8 | AxeCore | TRAITÉ | axe(partial)+agent-Holo | Dans chaque page web, le code de langue de chaque changement de langue est-il va |
| 8.9 | Deterministic | TRAITÉ | gap-fix | Dans chaque page web, les balises ne doivent pas être utilisées uniquement à des |
| 8.10 | Deterministic | TRAITÉ | gap-fix+agent-Holo | Dans chaque page web, les changements du sens de lecture sont-ils signalés ? |
| 9.1 | AxeCore | TRAITÉ | axe(partial)+agent-Holo | Dans chaque page web, l’information est-elle structurée par l’utilisation approp |
| 9.2 | Holo | TRAITÉ | agent-Holo | Dans chaque page web, la structure du document est-elle cohérente (hors cas part |
| 9.3 | AxeCore | TRAITÉ | axe(complete)+agent-Holo | Dans chaque page web, chaque liste est-elle correctement structurée ? |
| 9.4 | Deterministic | TRAITÉ | gap-fix | Dans chaque page web, chaque citation est-elle correctement indiquée ? |
| 10.1 | Deterministic | TRAITÉ | gap-fix+agent-Holo | Dans le site web, des feuilles de styles sont-elles utilisées pour contrôler la  |
| 10.2 | Deterministic | TRAITÉ | gap-fix | Dans chaque page web, le contenu visible porteur d’information reste-t-il présen |
| 10.3 | Holo | TRAITÉ | agent-Holo | Dans chaque page web, l’information reste-t-elle compréhensible lorsque les feui |
| 10.4 | Deterministic | TRAITÉ | axe(partial) | Dans chaque page web, le texte reste-t-il lisible lorsque la taille des caractèr |
| 10.5 | Deterministic | TRAITÉ | gap-fix | Dans chaque page web, les déclarations CSS de couleurs de fond d’élément et de p |
| 10.6 | AxeCore | TRAITÉ | axe(complete)+agent-Holo | Dans chaque page web, chaque lien dont la nature n’est pas évidente est-il visib |
| 10.7 | Deterministic | TRAITÉ | gap-fix+agent-Holo | Dans chaque page web, pour chaque élément recevant le focus, la prise de focus e |
| 10.8 | AxeCore | TRAITÉ | axe(complete) | Pour chaque page web, les contenus cachés ont-ils vocation à être ignorés par le |
| 10.9 | Holo | NON TESTÉ | — | Dans chaque page web, l’information ne doit pas être donnée uniquement par la fo |
| 10.10 | Holo | TRAITÉ | agent-Holo | Dans chaque page web, l’information ne doit pas être donnée par la forme, taille |
| 10.11 | Deterministic | TRAITÉ | axe(partial)+gap-fix | Pour chaque page web, les contenus peuvent-ils être présentés sans perte d’infor |
| 10.12 | Deterministic | NON TESTÉ | — | Dans chaque page web, les propriétés d’espacement du texte peuvent-elles être re |
| 10.13 | Deterministic | NON TESTÉ (repli : agent-Holo) | agent-Holo | Dans chaque page web, les contenus additionnels apparaissant à la prise de focus |
| 10.14 | Deterministic | TRAITÉ | gap-fix | Dans chaque page web, les contenus additionnels apparaissant via les styles CSS  |
| 11.1 | AxeCore | TRAITÉ | axe(complete)+gap-fix+agent-Holo | Chaque champ de formulaire a-t-il une étiquette ? |
| 11.2 | Holo | TRAITÉ | axe(partial)+agent-Holo | Chaque étiquette associée à un champ de formulaire est-elle pertinente (hors cas |
| 11.3 | Deterministic | NON TESTÉ (repli : agent-Holo) | agent-Holo | Dans chaque formulaire, chaque étiquette associée à un champ de formulaire ayant |
| 11.4 | Deterministic | TRAITÉ | gap-fix+agent-Holo | Dans chaque formulaire, chaque étiquette de champ et son champ associé sont-ils  |
| 11.5 | Deterministic | TRAITÉ | gap-fix+agent-Holo | Dans chaque formulaire, les champs de même nature sont-ils regroupés, si nécessa |
| 11.6 | Deterministic | TRAITÉ | gap-fix | Dans chaque formulaire, chaque regroupement de champs de même nature a-t-il une  |
| 11.7 | Holo | TRAITÉ | agent-Holo | Dans chaque formulaire, chaque légende associée à un regroupement de champs de m |
| 11.8 | Deterministic | NON TESTÉ (repli : agent-Holo) | agent-Holo | Dans chaque formulaire, les items de même nature d’une liste de choix sont-ils r |
| 11.9 | AxeCore | TRAITÉ | axe(partial)+agent-Holo | Dans chaque formulaire, l’intitulé de chaque bouton est-il pertinent (hors cas p |
| 11.10 | Holo | TRAITÉ | agent-Holo | Dans chaque formulaire, le contrôle de saisie est-il utilisé de manière pertinen |
| 11.11 | Holo | NON TESTÉ | — | Dans chaque formulaire, le contrôle de saisie est-il accompagné, si nécessaire,  |
| 11.12 | Human | MANUEL (par conception) | agent-Holo | Pour chaque formulaire qui modifie ou supprime des données, ou qui transmet des |
| 11.13 | AxeCore | TRAITÉ | axe(partial)+agent-Holo | La finalité d’un champ de saisie peut-elle être déduite pour faciliter le rempli |
| 12.1 | Deterministic | NON TESTÉ | — | Chaque ensemble de pages dispose-t-il de deux systèmes de navigation différents, |
| 12.2 | Deterministic | NON TESTÉ | — | Dans chaque ensemble de pages, le menu et les barres de navigation sont-ils touj |
| 12.3 | Holo | TRAITÉ | agent-Holo | La page « plan du site » est-elle pertinente ? |
| 12.4 | Deterministic | NON TESTÉ | — | Dans chaque ensemble de pages, la page « plan du site » est-elle accessible à pa |
| 12.5 | Deterministic | NON TESTÉ | — | Dans chaque ensemble de pages, le moteur de recherche est-il atteignable de mani |
| 12.6 | AxeCore | TRAITÉ | axe(partial)+agent-Holo | Les zones de regroupement de contenus présentes dans plusieurs pages web (zones  |
| 12.7 | AxeCore | TRAITÉ | axe(complete)+gap-fix+agent-Holo | Dans chaque page web, un lien d’évitement ou d’accès rapide à la zone de contenu |
| 12.8 | Deterministic | NON TESTÉ (repli : agent-Holo) | agent-Holo | Dans chaque page web, l’ordre de tabulation est-il cohérent ? |
| 12.9 | Deterministic | NON TESTÉ | — | Dans chaque page web, la navigation ne doit pas contenir de piège au clavier. Ce |
| 12.10 | Deterministic | NON TESTÉ | — | Dans chaque page web, les raccourcis clavier n’utilisant qu’une seule touche (le |
| 12.11 | Deterministic | NON TESTÉ | — | Dans chaque page web, les contenus additionnels apparaissant au survol, à la pri |
| 13.1 | Human | MANUEL (par conception) | axe(partial) | Pour chaque page web, l’utilisateur a-t-il le contrôle de chaque limite de temps |
| 13.2 | Deterministic | TRAITÉ | gap-fix | Dans chaque page web, l’ouverture d’une nouvelle fenêtre ne doit pas être déclen |
| 13.3 | Deterministic | NON TESTÉ | — | Dans chaque page web, chaque document bureautique en téléchargement possède-t-il |
| 13.4 | Human | MANUEL (par conception) | agent-Holo | Pour chaque document bureautique ayant une version accessible, cette version off |
| 13.5 | Deterministic | TRAITÉ | gap-fix+agent-Holo | Dans chaque page web, chaque contenu cryptique (art ASCII, émoticône, syntaxe cr |
| 13.6 | Holo | TRAITÉ | agent-Holo | Dans chaque page web, pour chaque contenu cryptique (art ASCII, émoticône, synta |
| 13.7 | Human | MANUEL (par conception) | — | Dans chaque page web, les changements brusques de luminosité ou les effets de fl |
| 13.8 | AxeCore | TRAITÉ | axe(partial) | Dans chaque page web, chaque contenu en mouvement ou clignotant est-il contrôlab |
| 13.9 | AxeCore | TRAITÉ | axe(partial) | Dans chaque page web, le contenu proposé est-il consultable quelle que soit l’or |
| 13.10 | Deterministic | NON TESTÉ | — | Dans chaque page web, les fonctionnalités utilisables ou disponibles au moyen d’ |
| 13.11 | Deterministic | NON TESTÉ | — | Dans chaque page web, les actions déclenchées au moyen d’un dispositif de pointa |
| 13.12 | Deterministic | NON TESTÉ | — | Dans chaque page web, les fonctionnalités qui impliquent un mouvement de l’appar |
