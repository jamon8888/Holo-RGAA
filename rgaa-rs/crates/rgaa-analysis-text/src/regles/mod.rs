//! Règles déterministes et signaux estimés de la spec §3.

mod contenus;
pub mod formulaires;
mod liens;

use crate::mesures::Mesure;
use crate::snapshot::TextSnapshot;
use rgaa_analysis::{Rect, Subject};

/// Produit toutes les mesures d'une entrée, dans un ordre stable.
pub fn mesurer(snapshot: &TextSnapshot) -> Vec<Mesure> {
    let mut mesures = Vec::new();
    mesures.extend(liens::liens(snapshot));
    mesures.extend(liens::titres(snapshot));
    mesures.extend(liens::cadres(snapshot));
    mesures.extend(formulaires::champs(snapshot));
    mesures.extend(formulaires::legendes(snapshot));
    mesures.extend(formulaires::boutons(snapshot));
    mesures.extend(formulaires::formulaires(snapshot));
    mesures.extend(contenus::tableaux(snapshot));
    mesures.extend(contenus::plan_du_site(snapshot));
    mesures.extend(contenus::versions_alternatives(snapshot));
    mesures.extend(contenus::alternatives(snapshot));
    mesures
}

/// Sujet d'une mesure attachée à un élément identifié.
pub(crate) fn sujet_element(id: &str) -> Subject {
    Subject::Element {
        selecteur: format!("#{id}"),
        xpath: format!("//*[@id='{id}']"),
        bbox: Rect::default(),
    }
}

/// Sujet d'une mesure de page : l'URL est portée par le sélecteur, le contrat
/// P-00 n'ayant pas de variante `Page` paramétrée.
pub(crate) fn sujet_page(url: &str) -> Subject {
    Subject::Element {
        selecteur: format!("html[url={url}]"),
        xpath: "/html".to_owned(),
        bbox: Rect::default(),
    }
}
