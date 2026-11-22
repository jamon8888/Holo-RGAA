//! Taux de réouverture — indicateur de rappel de l'outil sur le pilote (#438).
//!
//! Une non-conformité posée par le relecteur est **rouverte** quand l'outil
//! n'avait rien signalé sur le test correspondant. Le dénominateur du taux ne
//! retient que les non-conformités dont le signal est déterminable ; les cas
//! indéterminables sont comptés à part, jamais dans le taux.
//!
//! Règle retenue (décision du coordinateur, 2026-11-21) :
//!
//! - on lit le `CriterionResult` **machine** de la même page (`page_url`) et du
//!   même critère (`criterion_id`) — `status` est le verdict machine,
//!   `verified_status` restant la décision du relecteur ;
//! - **signalée** : `status == Fail`, ou `status == NeedsReview` avec au moins
//!   un finding ou une preuve rattaché à cette page et ce critère (l'outil a
//!   attiré l'attention) ;
//! - **rouverte** : `status == Pass` ou `NotApplicable`, ou `NeedsReview` sans
//!   aucun finding ni preuve ;
//! - **indéterminable** : `status == NotTested` ou `Error`, ou aucun résultat
//!   machine pour ce couple page/critère.
//!
//! `taux = rouvertes / (rouvertes + signalees)` ; les indéterminables sont
//! comptés et exportés à part. En cas de plusieurs résultats machine pour le
//! même couple page/critère, le premier est retenu — la même convention que
//! `compute_audit_metrics`. Aucune recherche de la chaîne « suspect » n'est
//! faite : le contrat P-00 (`Indice::Suspect`, `Preuve.plugin`) n'est pas encore
//! dans `AuditBundle` ; [`plugin_id_for`] est le point d'extension prévu pour la
//! ventilation par plugin d'analyse quand il le sera.

use std::collections::BTreeMap;

use indexmap::IndexMap;
use rgaa_core::{AuditBundle, CriterionResult, CriterionStatus, NonConformity, PageAudit};
use serde::{Deserialize, Serialize};

use crate::ReportError;

/// Version du schéma de l'export pour le porteur.
pub const SCHEMA_REOUVERTURE: &str = "reouverture-1";

/// Compteurs d'une population de non-conformités, avec le taux dérivé.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CompteursReouverture {
    pub rouvertes: usize,
    pub signalees: usize,
    pub indeterminables: usize,
    /// `100 * rouvertes / (rouvertes + signalees)` ; 0 si le dénominateur est nul.
    pub taux_reouverture_percent: f64,
}

impl CompteursReouverture {
    /// Construit des compteurs cohérents : le taux est toujours dérivé des comptes.
    #[must_use]
    pub fn new(rouvertes: usize, signalees: usize, indeterminables: usize) -> Self {
        let determinables = rouvertes + signalees;
        let taux_reouverture_percent = if determinables == 0 {
            0.0
        } else {
            100.0 * rouvertes as f64 / determinables as f64
        };
        Self {
            rouvertes,
            signalees,
            indeterminables,
            taux_reouverture_percent,
        }
    }

    /// Toutes les non-conformités, y compris indéterminables.
    #[must_use]
    pub const fn total(self) -> usize {
        self.rouvertes + self.signalees + self.indeterminables
    }

    fn ajoute(&mut self, autre: Self) {
        *self = Self::new(
            self.rouvertes + autre.rouvertes,
            self.signalees + autre.signalees,
            self.indeterminables + autre.indeterminables,
        );
    }
}

impl Default for CompteursReouverture {
    fn default() -> Self {
        Self::new(0, 0, 0)
    }
}

/// Résultat pour un audit, dans l'export du porteur.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuditReouverture {
    pub audit_id: String,
    pub url: String,
    #[serde(flatten)]
    pub compteurs: CompteursReouverture,
}

/// Un des dix critères prioritaires, classés par taux de réouverture décroissant.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CriterePriorite {
    pub critere_id: String,
    #[serde(flatten)]
    pub compteurs: CompteursReouverture,
}

/// Export JSON pour le porteur : jamais rendu dans le rapport client.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExportReouverture {
    pub schema: String,
    pub global: CompteursReouverture,
    pub par_audit: Vec<AuditReouverture>,
    pub par_critere: IndexMap<String, CompteursReouverture>,
    pub par_plugin: IndexMap<String, CompteursReouverture>,
    pub top_10_criteres: Vec<CriterePriorite>,
}

/// Signal de l'outil sur le test d'une non-conformité.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Signal {
    /// L'outil a signalé : `fail` machine, ou `NeedsReview` avec preuve à l'appui.
    Signalee,
    /// L'outil n'a rien signalé : `Pass`, `NotApplicable`, ou `NeedsReview` sans preuve.
    Reouverte,
    /// Aucun résultat machine exploitable : `NotTested`, `Error`, ou critère absent.
    Indeterminable,
}

/// Point d'extension pour la ventilation par plugin d'analyse.
///
/// Le contrat P-00 ajoutera `Indice::Suspect` et `Preuve.plugin` au bundle.
/// Tant que ces champs n'existent pas, aucun plugin ne peut être attribué à une
/// non-conformité et `par_plugin` reste vide — la donnée n'existe pas encore.
fn plugin_id_for(
    _bundle: &AuditBundle,
    _page: &PageAudit,
    _criterion: &CriterionResult,
    _non_conformity: &NonConformity,
) -> Option<String> {
    None
}

/// Un finding du bundle global est rattaché par URL et critère ; un finding
/// porté par la page l'est déjà par la page, seul le critère doit concord.
fn has_finding(bundle: &AuditBundle, page: &PageAudit, criterion_id: &str) -> bool {
    page.findings
        .iter()
        .any(|finding| finding.criterion_id.as_deref() == Some(criterion_id))
        || bundle.findings.iter().any(|finding| {
            finding.criterion_id.as_deref() == Some(criterion_id) && finding.url == page.url
        })
}

fn has_evidence(criterion: &CriterionResult) -> bool {
    !criterion.evidence.is_empty()
        || criterion.tests.iter().any(|outcome| {
            outcome
                .evidence
                .as_deref()
                .is_some_and(|e| !e.trim().is_empty())
        })
}

/// Applique la règle du module au résultat machine trouvé pour une non-conformité.
#[must_use]
pub fn signal_pour(criterion: &CriterionResult, page: &PageAudit, bundle: &AuditBundle) -> Signal {
    match criterion.status {
        CriterionStatus::Fail => Signal::Signalee,
        CriterionStatus::NeedsReview => {
            if has_finding(bundle, page, &criterion.criterion_id) || has_evidence(criterion) {
                Signal::Signalee
            } else {
                Signal::Reouverte
            }
        }
        CriterionStatus::Pass | CriterionStatus::NotApplicable => Signal::Reouverte,
        CriterionStatus::Error | CriterionStatus::NotTested => Signal::Indeterminable,
    }
}

impl CompteursReouverture {
    fn ajoute_signal(&mut self, signal: Signal) {
        let (rouvertes, signalees, indeterminables) = match signal {
            Signal::Signalee => (0, 1, 0),
            Signal::Reouverte => (1, 0, 0),
            Signal::Indeterminable => (0, 0, 1),
        };
        self.ajoute(Self::new(rouvertes, signalees, indeterminables));
    }
}

/// Calcule le taux de réouverture sur un ou plusieurs bundles.
#[must_use]
pub fn compute_reouverture(bundles: &[&AuditBundle]) -> ExportReouverture {
    let mut global = CompteursReouverture::default();
    let mut par_audit = Vec::with_capacity(bundles.len());
    let mut par_critere: BTreeMap<String, CompteursReouverture> = BTreeMap::new();
    let mut par_plugin: BTreeMap<String, CompteursReouverture> = BTreeMap::new();

    for bundle in bundles {
        let mut locaux = CompteursReouverture::default();
        for non_conformity in &bundle.non_conformities {
            let machine = bundle
                .pages
                .iter()
                .find(|page| page.url == non_conformity.page_url)
                .and_then(|page| {
                    page.criteria
                        .iter()
                        .find(|criterion| criterion.criterion_id == non_conformity.criterion_id)
                        .map(|criterion| (page, criterion))
                });
            let (signal, plugin) = match machine {
                Some((page, criterion)) => (
                    signal_pour(criterion, page, bundle),
                    plugin_id_for(bundle, page, criterion, non_conformity),
                ),
                None => (Signal::Indeterminable, None),
            };

            locaux.ajoute_signal(signal);
            global.ajoute_signal(signal);
            par_critere
                .entry(non_conformity.criterion_id.clone())
                .or_default()
                .ajoute_signal(signal);
            if let Some(plugin) = plugin {
                par_plugin.entry(plugin).or_default().ajoute_signal(signal);
            }
        }
        par_audit.push(AuditReouverture {
            audit_id: bundle.audit_id.clone(),
            url: bundle.url.clone(),
            compteurs: locaux,
        });
    }

    let mut top: Vec<CriterePriorite> = par_critere
        .iter()
        .map(|(critere_id, compteurs)| CriterePriorite {
            critere_id: critere_id.clone(),
            compteurs: *compteurs,
        })
        .collect();
    top.sort_by(|left, right| {
        right
            .compteurs
            .taux_reouverture_percent
            .total_cmp(&left.compteurs.taux_reouverture_percent)
            .then_with(|| right.compteurs.rouvertes.cmp(&left.compteurs.rouvertes))
            .then_with(|| left.critere_id.cmp(&right.critere_id))
    });
    top.truncate(10);

    ExportReouverture {
        schema: SCHEMA_REOUVERTURE.into(),
        global,
        par_audit,
        par_critere: par_critere.into_iter().collect(),
        par_plugin: par_plugin.into_iter().collect(),
        top_10_criteres: top,
    }
}

/// Sérialise l'export en JSON.
///
/// # Errors
/// Returns [`ReportError::Execution`] when serialization fails.
pub fn export_reouverture_json(bundles: &[&AuditBundle]) -> Result<String, ReportError> {
    serde_json::to_string_pretty(&compute_reouverture(bundles))
        .map_err(|error| ReportError::execution(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{render, ReportFormat};
    use rgaa_core::{AuditConfig, EvidenceRef, Finding};

    fn result(criterion_id: &str, status: CriterionStatus) -> CriterionResult {
        CriterionResult {
            criterion_id: criterion_id.into(),
            title: "Test".into(),
            classification: rgaa_core::Classification::Deterministe,
            status,
            violations: Vec::new(),
            raw_confidence: None,
            confidence: None,
            justification: None,
            source: "axe-core".into(),
            citations: Vec::new(),
            considered_sources: Vec::new(),
            tests: Vec::new(),
            automated_verdict: None,
            verdict_basis: Vec::new(),
            evidence: Vec::new(),
            confidence_calibration_version: None,
            review_required: false,
            review_reason: None,
            verified_status: None,
            review_events: Vec::new(),
        }
    }

    fn nc(id: &str, criterion_id: &str, url: &str) -> NonConformity {
        NonConformity::new(id, criterion_id, url, "constat")
    }

    fn bundle() -> AuditBundle {
        AuditBundle::new("audit-1", "https://example.test", AuditConfig::default())
    }

    /// Attendus calculés à la main d'après la règle du ticket, jamais avec le code.
    #[test]
    fn classe_rouvertes_signalees_et_indeterminables() {
        let url = "https://example.test/a";
        let mut bun = bundle();
        let page = PageAudit {
            page_id: "page-0".into(),
            url: url.into(),
            title: None,
            criteria: vec![
                result("1.1", CriterionStatus::Fail),
                result("1.2", CriterionStatus::NeedsReview),
                result("1.3", CriterionStatus::NeedsReview),
                result("1.4", CriterionStatus::Pass),
                result("1.5", CriterionStatus::NotApplicable),
                result("1.6", CriterionStatus::NotTested),
                result("1.7", CriterionStatus::Error),
            ],
            findings: vec![{
                let mut finding = Finding::new("finding-1");
                finding.criterion_id = Some("1.2".into());
                finding.url = url.into();
                finding.rule = "rgaa-1.2".into();
                finding.target = "#main".into();
                finding
            }],
            errors: Vec::new(),
            completed: true,
            duration_ms: 0,
        };
        bun.pages.push(page);
        bun.non_conformities = vec![
            nc("nc-fail", "1.1", url),
            nc("nc-review-finding", "1.2", url),
            nc("nc-review-bare", "1.3", url),
            nc("nc-pass", "1.4", url),
            nc("nc-na", "1.5", url),
            nc("nc-nt", "1.6", url),
            nc("nc-error", "1.7", url),
            nc("nc-absent", "9.9", url),
        ];

        let export = compute_reouverture(&[&bun]);
        // 3 rouvertes (1.3, Pass, NA), 2 signalées (Fail, NeedsReview+finding),
        // 3 indéterminables (NotTested, Error, critère absent) => 3/5 = 60 %.
        assert_eq!(export.global.rouvertes, 3);
        assert_eq!(export.global.signalees, 2);
        assert_eq!(export.global.indeterminables, 3);
        assert_eq!(export.global.total(), 8);
        assert!((export.global.taux_reouverture_percent - 60.0).abs() < 1e-9);
    }

    /// Une preuve suffit à qualifier un `NeedsReview` de « signalé ».
    #[test]
    fn une_preuve_qualifie_un_needs_review_de_signale() {
        let url = "https://example.test/a";
        let mut bun = bundle();
        let mut with_evidence = result("3.1", CriterionStatus::NeedsReview);
        with_evidence
            .evidence
            .push(EvidenceRef::new("screenshot", "sha256:abc"));
        let mut with_test_evidence = result("3.2", CriterionStatus::NeedsReview);
        let mut outcome = rgaa_core::TestOutcome {
            test_key: "1".into(),
            status: CriterionStatus::NeedsReview,
            source: "axe-core".into(),
            evidence: None,
        };
        outcome.evidence = Some("contraste mesuré".into());
        with_test_evidence.tests.push(outcome);
        bun.pages.push(PageAudit {
            page_id: "page-0".into(),
            url: url.into(),
            title: None,
            criteria: vec![with_evidence, with_test_evidence],
            findings: Vec::new(),
            errors: Vec::new(),
            completed: true,
            duration_ms: 0,
        });
        bun.non_conformities = vec![nc("a", "3.1", url), nc("b", "3.2", url)];

        let export = compute_reouverture(&[&bun]);
        assert_eq!(export.global.signalees, 2);
        assert_eq!(export.global.rouvertes, 0);
        assert_eq!(export.global.taux_reouverture_percent, 0.0);
    }

    /// Le même critère sur une autre page ne peut pas signaler la non-conformité.
    #[test]
    fn le_signal_est_rattache_a_la_page() {
        let mut bun = bundle();
        for (url, status) in [
            ("https://example.test/a", CriterionStatus::Fail),
            ("https://example.test/b", CriterionStatus::Pass),
        ] {
            bun.pages.push(PageAudit {
                page_id: url.into(),
                url: url.into(),
                title: None,
                criteria: vec![result("1.1", status)],
                findings: Vec::new(),
                errors: Vec::new(),
                completed: true,
                duration_ms: 0,
            });
        }
        bun.non_conformities = vec![
            nc("a", "1.1", "https://example.test/a"),
            nc("b", "1.1", "https://example.test/b"),
            nc("c", "1.1", "https://example.test/c"),
        ];

        let export = compute_reouverture(&[&bun]);
        // a : signalée (Fail), b : rouverte (Pass), c : indéterminable (page absente).
        assert_eq!(export.global.signalees, 1);
        assert_eq!(export.global.rouvertes, 1);
        assert_eq!(export.global.indeterminables, 1);
        assert!((export.global.taux_reouverture_percent - 50.0).abs() < 1e-9);
    }

    /// Un finding global (hors page) est rattaché par URL + critère.
    #[test]
    fn un_finding_global_peut_signaler() {
        let url = "https://example.test/a";
        let mut bun = bundle();
        bun.pages.push(PageAudit {
            page_id: "page-0".into(),
            url: url.into(),
            title: None,
            criteria: vec![result("2.1", CriterionStatus::NeedsReview)],
            findings: Vec::new(),
            errors: Vec::new(),
            completed: true,
            duration_ms: 0,
        });
        let mut finding = Finding::new("finding-global");
        finding.criterion_id = Some("2.1".into());
        finding.url = url.into();
        finding.rule = "rgaa-2.1".into();
        finding.target = "#main".into();
        bun.findings.push(finding);
        bun.non_conformities = vec![nc("a", "2.1", url)];

        let export = compute_reouverture(&[&bun]);
        assert_eq!(export.global.signalees, 1);
    }

    /// Agrégation par audit, par critère et top 10 ; pas de plugin sans donnée P-00.
    #[test]
    fn agregue_par_audit_et_par_critere_avec_ordre_deterministe() {
        let url = "https://example.test/a";
        let mut first = bundle();
        first.pages.push(PageAudit {
            page_id: "page-0".into(),
            url: url.into(),
            title: None,
            criteria: vec![
                result("1.1", CriterionStatus::Pass),
                result("2.1", CriterionStatus::Fail),
            ],
            findings: Vec::new(),
            errors: Vec::new(),
            completed: true,
            duration_ms: 0,
        });
        first.non_conformities = vec![nc("a", "1.1", url), nc("c", "2.1", url)];
        let mut second =
            AuditBundle::new("audit-2", "https://example.test", AuditConfig::default());
        second.pages.push(PageAudit {
            page_id: "page-0".into(),
            url: url.into(),
            title: None,
            criteria: vec![result("1.1", CriterionStatus::Pass)],
            findings: Vec::new(),
            errors: Vec::new(),
            completed: true,
            duration_ms: 0,
        });
        second.non_conformities = vec![nc("d", "1.1", url)];

        let export = compute_reouverture(&[&second, &first]);
        assert_eq!(export.par_audit.len(), 2);
        assert_eq!(export.par_audit[0].audit_id, "audit-2");
        assert_eq!(export.par_audit[1].audit_id, "audit-1");
        // 1.1 : 2 rouvertes (un Pass dans chaque audit) ; 2.1 : 1 signalée (Fail).
        assert_eq!(
            export.par_critere.get("1.1"),
            Some(&CompteursReouverture::new(2, 0, 0))
        );
        assert_eq!(
            export.par_critere.keys().collect::<Vec<_>>(),
            ["1.1", "2.1"]
        );
        assert!(export.par_plugin.is_empty());
        // 2.1 a un taux de 0 %, 1.1 de 100 / 3 : 1.1 vient d'abord.
        assert_eq!(export.top_10_criteres.len(), 2);
        assert_eq!(export.top_10_criteres[0].critere_id, "1.1");
        assert_eq!(export.top_10_criteres[1].critere_id, "2.1");
    }

    #[test]
    fn bundle_sans_non_conformite_donne_un_taux_nul() {
        let bun = bundle();
        let export = compute_reouverture(&[&bun]);
        assert_eq!(export.global, CompteursReouverture::default());
        assert_eq!(export.global.total(), 0);
        assert_eq!(export.global.taux_reouverture_percent, 0.0);
        assert_eq!(export.par_audit.len(), 1);
        assert!(export.par_critere.is_empty());
    }

    /// #438 : l'indicateur reste hors du rapport client, quels que soient les formats.
    #[test]
    fn indicateur_absent_du_rapport_client() {
        let mut bun = bundle();
        let url = "https://example.test/a";
        bun.pages.push(PageAudit {
            page_id: "page-0".into(),
            url: url.into(),
            title: None,
            criteria: vec![result("1.1", CriterionStatus::Pass)],
            findings: Vec::new(),
            errors: Vec::new(),
            completed: true,
            duration_ms: 0,
        });
        bun.non_conformities = vec![nc("nc-1", "1.1", url)];

        let export = export_reouverture_json(&[&bun]).expect("export");
        assert!(export.contains("taux_reouverture_percent"));
        assert!(export.contains("indeterminables"));

        for format in [
            ReportFormat::Json,
            ReportFormat::Markdown,
            ReportFormat::Html,
            ReportFormat::Sarif,
            ReportFormat::Junit,
        ] {
            let rendered = render(&bun, format).expect("client report");
            for marqueur in [
                "réouverture",
                "reouverture",
                "réouverte",
                "indéterminable",
                "indeterminable",
                "taux de rappel",
            ] {
                assert!(
                    !rendered.to_lowercase().contains(&marqueur.to_lowercase()),
                    "{format:?} contient {marqueur:?}"
                );
            }
        }
    }
}
