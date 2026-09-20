//! Export guardrails: refuse legally indefensible documents.
//!
//! Every check maps to a decided rule: 5-page sample with small-site
//! exception, mandatory feedback contact, 4 proofs per non-conformity,
//! complete derogations. Nothing here changes a legal status; it only
//! blocks the export of an undefendable file.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use rgaa_core::{AuditBundle, CriterionStatus};

use crate::ReportError;

/// Minimum sample pages (home, contact, legal, accessibility, sitemap).
pub const ECHANTILLON_MIN: usize = 5;

/// Feedback contact of the declaration (UE 2018/1523, mandatory everywhere).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
pub struct Contact {
    /// Channel kind: email, form, phone.
    pub canal: String,
    pub email: Option<String>,
    pub telephone: Option<String>,
    pub formulaire: Option<String>,
    pub delai_reponse: Option<String>,
}

/// One disproportionate-burden derogation: all three fields or blocked.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
pub struct Derogation {
    pub contenu: String,
    pub motif: String,
    pub alternative: String,
    pub date_reexamen: String,
}

/// Content outside the obligation (UE off-scope list + national extensions).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
pub struct ContenuNonSoumis {
    pub categorie: String,
    pub justification: String,
    pub alternative: Option<String>,
}

/// One sampled page of the regulatory sample.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
pub struct PageEchantillon {
    pub id: String,
    #[serde(rename = "type")]
    pub page_type: String,
    pub url: String,
}

/// Declaration inputs beside the audit bundle. `extensions` holds one object
/// per country and never duplicates the core.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, JsonSchema)]
pub struct ExportPack {
    pub pages: Vec<PageEchantillon>,
    pub contact: Option<Contact>,
    #[serde(default)]
    pub derogations: Vec<Derogation>,
    #[serde(default)]
    pub non_soumis: Vec<ContenuNonSoumis>,
    /// Justification when the site holds fewer than [`ECHANTILLON_MIN`] pages.
    #[serde(default)]
    pub site_petit_justification: Option<String>,
    #[serde(default)]
    pub extensions: serde_json::Value,
}

fn vide(value: &str) -> bool {
    value.trim().is_empty()
}

/// Feedback destination selected by the channel kind: `email` reads the
/// address, `telephone` the number, `formulaire` the form URL. Any other
/// kind accepts the first non-blank destination as fallback.
pub fn destination_contact(contact: &Contact) -> Option<&str> {
    match contact.canal.trim() {
        "email" => contact.email.as_deref().filter(|d| !vide(d)),
        "telephone" => contact.telephone.as_deref().filter(|d| !vide(d)),
        "formulaire" => contact.formulaire.as_deref().filter(|d| !vide(d)),
        _ => [
            contact.email.as_deref(),
            contact.telephone.as_deref(),
            contact.formulaire.as_deref(),
        ]
        .into_iter()
        .flatten()
        .find(|d| !vide(d)),
    }
}

/// Refuses the export when any guardrail fails. The first failure wins so
/// the caller fixes issues one precise message at a time.
pub fn validate_export(bundle: &AuditBundle, pack: &ExportPack) -> Result<(), ReportError> {
    let petit = pack
        .site_petit_justification
        .as_deref()
        .is_some_and(|j| !vide(j));
    if pack.pages.len() < ECHANTILLON_MIN && !petit {
        return Err(ReportError::invalid_input(format!(
            "échantillon insuffisant : {} pages, {} minimum (ou justification site petit)",
            pack.pages.len(),
            ECHANTILLON_MIN
        )));
    }
    let contact = pack.contact.as_ref().ok_or_else(|| {
        ReportError::invalid_input("contact de retour d'information manquant".to_string())
    })?;
    if destination_contact(contact).is_none() {
        return Err(ReportError::invalid_input(
            "destination du contact de retour d'information manquante".to_string(),
        ));
    }

    let findings = bundle
        .findings
        .iter()
        .chain(bundle.pages.iter().flat_map(|page| page.findings.iter()));
    for finding in findings {
        if !matches!(
            finding.status,
            CriterionStatus::Fail | CriterionStatus::Error
        ) {
            continue;
        }
        if finding.description.as_deref().is_some_and(|d| !vide(d))
            && !vide(&finding.url)
            && !finding.evidence.is_empty()
            && finding.remediation.as_deref().is_some_and(|r| !vide(r))
        {
            continue;
        }
        return Err(ReportError::invalid_input(format!(
            "preuve incomplète pour la non-conformité '{}' : intitulé, URL, DOM/capture et recommandation exigés",
            finding.id
        )));
    }

    for derogation in &pack.derogations {
        if vide(&derogation.contenu)
            || vide(&derogation.motif)
            || vide(&derogation.alternative)
            || vide(&derogation.date_reexamen)
        {
            return Err(ReportError::invalid_input(format!(
                "dérogation incomplète pour '{}' : motif, alternative et réexamen exigés",
                derogation.contenu
            )));
        }
    }
    Ok(())
}

/// JSON Schema of the pivot payload (core aligned to UE 2018/1523 plus
/// per-country `extensions`). The schema is derived, never hand-maintained.
#[must_use]
pub fn schema_export_pack() -> serde_json::Value {
    serde_json::to_value(schemars::schema_for!(ExportPack)).unwrap_or(serde_json::Value::Null)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rgaa_core::{AuditConfig, Finding};

    fn proved(url: &str) -> Finding {
        let mut finding = Finding::new("finding-1");
        finding.rule = "rgaa-1.1".into();
        finding.url = url.into();
        finding.target = "#main".into();
        finding.status = CriterionStatus::Fail;
        finding.description = Some("missing alternative text".into());
        finding.remediation = Some("add alt text".into());
        finding.evidence = vec![rgaa_core::EvidenceRef::new("dom", "abc123")];
        finding
    }

    fn pack() -> ExportPack {
        ExportPack {
            pages: (1..=5)
                .map(|i| PageEchantillon {
                    id: format!("P{i:02}"),
                    page_type: "Page".into(),
                    url: format!("https://example.test/{i}"),
                })
                .collect(),
            contact: Some(Contact {
                canal: "email".into(),
                email: Some("aide@example.test".into()),
                telephone: None,
                formulaire: None,
                delai_reponse: None,
            }),
            derogations: vec![Derogation {
                contenu: "carte interactive".into(),
                motif: "refonte chiffrée".into(),
                alternative: "tableau sur demande".into(),
                date_reexamen: "2027-09-19".into(),
            }],
            non_soumis: Vec::new(),
            site_petit_justification: None,
            extensions: serde_json::json!({}),
        }
    }

    fn bundle_with(findings: Vec<Finding>) -> AuditBundle {
        let mut bundle =
            AuditBundle::new("audit-1", "https://example.test", AuditConfig::default());
        bundle.findings = findings;
        bundle
    }

    #[test]
    fn pack_complet_passe() {
        let bundle = bundle_with(vec![proved("https://example.test/contact")]);
        assert!(validate_export(&bundle, &pack()).is_ok());
    }

    #[test]
    fn contact_manquant_bloque() {
        let mut pack = pack();
        pack.contact = None;
        let bundle = bundle_with(Vec::new());
        assert!(validate_export(&bundle, &pack).is_err());
    }

    #[test]
    fn echantillon_insuffisant_bloque_sauf_site_petit() {
        let mut pack = pack();
        pack.pages.truncate(2);
        let bundle = bundle_with(Vec::new());
        assert!(validate_export(&bundle, &pack).is_err());
        pack.site_petit_justification = Some("site vitrine de 2 pages".into());
        assert!(validate_export(&bundle, &pack).is_ok());
    }

    #[test]
    fn preuve_manquante_bloque() {
        let mut bare = proved("https://example.test/contact");
        bare.evidence.clear();
        bare.remediation = None;
        let bundle = bundle_with(vec![bare]);
        let error = validate_export(&bundle, &pack()).unwrap_err();
        assert!(error.to_string().contains("finding-1"));
    }

    #[test]
    fn derogation_incomplete_bloque() {
        let mut pack = pack();
        pack.derogations[0].alternative.clear();
        let bundle = bundle_with(Vec::new());
        assert!(validate_export(&bundle, &pack).is_err());
    }

    #[test]
    fn schema_derive_noyau_et_extensions() {
        let schema = schema_export_pack();
        let props = &schema["properties"];
        for key in [
            "pages",
            "contact",
            "derogations",
            "non_soumis",
            "extensions",
        ] {
            assert!(
                props.get(key).is_some(),
                "propriété {key} absente du schéma"
            );
        }
    }

    #[test]
    fn serde_rejette_contact_sans_canal() {
        let raw = serde_json::json!({
            "pages": [],
            "contact": { "email": "aide@example.test" },
        });
        assert!(serde_json::from_value::<ExportPack>(raw).is_err());
    }
}
