/// Criterion definition used to enrich evaluation prompts.
///
/// Each definition provides the RGAA criterion ID, its human-readable title,
/// the relevant WCAG success-criterion references, and the official French
/// definition text used by the IA-assistée evaluator.
#[derive(Copy, Clone)]
pub struct CriterionDefinition {
    /// RGAA criterion identifier (e.g. "1.3", "13.12").
    pub id: &'static str,
    /// Short French title of the criterion.
    pub title: &'static str,
    /// Comma-separated WCAG success-criterion references.
    pub wcag_refs: &'static str,
    /// Official French definition from the RGAA référentiel.
    pub definition: &'static str,
}

/// Looks up a [`CriterionDefinition`] by its RGAA criterion ID.
///
/// # Returns
/// `Some(CriterionDefinition)` if `criterion_id` is known, `None` otherwise.
pub fn get_criterion_definition(criterion_id: &str) -> Option<CriterionDefinition> {
    DEFINITIONS.iter().find(|d| d.id == criterion_id).copied()
}

const DEFINITIONS: &[CriterionDefinition] = &[
    // ── Topic 1: Images ─────────────────────────────────────────────
    CriterionDefinition {
        id: "1.1",
        title: "Alternative textuelle image porteuse d'information",
        wcag_refs: "1.1.1",
        definition: "Chaque image porteuse d'information a-t-elle une alternative textuelle ?",
    },
    CriterionDefinition {
        id: "1.2",
        title: "Image de décoration ignorée par TA",
        wcag_refs: "1.1.1, 4.1.2",
        definition: "Chaque image de décoration est-elle correctement ignorée par les technologies d'assistance ?",
    },
    CriterionDefinition {
        id: "1.3",
        title: "Alternative textuelle pertinente",
        wcag_refs: "1.1.1, 4.1.2",
        definition: "Pour chaque image porteuse d'information ayant une alternative textuelle, cette alternative est-elle pertinente ?",
    },
    CriterionDefinition {
        id: "1.4",
        title: "Alternative CAPTCHA/image-test pertinente",
        wcag_refs: "1.1.1",
        definition: "Pour chaque image utilisée comme CAPTCHA ou image-test, ayant une alternative textuelle, cette alternative permet-elle d'identifier la nature et la fonction de l'image ?",
    },
    CriterionDefinition {
        id: "1.5",
        title: "Solution alternative CAPTCHA",
        wcag_refs: "1.1.1",
        definition: "Pour chaque image utilisée comme CAPTCHA, une solution d'accès alternatif au contenu ou à la fonction du CAPTCHA est-elle présente ?",
    },
    CriterionDefinition {
        id: "1.6",
        title: "Description détaillée image",
        wcag_refs: "1.1.1",
        definition: "Chaque image porteuse d'information a-t-elle, si nécessaire, une description détaillée ?",
    },
    CriterionDefinition {
        id: "1.7",
        title: "Description détaillée pertinente",
        wcag_refs: "1.1.1",
        definition: "Pour chaque image porteuse d'information ayant une description détaillée, cette description est-elle pertinente ?",
    },
    CriterionDefinition {
        id: "1.8",
        title: "Image texte remplacée par texte stylé",
        wcag_refs: "1.4.5",
        definition: "Chaque image texte porteuse d'information, en l'absence d'un mécanisme de remplacement, doit si possible être remplacée par du texte stylé. Cette règle est-elle respectée ?",
    },
    CriterionDefinition {
        id: "1.9",
        title: "Légende d'image correctement reliée",
        wcag_refs: "1.1.1",
        definition: "Chaque légende d'image est-elle, si nécessaire, correctement reliée à l'image correspondante ?",
    },
    // ── Topic 2: Cadres ─────────────────────────────────────────────
    CriterionDefinition {
        id: "2.1",
        title: "Cadre a un titre",
        wcag_refs: "4.1.2",
        definition: "Chaque cadre (iframe) a-t-il un titre de cadre ?",
    },
    CriterionDefinition {
        id: "2.2",
        title: "Titre de cadre pertinent",
        wcag_refs: "4.1.2",
        definition: "Pour chaque cadre ayant un titre, ce titre est-il pertinent pour décrire le contenu du cadre ?",
    },
    // ── Topic 3: Couleurs ───────────────────────────────────────────
    CriterionDefinition {
        id: "3.1",
        title: "Information non donnée uniquement par couleur",
        wcag_refs: "1.3.1, 1.4.1",
        definition: "L'information ne doit pas être donnée uniquement par la couleur. Les différences d'information sont-elles perceptibles par d'autres moyens ?",
    },
    CriterionDefinition {
        id: "3.2",
        title: "Contraste texte/arrière-plan suffisant",
        wcag_refs: "1.4.3",
        definition: "Le contraste entre la couleur du texte et la couleur de son arrière-plan est-il suffisamment élevé ?",
    },
    CriterionDefinition {
        id: "3.3",
        title: "Contraste composants d'interface suffisant",
        wcag_refs: "1.4.11",
        definition: "Les couleurs utilisées dans les composants d'interface ou les éléments graphiques porteurs d'informations sont-elles suffisamment contrastées ?",
    },
    // ── Topic 4: Médias ─────────────────────────────────────────────
    CriterionDefinition {
        id: "4.1",
        title: "Transcription/audiodescription média temporel",
        wcag_refs: "1.2.1, 1.2.3, 1.2.5",
        definition: "Chaque média temporel pré-enregistré a-t-il, si nécessaire, une transcription textuelle ou une audiodescription ?",
    },
    CriterionDefinition {
        id: "4.2",
        title: "Transcription/audiodescription pertinente",
        wcag_refs: "1.2.1, 1.2.3",
        definition: "Pour chaque média temporel pré-enregistré ayant une transcription ou une audiodescription synchronisée, celles-ci sont-elles pertinentes ?",
    },
    CriterionDefinition {
        id: "4.3",
        title: "Sous-titres synchronisés média temporel",
        wcag_refs: "1.2.2",
        definition: "Chaque média temporel synchronisé pré-enregistré a-t-il, si nécessaire, des sous-titres synchronisés ?",
    },
    CriterionDefinition {
        id: "4.4",
        title: "Sous-titres pertinents",
        wcag_refs: "1.2.2",
        definition: "Pour chaque média temporel synchronisé pré-enregistré ayant des sous-titres synchronisés, ces sous-titres sont-ils pertinents ?",
    },
    CriterionDefinition {
        id: "4.5",
        title: "Audiodescription synchronisée média temporel",
        wcag_refs: "1.2.5",
        definition: "Chaque média temporel pré-enregistré a-t-il, si nécessaire, une audiodescription synchronisée ?",
    },
    CriterionDefinition {
        id: "4.6",
        title: "Audiodescription pertinente",
        wcag_refs: "1.2.5",
        definition: "Pour chaque média temporel pré-enregistré ayant une audiodescription synchronisée, celle-ci est-elle pertinente ?",
    },
    CriterionDefinition {
        id: "4.7",
        title: "Média temporel clairement identifié",
        wcag_refs: "1.2.1",
        definition: "Chaque média temporel est-il clairement identifiable ?",
    },
    CriterionDefinition {
        id: "4.8",
        title: "Alternative média non temporel",
        wcag_refs: "1.1.1",
        definition: "Chaque média non temporel a-t-il, si nécessaire, une alternative ?",
    },
    CriterionDefinition {
        id: "4.9",
        title: "Alternative pertinente média non temporel",
        wcag_refs: "1.1.1",
        definition: "Pour chaque média non temporel ayant une alternative, cette alternative est-elle pertinente ?",
    },
    CriterionDefinition {
        id: "4.10",
        title: "Son déclenché automatiquement contrôlable",
        wcag_refs: "1.4.2",
        definition: "Chaque son déclenché automatiquement est-il contrôlable par l'utilisateur ?",
    },
    CriterionDefinition {
        id: "4.11",
        title: "Média temporel contrôlable clavier/pointage",
        wcag_refs: "2.1.1, 2.1.2",
        definition: "La consultation de chaque média temporel est-elle, si nécessaire, contrôlable par le clavier et tout dispositif de pointage ?",
    },
    CriterionDefinition {
        id: "4.12",
        title: "Média non temporel contrôlable clavier/pointage",
        wcag_refs: "2.1.1, 2.1.2",
        definition: "La consultation de chaque média non temporel est-elle contrôlable par le clavier et tout dispositif de pointage ?",
    },
    CriterionDefinition {
        id: "4.13",
        title: "Média compatible avec TA",
        wcag_refs: "4.1.2",
        definition: "Chaque média temporel et non temporel est-il compatible avec les technologies d'assistance ?",
    },
    // ── Topic 5: Tableaux ───────────────────────────────────────────
    CriterionDefinition {
        id: "5.1",
        title: "Tableau de données complexe a un résumé",
        wcag_refs: "1.3.1",
        definition: "Chaque tableau de données complexe a-t-il un résumé ?",
    },
    CriterionDefinition {
        id: "5.2",
        title: "Résumé pertinent tableau complexe",
        wcag_refs: "1.3.1",
        definition: "Pour chaque tableau de données complexe ayant un résumé, celui-ci est-il pertinent ?",
    },
    CriterionDefinition {
        id: "5.3",
        title: "Contenu linéarisé compréhensible",
        wcag_refs: "1.3.2",
        definition: "Pour chaque tableau de mise en forme, le contenu linéarisé reste-t-il compréhensible ?",
    },
    CriterionDefinition {
        id: "5.4",
        title: "Titre tableau correctement associé",
        wcag_refs: "1.3.1",
        definition: "Pour chaque tableau de données ayant un titre, le titre est-il correctement associé au tableau ?",
    },
    CriterionDefinition {
        id: "5.5",
        title: "Titre pertinent tableau",
        wcag_refs: "1.3.1",
        definition: "Pour chaque tableau de données ayant un titre, celui-ci est-il pertinent ?",
    },
    CriterionDefinition {
        id: "5.6",
        title: "En-têtes colonnes/lignes correctement déclarés",
        wcag_refs: "1.3.1",
        definition: "Pour chaque tableau de données, chaque en-tête de colonne et chaque en-tête de ligne sont-ils correctement déclarés ?",
    },
    CriterionDefinition {
        id: "5.7",
        title: "Association cellules-en-têtes appropriée",
        wcag_refs: "1.3.1",
        definition: "Pour chaque tableau de données, la technique appropriée permettant d'associer chaque cellule avec ses en-têtes est-elle utilisée ?",
    },
    CriterionDefinition {
        id: "5.8",
        title: "Tableau mise en forme sans éléments données",
        wcag_refs: "1.3.1",
        definition: "Chaque tableau de mise en forme ne doit pas utiliser d'éléments propres aux tableaux de données. Cette règle est-elle respectée ?",
    },
    // ── Topic 6: Liens ──────────────────────────────────────────────
    CriterionDefinition {
        id: "6.1",
        title: "Lien explicite",
        wcag_refs: "2.4.4",
        definition: "Chaque lien est-il explicite ?",
    },
    CriterionDefinition {
        id: "6.2",
        title: "Lien a un intitulé",
        wcag_refs: "2.4.4, 4.1.2",
        definition: "Dans chaque page web, chaque lien a-t-il un intitulé ?",
    },
    // ── Topic 7: Scripts ────────────────────────────────────────────
    CriterionDefinition {
        id: "7.1",
        title: "Script compatible avec TA",
        wcag_refs: "4.1.2",
        definition: "Chaque script est-il, si nécessaire, compatible avec les technologies d'assistance ?",
    },
    CriterionDefinition {
        id: "7.2",
        title: "Alternative script pertinente",
        wcag_refs: "1.1.1, 4.1.2",
        definition: "Pour chaque script ayant une alternative, celle-ci est-elle pertinente ?",
    },
    CriterionDefinition {
        id: "7.3",
        title: "Script contrôlable clavier/pointage",
        wcag_refs: "2.1.1, 2.1.2",
        definition: "Chaque script est-il contrôlable par le clavier et par tout dispositif de pointage ?",
    },
    CriterionDefinition {
        id: "7.4",
        title: "Changement de contexte contrôlable",
        wcag_refs: "3.2.1",
        definition: "Pour chaque script qui initie un changement de contexte, l'utilisateur est-il averti ou en a-t-il le contrôle ?",
    },
    CriterionDefinition {
        id: "7.5",
        title: "Messages de statut restitués par TA",
        wcag_refs: "4.1.3",
        definition: "Dans chaque page web, les messages de statut sont-ils correctement restitués par les technologies d'assistance ?",
    },
    // ── Topic 8: Éléments de structure ──────────────────────────────
    CriterionDefinition {
        id: "8.1",
        title: "Type de document défini",
        wcag_refs: "4.1.1",
        definition: "Chaque page web est-elle définie par un type de document ?",
    },
    CriterionDefinition {
        id: "8.2",
        title: "Code source valide",
        wcag_refs: "4.1.1",
        definition: "Pour chaque page web, le code source généré est-il valide selon le type de document spécifié ?",
    },
    CriterionDefinition {
        id: "8.3",
        title: "Langue par défaut présente",
        wcag_refs: "3.1.1",
        definition: "Dans chaque page web, la langue par défaut est-elle présente ?",
    },
    CriterionDefinition {
        id: "8.4",
        title: "Code de langue pertinent",
        wcag_refs: "3.1.1",
        definition: "Pour chaque page web ayant une langue par défaut, le code de langue est-il pertinent ?",
    },
    CriterionDefinition {
        id: "8.5",
        title: "Page a un titre",
        wcag_refs: "2.4.2",
        definition: "Chaque page web a-t-elle un titre de page ?",
    },
    CriterionDefinition {
        id: "8.6",
        title: "Titre de page pertinent",
        wcag_refs: "2.4.2",
        definition: "Pour chaque page web ayant un titre de page, ce titre est-il pertinent ?",
    },
    CriterionDefinition {
        id: "8.7",
        title: "Changement de langue indiqué dans le code",
        wcag_refs: "3.1.2",
        definition: "Dans chaque page web, chaque changement de langue est-il indiqué dans le code source ?",
    },
    CriterionDefinition {
        id: "8.8",
        title: "Code de langue changement pertinent",
        wcag_refs: "3.1.2",
        definition: "Dans chaque page web, le code de langue de chaque changement de langue est-il valide et pertinent ?",
    },
    CriterionDefinition {
        id: "8.9",
        title: "Balises non utilisées uniquement à fins de présentation",
        wcag_refs: "1.3.1",
        definition: "Dans chaque page web, les balises ne doivent pas être utilisées uniquement à des fins de présentation. Cette règle est-elle respectée ?",
    },
    CriterionDefinition {
        id: "8.10",
        title: "Changements sens de lecture signalés",
        wcag_refs: "1.3.2, 1.3.3",
        definition: "Dans chaque page web, les changements du sens de lecture sont-ils signalés ?",
    },
    // ── Topic 9: Présentation ───────────────────────────────────────
    CriterionDefinition {
        id: "9.1",
        title: "Information structurée par titres",
        wcag_refs: "1.3.1",
        definition: "Dans chaque page web, l'information est-elle structurée par l'utilisation appropriée de titres ?",
    },
    CriterionDefinition {
        id: "9.2",
        title: "Structure document cohérente",
        wcag_refs: "1.3.1",
        definition: "Dans chaque page web, la structure du document est-elle cohérente ?",
    },
    CriterionDefinition {
        id: "9.3",
        title: "Liste correctement structurée",
        wcag_refs: "1.3.1",
        definition: "Dans chaque page web, chaque liste est-elle correctement structurée ?",
    },
    CriterionDefinition {
        id: "9.4",
        title: "Citation correctement indiquée",
        wcag_refs: "1.3.1",
        definition: "Dans chaque page web, chaque citation est-elle correctement indiquée ?",
    },
    // ── Topic 10: Contrôles de présentation ──────────────────────────
    CriterionDefinition {
        id: "10.1",
        title: "Feuilles de styles utilisées pour la présentation",
        wcag_refs: "1.3.1",
        definition: "Dans le site web, des feuilles de styles sont-elles utilisées pour contrôler la présentation de l'information ?",
    },
    CriterionDefinition {
        id: "10.2",
        title: "Contenu visible présent sans CSS",
        wcag_refs: "1.3.1, 2.4.3",
        definition: "Dans chaque page web, le contenu visible porteur d'information reste-t-il présent lorsque les feuilles de styles sont désactivées ?",
    },
    CriterionDefinition {
        id: "10.3",
        title: "Information compréhensible sans CSS",
        wcag_refs: "1.3.2, 2.4.3",
        definition: "Dans chaque page web, l'information reste-t-elle compréhensible lorsque les feuilles de styles sont désactivées ?",
    },
    CriterionDefinition {
        id: "10.4",
        title: "Texte lisible à 200% de taille",
        wcag_refs: "1.4.4",
        definition: "Dans chaque page web, le texte reste-t-il lisible lorsque la taille des caractères est augmentée jusqu'à 200%, au moins ?",
    },
    CriterionDefinition {
        id: "10.5",
        title: "Déclarations CSS couleurs correctement utilisées",
        wcag_refs: "1.4.3, 1.4.11",
        definition: "Dans chaque page web, les déclarations CSS de couleurs de fond d'élément et de police sont-elles correctement utilisées ?",
    },
    CriterionDefinition {
        id: "10.6",
        title: "Lien nature évidente visible par rapport au texte",
        wcag_refs: "1.4.1",
        definition: "Dans chaque page web, chaque lien dont la nature n'est pas évidente est-il visible par rapport au texte environnant ?",
    },
    CriterionDefinition {
        id: "10.7",
        title: "Prise de focus visible",
        wcag_refs: "2.4.7",
        definition: "Dans chaque page web, pour chaque élément recevant le focus, la prise de focus est-elle visible ?",
    },
    CriterionDefinition {
        id: "10.8",
        title: "Contenus cachés ignorés par TA",
        wcag_refs: "1.3.2",
        definition: "Pour chaque page web, les contenus cachés ont-ils vocation à être ignorés par les technologies d'assistance ?",
    },
    CriterionDefinition {
        id: "10.9",
        title: "Information non donnée uniquement par forme/taille/position",
        wcag_refs: "1.3.3",
        definition: "Dans chaque page web, l'information ne doit pas être donnée uniquement par la forme, taille ou position. Cette règle est-elle respectée ?",
    },
    CriterionDefinition {
        id: "10.10",
        title: "Implémentation pertinente forme/taille/position",
        wcag_refs: "1.3.3, 1.4.1",
        definition: "Dans chaque page web, l'information ne doit pas être donnée par la forme, taille ou position uniquement. Cette règle est-elle implémentée de façon pertinente ?",
    },
    CriterionDefinition {
        id: "10.11",
        title: "Contenus présentés sans perte d'information (reflow)",
        wcag_refs: "1.4.10",
        definition: "Pour chaque page web, les contenus peuvent-ils être présentés sans perte d'information ou de fonctionnalité et sans défilement horizontal pour 320px ?",
    },
    CriterionDefinition {
        id: "10.12",
        title: "Propriétés espacement texte redéfinissables",
        wcag_refs: "1.4.12",
        definition: "Dans chaque page web, les propriétés d'espacement du texte peuvent-elles être redéfinies par l'utilisateur sans perte de contenu ?",
    },
    CriterionDefinition {
        id: "10.13",
        title: "Contenus additionnels focus/survol contrôlables",
        wcag_refs: "1.4.13",
        definition: "Dans chaque page web, les contenus additionnels apparaissant à la prise de focus ou au survol d'un composant d'interface sont-ils contrôlables par l'utilisateur ?",
    },
    CriterionDefinition {
        id: "10.14",
        title: "Contenus CSS rendus visibles au clavier/pointage",
        wcag_refs: "1.4.13",
        definition: "Dans chaque page web, les contenus additionnels apparaissant via les styles CSS uniquement peuvent-ils être rendus visibles au clavier et par tout dispositif de pointage ?",
    },
    // ── Topic 11: Formulaires ───────────────────────────────────────
    CriterionDefinition {
        id: "11.1",
        title: "Champ de formulaire a une étiquette",
        wcag_refs: "1.3.1, 3.3.2",
        definition: "Chaque champ de formulaire a-t-il une étiquette ?",
    },
    CriterionDefinition {
        id: "11.2",
        title: "Étiquette champ pertinente",
        wcag_refs: "2.4.6, 2.5.3, 3.3.2",
        definition: "Chaque étiquette associée à un champ de formulaire est-elle pertinente ?",
    },
    CriterionDefinition {
        id: "11.3",
        title: "Étiquettes cohérentes même fonction",
        wcag_refs: "3.2.4",
        definition: "Dans chaque formulaire, chaque étiquette associée à un champ ayant la même fonction est-elle cohérente ?",
    },
    CriterionDefinition {
        id: "11.4",
        title: "Étiquette et champ accolés",
        wcag_refs: "1.3.1, 3.3.2",
        definition: "Dans chaque formulaire, chaque étiquette de champ et son champ associé sont-ils accolés ?",
    },
    CriterionDefinition {
        id: "11.5",
        title: "Champs de même nature regroupés",
        wcag_refs: "1.3.1",
        definition: "Dans chaque formulaire, les champs de même nature sont-ils regroupés, si nécessaire ?",
    },
    CriterionDefinition {
        id: "11.6",
        title: "Regroupement de champs a une légende",
        wcag_refs: "1.3.1",
        definition: "Dans chaque formulaire, chaque regroupement de champs de même nature a-t-il une légende ?",
    },
    CriterionDefinition {
        id: "11.7",
        title: "Légende regroupement pertinente",
        wcag_refs: "1.3.1, 3.3.2",
        definition: "Dans chaque formulaire, chaque légende associée à un regroupement de champs est-elle pertinente ?",
    },
    CriterionDefinition {
        id: "11.8",
        title: "Items liste choix regroupés pertinemment",
        wcag_refs: "1.3.1",
        definition: "Dans chaque formulaire, les items de même nature d'une liste de choix sont-ils regroupés de manière pertinente ?",
    },
    CriterionDefinition {
        id: "11.9",
        title: "Intitulé bouton pertinent",
        wcag_refs: "2.5.3, 4.1.2",
        definition: "Dans chaque formulaire, l'intitulé de chaque bouton est-il pertinent ?",
    },
    CriterionDefinition {
        id: "11.10",
        title: "Contrôle saisie utilisé pertinemment",
        wcag_refs: "1.3.1",
        definition: "Dans chaque formulaire, le contrôle de saisie est-il utilisé de manière pertinente ?",
    },
    CriterionDefinition {
        id: "11.11",
        title: "Suggestions facilitant correction erreurs",
        wcag_refs: "3.3.3",
        definition: "Dans chaque formulaire, le contrôle de saisie est-il accompagné, si nécessaire, de suggestions facilitant la correction des erreurs de saisie ?",
    },
    CriterionDefinition {
        id: "11.12",
        title: "Données modifiables/récupérables",
        wcag_refs: "3.3.4",
        definition: "Pour chaque formulaire qui modifie ou supprime des données, les données saisies peuvent-elles être modifiées, mises à jour ou récupérées par l'utilisateur ?",
    },
    CriterionDefinition {
        id: "11.13",
        title: "Finalité champ déductible pour autofill",
        wcag_refs: "1.3.5",
        definition: "La finalité d'un champ de saisie peut-elle être déduite pour faciliter le remplissage automatique des champs avec les données de l'utilisateur ?",
    },
    // ── Topic 12: Navigation ────────────────────────────────────────
    CriterionDefinition {
        id: "12.1",
        title: "Deux systèmes de navigation différents",
        wcag_refs: "2.4.5",
        definition: "Chaque ensemble de pages dispose-t-il de deux systèmes de navigation différents, au moins ?",
    },
    CriterionDefinition {
        id: "12.2",
        title: "Menu/barres navigation à la même place",
        wcag_refs: "3.2.3",
        definition: "Dans chaque ensemble de pages, le menu et les barres de navigation sont-ils toujours à la même place ?",
    },
    CriterionDefinition {
        id: "12.3",
        title: "Plan du site pertinent",
        wcag_refs: "2.4.5",
        definition: "La page « plan du site » est-elle pertinente ?",
    },
    CriterionDefinition {
        id: "12.4",
        title: "Plan du site accessible fonctionnalité identique",
        wcag_refs: "2.4.5",
        definition: "Dans chaque ensemble de pages, la page « plan du site » est-elle accessible à partir d'une fonctionnalité identique ?",
    },
    CriterionDefinition {
        id: "12.5",
        title: "Moteur de recherche atteignable de manière identique",
        wcag_refs: "3.2.3",
        definition: "Dans chaque ensemble de pages, le moteur de recherche est-il atteignable de manière identique ?",
    },
    CriterionDefinition {
        id: "12.6",
        title: "Zones de regroupement atteignables/évitées",
        wcag_refs: "1.3.1, 2.4.1",
        definition: "Les zones de regroupement de contenus présentes dans plusieurs pages web peuvent-elles être atteintes ou évitées ?",
    },
    CriterionDefinition {
        id: "12.7",
        title: "Lien d'évitement/accès rapide contenu principal",
        wcag_refs: "2.4.1",
        definition: "Dans chaque page web, un lien d'évitement ou d'accès rapide à la zone de contenu principal est-il présent ?",
    },
    CriterionDefinition {
        id: "12.8",
        title: "Ordre tabulation cohérent",
        wcag_refs: "2.4.3",
        definition: "Dans chaque page web, l'ordre de tabulation est-il cohérent ?",
    },
    CriterionDefinition {
        id: "12.9",
        title: "Pas de piège au clavier",
        wcag_refs: "2.1.2",
        definition: "Dans chaque page web, la navigation ne doit pas contenir de piège au clavier. Cette règle est-elle respectée ?",
    },
    CriterionDefinition {
        id: "12.10",
        title: "Raccourcis clavier contrôlables",
        wcag_refs: "2.1.4",
        definition: "Dans chaque page web, les raccourcis clavier n'utilisant qu'une seule touche sont-ils contrôlables par l'utilisateur ?",
    },
    CriterionDefinition {
        id: "12.11",
        title: "Contenus additionnels atteignables au clavier",
        wcag_refs: "2.1.1",
        definition: "Dans chaque page web, les contenus additionnels apparaissant au survol ou à la prise de focus sont-ils si nécessaire atteignables au clavier ?",
    },
    // ── Topic 13: Consultation ──────────────────────────────────────
    CriterionDefinition {
        id: "13.1",
        title: "Contrôle limite de temps",
        wcag_refs: "2.2.1",
        definition: "Pour chaque page web, l'utilisateur a-t-il le contrôle de chaque limite de temps modifiant le contenu ?",
    },
    CriterionDefinition {
        id: "13.2",
        title: "Overture nouvelle fenêtre contrôlée",
        wcag_refs: "3.2.5",
        definition: "Dans chaque page web, l'ouverture d'une nouvelle fenêtre ne doit pas être déclenchée sans action de l'utilisateur. Cette règle est-elle respectée ?",
    },
    CriterionDefinition {
        id: "13.3",
        title: "Document bureautique version accessible",
        wcag_refs: "1.1.1",
        definition: "Dans chaque page web, chaque document bureautique en téléchargement possède-t-il, si nécessaire, une version accessible ?",
    },
    CriterionDefinition {
        id: "13.4",
        title: "Version accessible document offre même information",
        wcag_refs: "1.1.1",
        definition: "Pour chaque document bureautique ayant une version accessible, cette version offre-t-elle la même information ?",
    },
    CriterionDefinition {
        id: "13.5",
        title: "Contenu cryptique a une alternative",
        wcag_refs: "1.1.1",
        definition: "Dans chaque page web, chaque contenu cryptique (art ASCII, émoticône, syntaxe cryptique) a-t-il une alternative ?",
    },
    CriterionDefinition {
        id: "13.6",
        title: "Alternative pertinente contenu cryptique",
        wcag_refs: "1.1.1",
        definition: "Pour chaque contenu cryptique ayant une alternative, cette alternative est-elle pertinente ?",
    },
    CriterionDefinition {
        id: "13.7",
        title: "Changements brusques luminosité/flash corrects",
        wcag_refs: "2.3.1",
        definition: "Dans chaque page web, les changements brusques de luminosité ou les effets de flash sont-ils correctement utilisés ?",
    },
    CriterionDefinition {
        id: "13.8",
        title: "Contenu mouvement/clignotement contrôlable",
        wcag_refs: "2.2.2",
        definition: "Dans chaque page web, chaque contenu en mouvement ou clignotant est-il contrôlable par l'utilisateur ?",
    },
    CriterionDefinition {
        id: "13.9",
        title: "Contenu consultable quelle que soit l'orientation",
        wcag_refs: "1.3.4",
        definition: "Dans chaque page web, le contenu proposé est-il consultable quelle que soit l'orientation de l'écran ?",
    },
    CriterionDefinition {
        id: "13.10",
        title: "Fonctionnalités geste complexes disponibles en geste simple",
        wcag_refs: "2.5.1",
        definition: "Dans chaque page web, les fonctionnalités utilisables au moyen d'un geste complexe peuvent-elles être également disponibles au moyen d'un geste simple ?",
    },
    CriterionDefinition {
        id: "13.11",
        title: "Actions pointage annulables",
        wcag_refs: "2.5.2",
        definition: "Dans chaque page web, les actions déclenchées au moyen d'un dispositif de pointage sur un point unique peuvent-elles faire l'objet d'une annulation ?",
    },
    CriterionDefinition {
        id: "13.12",
        title: "Fonctionnalités mouvement dispositif alternatives",
        wcag_refs: "2.5.4",
        definition: "Dans chaque page web, les fonctionnalités qui impliquent un mouvement de l'appareil ou vers l'appareil peuvent-elles être satisfaites de manière alternative ?",
    },
];

/// Criteria that require visual understanding or complex reasoning.
/// Routed to the 122b model.
pub const VISUAL_CRITERIA: &[&str] = &[
    "1.3",   // alt text relevance — compare alt vs actual image
    "1.7",   // detailed description relevance
    "3.1",   // color-only information — must SEE the page
    "5.3",   // content linearization — must SEE reading order
    "10.3",  // info comprehensible without CSS — must SEE rendering
    "10.10", // shape/size/position — must SEE visual presentation
    "11.2",  // label relevance — must SEE label next to input
    "11.7",  // fieldset legend relevance — must SEE form grouping
    "11.8",  // list items grouping — must SEE structure
    "11.9",  // button label relevance — complex reasoning
    "11.10", // input control type — complex reasoning
    "12.3",  // site map relevance — must SEE navigation structure
    "12.8",  // focus order — must INTERACT with page
    "13.6",  // CAPTCHA alternative relevance — complex reasoning
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_106_criteria_present() {
        assert_eq!(
            DEFINITIONS.len(),
            106,
            "Expected 106 RGAA criteria definitions, found {}",
            DEFINITIONS.len()
        );
    }

    #[test]
    fn no_duplicate_ids() {
        let mut ids: Vec<&str> = DEFINITIONS.iter().map(|d| d.id).collect();
        let original_len = ids.len();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), original_len, "Found duplicate criterion IDs");
    }

    #[test]
    fn get_criterion_definition_works() {
        let def = get_criterion_definition("1.3").unwrap();
        assert_eq!(def.id, "1.3");
        assert!(def.title.contains("alternative") || def.definition.contains("alternative"));
    }

    #[test]
    fn get_criterion_definition_returns_none_for_unknown() {
        assert!(get_criterion_definition("99.99").is_none());
    }

    #[test]
    fn visual_criteria_are_subset_of_definitions() {
        let ids: Vec<&str> = DEFINITIONS.iter().map(|d| d.id).collect();
        for vc in VISUAL_CRITERIA {
            assert!(
                ids.contains(vc),
                "VISUAL_CRITERIA contains '{}' which is not in DEFINITIONS",
                vc
            );
        }
    }

    #[test]
    fn all_criteria_have_non_empty_fields() {
        for def in DEFINITIONS {
            assert!(!def.id.is_empty(), "Empty id in definition");
            assert!(!def.title.is_empty(), "Empty title for {}", def.id);
            assert!(!def.wcag_refs.is_empty(), "Empty wcag_refs for {}", def.id);
            assert!(
                !def.definition.is_empty(),
                "Empty definition for {}",
                def.id
            );
        }
    }
}
