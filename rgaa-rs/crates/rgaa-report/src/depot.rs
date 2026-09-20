//! Filing targets: where each declaration canonically lives.
//!
//! Only derivable canonical URLs are built here (Danish WAS pattern,
//! Portuguese `/acessibilidade` suffix). Central registries needing manual
//! entry (Italy, Netherlands) and local publications return `None`:
//! fetching or filing into portals is a later network ticket, not this one.

use crate::packs::Pays;

/// Canonical public URL of the declaration, when derivable from a slug.
#[must_use]
pub fn url_canonique(pays: Pays, identifiant: &str) -> Option<String> {
    match pays {
        Pays::Dk => Some(format!("https://was.digst.dk/{identifiant}")),
        Pays::Pt => Some(format!("https://{identifiant}/acessibilidade")),
        Pays::Fr
        | Pays::De
        | Pays::Es
        | Pays::It
        | Pays::Be
        | Pays::Nl
        | Pays::Lu
        | Pays::At
        | Pays::Ie
        | Pays::Se => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn motifs_derivables_dk_pt() {
        assert_eq!(
            url_canonique(Pays::Dk, "stpk-dk").as_deref(),
            Some("https://was.digst.dk/stpk-dk")
        );
        assert_eq!(
            url_canonique(Pays::Pt, "www.presidencia.pt").as_deref(),
            Some("https://www.presidencia.pt/acessibilidade")
        );
    }

    #[test]
    fn registres_et_publications_locales_sans_url() {
        assert_eq!(url_canonique(Pays::It, "x"), None);
        assert_eq!(url_canonique(Pays::Nl, "9751"), None);
        assert_eq!(url_canonique(Pays::Fr, "service"), None);
    }
}
