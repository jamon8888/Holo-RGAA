//! Aggregator that builds the analysis plugin registry for a binary (P-00 §4).
//!
//! `rgaa-analysis` owns the contract and cannot depend on the crates that
//! implement it. This crate depends on each native plugin behind a feature and
//! assembles the linked ones, followed by the discovered sidecars, into one
//! explicit [`Registry`]. `rgaa-cli` and `rgaa-mcp` use this function instead of
//! a feature-driven global registry.

use rgaa_analysis::Registry;

/// Builds the registry of every plugin linked into this binary.
///
/// Plugins are sorted by stable identifier by [`Registry::builder`]; a
/// duplicate identifier is reported as an error rather than silently making
/// `run_analysis_plugin` ambiguous.
///
/// # Errors
///
/// Returns [`rgaa_analysis::RgaaError::Analysis`] when two linked plugins
/// declare the same identifier.
pub fn default_registry() -> rgaa_analysis::Result<Registry> {
    let mut builder = Registry::builder();

    #[cfg(feature = "plugin-echo")]
    {
        builder = builder.with(rgaa_analysis::echo_plugin());
    }
    // P-06 links `rgaa-analysis-lang` here under `feature = "plugin-lang"`.

    for sidecar in rgaa_analysis::sidecar::discover_default() {
        builder = builder.with(Box::new(sidecar));
    }

    builder.build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(feature = "plugin-echo")]
    fn default_registry_contains_the_echo_plugin() {
        let registry = default_registry().expect("registry");
        let ids: Vec<&str> = registry.iter().map(|plugin| plugin.id()).collect();
        assert!(ids.contains(&"echo"), "{ids:?}");
        let mut sorted = ids.clone();
        sorted.sort_unstable();
        assert_eq!(ids, sorted, "the registry must be deterministic");
    }

    #[test]
    #[cfg(not(feature = "plugin-echo"))]
    fn default_registry_can_be_empty() {
        let registry = default_registry().expect("registry");
        assert!(registry.is_empty());
    }
}
