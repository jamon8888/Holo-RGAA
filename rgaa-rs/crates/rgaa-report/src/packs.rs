//! National packs: one country, one legal identity.
//!
//! A pack carries only what the law of its country imposes: footer mention,
//! declaration template identity and enforcement contact. Anything shared
//! stays in the core; [`Pays`] is the single selector.

/// The twelve covered countries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pays {
    Fr,
    De,
    Es,
    It,
    Be,
    Nl,
    Lu,
    Pt,
    At,
    Ie,
    Se,
    Dk,
}

impl Pays {
    /// ISO code used as `extensions` key and template selector.
    #[must_use]
    pub fn code(self) -> &'static str {
        match self {
            Self::Fr => "fr",
            Self::De => "de",
            Self::Es => "es",
            Self::It => "it",
            Self::Be => "be",
            Self::Nl => "nl",
            Self::Lu => "lu",
            Self::Pt => "pt",
            Self::At => "at",
            Self::Ie => "ie",
            Self::Se => "se",
            Self::Dk => "dk",
        }
    }
}

/// Footer mention required on the site home (RGAA 1.8.1).
#[must_use]
pub fn mention_fr(taux_global: f64, audit_incomplet: bool) -> &'static str {
    if audit_incomplet || taux_global < 50.0 {
        "Accessibilité : non conforme"
    } else if taux_global >= 100.0 {
        "Accessibilité : totalement conforme"
    } else {
        "Accessibilité : partiellement conforme"
    }
}

/// Legal status sentence of the French declaration header.
#[must_use]
pub fn etat_fr(etat_conformite: &str) -> &'static str {
    match etat_conformite {
        "totale" => "totalement conforme",
        "partielle" => "partiellement conforme",
        _ => "non conforme",
    }
}

/// Fixed enforcement block: Défenseur des droits.
pub const RECOURS_FR: &str = "Défenseur des droits (formulaire.defenseurdesdroits.fr, Libre réponse 71120, 75342 Paris CEDEX 07)";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mentions_fr_suivent_seuils_et_nt() {
        assert_eq!(
            mention_fr(100.0, false),
            "Accessibilité : totalement conforme"
        );
        assert_eq!(
            mention_fr(80.0, false),
            "Accessibilité : partiellement conforme"
        );
        assert_eq!(mention_fr(49.9, false), "Accessibilité : non conforme");
        assert_eq!(mention_fr(100.0, true), "Accessibilité : non conforme");
    }

    #[test]
    fn codes_pays_stables() {
        assert_eq!(Pays::Fr.code(), "fr");
        assert_eq!(Pays::De.code(), "de");
        assert_eq!(Pays::Dk.code(), "dk");
    }
}
