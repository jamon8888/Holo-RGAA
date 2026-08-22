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
    CriterionDefinition {
        id: "1.3",
        title: "Alternative textuelle pertinente",
        wcag_refs: "1.1.1, 4.1.2",
        definition: "Pour chaque image porteuse d'information ayant une alternative textuelle, cette alternative est-elle pertinente ?",
    },
    CriterionDefinition {
        id: "1.7",
        title: "Description détaillée pertinente",
        wcag_refs: "1.1.1",
        definition: "Pour chaque image porteuse d'information ayant une description détaillée, cette description est-elle pertinente ?",
    },
    CriterionDefinition {
        id: "2.2",
        title: "Titre de cadre pertinent",
        wcag_refs: "4.1.2",
        definition: "Pour chaque cadre (iframe) ayant un titre, ce titre est-il pertinent pour décrire le contenu du cadre ?",
    },
    CriterionDefinition {
        id: "3.1",
        title: "Information non donnée uniquement par couleur",
        wcag_refs: "1.3.1, 1.4.1",
        definition: "L'information ne doit pas être donnée uniquement par la couleur. Les différences d'information sont-elles perceptibles par d'autres moyens (texte, icône, motif) ?",
    },
    CriterionDefinition {
        id: "4.2",
        title: "Transcription/audiodescription pertinente",
        wcag_refs: "1.2.1, 1.2.3",
        definition: "Pour chaque média temporel ayant une transcription ou une audiodescription, celles-ci sont-elles pertinentes et complètes ?",
    },
    CriterionDefinition {
        id: "4.4",
        title: "Sous-titres pertinents",
        wcag_refs: "1.2.2",
        definition: "Pour chaque média temporel ayant des sous-titres synchronisés, ces sous-titres sont-ils pertinents et fidèles à l'audio ?",
    },
    CriterionDefinition {
        id: "4.6",
        title: "Audiodescription pertinente",
        wcag_refs: "1.2.5",
        definition: "Pour chaque média temporel ayant une audiodescription synchronisée, celle-ci est-elle pertinente et complète ?",
    },
    CriterionDefinition {
        id: "4.9",
        title: "Alternative pertinente média non temporel",
        wcag_refs: "1.1.1",
        definition: "Pour chaque média non temporel (image, graphique, schéma) ayant une alternative, celle-ci est-elle pertinente pour comprendre le contenu ?",
    },
    CriterionDefinition {
        id: "5.2",
        title: "Résumé pertinent tableau complexe",
        wcag_refs: "1.3.1",
        definition: "Pour chaque tableau de données complexe ayant un résumé, ce résumé est-il pertinent et décrit-il correctement le tableau ?",
    },
    CriterionDefinition {
        id: "5.3",
        title: "Contenu linéarisé compréhensible",
        wcag_refs: "1.3.2, 4.1.2",
        definition: "Le contenu linéarisé doit-il être compréhensible dans un ordre de lecture différent ? L'ordre de lecture est-il cohérent ?",
    },
    CriterionDefinition {
        id: "5.5",
        title: "Titre pertinent tableau",
        wcag_refs: "1.3.1",
        definition: "Pour chaque tableau de données ayant un titre, ce titre est-il pertinent et décrit-il le contenu du tableau ?",
    },
    CriterionDefinition {
        id: "7.2",
        title: "Alternative script pertinente",
        wcag_refs: "1.1.1, 4.1.2",
        definition: "Pour chaque script qui génère du contenu ou des composants d'interface, une alternative pertinente existe-t-elle ?",
    },
    CriterionDefinition {
        id: "8.4",
        title: "Code de langue pertinent",
        wcag_refs: "3.1.1",
        definition: "Le code de langue par défaut est-il pertinent et correspond-il à la langue réelle du contenu ?",
    },
    CriterionDefinition {
        id: "8.6",
        title: "Titre de page pertinent",
        wcag_refs: "2.4.2",
        definition: "Le titre de page est-il pertinent et décrit-il le contenu de la page ?",
    },
    CriterionDefinition {
        id: "8.8",
        title: "Code de langue changement pertinent",
        wcag_refs: "3.1.2",
        definition: "Pour chaque changement de langue dans le document, le code de langue est-il correct et pertinent ?",
    },
    CriterionDefinition {
        id: "9.2",
        title: "Structure document cohérente",
        wcag_refs: "1.3.1",
        definition: "La structure du document est-elle cohérente et hiérarchique ? Les titres et éléments structurels suivent-ils une logique ?",
    },
    CriterionDefinition {
        id: "10.3",
        title: "Information compréhensible sans CSS",
        wcag_refs: "1.3.2, 2.4.3",
        definition: "L'information est-elle compréhensible même sans CSS ? L'ordre de lecture et la hiérarchie sont-ils maintenus ?",
    },
    CriterionDefinition {
        id: "10.10",
        title: "Implémentation pertinente forme/taille/position",
        wcag_refs: "1.3.3, 1.4.1",
        definition: "L'information ne doit pas être donnée uniquement par la forme, la taille ou la position. Ces éléments visuels sont-ils complétés par du contenu textuel ou sémantique ?",
    },
    CriterionDefinition {
        id: "11.2",
        title: "Étiquette champ pertinente",
        wcag_refs: "2.4.6, 2.5.3, 3.3.2",
        definition: "Pour chaque champ de formulaire ayant une étiquette, celle-ci est-elle pertinente et suffisamment descriptive ?",
    },
    CriterionDefinition {
        id: "11.3",
        title: "Étiquettes cohérentes même fonction",
        wcag_refs: "3.2.4",
        definition: "Les champs ayant la même fonction utilisent-ils des étiquettes cohérentes sur l'ensemble du site ?",
    },
    CriterionDefinition {
        id: "11.7",
        title: "Légende regroupement pertinente",
        wcag_refs: "1.3.1, 3.3.2",
        definition: "Pour chaque regroupement de champs de formulaire ayant une légende, celle-ci est-elle pertinente et décrit-elle le groupe ?",
    },
    CriterionDefinition {
        id: "11.8",
        title: "Items liste choix regroupés pertinemment",
        wcag_refs: "1.3.1",
        definition: "Pour chaque liste de choix ou liste de données, les items sont-ils correctement regroupés et structurés de manière pertinente ?",
    },
    CriterionDefinition {
        id: "11.9",
        title: "Intitulé bouton pertinent",
        wcag_refs: "2.5.3, 4.1.2",
        definition: "Pour chaque bouton, l'intitulé est-il pertinent et décrit-il clairement l'action déclenchée ?",
    },
    CriterionDefinition {
        id: "11.10",
        title: "Contrôle saisie utilisé pertinemment",
        wcag_refs: "3.3.1, 3.3.2",
        definition: "Pour chaque contrôle de saisie, le type utilisé est-il approprié pour la nature de la donnée attendue ?",
    },
    CriterionDefinition {
        id: "12.3",
        title: "Plan du site pertinent",
        wcag_refs: "2.4.5",
        definition: "Le plan du site est-il pertinent, à jour et décrit-il correctement l'architecture du site ?",
    },
    CriterionDefinition {
        id: "12.8",
        title: "Ordre tabulation cohérent",
        wcag_refs: "2.4.3",
        definition: "L'ordre de tabulation est-il cohérent et suit-il la logique de lecture et d'interaction de la page ?",
    },
    CriterionDefinition {
        id: "13.6",
        title: "Alternative pertinente contenu cryptique",
        wcag_refs: "1.1.1",
        definition: "Pour chaque contenu cryptique (CAPTCHA, code, image-test) ayant une alternative, celle-ci est-elle pertinente ?",
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
