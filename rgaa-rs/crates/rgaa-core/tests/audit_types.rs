//! Audit-type lists come from the official DesignGouv pages and must stay
//! versioned data under `data/rgaa-4.1.2/`, never hard-coded in the code.
//!
//! Expected values below were transcribed from
//! https://design.numerique.gouv.fr/outils/audit-rapide/ and
//! /outils/audit-complementaire/ (the same lists DISIC/Ara carries in
//! `confiture-rest-api/src/audits/criteria.ts`).

use rgaa_core::{AuditType, RgaaCatalog};

/// The 25 criteria of the official "audit rapide", in DesignGouv order.
const RAPIDE_OFFICIEL: &[&str] = &[
    "1.1", "3.1", "4.1", "4.10", "5.3", "5.7", "6.1", "6.2", "7.1", "7.3", "8.3", "8.4", "8.5",
    "9.1", "10.3", "10.6", "10.7", "11.1", "11.2", "11.5", "11.6", "11.9", "11.10", "12.8", "12.9",
];

/// The 25 *additional* criteria of the official "audit complémentaire".
const COMPLEMENTAIRE_OFFICIEL: &[&str] = &[
    "1.3", "1.5", "1.6", "1.7", "4.2", "4.4", "4.8", "4.9", "5.4", "5.6", "7.2", "8.2", "8.6",
    "8.10", "10.2", "10.8", "10.9", "10.10", "13.1", "13.3", "13.4", "13.5", "13.6", "13.7",
    "13.8",
];

#[test]
fn rapide_carries_exactly_the_25_official_criteria() {
    let ids = AuditType::Rapide.criterion_ids();
    assert_eq!(ids.len(), 25, "the official rapid audit has 25 criteria");
    assert_eq!(ids, RAPIDE_OFFICIEL);
}

#[test]
fn complementaire_is_the_25_official_additions_on_top_of_rapide() {
    let rapide = AuditType::Rapide.criterion_ids();
    let complementaire = AuditType::Complementaire.criterion_ids();

    assert_eq!(
        complementaire.len(),
        50,
        "rapide + complémentaire = 50 criteria"
    );
    for id in &rapide {
        assert!(
            complementaire.contains(id),
            "complémentaire must contain rapid criterion {id}"
        );
    }
    // No criterion is counted twice, and the additions are exactly the official 25.
    let mut additions: Vec<&str> = complementaire
        .iter()
        .map(String::as_str)
        .filter(|id| !rapide.iter().any(|r| r == id))
        .collect();
    additions.sort_by_key(|id| {
        let mut parts = id.split('.');
        let theme: u32 = parts.next().unwrap_or_default().parse().unwrap_or_default();
        let crit: u32 = parts.next().unwrap_or_default().parse().unwrap_or_default();
        (theme, crit)
    });
    let mut officiel = COMPLEMENTAIRE_OFFICIEL.to_vec();
    officiel.sort_by_key(|id| {
        let mut parts = id.split('.');
        let theme: u32 = parts.next().unwrap_or_default().parse().unwrap_or_default();
        let crit: u32 = parts.next().unwrap_or_default().parse().unwrap_or_default();
        (theme, crit)
    });
    assert_eq!(additions, officiel);
}

#[test]
fn complet_covers_every_catalog_criterion() {
    let ids = AuditType::Complet.criterion_ids();
    assert_eq!(ids.len(), 106);
    assert_eq!(RgaaCatalog::count(), 106);
}

#[test]
fn every_listed_id_exists_in_the_catalog() {
    for audit_type in AuditType::ALL {
        for id in audit_type.criterion_ids() {
            assert!(
                RgaaCatalog::by_id(&id).is_some(),
                "{audit_type:?} lists {id}, which is absent from criteres.json"
            );
        }
    }
}

#[test]
fn audit_type_serde_names_are_stable_and_default_to_complet() {
    for (value, json) in [
        (AuditType::Rapide, r#""rapide""#),
        (AuditType::Complementaire, r#""complementaire""#),
        (AuditType::Complet, r#""complet""#),
    ] {
        assert_eq!(serde_json::to_string(&value).unwrap(), json);
        assert_eq!(serde_json::from_str::<AuditType>(json).unwrap(), value);
    }
    #[derive(serde::Deserialize)]
    struct Holder {
        #[serde(default)]
        audit_type: AuditType,
    }
    let holder: Holder = serde_json::from_str("{}").unwrap();
    assert_eq!(holder.audit_type, AuditType::Complet);
    assert!(serde_json::from_str::<AuditType>(r#""inconnu""#).is_err());
}

#[test]
fn a_rapide_audit_never_allows_a_conformity_declaration() {
    assert!(!AuditType::Rapide.allows_conformity_declaration());
    assert!(AuditType::Complementaire.allows_conformity_declaration());
    assert!(AuditType::Complet.allows_conformity_declaration());
}
