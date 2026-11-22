//! Data model shared by every analysis plugin (P-00 §5).
//!
//! A [`Preuve`] carries measurements and artefacts, never a status: the
//! orchestrator alone turns it into a verdict (R-P1).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A bounding box in CSS pixels, origin at the top-left.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// What the proof is about.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub enum Subject {
    /// A DOM element, with both a CSS selector and an XPath.
    Element {
        selecteur: String,
        xpath: String,
        bbox: Rect,
    },
    /// A media resource downloaded from the page.
    Media { url: String },
    /// The whole page.
    Page,
}

/// Whether the proof is an exact calculation or an estimate (R-P2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum Portee {
    /// Exact calculation on a reliable input; may feed an Automatique test.
    Mesuree,
    /// Sampling or model output; always ends in review.
    Estimee,
}

/// Review-queue hint. Never a status, never a verdict (R3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum Indice {
    Suspect { raison: String },
    Neutre,
    Rassurant { raison: String },
}

/// A file written into the audit bundle under `analysis/<plugin>/<sha256>.<ext>`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Artefact {
    pub nom: String,
    pub media_type: String,
    pub chemin_bundle: String,
    pub sha256: String,
}

/// One measurement produced by a plugin for one or more RGAA tests.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Preuve {
    /// Stable plugin identifier, kebab-case.
    pub plugin: String,
    pub version_plugin: String,
    pub version_modele: Option<String>,
    /// RGAA test identifiers served by this proof, e.g. `["3.2.1"]`.
    pub tests: Vec<String>,
    pub sujet: Subject,
    pub portee: Portee,
    /// Plugin-specific measurements; their JSON Schema lives in `types/`.
    pub mesures: serde_json::Value,
    pub indice: Option<Indice>,
    pub artefacts: Vec<Artefact>,
    /// SHA-256 of the analysed input (R-P4).
    pub empreinte_entree: String,
}

/// A local model required by a plugin (P-00 §6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ModelSpec {
    pub nom: &'static str,
    pub url: &'static str,
    pub sha256: &'static str,
    pub taille_octets: u64,
    pub licence: &'static str,
}

/// Model state as shown by `rgaa plugins list`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ModelInfo {
    pub nom: String,
    pub licence: String,
    pub present: bool,
}

/// Per-call options, serialised as JSON on the MCP and sidecar boundaries.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct PluginOptions {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// Terms a plugin should ignore (P-06 uses it for proper nouns and
    /// loanwords). Absent means the plugin's own default list.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exclusions: Option<Vec<String>>,
    /// P-04: ignore media longer than this many seconds (`None` → 3600).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duree_max_s: Option<u64>,
    /// P-04: URL of the hand-written reference transcription standing in for
    /// ASR output until the model-backed plugin (PR 2) is enabled.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transcription_reference_url: Option<String>,
}

/// Native or sidecar plugin (the orchestrator makes no distinction).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum PluginKind {
    Natif,
    Sidecar,
}

/// The part of a plugin's description that is displayable without running it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct PluginInfo {
    pub id: String,
    pub version: String,
    pub tests: Vec<String>,
    pub models: Vec<ModelInfo>,
    pub kind: PluginKind,
    /// Exact command line for a sidecar, for the `rgaa plugins list` audit trail.
    pub commande: Option<String>,
    /// SHA-256 of the sidecar executable; a change between audits is warned about.
    pub sha256_executable: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proof_round_trips_through_json() {
        let proof = Preuve {
            plugin: "echo".into(),
            version_plugin: "0.1.0".into(),
            version_modele: None,
            tests: vec!["1.1.1".into()],
            sujet: Subject::Page,
            portee: Portee::Mesuree,
            mesures: serde_json::json!({"echo": "ok"}),
            indice: Some(Indice::Neutre),
            artefacts: Vec::new(),
            empreinte_entree: "sha256:abc".into(),
        };
        let json = serde_json::to_string(&proof).expect("serialize");
        let decoded: Preuve = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(decoded, proof);
    }
}
