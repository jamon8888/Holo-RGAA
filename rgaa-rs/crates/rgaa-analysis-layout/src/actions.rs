//! Simulated-action guards shared with the PRD's F-SEC rule (P-07 §6.3).

/// Actions on submit / pay / delete elements are never simulated: a click on
/// such an element could change real data. The French words are the canonical
/// list from the spec; the English forms cover the same intents on English
/// pages (P-06 decides the language, this plugin does not call a model).
pub const FORBIDDEN_ACTION_WORDS: &[&str] =
    &["soumettre", "payer", "supprimer", "submit", "pay", "delete"];

/// Default bound on simulated widgets per page (spec §6.3), configurable via
/// [`crate::LayoutPlugin::with_max_actions`].
pub const DEFAULT_MAX_ACTIONS: usize = 20;
