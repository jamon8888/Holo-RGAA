//! Pack governance: no owner, no document; no review, no document.
//!
//! Each pack carries its reviewer, version and review date. Generation is
//! refused for ownerless packs and for packs unreviewed for over a year,
//! unless a named, timestamped override traces the emergency. Today is
//! injected as ISO text so tests stay deterministic.

use chrono::NaiveDate;

use crate::packs::Pays;
use crate::ReportError;

/// A pack's governance record.
pub struct PackVersion {
    pub pays: Pays,
    pub version: &'static str,
    /// ISO review date, e.g. `"2026-09-19"`.
    pub date_revue: &'static str,
    /// Named reviewer; `None` means no generation for this country.
    pub owner: Option<&'static str>,
}

/// Days after which a pack is stale.
pub const PEREMPTION_JOURS: i64 = 365;

/// Emergency override: named, timestamped, motivated.
pub struct OverrideGouvernance<'a> {
    pub responsable: &'a str,
    pub horodatage: &'a str,
    pub motif: &'a str,
}

fn jours_depuis(date_revue: &str, aujourd_hui: &str) -> Result<i64, ReportError> {
    let revue = NaiveDate::parse_from_str(date_revue, "%Y-%m-%d").map_err(|_| {
        ReportError::invalid_input(format!("date de revue invalide : {date_revue}"))
    })?;
    let jour = NaiveDate::parse_from_str(aujourd_hui, "%Y-%m-%d").map_err(|_| {
        ReportError::invalid_input(format!("date du jour invalide : {aujourd_hui}"))
    })?;
    Ok((jour - revue).num_days())
}

/// Authorizes generation for `version` today, with an optional traced override.
pub fn autoriser_generation(
    version: &PackVersion,
    aujourd_hui: &str,
    contournement: Option<&OverrideGouvernance>,
) -> Result<(), ReportError> {
    let owner = version
        .owner
        .filter(|o| !o.trim().is_empty())
        .ok_or_else(|| {
            ReportError::invalid_input(format!(
                "aucun owner nommé pour {} : génération interdite",
                version.pays.code()
            ))
        })?;
    let _ = owner;
    let age = jours_depuis(version.date_revue, aujourd_hui)?;
    if age <= PEREMPTION_JOURS {
        return Ok(());
    }
    match contournement {
        Some(override_) if !override_.responsable.trim().is_empty() => Ok(()),
        _ => Err(ReportError::invalid_input(format!(
            "pack {} périmé depuis {} jours : revue exigée ou contournement nominatif horodaté",
            version.pays.code(),
            age - PEREMPTION_JOURS
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::guard::Contact;
    use crate::packs::{pack, PACKS};
    use crate::ue::{render_declaration_ue, DeclarationUeInput};

    fn version(pays: Pays, date_revue: &'static str, owner: Option<&'static str>) -> PackVersion {
        PackVersion {
            pays,
            version: "1",
            date_revue,
            owner,
        }
    }

    #[test]
    fn pack_frais_avec_owner_passe() {
        let v = version(Pays::Fr, "2026-09-19", Some("owner-fr"));
        assert!(autoriser_generation(&v, "2026-09-19", None).is_ok());
    }

    #[test]
    fn pack_perime_bloque_sauf_override_trace() {
        let v = version(Pays::De, "2024-01-01", Some("owner-de"));
        assert!(autoriser_generation(&v, "2026-09-19", None).is_err());
        let contournement = OverrideGouvernance {
            responsable: "dpo",
            horodatage: "2026-09-19T10:00:00Z",
            motif: "urgence publication",
        };
        assert!(autoriser_generation(&v, "2026-09-19", Some(&contournement)).is_ok());
        let anonyme = OverrideGouvernance {
            responsable: "  ",
            horodatage: "2026-09-19T10:00:00Z",
            motif: "urgence",
        };
        assert!(autoriser_generation(&v, "2026-09-19", Some(&anonyme)).is_err());
    }

    #[test]
    fn sans_owner_pas_de_document() {
        let v = version(Pays::It, "2026-09-19", None);
        assert!(autoriser_generation(&v, "2026-09-19", None).is_err());
    }

    #[test]
    fn dates_invalides_rejetees() {
        let v = version(Pays::Es, "19/09/2026", Some("owner-es"));
        assert!(autoriser_generation(&v, "2026-09-19", None).is_err());
    }

    #[test]
    fn golden_douze_packs_rendent_recours() {
        let contact = Contact {
            canal: "email".into(),
            email: Some("aide@example.test".into()),
            telephone: None,
            formulaire: None,
            delai_reponse: None,
        };
        for pack_courant in &PACKS {
            let input = DeclarationUeInput {
                pays: pack_courant.pays,
                service: "Service",
                organisme: "Organisme",
                statut: "partiellement conforme",
                non_conformites: &[],
                derogations: &[],
                contact: &contact,
                date_declaration: "2026-09-19",
                methode_evaluation: "auto",
            };
            let html = render_declaration_ue(&input).expect("rendu");
            let attendu = pack(pack_courant.pays);
            // Only DE has a validated translation; every other pack renders
            // the English template and is labeled `lang="en"` (see `ue.rs`).
            let langue_attendue = match pack_courant.pays {
                Pays::De => "lang=\"de\"",
                _ => "lang=\"en\"",
            };
            assert!(
                html.contains(langue_attendue),
                "langue {} pour {:?}",
                langue_attendue,
                pack_courant.pays
            );
            assert!(
                html.contains(attendu.recours_nom),
                "recours {}",
                attendu.recours_nom
            );
        }
    }
}
