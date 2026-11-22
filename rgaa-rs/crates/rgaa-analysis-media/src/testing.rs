//! Shared guard for tests that need a local model (P-04 PR 2).
//!
//! Mirrors `rgaa_test_corpus::e2e::enabled`: a test that returns early is
//! otherwise indistinguishable from a test that passed. Model tests are gated
//! by the *presence* of the model in `$RGAA_MODELS_DIR`, and the CI job sets
//! `RGAA_MODEL_TESTS_REQUIRED=1` so losing the opt-in fails the job instead of
//! reporting a green run that verified nothing.

use rgaa_analysis::ModelSpec;

/// Returns `true` only when the model test must actually run.
///
/// # Panics
///
/// Panics in CI (`RGAA_MODEL_TESTS_REQUIRED=1`) when the opt-in is missing, or
/// when the opt-in is present but the declared model is absent from the cache.
#[must_use]
pub fn model_test_enabled(test_name: &str, spec: &ModelSpec) -> bool {
    let run = std::env::var("RUN_MODEL_TESTS").ok().as_deref() == Some("1");
    if run {
        assert!(
            rgaa_analysis::models::is_present(spec),
            "{test_name} : RUN_MODEL_TESTS=1 mais le modèle {} est absent — lancez `rgaa models pull media`",
            spec.nom
        );
        return true;
    }
    assert!(
        std::env::var("RGAA_MODEL_TESTS_REQUIRED")
            .ok()
            .as_deref()
            != Some("1"),
        "{test_name} : RGAA_MODEL_TESTS_REQUIRED=1 est posé mais RUN_MODEL_TESTS={run:?} — ce test \
         sortirait sans rien affirmer. Poser RUN_MODEL_TESTS=1, ou retirer RGAA_MODEL_TESTS_REQUIRED."
    );
    eprintln!("{test_name} ignoré (poser RUN_MODEL_TESTS=1 pour l'activer)");
    false
}
