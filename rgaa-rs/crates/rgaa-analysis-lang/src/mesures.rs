//! Shape of the `mesures` object emitted by P-06 (spec §3).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// One divergent block, as described in the spec's `blocs_divergents`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct BlocDivergent {
    pub selecteur: String,
    pub lang_herite: String,
    pub detectee: String,
    pub confiance: f64,
    pub extrait: String,
}

/// Page-level measurements.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct LangMesures {
    pub declaree: Option<String>,
    pub detectee: Option<String>,
    pub confiance: f64,
    pub blocs_divergents: Vec<BlocDivergent>,
}
