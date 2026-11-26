//! P-10.a — sidecar lecteur d'écran (NVDA + Speech Logger, §1.2 / §4 de la
//! recherche `docs/research/lecteur-ecran-sidecar.md`).
//!
//! Le binaire parle le JSON-RPC stdio de P-00 §7 bis. Aucun code NVDA n'est
//! lié ni copié : le sidecar lit uniquement un fichier journal UTF-8 produit
//! par l'add-on GPL, livré à part.
//!
//! # Horloge
//!
//! L'horodatage est posé **à la réception** (`horodatage_type: "reception"`) :
//! Speech Logger n'horodate pas chaque énoncé (§1.2). L'horloge est injectée
//! via [`Horloge`] afin que les tests soient déterministes, sans `sleep`.

pub mod correlation;
pub mod journal;
pub mod protocole;

use chrono::{DateTime, Utc};

/// Source de temps injectable : `[HorlogeSysteme]` en production, horloge
/// fixe dans les tests.
pub trait Horloge {
    /// Instant attribué à un énoncé au moment de sa lecture.
    fn maintenant(&self) -> DateTime<Utc>;
}

impl<H: Horloge + ?Sized> Horloge for Box<H> {
    fn maintenant(&self) -> DateTime<Utc> {
        (**self).maintenant()
    }
}

/// Horloge réelle, utilisée par le binaire en audit.
#[derive(Debug, Clone, Copy, Default)]
pub struct HorlogeSysteme;

impl Horloge for HorlogeSysteme {
    fn maintenant(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

/// Horloge constante, pour les tests unitaires sans `sleep`.
#[derive(Debug, Clone, Copy)]
pub struct HorlogeFixe(pub DateTime<Utc>);

impl Horloge for HorlogeFixe {
    fn maintenant(&self) -> DateTime<Utc> {
        self.0
    }
}
