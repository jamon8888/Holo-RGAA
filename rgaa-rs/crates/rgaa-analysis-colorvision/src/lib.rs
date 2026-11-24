//! P-02 — native colour-vision plugin for RGAA 3.1.
//!
//! The plugin simulates four visual deficiencies in linear sRGB, measures the
//! ΔE2000 loss between colour-coded elements and their neighbours, and emits
//! [`rgaa_analysis::Preuve`]s with measurements and vignettes. It never emits a
//! status: the orchestrator turns proofs into `review` (R-P1, R-P2).
//!
//! Colour maths (WCAG contrast, sRGB linearisation, CIEDE2000, CSS colour
//! parsing) comes from the single canonical module [`rgaa_analysis::color`].
//! This crate only owns what is specific to colour vision: the Machado and
//! Brettel matrices, the full-image simulations and the dHash comparison.

pub mod candidates;
pub mod matrices;
pub mod plugin;
pub mod simulate;

/// An 8-bit sRGB colour without alpha, in `R, G, B` order.
pub type Rgb8 = [u8; 3];

pub use candidates::{detect_candidates, CandidateKind, DomCandidate, Series};
pub use matrices::{
    brettel_tritan, machado, matrices_document, simulate_rgb, CvdKind, Matrix3, ALL_KINDS,
    MATRICES_JSON,
};
pub use plugin::{
    images_for_model, AvisModele, ColorVisionOptions, ColorVisionPlugin, Mesures, SimulationMesure,
    COLORVISION_PLUGIN, TESTS,
};
