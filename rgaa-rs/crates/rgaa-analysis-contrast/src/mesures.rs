//! The plugin-specific `mesures` schema (P-01 §4) and the proof it is wrapped in.
//!
//! A [`Mesures`] object never carries a `status` or a `verdict`: the engine
//! decides those from `portee` and `ratio` (R-P1).

use rgaa_analysis::{Artefact, Indice, Portee};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Which step of the spec produced the measurement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum Etape {
    /// CSS measurement, on a reliably known background.
    A,
    /// Pixel measurement, on an image, gradient, opacity or overlap.
    B,
    /// Non-textual component measurement.
    C,
}

/// State replayed by the orchestrator before it hands over the capture (§3 D).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Etat {
    #[default]
    Repos,
    Hover,
    Focus,
}

/// Exactly the fields of the `mesures` JSON object documented in P-01 §4.
/// `null` means "not applicable to this step", for instance `seuil` on a
/// disabled text where the spec says there is no threshold.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Mesures {
    pub etape: Etape,
    pub etat: Etat,
    pub ratio: Option<f64>,
    pub seuil: Option<f64>,
    pub texte_grand: bool,
    pub couleur_avant: Option<String>,
    pub couleur_fond: Option<String>,
    pub fond_p5: Option<String>,
    pub fond_p95: Option<String>,
    pub taille_px: Option<f64>,
    pub graisse: Option<u16>,
    /// Reason for a `Neutre` index or an uncertain separation, when any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raison: Option<String>,
}

/// What one measurement step returns before it is wrapped into a `Preuve`.
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
    pub fn new(mesures: Mesures, portee: Portee, tests: Vec<String>) -> Self {
        Self {
            mesures,
            portee,
            indice: None,
            tests,
            artefacts: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_indice(mut self, indice: Indice) -> Self {
        self.indice = Some(indice);
        self
    }

    #[must_use]
    pub fn with_artefact(mut self, artefact: Artefact) -> Self {
        self.artefacts.push(artefact);
        self
    }
}

/// Steps A/B: the RGAA 3.2 test selected by the computed size and weight.
#[must_use]
pub fn text_tests(size_px: f64, weight: u16) -> Vec<String> {
    let test = match (size_px >= 24.0, size_px >= 18.66 && weight >= 700) {
        (false, false) => "3.2.1",
        (false, true) => "3.2.2",
        (true, false) => "3.2.3",
        (true, true) => "3.2.4",
    };
    vec![test.to_owned()]
}

#[must_use]
pub const fn is_large_text(size_px: f64, weight: u16) -> bool {
    size_px >= 24.0 || (size_px >= 18.66 && weight >= 700)
}

/// Step A/B threshold: 4.5:1, or 3:1 for large text.
#[must_use]
pub const fn text_threshold(large: bool) -> f64 {
    if large {
        3.0
    } else {
        4.5
    }
}

/// Step C threshold (§3 C).
pub const COMPONENT_THRESHOLD: f64 = 3.0;

/// Review-queue hint shared by steps B and C (§5).
#[must_use]
pub fn estimate_indice(ratio: f64, threshold: f64, reason: &str) -> Indice {
    if ratio < threshold {
        Indice::Suspect {
            raison: reason.to_owned(),
        }
    } else if ratio >= threshold * 1.2 {
        Indice::Rassurant {
            raison: format!("ratio {ratio:.2}:1 ≥ {:.1}:1", threshold * 1.2),
        }
    } else {
        Indice::Neutre
    }
}
