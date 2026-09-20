//! French declaration: the 7 DINUM sections as semantic HTML.
//!
//! The template is parameterized, never hard-coded: every legal sentence
//! comes from the metrics and the pack. Other countries get their own
//! template from the UE 2018/1523 core.

use std::fmt::Write;

use rgaa_core::Finding;

use crate::guard::{Contact, ContenuNonSoumis, Derogation, PageEchantillon};
use crate::packs::{etat_fr, RECOURS_FR};
use crate::{ReportError, SiteMetrics};

/// One proven non-conformity, built only from a fully proved finding.
pub struct NcEntry {
    pub intitule: String,
    pub url: String,
    pub preuve: String,
    pub recommandation: String,
}

impl NcEntry {
    /// Returns `None` when the finding lacks any of the 4 proofs.
    pub fn from_finding(finding: &Finding) -> Option<Self> {
        let intitule = finding
            .description
            .clone()
            .filter(|d| !d.trim().is_empty())?;
        let recommandation = finding
            .remediation
            .clone()
            .filter(|r| !r.trim().is_empty())?;
        if finding.url.trim().is_empty() || finding.evidence.is_empty() {
            return None;
        }
        let preuve = finding
            .evidence
            .iter()
            .map(|e| {
                e.location.clone().unwrap_or_else(|| {
                    format!("{}:{}", e.kind, e.hash.chars().take(8).collect::<String>())
                })
            })
            .collect::<Vec<_>>()
            .join("; ");
        Some(Self {
            intitule,
            url: finding.url.clone(),
            preuve,
            recommandation,
        })
    }
}

/// Everything the French template needs, nothing it doesn't.
pub struct DeclarationFrInput<'a> {
    pub service: &'a str,
    pub organisme: &'a str,
    pub metrics: &'a SiteMetrics,
    pub conformes: usize,
    pub non_conformes: usize,
    pub non_applicables: usize,
    pub non_testes: usize,
    pub non_conformites: &'a [NcEntry],
    pub derogations: &'a [Derogation],
    pub non_soumis: &'a [ContenuNonSoumis],
    pub contact: &'a Contact,
    pub date_declaration: &'a str,
    pub technologies: &'a [String],
    pub pages: &'a [PageEchantillon],
    pub environnement: &'a str,
    pub schema_pluriannuel_url: Option<&'a str>,
    pub plan_action_url: Option<&'a str>,
}

fn echappe(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Renders the French declaration, ready to publish on `/accessibilite`.
/// Fails when the contact channel is missing: a declaration without
/// feedback is invalid on its face.
pub fn render_declaration_fr(input: &DeclarationFrInput) -> Result<String, ReportError> {
    if input.contact.canal.trim().is_empty() {
        return Err(ReportError::invalid_input(
            "contact de retour d'information manquant".to_string(),
        ));
    }
    let mut out = String::new();
    let _ = writeln!(out, "<!DOCTYPE html>");
    let _ = writeln!(out, "<html lang=\"fr\">");
    let _ = writeln!(out, "<head><meta charset=\"utf-8\">");
    let _ = writeln!(
        out,
        "<title>Déclaration d'accessibilité — {}</title></head>",
        echappe(input.service)
    );
    let _ = writeln!(out, "<body><main>");
    let _ = writeln!(out, "<h1>Déclaration d'accessibilité</h1>");

    let _ = writeln!(out, "<h2>1. Engagement</h2>");
    let _ = writeln!(
        out,
        "<p><strong>{}</strong> s'engage à rendre son service <em>{}</em> accessible conformément à l'article 47 de la loi n° 2005-102 du 11 février 2005.</p>",
        echappe(input.organisme),
        echappe(input.service)
    );

    let _ = writeln!(out, "<h2>2. État de conformité</h2>");
    let _ = writeln!(
        out,
        "<p><strong>{}</strong> est <strong>{}</strong> avec le Référentiel Général d'Amélioration de l'Accessibilité (RGAA), version 4.1.2. Le taux de conformité s'élève à <strong>{:.1} %</strong> ({} critères conformes, {} non conformes, {} non applicables, {} non testés).</p>",
        echappe(input.service),
        etat_fr(&input.metrics.etat_conformite),
        input.metrics.taux_global,
        input.conformes,
        input.non_conformes,
        input.non_applicables,
        input.non_testes
    );

    let _ = writeln!(
        out,
        "<h2>3. Résultats des tests et contenus non accessibles</h2>"
    );
    let _ = writeln!(out, "<h3>Non-conformités</h3>");
    if input.non_conformites.is_empty() {
        let _ = writeln!(out, "<p>Aucune non-conformité relevée.</p>");
    } else {
        let _ = writeln!(out, "<ul>");
        for nc in input.non_conformites {
            let _ = writeln!(
                out,
                "<li>{} — {} (preuve : {} ; recommandation : {}).</li>",
                echappe(&nc.intitule),
                echappe(&nc.url),
                echappe(&nc.preuve),
                echappe(&nc.recommandation)
            );
        }
        let _ = writeln!(out, "</ul>");
    }
    let _ = writeln!(out, "<h3>Dérogations pour charge disproportionnée</h3>");
    if input.derogations.is_empty() {
        let _ = writeln!(out, "<p>Aucune dérogation.</p>");
    } else {
        let _ = writeln!(out, "<ul>");
        for derogation in input.derogations {
            let _ = writeln!(
                out,
                "<li>{} (motif : {}) : alternative = {} ; réexamen le {}.</li>",
                echappe(&derogation.contenu),
                echappe(&derogation.motif),
                echappe(&derogation.alternative),
                echappe(&derogation.date_reexamen)
            );
        }
        let _ = writeln!(out, "</ul>");
    }
    let _ = writeln!(out, "<h3>Contenus non soumis à l'obligation</h3>");
    if input.non_soumis.is_empty() {
        let _ = writeln!(out, "<p>Aucun contenu non soumis.</p>");
    } else {
        let _ = writeln!(out, "<ul>");
        for contenu in input.non_soumis {
            let _ = writeln!(
                out,
                "<li>{} : {}.</li>",
                echappe(&contenu.categorie),
                echappe(&contenu.justification)
            );
        }
        let _ = writeln!(out, "</ul>");
    }

    let _ = writeln!(out, "<h2>4. Établissement de cette déclaration</h2>");
    let _ = writeln!(
        out,
        "<p>Déclaration établie le {}. Technologies utilisées : {}. Environnement de test : {}. Pages auditées : {}.</p>",
        echappe(input.date_declaration),
        echappe(&input.technologies.join(", ")),
        echappe(input.environnement),
        echappe(
            &input
                .pages
                .iter()
                .map(|p| p.page_type.clone())
                .collect::<Vec<_>>()
                .join(", ")
        )
    );

    let _ = writeln!(out, "<h2>5. Retour d'information et contact</h2>");
    let contact_affiche = input
        .contact
        .email
        .clone()
        .unwrap_or_else(|| input.contact.canal.clone());
    let _ = writeln!(out, "<p>Contact : {}.</p>", echappe(&contact_affiche));

    let _ = writeln!(out, "<h2>6. Voies de recours</h2>");
    let _ = writeln!(out, "<p>{}</p>", RECOURS_FR);

    let _ = writeln!(out, "<h2>7. Schéma pluriannuel et plan d'action</h2>");
    match (input.schema_pluriannuel_url, input.plan_action_url) {
        (Some(schema), Some(plan)) => {
            let _ = writeln!(
                out,
                "<p>Voir le <a href=\"{}\">schéma pluriannuel</a> et le <a href=\"{}\">plan d'action de l'année en cours</a>.</p>",
                echappe(schema),
                echappe(plan)
            );
        }
        _ => {
            let _ = writeln!(
                out,
                "<p>Schéma pluriannuel et plan d'action en cours de publication.</p>"
            );
        }
    }

    let _ = writeln!(out, "</main></body></html>");
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Contact, RGAA_41};
    use rgaa_core::{Classification, CriterionStatus};

    fn proved_finding() -> Finding {
        let mut finding = Finding::new("finding-1");
        finding.rule = "rgaa-1.1".into();
        finding.url = "https://example.test/contact".into();
        finding.target = "#main".into();
        finding.status = CriterionStatus::Fail;
        finding.description = Some("image sans alternative".into());
        finding.remediation = Some("ajouter un alt".into());
        finding.evidence = vec![rgaa_core::EvidenceRef::new("dom", "abc123")];
        finding
    }

    fn input<'a>(
        metrics: &'a SiteMetrics,
        ncs: &'a [NcEntry],
        contact: &'a Contact,
    ) -> DeclarationFrInput<'a> {
        static DEROGATIONS: std::sync::OnceLock<Vec<Derogation>> = std::sync::OnceLock::new();
        static NON_SOUMIS: std::sync::OnceLock<Vec<ContenuNonSoumis>> = std::sync::OnceLock::new();
        static TECHNOLOGIES: std::sync::OnceLock<Vec<String>> = std::sync::OnceLock::new();
        static PAGES: std::sync::OnceLock<Vec<PageEchantillon>> = std::sync::OnceLock::new();
        DeclarationFrInput {
            service: "Service Client",
            organisme: "Ministère Exemple",
            metrics,
            conformes: 48,
            non_conformes: 12,
            non_applicables: 46,
            non_testes: 0,
            non_conformites: ncs,
            derogations: DEROGATIONS.get_or_init(Vec::new),
            non_soumis: NON_SOUMIS.get_or_init(Vec::new),
            contact,
            date_declaration: "2026-09-19",
            technologies: TECHNOLOGIES.get_or_init(|| vec!["HTML5".into()]),
            pages: PAGES.get_or_init(Vec::new),
            environnement: "Firefox + NVDA",
            schema_pluriannuel_url: Some("https://example.test/schema"),
            plan_action_url: Some("https://example.test/plan"),
        }
    }

    fn metrics_80() -> SiteMetrics {
        crate::compute_metrics(
            &[
                rgaa_core::CriterionResult {
                    criterion_id: "1.1".into(),
                    title: "t".into(),
                    classification: Classification::Deterministe,
                    status: CriterionStatus::Pass,
                    violations: Vec::new(),
                    confidence: None,
                    justification: None,
                    source: "t".into(),
                },
                rgaa_core::CriterionResult {
                    criterion_id: "1.2".into(),
                    title: "t".into(),
                    classification: Classification::Deterministe,
                    status: CriterionStatus::Fail,
                    violations: Vec::new(),
                    confidence: None,
                    justification: None,
                    source: "t".into(),
                },
            ],
            &RGAA_41,
        )
    }

    #[test]
    fn golden_fr_partiellement_conforme() {
        let metrics = metrics_80();
        let ncs = [NcEntry::from_finding(&proved_finding()).expect("preuve complète")];
        let contact = Contact {
            canal: "email".into(),
            email: Some("aide@example.test".into()),
            telephone: None,
            formulaire: None,
            delai_reponse: None,
        };
        let html = render_declaration_fr(&input(&metrics, &ncs, &contact)).expect("rendu");
        for section in [
            "1. Engagement",
            "2. État de conformité",
            "3. Résultats des tests",
            "4. Établissement",
            "5. Retour d'information",
            "6. Voies de recours",
            "7. Schéma pluriannuel",
        ] {
            assert!(html.contains(section), "section {section} absente");
        }
        assert!(html.contains("partiellement conforme"));
        assert!(html.contains("Défenseur des droits"));
        assert!(html.contains("https://example.test/schema"));
        assert!(html.contains("image sans alternative"));
    }

    #[test]
    fn nc_non_prouvee_rejetee_du_rendu() {
        let mut finding = proved_finding();
        finding.remediation = None;
        assert!(NcEntry::from_finding(&finding).is_none());
    }

    #[test]
    fn contact_vide_bloque_le_rendu() {
        let metrics = metrics_80();
        let contact = Contact {
            canal: "   ".into(),
            email: None,
            telephone: None,
            formulaire: None,
            delai_reponse: None,
        };
        let ncs: [NcEntry; 0] = [];
        assert!(render_declaration_fr(&input(&metrics, &ncs, &contact)).is_err());
    }
}
