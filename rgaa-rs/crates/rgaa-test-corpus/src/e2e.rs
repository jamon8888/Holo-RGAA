//! Politique partagée des tests E2E navigateur du workspace.
//!
//! Ces tests n'affirment rien quand ils sortent en tête : sans navigateur, un
//! `return` précoce est indiscernable d'un succès. La politique vit ici, en un
//! seul endroit et pour les deux crates qui en ont (`rgaa-obscura` et
//! `rgaa-orchestrator`), pour que « ne jamais passer en silence » soit une
//! propriété du harnais et non une convention recopiée dans chaque fichier.

/// Indique si ce run doit exercer le navigateur.
///
/// `RUN_E2E=1` l'active. `RGAA_E2E_REQUIRED=1` — posé par le job CI
/// `E2E (navigateur)` — rend l'absence de `RUN_E2E=1` **fatale** : si la CI
/// perd la variable, le job échoue au lieu de rapporter un vert sur un test
/// qui n'a rien vérifié. C'est exactement le défaut que cette suite corrige
/// dans le produit (un `pass` sur le silence) ; il n'a pas sa place dans son
/// propre harnais.
///
/// # Panics
///
/// Panique quand `RGAA_E2E_REQUIRED=1` est posé sans `RUN_E2E=1`.
#[must_use]
pub fn enabled(test_name: &str) -> bool {
    let run_e2e = std::env::var("RUN_E2E").ok();
    if run_e2e.as_deref() == Some("1") {
        return true;
    }
    assert!(
        std::env::var("RGAA_E2E_REQUIRED").ok().as_deref() != Some("1"),
        "{test_name} : RGAA_E2E_REQUIRED=1 est posé mais RUN_E2E={run_e2e:?} — ce test \
         sortirait sans rien affirmer. Poser RUN_E2E=1, ou retirer RGAA_E2E_REQUIRED."
    );
    eprintln!("{test_name} ignoré (poser RUN_E2E=1 pour l'activer)");
    false
}
