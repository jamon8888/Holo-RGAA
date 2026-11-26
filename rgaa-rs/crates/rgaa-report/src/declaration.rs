//! French declaration: the 7 DINUM sections as semantic HTML.
//!
//! The template is parameterized, never hard-coded: every legal sentence
//! comes from the metrics and the pack. Other countries get their own
//! template from the UE 2018/1523 core.

use std::fmt::Write;

use rgaa_core::{AuditType, Finding};

use crate::guard::{Contact, ContenuNonSoumis, Derogation, PageEchantillon};
use crate::packs::{etat_fr, RECOURS_FR};
use crate::{ReportError, SiteMetrics};

/// One test environment actually used during an audit, mirroring Ara's
/// `TestEnvironment`. Every field is optional: an environment is declared only
/// to the extent it is known, and no reference environment is invented by
/// default. An assistive technology belongs here only when a reviewer cited
/// it in a decision or the screen-reader plugin ran.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TestEnvironment {
    /// Platform, e.g. `ordinateur` or `mobile`.
    pub plateforme: Option<String>,
    /// Operating system, e.g. `Windows 11`.
    pub systeme: Option<String>,
    /// Assistive technology, e.g. `NVDA`. `None` when none was used.
    pub technologie_assistance: Option<String>,
    /// Browser used for this environment, e.g. `Firefox`.
    pub navigateur: Option<String>,
}

impl TestEnvironment {
    fn est_vide(&self) -> bool {
        [
            &self.plateforme,
            &self.systeme,
            &self.technologie_assistance,
            &self.navigateur,
        ]
        .iter()
        .all(|champ| champ.as_deref().is_none_or(|v| v.trim().is_empty()))
    }
}

/// Builds the environments the declaration must publish: the browser that
/// actually ran the audit is always added automatically, followed by the
/// environments that were really used. Nothing is invented when the list is
/// empty — the absence of assistive technology is stated instead.
#[must_use]
pub fn environnements_utilises(
    navigateur_audit: Option<&str>,
    environnements: &[TestEnvironment],
) -> Vec<TestEnvironment> {
    let mut reels =
        Vec::with_capacity(environnements.len() + usize::from(navigateur_audit.is_some()));
    if let Some(navigateur) = navigateur_audit.filter(|n| !n.trim().is_empty()) {
        reels.push(TestEnvironment {
            navigateur: Some(navigateur.to_string()),
            ..Default::default()
        });
    }
    reels.extend(environnements.iter().filter(|e| !e.est_vide()).cloned());
    reels
}

fn format_environnement(environnement: &TestEnvironment) -> Option<String> {
    let mut parties: Vec<String> = Vec::with_capacity(4);
    for (libelle, valeur) in [
        ("Plateforme", &environnement.plateforme),
        ("Système", &environnement.systeme),
        ("Navigateur", &environnement.navigateur),
        (
            "Technologie d'assistance",
            &environnement.technologie_assistance,
        ),
    ] {
        if let Some(valeur) = valeur.as_deref().filter(|v| !v.trim().is_empty()) {
            parties.push(format!("{libelle} : {valeur}"));
        }
    }
    (!parties.is_empty()).then(|| parties.join(" ; "))
}

/// French rendering of the declared environments. When none of them names an
/// assistive technology, the text says so instead of falling back to a
/// reference environment nobody used.
#[must_use]
pub fn environnements_en_texte(environnements: &[TestEnvironment]) -> String {
    let declares: Vec<String> = environnements
        .iter()
        .filter_map(format_environnement)
        .collect();
    let mut texte = declares.join(" ; ");
    let avec_assistance = environnements.iter().any(|e| {
        e.technologie_assistance
            .as_deref()
            .is_some_and(|v| !v.trim().is_empty())
    });
    if !avec_assistance {
        if !texte.is_empty() {
            texte.push_str(" ; ");
        }
        texte.push_str("aucune technologie d'assistance utilisée");
    }
    texte
}

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
                let court: String = e.hash.chars().take(8).collect();
                e.location
                    .clone()
                    .unwrap_or_else(|| format!("{}:{court}", e.kind))
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
    /// Scope the declaration would be based on. A rapide audit is partial and
    /// can never produce a conformity declaration.
    pub audit_type: AuditType,
    pub service: &'a str,
    pub organisme: &'a str,
    pub metrics: &'a SiteMetrics,
    pub non_conformites: &'a [NcEntry],
    pub derogations: &'a [Derogation],
    pub non_soumis: &'a [ContenuNonSoumis],
    pub contact: &'a Contact,
    pub date_declaration: &'a str,
    pub technologies: &'a [String],
    pub pages: &'a [PageEchantillon],
    /// Environments that were actually used (reviewer decisions citing an
    /// assistive technology, screen-reader plugin runs).
    pub environnements: &'a [TestEnvironment],
    /// Browser that ran the audit; added to the declaration automatically.
    pub navigateur_audit: Option<&'a str>,
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

/// Renders `url` as a link only for safe schemes (HTTP/S or site-relative);
/// anything else becomes plain text so no executable scheme lands in `href`.
fn lien_sur(url: &str, texte: &str) -> String {
    let cible = url.trim();
    if cible.starts_with("https://") || cible.starts_with("http://") || cible.starts_with('/') {
        format!("<a href=\"{}\">{}</a>", echappe(cible), echappe(texte))
    } else {
        echappe(cible)
    }
}

/// Refuse to render a conformity declaration from a partial audit.
///
/// A *rapide* audit checks 25 of 106 criteria and a *complémentaire* 50: any
/// "conforme" statement drawn from either would be a claim the measurement
/// cannot support (#428). The caller still gets the audit report, which says
/// "audit partiel, ne vaut pas déclaration de conformité".
///
/// # Errors
///
/// Returns [`ReportError::InvalidInput`] for a scope that cannot support a
/// declaration.
pub fn refuse_partial_declaration(audit_type: AuditType) -> Result<(), ReportError> {
    if audit_type.allows_conformity_declaration() {
        Ok(())
    } else {
        Err(ReportError::invalid_input(format!(
            "audit {} : audit partiel, aucune déclaration de conformité ne peut être produite",
            audit_type.as_str()
        )))
    }
}

/// Renders the French declaration, ready to publish on `/accessibilite`.
/// Fails when no feedback destination exists: a declaration without
/// feedback is invalid on its face. Also fails for a partial audit scope.
pub fn render_declaration_fr(input: &DeclarationFrInput) -> Result<String, ReportError> {
    refuse_partial_declaration(input.audit_type)?;
    let destination = crate::guard::destination_contact(input.contact).ok_or_else(|| {
        ReportError::invalid_input(
            "destination du contact de retour d'information manquante".to_string(),
        )
    })?;
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
    let _ = writeln!(
        out,
        "<p>Type d'audit : {} ({} critères).</p>",
        echappe(input.audit_type.label()),
        input.audit_type.count()
    );

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
        // #203 recommendation 4: the rate never appears without the coverage it rests on.
        // This sentence used to publish `taux_global` alone, which is exactly how a rate
        // of 81.08 % went out over an audit that had never tested 48 criteria.
        "<p><strong>{}</strong> est <strong>{}</strong> avec le Référentiel Général d'Amélioration de l'Accessibilité (RGAA), version 4.1.2. Le taux de conformité s'élève à <strong>{:.1} %</strong>, établi sur <strong>{:.1} %</strong> de la surface testable ({} critères conformes, {} non conformes, {} non applicables, {} non testés).{}</p>",
        echappe(input.service),
        etat_fr(&input.metrics.etat_conformite),
        input.metrics.taux_global,
        input.metrics.coverage_percent,
        input.metrics.conformes,
        input.metrics.non_conformes,
        input.metrics.non_applicables,
        input.metrics.non_testes,
        if input.metrics.rate_is_a_conformance_claim() {
            ""
        } else {
            " <strong>Audit incomplet : ce taux est une mesure, pas une déclaration de conformité.</strong>"
        }
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

    let environnements = environnements_utilises(input.navigateur_audit, input.environnements);
    let environnements_texte = environnements_en_texte(&environnements);
    let _ = writeln!(out, "<h2>4. Établissement de cette déclaration</h2>");
    let _ = writeln!(
        out,
        "<p>Déclaration établie le {}. Technologies utilisées : {}. Environnements de test : {}. Pages auditées : {}.</p>",
        echappe(input.date_declaration),
        echappe(&input.technologies.join(", ")),
        echappe(&environnements_texte),
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
    let _ = writeln!(out, "<p>Contact : {}.</p>", echappe(destination));

    let _ = writeln!(out, "<h2>6. Voies de recours</h2>");
    let _ = writeln!(out, "<p>{}</p>", RECOURS_FR);

    let _ = writeln!(out, "<h2>7. Schéma pluriannuel et plan d'action</h2>");
    match (input.schema_pluriannuel_url, input.plan_action_url) {
        (Some(schema), Some(plan)) => {
            let _ = writeln!(
                out,
                "<p>Voir le {} et le {}.</p>",
                lien_sur(schema, "schéma pluriannuel"),
                lien_sur(plan, "plan d'action de l'année en cours")
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
            audit_type: AuditType::Complet,
            service: "Service Client",
            organisme: "Ministère Exemple",
            metrics,
            non_conformites: ncs,
            derogations: DEROGATIONS.get_or_init(Vec::new),
            non_soumis: NON_SOUMIS.get_or_init(Vec::new),
            contact,
            date_declaration: "2026-09-19",
            technologies: TECHNOLOGIES.get_or_init(|| vec!["HTML5".into()]),
            pages: PAGES.get_or_init(Vec::new),
            environnements: &[],
            navigateur_audit: None,
            schema_pluriannuel_url: Some("https://example.test/schema"),
            plan_action_url: Some("https://example.test/plan"),
        }
    }

    /// A rendered declaration built from the same fixtures the other tests use.
    fn render_for_test() -> String {
        let metrics = metrics_80();
        let ncs: Vec<NcEntry> = Vec::new();
        let contact = Contact {
            canal: "email".into(),
            email: Some("aide@example.test".into()),
            telephone: None,
            formulaire: None,
            delai_reponse: None,
        };
        render_declaration_fr(&input(&metrics, &ncs, &contact)).expect("rendu")
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
                    raw_confidence: None,
                    justification: None,
                    source: "t".into(),
                    citations: Vec::new(),
                    considered_sources: vec![],
                    tests: vec![],
                    automated_verdict: None,
                    verdict_basis: Vec::new(),
                    evidence: Vec::new(),
                    confidence_calibration_version: None,
                    review_required: false,
                    review_reason: None,
                    verified_status: None,
                    review_events: Vec::new(),
                },
                rgaa_core::CriterionResult {
                    criterion_id: "1.2".into(),
                    title: "t".into(),
                    classification: Classification::Deterministe,
                    status: CriterionStatus::Fail,
                    violations: Vec::new(),
                    confidence: None,
                    raw_confidence: None,
                    justification: None,
                    source: "t".into(),
                    citations: Vec::new(),
                    considered_sources: vec![],
                    tests: vec![],
                    automated_verdict: None,
                    verdict_basis: Vec::new(),
                    evidence: Vec::new(),
                    confidence_calibration_version: None,
                    review_required: false,
                    review_reason: None,
                    verified_status: None,
                    review_events: Vec::new(),
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
        assert!(html.contains("(1 critères conformes, 1 non conformes"));
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

    /// #203 recommendation 4, at the surface that matters: the published declaration is
    /// the document a client is handed, and it used to state the rate with no coverage
    /// beside it. Sourcery flagged on #209 that the new helpers were never wired in.
    /// #428: a 25-criteria audit is a partial measurement, never a claim.
    #[test]
    fn partial_audits_cannot_produce_a_conformity_declaration() {
        let metrics = metrics_80();
        let contact = Contact {
            canal: "email".into(),
            email: Some("accessibilite@example.test".into()),
            telephone: None,
            formulaire: None,
            delai_reponse: None,
        };
        for partial in [AuditType::Rapide, AuditType::Complementaire] {
            let mut declaration = input(&metrics, &[], &contact);
            declaration.audit_type = partial;
            let error = render_declaration_fr(&declaration)
                .expect_err("partial audits must never yield a conformity declaration");
            assert!(error.to_string().contains("audit partiel"), "{error}");
            assert!(refuse_partial_declaration(partial).is_err());
        }
        assert!(refuse_partial_declaration(AuditType::Complet).is_ok());
    }

    #[test]
    fn the_declaration_never_publishes_a_rate_without_its_coverage() {
        let html = render_for_test();
        assert!(
            html.contains("surface testable"),
            "the coverage must appear beside the rate: {html}"
        );
        let rate_pos = html.find("taux de conformité").expect("the rate is stated");
        let coverage_pos = html
            .find("surface testable")
            .expect("the coverage is stated");
        assert!(
            coverage_pos > rate_pos,
            "the coverage must qualify the rate in the same sentence"
        );
    }

    /// An incomplete audit must say so in the document, not only in the status word —
    /// and a complete one must not carry the caveat.
    #[test]
    fn only_an_incomplete_audit_is_labelled_in_the_declaration() {
        let complete = render_for_test();
        assert!(
            !complete.contains("Audit incomplet"),
            "a complete audit must not be labelled incomplete: {complete}"
        );

        let mut metrics = metrics_80();
        metrics.non_testes = 7;
        let ncs: Vec<NcEntry> = Vec::new();
        let contact = Contact {
            canal: "email".into(),
            email: Some("aide@example.test".into()),
            telephone: None,
            formulaire: None,
            delai_reponse: None,
        };
        let incomplete = render_declaration_fr(&input(&metrics, &ncs, &contact)).expect("rendu");
        assert!(
            incomplete.contains("Audit incomplet"),
            "7 criteria nobody closed must not read as a conformance declaration: {incomplete}"
        );
        assert!(
            incomplete.contains("mesure, pas une déclaration de conformité"),
            "the caveat must say what the number is: {incomplete}"
        );
    }

    // ---- #430: environnements réellement utilisés ----

    fn environnement_test(
        navigateur: Option<&str>,
        technologie_assistance: Option<&str>,
    ) -> super::TestEnvironment {
        super::TestEnvironment {
            plateforme: None,
            systeme: None,
            technologie_assistance: technologie_assistance.map(str::to_string),
            navigateur: navigateur.map(str::to_string),
        }
    }

    fn contact_test() -> Contact {
        Contact {
            canal: "email".into(),
            email: Some("aide@example.test".into()),
            telephone: None,
            formulaire: None,
            delai_reponse: None,
        }
    }

    fn input_avec_environnements<'a>(
        metrics: &'a SiteMetrics,
        ncs: &'a [NcEntry],
        contact: &'a Contact,
        navigateur_audit: Option<&'a str>,
        environnements: &'a [super::TestEnvironment],
    ) -> DeclarationFrInput<'a> {
        static DEROGATIONS: std::sync::OnceLock<Vec<Derogation>> = std::sync::OnceLock::new();
        static NON_SOUMIS: std::sync::OnceLock<Vec<ContenuNonSoumis>> = std::sync::OnceLock::new();
        static TECHNOLOGIES: std::sync::OnceLock<Vec<String>> = std::sync::OnceLock::new();
        static PAGES: std::sync::OnceLock<Vec<PageEchantillon>> = std::sync::OnceLock::new();
        DeclarationFrInput {
            audit_type: AuditType::Complet,
            service: "Service Client",
            organisme: "Ministère Exemple",
            metrics,
            non_conformites: ncs,
            derogations: DEROGATIONS.get_or_init(Vec::new),
            non_soumis: NON_SOUMIS.get_or_init(Vec::new),
            contact,
            date_declaration: "2026-09-19",
            technologies: TECHNOLOGIES.get_or_init(|| vec!["HTML5".into()]),
            pages: PAGES.get_or_init(Vec::new),
            environnements,
            navigateur_audit,
            schema_pluriannuel_url: Some("https://example.test/schema"),
            plan_action_url: Some("https://example.test/plan"),
        }
    }

    #[test]
    fn le_navigateur_d_audit_est_ajoute_automatiquement() {
        let envs = super::environnements_utilises(Some("Obscura"), &[]);
        assert_eq!(
            envs.len(),
            1,
            "le navigateur d'audit doit être déclaré seul"
        );
        assert_eq!(envs[0].navigateur.as_deref(), Some("Obscura"));
        assert_eq!(envs[0].technologie_assistance, None);
    }

    #[test]
    fn aucune_technologie_assistance_n_est_inventee() {
        let envs = super::environnements_utilises(Some("Chrome via Claude for Chrome"), &[]);
        let texte = super::environnements_en_texte(&envs);
        assert!(texte.contains("Chrome via Claude for Chrome"), "{texte}");
        assert!(
            texte.contains("aucune technologie d'assistance utilisée"),
            "sans technologie d'assistance, le texte doit le dire : {texte}"
        );
        assert!(
            !texte.contains("NVDA"),
            "rien ne justifie NVDA ici : {texte}"
        );
    }

    #[test]
    fn une_technologie_assistance_citee_declaree_sans_le_deni() {
        let envs = super::environnements_utilises(
            Some("Obscura"),
            &[environnement_test(None, Some("NVDA"))],
        );
        let texte = super::environnements_en_texte(&envs);
        assert!(texte.contains("NVDA"), "{texte}");
        assert!(
            !texte.contains("aucune technologie d'assistance utilisée"),
            "le déni serait faux ici : {texte}"
        );
    }

    #[test]
    fn la_declaration_ne_publie_que_les_environnements_utilises() {
        let metrics = metrics_80();
        let contact = contact_test();
        let envs = [
            environnement_test(Some("Firefox"), None),
            environnement_test(None, Some("NVDA")),
        ];
        let html = render_declaration_fr(&input_avec_environnements(
            &metrics,
            &[],
            &contact,
            Some("Obscura"),
            &envs,
        ))
        .expect("rendu");
        assert!(html.contains("Navigateur : Obscura"), "{html}");
        assert!(html.contains("Navigateur : Firefox"), "{html}");
        assert!(html.contains("Technologie d'assistance : NVDA"), "{html}");
        assert!(
            !html.contains("Firefox + NVDA"),
            "aucun environnement de référence codé en dur : {html}"
        );
    }

    #[test]
    fn la_declaration_sans_technologie_assistance_le_dit() {
        let metrics = metrics_80();
        let contact = contact_test();
        let html = render_declaration_fr(&input_avec_environnements(
            &metrics,
            &[],
            &contact,
            Some("Obscura"),
            &[],
        ))
        .expect("rendu");
        assert!(
            html.contains("aucune technologie d'assistance utilisée"),
            "{html}"
        );
        assert!(!html.contains("Firefox + NVDA"), "{html}");
    }
}
