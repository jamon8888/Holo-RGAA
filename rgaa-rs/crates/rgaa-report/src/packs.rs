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

/// Legal identity of one country: status regime, language, enforcement.
/// Sources: UE 2018/1523 model plus national research; contacts are the
/// monitoring or enforcement bodies, never invented.
pub struct PackPays {
    pub pays: Pays,
    /// BCP-47 language of the declaration.
    pub langue: &'static str,
    /// How the legal status is set (computed rate vs reviewer mapping).
    pub regime_statut: &'static str,
    /// Enforcement or monitoring body name.
    pub recours_nom: &'static str,
    /// Enforcement contact: URL or email as published.
    pub recours_contact: &'static str,
}

/// The twelve packs, indexed by [`Pays`] order below in [`pack`].
pub const PACKS: [PackPays; 12] = [
    PackPays {
        pays: Pays::Fr,
        langue: "fr",
        regime_statut: "taux C/(C+NC), seuils 100/50",
        recours_nom: "Défenseur des droits",
        recours_contact: "https://formulaire.defenseurdesdroits.fr/",
    },
    PackPays {
        pays: Pays::De,
        langue: "de",
        regime_statut: "triptyque UE qualitatif, sans %",
        recours_nom: "Schlichtungsstelle nach § 16 BGG",
        recours_contact: "https://www.schlichtungsstelle-bgg.de",
    },
    PackPays {
        pays: Pays::Es,
        langue: "es",
        regime_statut: "triptyque UE qualitatif, sans %",
        recours_nom: "Unidad responsable de accesibilidad",
        recours_contact: "https://administracionelectronica.gob.es/",
    },
    PackPays {
        pays: Pays::It,
        langue: "it",
        regime_statut: "triptyque UE qualitatif, sans %",
        recours_nom: "Difensore civico digitale",
        recours_contact: "https://www.agid.gov.it/",
    },
    PackPays {
        pays: Pays::Be,
        langue: "fr",
        regime_statut: "triptyque UE qualitatif, sans %",
        recours_nom: "Médiateur fédéral",
        recours_contact: "contact@mediateurfederal.be",
    },
    PackPays {
        pays: Pays::Nl,
        langue: "nl",
        regime_statut: "système national A–E mappé par le relecteur",
        recours_nom: "College voor de Rechten van de Mens",
        recours_contact: "https://www.logius.nl/",
    },
    PackPays {
        pays: Pays::Lu,
        langue: "fr",
        regime_statut: "triptyque UE qualitatif, sans %",
        recours_nom: "Service information et presse (SIP)",
        recours_contact: "accessibilite@sip.etat.lu",
    },
    PackPays {
        pays: Pays::Pt,
        langue: "pt",
        regime_statut: "seuils nationaux AccessMonitor mappés par le relecteur",
        recours_nom: "AMA (Agência para a Modernização Administrativa)",
        recours_contact: "https://www.acessibilidade.gov.pt",
    },
    PackPays {
        pays: Pays::At,
        langue: "de",
        regime_statut: "triptyque UE qualitatif, sans %",
        recours_nom: "FFG Beschwerdestelle",
        recours_contact: "https://www.sozialministeriumservice.gv.at/",
    },
    PackPays {
        pays: Pays::Ie,
        langue: "en",
        regime_statut: "triptyque UE qualitatif, sans %",
        recours_nom: "Ombudsman",
        recours_contact: "https://nda.ie",
    },
    PackPays {
        pays: Pays::Se,
        langue: "sv",
        regime_statut: "triptyque UE qualitatif, sans %",
        recours_nom: "DIGG",
        recours_contact: "https://www.digg.se",
    },
    PackPays {
        pays: Pays::Dk,
        langue: "da",
        regime_statut: "triptyque UE qualitatif, sans %",
        recours_nom: "Digitaliseringsstyrelsen",
        recours_contact: "https://was.digst.dk",
    },
];

/// Returns the pack of `pays`.
#[must_use]
pub fn pack(pays: Pays) -> &'static PackPays {
    PACKS
        .iter()
        .find(|pack| pack.pays == pays)
        .expect("douze packs pour douze pays")
}

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

    #[test]
    fn douze_packs_contacts_recherches() {
        assert_eq!(PACKS.len(), 12);
        assert_eq!(
            pack(Pays::De).recours_contact,
            "https://www.schlichtungsstelle-bgg.de"
        );
        assert_eq!(
            pack(Pays::Fr).recours_contact,
            "https://formulaire.defenseurdesdroits.fr/"
        );
        assert_eq!(pack(Pays::Dk).recours_contact, "https://was.digst.dk");
        assert!(pack(Pays::Nl).regime_statut.contains("A–E"));
        assert!(pack(Pays::Pt).regime_statut.contains("AccessMonitor"));
    }
}
