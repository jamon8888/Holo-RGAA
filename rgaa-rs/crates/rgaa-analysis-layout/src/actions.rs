//! Simulated-action guards shared with the PRD's F-SEC rule (P-07 §6.3).
//!
//! The forbidden-action list is not duplicated here: P-08 introduced the
//! canonical list in `rgaa_analysis::actions`, which both plugins read. Only
//! the P-07-specific bound on simulated widgets stays local.

/// Actions on submit / pay / delete elements are never simulated: a click on
/// such an element could change real data. The canonical list lives in the
/// `rgaa-analysis` contract (shared with P-08) and is re-exported so that
/// existing `rgaa_analysis_layout::FORBIDDEN_ACTION_WORDS` callers keep working.
pub use rgaa_analysis::actions::FORBIDDEN_ACTION_WORDS;

/// Default bound on simulated widgets per page (spec §6.3), configurable via
/// [`crate::LayoutPlugin::with_max_actions`].
pub const DEFAULT_MAX_ACTIONS: usize = 20;
