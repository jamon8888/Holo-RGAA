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
        init_tracing();
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

/// Autorise Obscura à joindre le serveur de fixtures en loopback.
///
/// `std::env::set_var` mute l'environnement du processus entier ; appelée depuis
/// plusieurs fils de test, pendant que d'autres lisent l'environnement ou lancent
/// Obscura, c'est une course de données. `Once` fait de l'écriture un événement
/// unique : tout test qui passe ici attend qu'elle soit faite avant de continuer.
///
/// Sous `cargo nextest`, chaque test tourne dans son propre processus et la course
/// n'existe pas ; sous `cargo test`, les tests d'un même binaire partagent le
/// processus, et c'est là que ça compte.
pub fn allow_private_network() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| std::env::set_var("OBSCURA_ALLOW_PRIVATE_NETWORK", "1"));
}

/// Installe un abonné `tracing` pour la durée du processus de test.
///
/// Le pont consigne ses échecs par `warn!` — par exemple `axe-core failed` avec l'erreur —
/// puis les omet du résultat du lot. Sans abonné ces événements sont perdus : le test
/// échoue sur « pas de résultat pour <url> » sans la raison, et il faut un second cycle de CI
/// pour la découvrir. Niveau `warn` par défaut, surchargeable par `RUST_LOG`. Idempotent :
/// `try_init` ignore un abonné déjà posé.
fn init_tracing() {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn"));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_test_writer()
        .try_init();
}
