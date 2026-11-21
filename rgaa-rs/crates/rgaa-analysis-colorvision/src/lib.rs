//! P-02 — native colour-vision plugin for RGAA 3.1.
//!
//! The plugin simulates four visual deficiencies in linear sRGB, measures the
//! ΔE2000 loss between colour-coded elements and their neighbours, and emits
//! [`rgaa_analysis::Preuve`]s with measurements and vignettes. It never emits a
//! status: the orchestrator turns proofs into `review` (R-P1, R-P2).
//!
//! The WCAG contrast formula lives here as [`color::contrast_ratio`], so P-01
//! and Obscura can share a single implementation.

pub mod candidates;
pub mod color;
pub mod matrices;
pub mod plugin;
pub mod simulate;

pub use candidates::{detect_candidates, CandidateKind, DomCandidate, Series};
pub use color::{
    contrast_ratio, delta_e_2000, delta_e_2000_lab, delta_e_ciede2000_lab, parse_color,
    relative_luminance, Rgb8,
};
pub use matrices::{
    brettel_tritan, machado, matrices_document, simulate_rgb, CvdKind, Matrix3, ALL_KINDS,
    MATRICES_JSON,
};
pub use plugin::{
    images_for_model, AvisModele, ColorVisionOptions, ColorVisionPlugin, Mesures, SimulationMesure,
    COLORVISION_PLUGIN, TESTS,
};
