---
name: criteria
description: Référence des 106 critères RGAA 4.1.2 — numéro, intitulé officiel, moteur qui porte le verdict, critères WCAG 2.1 associés. Use when the user asks what a criterion requires, how it is tested, or which criteria cover a topic.
---

<!-- FICHIER GÉNÉRÉ par scripts/gen-criteria-skill.py depuis rgaa-core/data/rgaa-4.1.2 — ne pas éditer à la main. -->

# criteria — référentiel RGAA 4.1.2

Intitulés : texte officiel du RGAA 4.1.2 (DINUM), sans les liens du glossaire. Pour la méthode
officielle d'un test, appeler l'outil MCP `get_test_methodology` avec la clé du test (ex. `1.2.3`) :
il retourne la méthode du RGAA, tirée de `RGAA/methodologies.json` (DISIC/
accessibilite.numerique.gouv.fr, commit épinglé).

**Moteur** : qui porte le verdict dans Holo-RGAA (`engine_plan.json`). Le moteur ne dit pas le
niveau d'automatisation de chaque test : un test *Assisté* rend toujours `review` (la machine
collecte la preuve, un relecteur décide), quel que soit le moteur. Aucun intitulé de ce
fichier ne doit être reformulé de mémoire : en cas de doute, citer le texte officiel.

## 1. Images

| Critère | Intitulé | Moteur | WCAG 2.1 |
|---|---|---|---|
| 1.1 | Chaque image porteuse d’information a-t-elle une alternative textuelle ? | Contrôles déterministes | 1.1.1 |
| 1.2 | Chaque image de décoration est-elle correctement ignorée par les technologies d’assistance ? | Preuve + IA (revue humaine) | 1.1.1, 4.1.2 |
| 1.3 | Pour chaque image porteuse d’information ayant une alternative textuelle, cette alternative est-elle pertinente (hors cas particuliers) ? | Preuve + IA (revue humaine) | 1.1.1, 4.1.2 |
| 1.4 | Pour chaque image utilisée comme CAPTCHA ou comme image-test, ayant une alternative textuelle, cette alternative permet-elle d’identifier la nature et la fonction de l’image ? | Preuve + IA (revue humaine) | 1.1.1 |
| 1.5 | Pour chaque image utilisée comme CAPTCHA, une solution d’accès alternatif au contenu ou à la fonction du CAPTCHA est-elle présente ? | Preuve + IA (revue humaine) | 1.1.1 |
| 1.6 | Chaque image porteuse d’information a-t-elle, si nécessaire, une description détaillée ? | Contrôles déterministes | 1.1.1 |
| 1.7 | Pour chaque image porteuse d’information ayant une description détaillée, cette description est-elle pertinente ? | Preuve + IA (revue humaine) | 1.1.1 |
| 1.8 | Chaque image texte porteuse d’information, en l’absence d’un mécanisme de remplacement, doit si possible être remplacée par du texte stylé. Cette règle est-elle respectée (hors cas particuliers) ? | Preuve + IA (revue humaine) | 1.4.5 |
| 1.9 | Chaque légende d’image est-elle, si nécessaire, correctement reliée à l’image correspondante ? | Contrôles déterministes | 1.1.1, 4.1.2 |

## 2. Cadres

| Critère | Intitulé | Moteur | WCAG 2.1 |
|---|---|---|---|
| 2.1 | Chaque cadre a-t-il un titre de cadre ? | axe-core | 4.1.2 |
| 2.2 | Pour chaque cadre ayant un titre de cadre, ce titre de cadre est-il pertinent ? | Preuve + IA (revue humaine) | 4.1.2 |

## 3. Couleurs

| Critère | Intitulé | Moteur | WCAG 2.1 |
|---|---|---|---|
| 3.1 | Dans chaque page web, l’information ne doit pas être donnée uniquement par la couleur. Cette règle est-elle respectée ? | Preuve + IA (revue humaine) | 1.3.1, 1.4.1 |
| 3.2 | Dans chaque page web, le contraste entre la couleur du texte et la couleur de son arrière-plan est-il suffisamment élevé (hors cas particuliers) ? | Contrôles déterministes | 1.4.3 |
| 3.3 | Dans chaque page web, les couleurs utilisées dans les composants d’interface ou les éléments graphiques porteurs d’informations sont-elles suffisamment contrastées (hors cas particuliers) ? | Contrôles déterministes | 1.4.11 |

## 4. Multimédia

| Critère | Intitulé | Moteur | WCAG 2.1 |
|---|---|---|---|
| 4.1 | Chaque média temporel pré-enregistré a-t-il, si nécessaire, une transcription textuelle ou une audiodescription (hors cas particuliers) ? | Contrôles déterministes | 1.2.1, 1.2.3 |
| 4.2 | Pour chaque média temporel pré-enregistré ayant une transcription textuelle ou une audiodescription synchronisée, celles-ci sont-elles pertinentes (hors cas particuliers) ? | Humain (pré-tri machine) | 1.2.1, 1.2.3 |
| 4.3 | Chaque média temporel synchronisé pré-enregistré a-t-il, si nécessaire, des sous-titres synchronisés (hors cas particuliers) ? | axe-core | 1.2.2 |
| 4.4 | Pour chaque média temporel synchronisé pré-enregistré ayant des sous-titres synchronisés, ces sous-titres sont-ils pertinents ? | Humain (pré-tri machine) | 1.2.2 |
| 4.5 | Chaque média temporel pré-enregistré a-t-il, si nécessaire, une audiodescription synchronisée (hors cas particuliers) ? | Contrôles déterministes | 1.2.5 |
| 4.6 | Pour chaque média temporel pré-enregistré ayant une audiodescription synchronisée, celle-ci est-elle pertinente ? | Humain (pré-tri machine) | 1.2.5 |
| 4.7 | Chaque média temporel est-il clairement identifiable (hors cas particuliers) ? | Contrôles déterministes | 1.1.1 |
| 4.8 | Chaque média non temporel a-t-il, si nécessaire, une alternative (hors cas particuliers) ? | Contrôles déterministes | 1.1.1 |
| 4.9 | Pour chaque média non temporel ayant une alternative, cette alternative est-elle pertinente ? | Preuve + IA (revue humaine) | 1.1.1 |
| 4.10 | Chaque son déclenché automatiquement est-il contrôlable par l’utilisateur ? | Contrôles déterministes | 1.4.2 |
| 4.11 | La consultation de chaque média temporel est-elle, si nécessaire, contrôlable par le clavier et tout dispositif de pointage ? | Contrôles déterministes | 2.1.1, 2.1.2 |
| 4.12 | La consultation de chaque média non temporel est-elle contrôlable par le clavier et tout dispositif de pointage ? | Contrôles déterministes | 2.1.1, 2.1.2 |
| 4.13 | Chaque média temporel et non temporel est-il compatible avec les technologies d’assistance (hors cas particuliers) ? | Contrôles déterministes | 4.1.2 |

## 5. Tableaux

| Critère | Intitulé | Moteur | WCAG 2.1 |
|---|---|---|---|
| 5.1 | Chaque tableau de données complexe a-t-il un résumé ? | Contrôles déterministes | 1.3.1 |
| 5.2 | Pour chaque tableau de données complexe ayant un résumé, celui-ci est-il pertinent ? | Preuve + IA (revue humaine) | 1.3.1 |
| 5.3 | Pour chaque tableau de mise en forme, le contenu linéarisé reste-t-il compréhensible ? | Preuve + IA (revue humaine) | 1.3.2, 4.1.2 |
| 5.4 | Pour chaque tableau de données ayant un titre, le titre est-il correctement associé au tableau de données ? | Contrôles déterministes | 1.3.1 |
| 5.5 | Pour chaque tableau de données ayant un titre, celui-ci est-il pertinent ? | Preuve + IA (revue humaine) | 1.3.1 |
| 5.6 | Pour chaque tableau de données, chaque en-tête de colonne et chaque en-tête de ligne sont-ils correctement déclarés ? | Preuve + IA (revue humaine) | 1.3.1 |
| 5.7 | Pour chaque tableau de données, la technique appropriée permettant d’associer chaque cellule avec ses en-têtes est-elle utilisée (hors cas particuliers) ? | Contrôles déterministes | 1.3.1 |
| 5.8 | Chaque tableau de mise en forme ne doit pas utiliser d’éléments propres aux tableaux de données. Cette règle est-elle respectée ? | Contrôles déterministes | 1.3.1 |

## 6. Liens

| Critère | Intitulé | Moteur | WCAG 2.1 |
|---|---|---|---|
| 6.1 | Chaque lien est-il explicite (hors cas particuliers) ? | Preuve + IA (revue humaine) | 1.1.1, 2.4.4, 2.5.3 |
| 6.2 | Dans chaque page web, chaque lien a-t-il un intitulé ? | axe-core | 1.1.1, 2.4.4 |

## 7. Scripts

| Critère | Intitulé | Moteur | WCAG 2.1 |
|---|---|---|---|
| 7.1 | Chaque script est-il, si nécessaire, compatible avec les technologies d’assistance ? | Preuve + IA (revue humaine) | 2.5.3, 4.1.2 |
| 7.2 | Pour chaque script ayant une alternative, cette alternative est-elle pertinente ? | Preuve + IA (revue humaine) | 1.1.1, 4.1.2 |
| 7.3 | Chaque script est-il contrôlable par le clavier et par tout dispositif de pointage (hors cas particuliers) ? | Contrôles déterministes | 1.3.1, 2.1.1, 2.4.7 |
| 7.4 | Pour chaque script qui initie un changement de contexte, l’utilisateur est-il averti ou en a-t-il le contrôle ? | Contrôles déterministes | 3.2.1, 3.2.2 |
| 7.5 | Dans chaque page web, les messages de statut sont-ils correctement restitués par les technologies d’assistance ? | Humain (pré-tri machine) | 4.1.3 |

## 8. Éléments obligatoires

| Critère | Intitulé | Moteur | WCAG 2.1 |
|---|---|---|---|
| 8.1 | Chaque page web est-elle définie par un type de document ? | Contrôles déterministes | 4.1.1 |
| 8.2 | Pour chaque page web, le code source généré est-il valide selon le type de document spécifié ? | Contrôles déterministes | 4.1.1, 4.1.2 |
| 8.3 | Dans chaque page web, la langue par défaut est-elle présente ? | axe-core | 3.1.1 |
| 8.4 | Pour chaque page web ayant une langue par défaut, le code de langue est-il pertinent ? | Preuve + IA (revue humaine) | 3.1.1 |
| 8.5 | Chaque page web a-t-elle un titre de page ? | axe-core | 2.4.2 |
| 8.6 | Pour chaque page web ayant un titre de page, ce titre est-il pertinent ? | Preuve + IA (revue humaine) | 2.4.2 |
| 8.7 | Dans chaque page web, chaque changement de langue est-il indiqué dans le code source (hors cas particuliers) ? | Contrôles déterministes | 3.1.2 |
| 8.8 | Dans chaque page web, le code de langue de chaque changement de langue est-il valide et pertinent ? | Preuve + IA (revue humaine) | 3.1.2 |
| 8.9 | Dans chaque page web, les balises ne doivent pas être utilisées uniquement à des fins de présentation. Cette règle est-elle respectée ? | Contrôles déterministes | 1.3.1 |
| 8.10 | Dans chaque page web, les changements du sens de lecture sont-ils signalés ? | Contrôles déterministes | 1.3.2 |

## 9. Structuration de l’information

| Critère | Intitulé | Moteur | WCAG 2.1 |
|---|---|---|---|
| 9.1 | Dans chaque page web, l’information est-elle structurée par l’utilisation appropriée de titres ? | Preuve + IA (revue humaine) | 1.3.1, 2.4.1, 2.4.6, 4.1.2 |
| 9.2 | Dans chaque page web, la structure du document est-elle cohérente (hors cas particuliers) ? | Preuve + IA (revue humaine) | 1.3.1 |
| 9.3 | Dans chaque page web, chaque liste est-elle correctement structurée ? | Contrôles déterministes | 1.3.1 |
| 9.4 | Dans chaque page web, chaque citation est-elle correctement indiquée ? | Contrôles déterministes | 1.3.1 |

## 10. Présentation de l’information

| Critère | Intitulé | Moteur | WCAG 2.1 |
|---|---|---|---|
| 10.1 | Dans le site web, des feuilles de styles sont-elles utilisées pour contrôler la présentation de l’information ? | Contrôles déterministes | 1.3.1, 1.3.2 |
| 10.2 | Dans chaque page web, le contenu visible porteur d’information reste-t-il présent lorsque les feuilles de styles sont désactivées ? | Contrôles déterministes | 1.1.1, 1.3.1 |
| 10.3 | Dans chaque page web, l’information reste-t-elle compréhensible lorsque les feuilles de styles sont désactivées ? | Preuve + IA (revue humaine) | 1.3.2, 2.4.3 |
| 10.4 | Dans chaque page web, le texte reste-t-il lisible lorsque la taille des caractères est augmentée jusqu’à 200 %, au moins (hors cas particuliers) ? | Contrôles déterministes | 1.4.4 |
| 10.5 | Dans chaque page web, les déclarations CSS de couleurs de fond d’élément et de police sont-elles correctement utilisées ? | Contrôles déterministes | 1.4.3 |
| 10.6 | Dans chaque page web, chaque lien dont la nature n’est pas évidente est-il visible par rapport au texte environnant ? | Contrôles déterministes | 1.4.1 |
| 10.7 | Dans chaque page web, pour chaque élément recevant le focus, la prise de focus est-elle visible ? | Contrôles déterministes | 1.4.1, 2.4.7 |
| 10.8 | Pour chaque page web, les contenus cachés ont-ils vocation à être ignorés par les technologies d’assistance ? | Contrôles déterministes | 1.3.2, 4.1.2 |
| 10.9 | Dans chaque page web, l’information ne doit pas être donnée uniquement par la forme, taille ou position. Cette règle est-elle respectée ? | Preuve + IA (revue humaine) | 1.3.3, 1.4.1 |
| 10.10 | Dans chaque page web, l’information ne doit pas être donnée par la forme, taille ou position uniquement. Cette règle est-elle implémentée de façon pertinente ? | Preuve + IA (revue humaine) | 1.3.3, 1.4.1 |
| 10.11 | Pour chaque page web, les contenus peuvent-ils être présentés sans perte d’information ou de fonctionnalité et sans avoir recours soit à un défilement vertical pour une fenêtre ayant une hauteur de 256 px, soit à un défilement horizontal pour une fenêtre ayant une largeur de 320 px (hors cas particuliers) ? | Contrôles déterministes | 1.4.10 |
| 10.12 | Dans chaque page web, les propriétés d’espacement du texte peuvent-elles être redéfinies par l’utilisateur sans perte de contenu ou de fonctionnalité (hors cas particuliers) ? | Contrôles déterministes | 1.4.12 |
| 10.13 | Dans chaque page web, les contenus additionnels apparaissant à la prise de focus ou au survol d’un composant d’interface sont-ils contrôlables par l’utilisateur (hors cas particuliers) ? | Contrôles déterministes | 1.4.13 |
| 10.14 | Dans chaque page web, les contenus additionnels apparaissant via les styles CSS uniquement peuvent-ils être rendus visibles au clavier et par tout dispositif de pointage ? | Contrôles déterministes | 2.1.1 |

## 11. Formulaires

| Critère | Intitulé | Moteur | WCAG 2.1 |
|---|---|---|---|
| 11.1 | Chaque champ de formulaire a-t-il une étiquette ? | axe-core | 1.3.1, 2.4.6, 3.3.2, 4.1.2 |
| 11.2 | Chaque étiquette associée à un champ de formulaire est-elle pertinente (hors cas particuliers) ? | Preuve + IA (revue humaine) | 2.4.6, 2.5.3, 3.3.2 |
| 11.3 | Dans chaque formulaire, chaque étiquette associée à un champ de formulaire ayant la même fonction et répétée plusieurs fois dans une même page ou dans un ensemble de pages est-elle cohérente ? | Contrôles déterministes | 3.2.4 |
| 11.4 | Dans chaque formulaire, chaque étiquette de champ et son champ associé sont-ils accolés (hors cas particuliers) ? | Contrôles déterministes | 3.3.2 |
| 11.5 | Dans chaque formulaire, les champs de même nature sont-ils regroupés, si nécessaire ? | Contrôles déterministes | 1.3.1, 3.3.2 |
| 11.6 | Dans chaque formulaire, chaque regroupement de champs de même nature a-t-il une légende ? | Contrôles déterministes | 1.3.1, 3.3.2 |
| 11.7 | Dans chaque formulaire, chaque légende associée à un regroupement de champs de même nature est-elle pertinente ? | Preuve + IA (revue humaine) | 1.3.1, 3.3.2 |
| 11.8 | Dans chaque formulaire, les items de même nature d’une liste de choix sont-ils regroupés de manière pertinente ? | Contrôles déterministes | 1.3.1 |
| 11.9 | Dans chaque formulaire, l’intitulé de chaque bouton est-il pertinent (hors cas particuliers) ? | Preuve + IA (revue humaine) | 2.5.3, 4.1.2 |
| 11.10 | Dans chaque formulaire, le contrôle de saisie est-il utilisé de manière pertinente (hors cas particuliers) ? | Preuve + IA (revue humaine) | 3.3.1, 3.3.2 |
| 11.11 | Dans chaque formulaire, le contrôle de saisie est-il accompagné, si nécessaire, de suggestions facilitant la correction des erreurs de saisie ? | Preuve + IA (revue humaine) | 3.3.3 |
| 11.12 | Pour chaque formulaire qui modifie ou supprime des données, ou qui transmet des réponses à un test ou à un examen, ou dont la validation a des conséquences financières ou juridiques, les données saisies peuvent-elles être modifiées, mises à jour ou récupérées par l’utilisateur ? | Humain (pré-tri machine) | 3.3.4 |
| 11.13 | La finalité d’un champ de saisie peut-elle être déduite pour faciliter le remplissage automatique des champs avec les données de l’utilisateur ? | Preuve + IA (revue humaine) | 1.3.5 |

## 12. Navigation

| Critère | Intitulé | Moteur | WCAG 2.1 |
|---|---|---|---|
| 12.1 | Chaque ensemble de pages dispose-t-il de deux systèmes de navigation différents, au moins (hors cas particuliers) ? | Contrôles déterministes | 2.4.5 |
| 12.2 | Dans chaque ensemble de pages, le menu et les barres de navigation sont-ils toujours à la même place (hors cas particuliers) ? | Contrôles déterministes | 3.2.3 |
| 12.3 | La page « plan du site » est-elle pertinente ? | Preuve + IA (revue humaine) | 2.4.5 |
| 12.4 | Dans chaque ensemble de pages, la page « plan du site » est-elle accessible à partir d’une fonctionnalité identique ? | Contrôles déterministes | 2.4.5, 3.2.3 |
| 12.5 | Dans chaque ensemble de pages, le moteur de recherche est-il atteignable de manière identique ? | Contrôles déterministes | 3.2.3 |
| 12.6 | Les zones de regroupement de contenus présentes dans plusieurs pages web (zones d’en-tête, de navigation principale, de contenu principal, de pied de page et de moteur de recherche) peuvent-elles être atteintes ou évitées ? | Contrôles déterministes | 1.3.1, 2.4.1, 4.1.2 |
| 12.7 | Dans chaque page web, un lien d’évitement ou d’accès rapide à la zone de contenu principal est-il présent (hors cas particuliers) ? | Contrôles déterministes | 2.4.1, 2.4.3, 3.2.3 |
| 12.8 | Dans chaque page web, l’ordre de tabulation est-il cohérent ? | Contrôles déterministes | 2.4.3 |
| 12.9 | Dans chaque page web, la navigation ne doit pas contenir de piège au clavier. Cette règle est-elle respectée ? | Contrôles déterministes | 2.1.1, 2.1.2 |
| 12.10 | Dans chaque page web, les raccourcis clavier n’utilisant qu’une seule touche (lettre minuscule ou majuscule, ponctuation, chiffre ou symbole) sont-ils contrôlables par l’utilisateur ? | Contrôles déterministes | 2.1.4 |
| 12.11 | Dans chaque page web, les contenus additionnels apparaissant au survol, à la prise de focus ou à l’activation d’un composant d’interface sont-ils si nécessaire atteignables au clavier ? | Contrôles déterministes | 2.1.1 |

## 13. Consultation

| Critère | Intitulé | Moteur | WCAG 2.1 |
|---|---|---|---|
| 13.1 | Pour chaque page web, l’utilisateur a-t-il le contrôle de chaque limite de temps modifiant le contenu (hors cas particuliers) ? | Humain (pré-tri machine) | 2.2.1, 2.2.2 |
| 13.2 | Dans chaque page web, l’ouverture d’une nouvelle fenêtre ne doit pas être déclenchée sans action de l’utilisateur. Cette règle est-elle respectée ? | Contrôles déterministes | 3.2.1 |
| 13.3 | Dans chaque page web, chaque document bureautique en téléchargement possède-t-il, si nécessaire, une version accessible (hors cas particuliers) ? | Contrôles déterministes | 1.1.1, 1.3.1, 1.3.2, 2.4.1, 2.4.3, 3.1.1, 4.1.2 |
| 13.4 | Pour chaque document bureautique ayant une version accessible, cette version offre-t-elle la même information ? | Humain (pré-tri machine) | 1.1.1, 1.3.1, 1.3.2, 2.4.1, 2.4.3, 3.1.1, 4.1.2 |
| 13.5 | Dans chaque page web, chaque contenu cryptique (art ASCII, émoticône, syntaxe cryptique) a-t-il une alternative ? | Contrôles déterministes | 1.1.1 |
| 13.6 | Dans chaque page web, pour chaque contenu cryptique (art ASCII, émoticône, syntaxe cryptique) ayant une alternative, cette alternative est-elle pertinente ? | Preuve + IA (revue humaine) | 1.1.1 |
| 13.7 | Dans chaque page web, les changements brusques de luminosité ou les effets de flash sont-ils correctement utilisés ? | Humain (pré-tri machine) | 2.3.1 |
| 13.8 | Dans chaque page web, chaque contenu en mouvement ou clignotant est-il contrôlable par l’utilisateur ? | Contrôles déterministes | 2.2.1, 2.2.2 |
| 13.9 | Dans chaque page web, le contenu proposé est-il consultable quelle que soit l’orientation de l’écran (portrait ou paysage) (hors cas particuliers) ? | Contrôles déterministes | 1.3.4 |
| 13.10 | Dans chaque page web, les fonctionnalités utilisables ou disponibles au moyen d’un geste complexe peuvent-elles être également disponibles au moyen d’un geste simple (hors cas particuliers) ? | Contrôles déterministes | 2.5.1 |
| 13.11 | Dans chaque page web, les actions déclenchées au moyen d’un dispositif de pointage sur un point unique de l’écran peuvent-elles faire l’objet d’une annulation (hors cas particuliers) ? | Contrôles déterministes | 2.5.2 |
| 13.12 | Dans chaque page web, les fonctionnalités qui impliquent un mouvement de l’appareil ou vers l’appareil peuvent-elles être satisfaites de manière alternative (hors cas particuliers) ? | Contrôles déterministes | 2.5.4 |

## Répartition par moteur

| Moteur | Critères |
|---|---|
| axe-core | 6 |
| Contrôles déterministes | 60 |
| Preuve + IA (revue humaine) | 32 |
| Humain (pré-tri machine) | 8 |
| **Total** | **106** |
