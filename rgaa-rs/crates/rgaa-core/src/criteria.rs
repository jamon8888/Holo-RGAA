use crate::types::Classification;

#[derive(Debug, Clone)]
pub struct Criterion {
    pub id: &'static str,
    pub title: &'static str,
    pub classification: Classification,
    pub wcag_refs: &'static str,
}

pub struct RgaaCriteria;

impl RgaaCriteria {
    pub fn all() -> Vec<Criterion> {
        vec![
            Criterion { id: "1.1", title: "Alternative textuelle présente", classification: Classification::Deterministe, wcag_refs: "1.1.1" },
            Criterion { id: "1.2", title: "Image décorative ignorée", classification: Classification::Deterministe, wcag_refs: "1.1.1, 4.1.2" },
            Criterion { id: "1.3", title: "Alternative textuelle pertinente", classification: Classification::IaAssiste, wcag_refs: "1.1.1, 4.1.2" },
            Criterion { id: "1.4", title: "Alternative CAPTCHA/image-test", classification: Classification::Deterministe, wcag_refs: "1.1.1" },
            Criterion { id: "1.5", title: "Solution accès alternatif CAPTCHA", classification: Classification::Deterministe, wcag_refs: "1.1.1" },
            Criterion { id: "1.6", title: "Description détaillée présente", classification: Classification::Deterministe, wcag_refs: "1.1.1" },
            Criterion { id: "1.7", title: "Description détaillée pertinente", classification: Classification::IaAssiste, wcag_refs: "1.1.1" },
            Criterion { id: "1.8", title: "Image texte remplacée par texte stylé", classification: Classification::Deterministe, wcag_refs: "1.4.5" },
            Criterion { id: "1.9", title: "Légende reliée à l'image", classification: Classification::Deterministe, wcag_refs: "1.1.1, 4.1.2" },
            Criterion { id: "2.1", title: "Cadre a un titre", classification: Classification::Deterministe, wcag_refs: "4.1.2" },
            Criterion { id: "2.2", title: "Titre de cadre pertinent", classification: Classification::IaAssiste, wcag_refs: "4.1.2" },
            Criterion { id: "3.1", title: "Information non donnée uniquement par couleur", classification: Classification::IaAssiste, wcag_refs: "1.3.1, 1.4.1" },
            Criterion { id: "3.2", title: "Contraste texte/fond suffisant", classification: Classification::Deterministe, wcag_refs: "1.4.3" },
            Criterion { id: "3.3", title: "Contraste composants graphiques suffisant", classification: Classification::Deterministe, wcag_refs: "1.4.11" },
            Criterion { id: "4.1", title: "Transcription/audiodescription présente", classification: Classification::Deterministe, wcag_refs: "1.2.1, 1.2.3" },
            Criterion { id: "4.2", title: "Transcription/audiodescription pertinente", classification: Classification::IaAssiste, wcag_refs: "1.2.1, 1.2.3" },
            Criterion { id: "4.3", title: "Sous-titres synchronisés présents", classification: Classification::Deterministe, wcag_refs: "1.2.2" },
            Criterion { id: "4.4", title: "Sous-titres pertinents", classification: Classification::IaAssiste, wcag_refs: "1.2.2" },
            Criterion { id: "4.5", title: "Audiodescription présente", classification: Classification::Deterministe, wcag_refs: "1.2.5" },
            Criterion { id: "4.6", title: "Audiodescription pertinente", classification: Classification::IaAssiste, wcag_refs: "1.2.5" },
            Criterion { id: "4.7", title: "Média temporel identifiable", classification: Classification::Deterministe, wcag_refs: "1.1.1" },
            Criterion { id: "4.8", title: "Alternative média non temporel", classification: Classification::Deterministe, wcag_refs: "1.1.1" },
            Criterion { id: "4.9", title: "Alternative pertinente média non temporel", classification: Classification::IaAssiste, wcag_refs: "1.1.1" },
            Criterion { id: "4.10", title: "Son contrôlable", classification: Classification::Deterministe, wcag_refs: "1.4.2" },
            Criterion { id: "4.11", title: "Média temporel contrôlable clavier", classification: Classification::Deterministe, wcag_refs: "2.1.1, 2.1.2" },
            Criterion { id: "4.12", title: "Média non temporel contrôlable clavier", classification: Classification::Deterministe, wcag_refs: "2.1.1, 2.1.2" },
            Criterion { id: "4.13", title: "Média compatible AT", classification: Classification::Deterministe, wcag_refs: "4.1.2" },
            Criterion { id: "5.1", title: "Tableau complexe a résumé", classification: Classification::Deterministe, wcag_refs: "1.3.1" },
            Criterion { id: "5.2", title: "Résumé pertinent tableau complexe", classification: Classification::IaAssiste, wcag_refs: "1.3.1" },
            Criterion { id: "5.3", title: "Contenu linéarisé compréhensible", classification: Classification::IaAssiste, wcag_refs: "1.3.2, 4.1.2" },
            Criterion { id: "5.4", title: "Titre tableau correctement associé", classification: Classification::Deterministe, wcag_refs: "1.3.1" },
            Criterion { id: "5.5", title: "Titre pertinent tableau", classification: Classification::IaAssiste, wcag_refs: "1.3.1" },
            Criterion { id: "5.6", title: "En-têtes déclarés correctement", classification: Classification::Deterministe, wcag_refs: "1.3.1" },
            Criterion { id: "5.7", title: "Association cellules/en-têtes", classification: Classification::Deterministe, wcag_refs: "1.3.1" },
            Criterion { id: "5.8", title: "Tableau mise en forme sans éléments données", classification: Classification::Deterministe, wcag_refs: "1.3.1" },
            Criterion { id: "6.1", title: "Lien explicite", classification: Classification::Deterministe, wcag_refs: "1.1.1, 2.4.4, 2.5.3" },
            Criterion { id: "6.2", title: "Lien a un intitulé", classification: Classification::Deterministe, wcag_refs: "1.1.1, 2.4.4" },
            Criterion { id: "7.1", title: "Script compatible AT", classification: Classification::Deterministe, wcag_refs: "2.5.3, 4.1.2" },
            Criterion { id: "7.2", title: "Alternative script pertinente", classification: Classification::IaAssiste, wcag_refs: "1.1.1, 4.1.2" },
            Criterion { id: "7.3", title: "Script contrôlable clavier", classification: Classification::Deterministe, wcag_refs: "1.3.1, 2.1.1, 2.4.7" },
            Criterion { id: "7.4", title: "Changement de contexte averti/contrôlé", classification: Classification::Deterministe, wcag_refs: "3.2.1, 3.2.2" },
            Criterion { id: "7.5", title: "Messages de statut restitués AT", classification: Classification::Manuel, wcag_refs: "4.1.3" },
            Criterion { id: "8.1", title: "Type de document défini", classification: Classification::Deterministe, wcag_refs: "4.1.1" },
            Criterion { id: "8.2", title: "Code valide selon doctype", classification: Classification::Deterministe, wcag_refs: "4.1.1, 4.1.2" },
            Criterion { id: "8.3", title: "Langue par défaut présente", classification: Classification::Deterministe, wcag_refs: "3.1.1" },
            Criterion { id: "8.4", title: "Code de langue pertinent", classification: Classification::IaAssiste, wcag_refs: "3.1.1" },
            Criterion { id: "8.5", title: "Titre de page", classification: Classification::Deterministe, wcag_refs: "2.4.2" },
            Criterion { id: "8.6", title: "Titre de page pertinent", classification: Classification::IaAssiste, wcag_refs: "2.4.2" },
            Criterion { id: "8.7", title: "Changement de langue indiqué", classification: Classification::Deterministe, wcag_refs: "3.1.2" },
            Criterion { id: "8.8", title: "Code de langue changement pertinent", classification: Classification::IaAssiste, wcag_refs: "3.1.2" },
            Criterion { id: "8.9", title: "Balises pas uniquement présentation", classification: Classification::Deterministe, wcag_refs: "1.3.1" },
            Criterion { id: "8.10", title: "Changements sens lecture signalés", classification: Classification::Deterministe, wcag_refs: "1.3.2" },
            Criterion { id: "9.1", title: "Structure par titres appropriée", classification: Classification::Deterministe, wcag_refs: "1.3.1, 2.4.1, 2.4.6, 4.1.2" },
            Criterion { id: "9.2", title: "Structure document cohérente", classification: Classification::IaAssiste, wcag_refs: "1.3.1" },
            Criterion { id: "9.3", title: "Liste correctement structurée", classification: Classification::Deterministe, wcag_refs: "1.3.1" },
            Criterion { id: "9.4", title: "Citation correctement indiquée", classification: Classification::Deterministe, wcag_refs: "1.3.1" },
            Criterion { id: "10.1", title: "CSS pour présentation", classification: Classification::Deterministe, wcag_refs: "1.3.1, 1.3.2" },
            Criterion { id: "10.2", title: "Contenu visible sans CSS", classification: Classification::Deterministe, wcag_refs: "1.1.1, 1.3.1" },
            Criterion { id: "10.3", title: "Information compréhensible sans CSS", classification: Classification::IaAssiste, wcag_refs: "1.3.2, 2.4.3" },
            Criterion { id: "10.4", title: "Texte lisible zoom 200%", classification: Classification::Deterministe, wcag_refs: "1.4.4" },
            Criterion { id: "10.5", title: "Déclarations CSS couleurs correctes", classification: Classification::Deterministe, wcag_refs: "1.4.3" },
            Criterion { id: "10.6", title: "Lien visible vs texte environnant", classification: Classification::Deterministe, wcag_refs: "1.4.1" },
            Criterion { id: "10.7", title: "Focus visible", classification: Classification::Deterministe, wcag_refs: "1.4.1, 2.4.7" },
            Criterion { id: "10.8", title: "Contenus cachés ignorés AT", classification: Classification::Deterministe, wcag_refs: "1.3.2, 4.1.2" },
            Criterion { id: "10.9", title: "Info non donnée par forme/taille/position", classification: Classification::Deterministe, wcag_refs: "1.3.3, 1.4.1" },
            Criterion { id: "10.10", title: "Implémentation pertinente forme/taille/position", classification: Classification::IaAssiste, wcag_refs: "1.3.3, 1.4.1" },
            Criterion { id: "10.11", title: "Reflow 320px/256px", classification: Classification::Deterministe, wcag_refs: "1.4.10" },
            Criterion { id: "10.12", title: "Espacement texte redéfinissable", classification: Classification::Deterministe, wcag_refs: "1.4.12" },
            Criterion { id: "10.13", title: "Contenus additionnels focus/survol contrôlables", classification: Classification::Deterministe, wcag_refs: "1.4.13" },
            Criterion { id: "10.14", title: "Contenus CSS only accessibles clavier", classification: Classification::Deterministe, wcag_refs: "2.1.1" },
            Criterion { id: "11.1", title: "Champ a étiquette", classification: Classification::Deterministe, wcag_refs: "1.3.1, 2.4.6, 3.3.2, 4.1.2" },
            Criterion { id: "11.2", title: "Étiquette champ pertinente", classification: Classification::IaAssiste, wcag_refs: "2.4.6, 2.5.3, 3.3.2" },
            Criterion { id: "11.3", title: "Étiquettes cohérentes même fonction", classification: Classification::IaAssiste, wcag_refs: "3.2.4" },
            Criterion { id: "11.4", title: "Étiquette et champ accolés", classification: Classification::Deterministe, wcag_refs: "3.3.2" },
            Criterion { id: "11.5", title: "Champs même nature regroupés", classification: Classification::Deterministe, wcag_refs: "1.3.1, 3.3.2" },
            Criterion { id: "11.6", title: "Regroupement a légende", classification: Classification::Deterministe, wcag_refs: "1.3.1, 3.3.2" },
            Criterion { id: "11.7", title: "Légende regroupement pertinente", classification: Classification::IaAssiste, wcag_refs: "1.3.1, 3.3.2" },
            Criterion { id: "11.8", title: "Items liste choix regroupés pertinemment", classification: Classification::IaAssiste, wcag_refs: "1.3.1" },
            Criterion { id: "11.9", title: "Intitulé bouton pertinent", classification: Classification::IaAssiste, wcag_refs: "2.5.3, 4.1.2" },
            Criterion { id: "11.10", title: "Contrôle saisie utilisé pertinemment", classification: Classification::IaAssiste, wcag_refs: "3.3.1, 3.3.2" },
            Criterion { id: "11.11", title: "Suggestions correction erreurs", classification: Classification::Deterministe, wcag_refs: "3.3.3" },
            Criterion { id: "11.12", title: "Données modifiables/récupérables", classification: Classification::Deterministe, wcag_refs: "3.3.4" },
            Criterion { id: "11.13", title: "Finalité champ déductible", classification: Classification::Deterministe, wcag_refs: "1.3.5" },
            Criterion { id: "12.1", title: "Deux systèmes navigation", classification: Classification::Deterministe, wcag_refs: "2.4.5" },
            Criterion { id: "12.2", title: "Navigation même place", classification: Classification::Deterministe, wcag_refs: "3.2.3" },
            Criterion { id: "12.3", title: "Plan du site pertinent", classification: Classification::IaAssiste, wcag_refs: "2.4.5" },
            Criterion { id: "12.4", title: "Plan site accessible identique", classification: Classification::Deterministe, wcag_refs: "2.4.5, 3.2.3" },
            Criterion { id: "12.5", title: "Moteur recherche atteignable identiquement", classification: Classification::Deterministe, wcag_refs: "3.2.3" },
            Criterion { id: "12.6", title: "Zones regroupement atteignables", classification: Classification::Deterministe, wcag_refs: "1.3.1, 2.4.1, 4.1.2" },
            Criterion { id: "12.7", title: "Lien évitement contenu principal", classification: Classification::Deterministe, wcag_refs: "2.4.1, 2.4.3, 3.2.3" },
            Criterion { id: "12.8", title: "Ordre tabulation cohérent", classification: Classification::IaAssiste, wcag_refs: "2.4.3" },
            Criterion { id: "12.9", title: "Pas de piège clavier", classification: Classification::Deterministe, wcag_refs: "2.1.1, 2.1.2" },
            Criterion { id: "12.10", title: "Raccourcis clavier contrôlables", classification: Classification::Deterministe, wcag_refs: "2.1.4" },
            Criterion { id: "12.11", title: "Contenus additionnels atteignables clavier", classification: Classification::Deterministe, wcag_refs: "2.1.1" },
            Criterion { id: "13.1", title: "Contrôle limites temps", classification: Classification::Deterministe, wcag_refs: "2.2.1, 2.2.2" },
            Criterion { id: "13.2", title: "Pas ouverture fenêtre sans action", classification: Classification::Deterministe, wcag_refs: "3.2.1" },
            Criterion { id: "13.3", title: "Document bureautique version accessible", classification: Classification::Deterministe, wcag_refs: "1.1.1, 1.3.1, 1.3.2, 2.4.1, 2.4.3, 3.1.1, 4.1.2" },
            Criterion { id: "13.4", title: "Version accessible même information", classification: Classification::Deterministe, wcag_refs: "1.1.1, 1.3.1, 1.3.2, 2.4.1, 2.4.3, 3.1.1, 4.1.2" },
            Criterion { id: "13.5", title: "Contenu cryptique a alternative", classification: Classification::Deterministe, wcag_refs: "1.1.1" },
            Criterion { id: "13.6", title: "Alternative pertinente contenu cryptique", classification: Classification::IaAssiste, wcag_refs: "1.1.1" },
            Criterion { id: "13.7", title: "Flash/luminosité corrects", classification: Classification::Deterministe, wcag_refs: "2.3.1" },
            Criterion { id: "13.8", title: "Contenu mouvement/clignotant contrôlable", classification: Classification::Deterministe, wcag_refs: "2.2.1, 2.2.2" },
            Criterion { id: "13.9", title: "Orientation portrait/paysage", classification: Classification::Deterministe, wcag_refs: "1.3.4" },
            Criterion { id: "13.10", title: "Geste complexe = geste simple", classification: Classification::Deterministe, wcag_refs: "2.5.1" },
            Criterion { id: "13.11", title: "Annulation action pointage", classification: Classification::Deterministe, wcag_refs: "2.5.2" },
            Criterion { id: "13.12", title: "Mouvement appareil alternative", classification: Classification::Deterministe, wcag_refs: "2.5.4" },
        ]
    }

    pub fn find(id: &str) -> Option<Criterion> {
        Self::all().into_iter().find(|c| c.id == id)
    }

    pub fn deterministic() -> Vec<Criterion> {
        Self::all().into_iter()
            .filter(|c| c.classification == Classification::Deterministe)
            .collect()
    }

    pub fn ia_assiste() -> Vec<Criterion> {
        Self::all().into_iter()
            .filter(|c| c.classification == Classification::IaAssiste)
            .collect()
    }

    pub fn count() -> usize {
        Self::all().len()
    }
}
