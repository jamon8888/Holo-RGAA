#[derive(Copy, Clone)]
pub struct CriterionDefinition {
    pub id: &'static str,
    pub title: &'static str,
    pub wcag_refs: &'static str,
    pub definition: &'static str,
}

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
        wcag_refs: "2.4.1",
        definition: "Pour chaque cadre ayant un titre de cadre, ce titre est-il pertinent ?",
    },
    CriterionDefinition {
        id: "3.1",
        title: "Information non donnée uniquement par la couleur",
        wcag_refs: "1.4.1",
        definition: "L'information ne doit pas être donnée uniquement par la couleur, cette règle est-elle respectée ?",
    },
    CriterionDefinition {
        id: "4.2",
        title: "Transcription ou audiodescription pertinente",
        wcag_refs: "1.2.3, 1.2.5",
        definition: "Pour chaque média ayant une transcription ou audiodescription, celles-ci sont-elles pertinentes ?",
    },
    CriterionDefinition {
        id: "4.4",
        title: "Sous-titres synchronisés pertinents",
        wcag_refs: "1.2.2",
        definition: "Pour chaque média ayant des sous-titres synchronisés, ces sous-titres sont-ils pertinents ?",
    },
    CriterionDefinition {
        id: "4.6",
        title: "Audiodescription synchronisée pertinente",
        wcag_refs: "1.2.5",
        definition: "Pour chaque média ayant une audiodescription synchronisée, celle-ci est-elle pertinente ?",
    },
    CriterionDefinition {
        id: "4.9",
        title: "Version de remplacement pertinente",
        wcag_refs: "1.2.8",
        definition: "Pour chaque média ayant une version de remplacement, celle-ci est-elle pertinente ?",
    },
    CriterionDefinition {
        id: "5.2",
        title: "En-têtes de tableau pertinents",
        wcag_refs: "1.3.1",
        definition: "Pour chaque tableau de données complexe, les en-têtes de tableau sont-ils pertinents ?",
    },
    CriterionDefinition {
        id: "5.3",
        title: "Titre de tableau pertinent",
        wcag_refs: "1.3.1",
        definition: "Pour chaque tableau de données, le titre de tableau est-il pertinent ?",
    },
    CriterionDefinition {
        id: "5.5",
        title: "Linéarisation pertinente",
        wcag_refs: "1.3.2",
        definition: "Pour chaque tableau de données, la linéarisation est-elle pertinente ?",
    },
    CriterionDefinition {
        id: "7.2",
        title: "Alternatives aux scripts",
        wcag_refs: "4.1.2",
        definition: "Pour chaque script qui génère du contenu ou des composants d'interface, alternatives existent-elles ?",
    },
    CriterionDefinition {
        id: "8.4",
        title: "Langue pertinente",
        wcag_refs: "3.1.1, 3.1.2",
        definition: "La langue par défaut est-elle pertinente ? Pour chaque élément avec changement de langue, le changement est-il pertinent ?",
    },
    CriterionDefinition {
        id: "8.6",
        title: "Titre de page pertinent",
        wcag_refs: "2.4.2",
        definition: "Le titre de page est-il pertinent ?",
    },
    CriterionDefinition {
        id: "8.8",
        title: "Évitement des blocs de contenu répétitifs",
        wcag_refs: "2.4.1",
        definition: "Un moyen d'éviter les blocs de contenu répétitifs est-il présent ?",
    },
    CriterionDefinition {
        id: "9.2",
        title: "Structure de liste pertinente",
        wcag_refs: "1.3.1",
        definition: "Chaque liste est-elle structurée de manière pertinente ?",
    },
    CriterionDefinition {
        id: "10.3",
        title: "Ordre de lecture pertinent",
        wcag_refs: "1.3.2, 2.4.3",
        definition: "L'ordre de lecture est-il pertinent ?",
    },
    CriterionDefinition {
        id: "10.10",
        title: "Contenu positionné par CSS pertinent",
        wcag_refs: "1.3.2",
        definition: "Le contenu positionné par CSS est-il dans un ordre de lecture pertinent ?",
    },
    CriterionDefinition {
        id: "11.2",
        title: "Étiquette de champ pertinente",
        wcag_refs: "1.3.1, 4.1.2",
        definition: "Pour chaque champ de formulaire, l'étiquette est-elle pertinente ?",
    },
    CriterionDefinition {
        id: "11.3",
        title: "Regroupement de champs pertinent",
        wcag_refs: "1.3.1",
        definition: "Pour chaque regroupement de champs de formulaire, le regroupement est-il pertinent ?",
    },
    CriterionDefinition {
        id: "11.7",
        title: "Suggestions de correction pertinentes",
        wcag_refs: "3.3.3",
        definition: "Pour chaque champ de formulaire ayant une suggestion de correction, la suggestion est-elle pertinente ?",
    },
    CriterionDefinition {
        id: "11.8",
        title: "Identification des erreurs pertinente",
        wcag_refs: "3.3.1",
        definition: "Pour chaque champ de formulaire ayant une erreur de saisie, l'erreur est-elle identifiée de manière pertinente ?",
    },
    CriterionDefinition {
        id: "11.9",
        title: "Indication des champs obligatoires pertinente",
        wcag_refs: "3.3.2",
        definition: "Pour chaque champ obligatoire, l'indication est-elle pertinente ?",
    },
    CriterionDefinition {
        id: "11.10",
        title: "Finalité du champ pertinente",
        wcag_refs: "1.3.5",
        definition: "Pour chaque champ de formulaire, la finalité du champ est-elle pertinente ?",
    },
    CriterionDefinition {
        id: "12.3",
        title: "Structure de menu pertinente",
        wcag_refs: "1.3.1",
        definition: "Chaque menu est-il structuré de manière pertinente ?",
    },
    CriterionDefinition {
        id: "12.8",
        title: "Ordre de tabulation pertinent",
        wcag_refs: "2.4.3",
        definition: "L'ordre de tabulation est-il pertinent ?",
    },
    CriterionDefinition {
        id: "13.6",
        title: "Linéarisation des tableaux pertinente",
        wcag_refs: "1.3.2",
        definition: "Pour chaque tableau de données, la linéarisation est-elle pertinente ?",
    },
];

/// Criteria that require visual understanding or complex reasoning.
/// Routed to the 122b model.
pub const VISUAL_CRITERIA: &[&str] = &[
    "1.3",   // alt text relevance — compare alt vs actual image
    "1.7",   // detailed description relevance
    "3.1",   // color-only information — must SEE the page
    "10.3",  // reading order — must SEE layout
    "10.10", // CSS-positioned content — must SEE rendering
    "11.2",  // label relevance — must SEE label next to input
    "11.3",  // fieldset/legend — must SEE form grouping
    "11.7",  // error suggestion — complex reasoning
    "11.8",  // error identification — complex reasoning
    "11.9",  // mandatory field indication — complex reasoning
    "11.10", // form field purpose — complex reasoning
    "12.8",  // focus order — must INTERACT with page
    "13.6",  // table linearization — must SEE table rendering
];
