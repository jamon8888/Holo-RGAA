//! The plugin-specific `mesures` schemas (P-00 §5, P-07 §4).
//!
//! A [`Mesures`] value never carries a `status` or a `verdict`: the engine
//! decides those from `portee` and the review hint (R-P1). `avis_modele` is an
//! extension point for a future LLM second opinion; this plugin leaves it
//! `None` and makes no network call (R-P3).

use rgaa_analysis::{Artefact, Indice, Portee, Rect};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// One detected visual heading (9.1).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct TitreVisuel {
    pub texte: String,
    pub taille_px: f64,
    pub graisse: u16,
    /// Whether a DOM/AX heading with the same text was found.
    pub expose: bool,
}

/// One exposed heading (9.1).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct TitreExpose {
    pub niveau: Option<u8>,
    pub texte: String,
    pub taille_px: Option<f64>,
}

/// A repeated visual zone compared with AX landmarks (9.2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ZoneRepetee {
    pub selecteur: String,
    pub occurrences: usize,
    pub bbox: Option<Rect>,
    /// Whether an AX landmark covers this zone.
    pub repere: bool,
    pub role_attendu: String,
}

/// An AX landmark found on the page (9.2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Repere {
    pub role: String,
    pub selecteur: String,
}

/// A visual-position jump on a positioned element (10.3).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SautPositionne {
    pub selecteur: String,
    pub rang_dom: usize,
    pub rang_visuel: usize,
    pub motif: String,
}

/// A `td` styled like a header but not marked as one (5.6).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct CelluleEnTete {
    pub selecteur: String,
    pub ligne: usize,
    pub colonne: usize,
    pub graisse: u16,
    /// CIEDE2000 distance to the table body background.
    pub delta_e: f64,
    pub raison: String,
}

/// A text referring to shape, size, position or colour (10.9 / 10.10).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct CandidatSensoriel {
    pub selecteur: String,
    pub texte: String,
    pub motif: String,
    pub categorie: String,
    pub langue: String,
}

/// A custom widget whose AX role or state did not follow the rendering (7.1).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct WidgetObserve {
    pub selecteur: String,
    pub role_avant: String,
    pub role_apres: String,
    pub etats_avant: Vec<String>,
    pub etats_apres: Vec<String>,
    pub pixels_changed: f64,
    pub raison: String,
}

/// A visible status message that was not exposed to assistive technology (7.5).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct MessageStatut {
    pub selecteur: String,
    pub texte: String,
    pub region_live: Option<String>,
    pub role: Option<String>,
    pub focus: bool,
}

/// Every `mesures` shape, tagged by analysis. The tag keeps the JSON
/// self-describing without ever looking like a status.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "analyse", rename_all = "snake_case")]
pub enum Mesures {
    /// 9.1 — heading hierarchy.
    Headings {
        titres_visuels: Vec<TitreVisuel>,
        titres_exposes: Vec<TitreExpose>,
        mediane_corps_px: f64,
        inversions: usize,
        sauts_niveau: Vec<String>,
        /// Extension point only: always `null` in this ticket.
        avis_modele: Option<Value>,
    },
    /// 9.2 — document structure.
    Landmarks {
        zones_repetees: Vec<ZoneRepetee>,
        reperes: Vec<Repere>,
        sans_repere: Vec<String>,
    },
    /// 10.3 — reading order.
    ReadingOrder {
        tau: f64,
        inversions: usize,
        ordre_dom: Vec<String>,
        ordre_visuel: Vec<String>,
        sauts_positionnes: Vec<SautPositionne>,
        direction: String,
        avis_modele: Option<Value>,
    },
    /// 5.3 — presentation-table linearisation.
    Linearisation {
        selecteur: String,
        tau: f64,
        inversions: usize,
        ordre_dom: Vec<String>,
        ordre_visuel: Vec<String>,
    },
    /// 5.6 — visual table headers.
    TableHeaders {
        selecteur: String,
        candidats: Vec<CelluleEnTete>,
    },
    /// 10.9 / 10.10 — sensory characteristics.
    Sensory {
        candidats: Vec<CandidatSensoriel>,
        langue: String,
        avis_modele: Option<Value>,
    },
    /// 7.1 — custom interactive widgets.
    Widget {
        observes: Vec<WidgetObserve>,
        actions_ignorees: usize,
    },
    /// 7.5 — status messages.
    Status {
        messages: Vec<MessageStatut>,
        actions_ignorees: usize,
    },
}

/// What one analysis returns before it is wrapped into a `Preuve`.
#[derive(Debug, Clone)]
pub struct Measurement {
    pub mesures: Mesures,
    pub portee: Portee,
    pub indice: Option<Indice>,
    pub tests: Vec<String>,
    pub artefacts: Vec<Artefact>,
}

impl Measurement {
    #[must_use]
    pub fn new(mesures: Mesures, portee: Portee, tests: &[&str]) -> Self {
        Self {
            mesures,
            portee,
            indice: None,
            tests: tests.iter().map(|test| (*test).to_owned()).collect(),
            artefacts: Vec::new(),
        }
    }

    /// Attaches a review hint — never a status (R-P1).
    #[must_use]
    pub fn suspect(mut self, raison: impl Into<String>) -> Self {
        self.indice = Some(Indice::Suspect {
            raison: raison.into(),
        });
        self
    }
}

/// Keeps only the first `limit` action indices (spec §6: 20 by default).
#[must_use]
pub fn bounded(indices: impl IntoIterator<Item = usize>, limit: usize) -> Vec<usize> {
    indices.into_iter().take(limit).collect()
}
