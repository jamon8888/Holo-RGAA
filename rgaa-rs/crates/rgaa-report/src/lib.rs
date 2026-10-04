//! Single source of truth for accessibility compliance computation.
//!
//! Every caller (orchestrator, CLI, TUI) must go through this crate so one
//! audit always yields the same figures. Computation is parameterized by
//! [`Referentiel`]: one constant per national framework, no duplication.

use std::collections::HashMap;

use rgaa_core::catalog::Automatable;
use rgaa_core::{ConformityStatus, CriterionResult, CriterionStatus, RgaaCatalog};

pub mod declaration;
pub mod depot;
pub mod format;
pub mod gouvernance;
pub mod guard;
pub mod packs;
pub mod pdf;
pub mod report;
pub mod sources;
pub mod ue;

pub use declaration::{render_declaration_fr, DeclarationFrInput, NcEntry};
pub use depot::url_canonique;
pub use format::ReportFormat;
pub use gouvernance::{autoriser_generation, OverrideGouvernance, PackVersion, PEREMPTION_JOURS};
pub use guard::{
    schema_export_pack, validate_export, Contact, ContenuNonSoumis, Derogation, ExportPack,
    PageEchantillon, ECHANTILLON_MIN,
};
pub use packs::{mention_fr, pack, PackPays, Pays};
pub use report::render;
pub use sources::{format_citation, sourced_criteria, sources_line};
pub use ue::{render_declaration_ue, DeclarationUeInput};

/// Errors from report generation.
#[derive(Debug, thiserror::Error)]
pub enum ReportError {
    /// Invalid payload or guardrail refusal.
    #[error("{0}")]
    InvalidInput(String),
    /// Rendering failed (serialization, IO shape).
    #[error("{0}")]
    Execution(String),
}

impl ReportError {
    /// Creates an invalid input error.
    pub fn invalid_input(message: impl Into<String>) -> Self {
        Self::InvalidInput(message.into())
    }

    /// Creates an execution error.
    pub fn execution(message: impl Into<String>) -> Self {
        Self::Execution(message.into())
    }
}

/// A national reference framework: thresholds and aggregation rules.
///
/// `retrograde_si_non_teste` encodes the French rule that any strictly
/// untested criterion (`NotTested`, not `NeedsReview`) drops a French audit
/// to non-compliant. No other framework funds that rule, so it stays off
/// everywhere else and untested criteria only raise [`SiteMetrics::audit_incomplet`].
#[derive(Debug, Clone, Copy)]
pub struct Referentiel {
    /// Stable id, e.g. `"rgaa-4.1.2"`.
    pub id: &'static str,
    /// Rate reaching full compliance.
    pub seuil_total: f64,
    /// Rate reaching partial compliance.
    pub seuil_partiel: f64,
    /// French-only downgrade on untested criteria.
    pub retrograde_si_non_teste: bool,
    /// False outside France: the rate stays informative and the legal
    /// status is set by the reviewer, never computed.
    pub taux_juridique: bool,
}

/// RGAA 4.1.2: official rate `C / (C + NC)`, thresholds 100 / 50.
pub const RGAA_41: Referentiel = Referentiel {
    id: "rgaa-4.1.2",
    seuil_total: 100.0,
    seuil_partiel: 50.0,
    retrograde_si_non_teste: true,
    taux_juridique: true,
};

/// UE 2018/1523 qualitative model: no computed status, reviewer sets it.
pub const UE_QUALITATIF: Referentiel = Referentiel {
    id: "ue-2018-1523",
    seuil_total: 100.0,
    seuil_partiel: 50.0,
    retrograde_si_non_teste: false,
    taux_juridique: false,
};

/// Site-wide metrics for one [`Referentiel`].
#[derive(Debug, Clone, PartialEq)]
pub struct SiteMetrics {
    /// Official global rate `C / (C + NC)`, NA/NT excluded.
    pub taux_global: f64,
    /// Share of automatable criteria actually executed.
    pub coverage_percent: f64,
    /// Legal status words of the framework (`"totale"`, `"partielle"`, `"non conforme"`).
    pub etat_conformite: String,
    /// Applicable criteria fully passing on every page.
    pub conformes: usize,
    /// Applicable criteria failing on at least one page.
    pub non_conformes: usize,
    /// Criteria not applicable everywhere.
    pub non_applicables: usize,
    /// Criteria untested or still awaiting review.
    pub non_testes: usize,
    /// True when at least one criterion was never tested.
    pub audit_incomplet: bool,
}

impl SiteMetrics {
    /// The conformance rate and the coverage it rests on, as one inseparable statement.
    ///
    /// #203 recommendation 4. The two numbers count different things — `taux_global`
    /// counts criteria, because that is the unit RGAA conformance and the *Déclaration
    /// d'accessibilité* are defined in; `coverage_percent` counts validated tests, per
    /// #190's settled denominator — and neither is derivable from the other.
    ///
    /// Publishing the rate alone is what produced the `taux_global: 81.08` claim over a
    /// run where two thirds of the passes could not fail, at a coverage figure of 100 %
    /// that counted 48 criteria nothing could contradict. So a rate with no coverage
    /// beside it is treated here as a defect rather than a formatting choice: any surface
    /// that shows the rate calls this, and gets both or neither.
    #[must_use]
    pub fn conformance_statement(&self) -> String {
        format!(
            "{:.2} % conforme, établi sur {:.2} % de la surface testable",
            self.taux_global, self.coverage_percent
        )
    }

    /// Whether the rate may be presented as a conformance claim at all.
    ///
    /// False when the audit is incomplete: under RGAA 4.1 an incomplete audit is
    /// retrograded regardless of the rate, so the number is a measurement, not a claim.
    ///
    /// `audit_incomplet` alone is not enough, as Sourcery pointed out on #209:
    /// `compute_metrics` raises it for a `NotTested` criterion but not for one awaiting
    /// review, so an audit whose criteria all reduced to `NeedsReview` would have read as
    /// a claim. `non_testes` counts both, which is the honest gate — a criterion nobody
    /// has closed is a criterion nobody has closed, whichever word it carries.
    #[must_use]
    pub fn rate_is_a_conformance_claim(&self) -> bool {
        !self.audit_incomplet && self.non_testes == 0
    }
}

/// Per-page rate: passing over passing plus failing, NA/NT excluded.
/// Entries reduce by `criterion_id` first (same site rule as
/// [`compute_metrics`]; identity on unique ids).
#[must_use]
pub fn compliance_rate(criteria: &[CriterionResult]) -> f64 {
    let mut par_critere: HashMap<&str, Vec<CriterionStatus>> = HashMap::new();
    for criterion in criteria {
        par_critere
            .entry(criterion.criterion_id.as_str())
            .or_default()
            .push(criterion.status.clone());
    }
    let mut pass = 0;
    let mut fail = 0;
    for statuts in par_critere.values() {
        match reduire_statuts(statuts) {
            ConformityStatus::Conforme => pass += 1,
            ConformityStatus::NonConforme => fail += 1,
            ConformityStatus::NonApplicable | ConformityStatus::NonTeste => {}
        }
    }
    if pass + fail > 0 {
        (pass as f64 / (pass + fail) as f64) * 100.0
    } else {
        0.0
    }
}

/// Worst status of one criterion across every page (site rule: a single
/// Fail or Error makes the whole site non-conforming on that criterion).
fn reduire_statuts(statuts: &[CriterionStatus]) -> ConformityStatus {
    if statuts
        .iter()
        .any(|s| matches!(s, CriterionStatus::Fail | CriterionStatus::Error))
    {
        ConformityStatus::NonConforme
    } else if statuts.iter().all(|s| *s == CriterionStatus::NotApplicable) {
        ConformityStatus::NonApplicable
    } else if statuts
        .iter()
        .any(|s| matches!(s, CriterionStatus::NeedsReview | CriterionStatus::NotTested))
    {
        ConformityStatus::NonTeste
    } else {
        ConformityStatus::Conforme
    }
}

/// Whether one criterion's per-page statuses amount to a verdict, which is what
/// `coverage_percent` counts as "executed".
///
/// A `Fail` on any page is a verdict: the criterion is non-conforming for the
/// site whatever the other pages say. Without one, the criterion is decided only
/// when every page is `Pass` or `NotApplicable`. `NeedsReview`, `NotTested` and
/// `Error` are not verdicts: counting them (as this used to for everything but
/// `NotTested`) credited the 45 partially-automatable criteria sent to review in
/// bulk, and criteria whose evaluation failed, as tested.
///
/// The single definition for both the per-page metrics and the site-wide
/// aggregation in the orchestrator, which had disagreed (`any` page tested versus
/// no page untested).
#[must_use]
pub fn is_validated(statuts: &[CriterionStatus]) -> bool {
    statuts.contains(&CriterionStatus::Fail)
        || statuts
            .iter()
            .all(|s| matches!(s, CriterionStatus::Pass | CriterionStatus::NotApplicable))
}

/// Site-wide metrics for `criteria` under `referentiel`.
///
/// Entries are reduced by `criterion_id` first, so concatenating several
/// pages yields the site rule instead of counting pages. A single page with
/// unique ids reduces to itself.
#[must_use]
pub fn compute_metrics(criteria: &[CriterionResult], referentiel: &Referentiel) -> SiteMetrics {
    let mut conformes = 0;
    let mut non_conformes = 0;
    let mut non_applicables = 0;
    let mut non_testes = 0;
    let mut validated_total = 0;
    let mut validated_executed = 0;
    let mut audit_incomplet = false;
    let mut par_critere: HashMap<&str, Vec<CriterionStatus>> = HashMap::new();

    for criterion in criteria {
        if criterion.status == CriterionStatus::NotTested {
            audit_incomplet = true;
        }
        par_critere
            .entry(criterion.criterion_id.as_str())
            .or_default()
            .push(criterion.status.clone());
    }
    for (id, statuts) in &par_critere {
        if let Some((_theme, cat)) = RgaaCatalog::by_id(id) {
            if matches!(
                cat.automatable,
                Automatable::FullyAutomatable | Automatable::PartiallyAutomatable
            ) {
                validated_total += 1;
                if is_validated(statuts) {
                    validated_executed += 1;
                }
            }
        }
        match reduire_statuts(statuts) {
            ConformityStatus::Conforme => conformes += 1,
            ConformityStatus::NonConforme => non_conformes += 1,
            ConformityStatus::NonApplicable => non_applicables += 1,
            ConformityStatus::NonTeste => non_testes += 1,
        }
    }

    let taux_global = if conformes + non_conformes > 0 {
        (conformes as f64 / (conformes + non_conformes) as f64) * 100.0
    } else {
        0.0
    };
    let coverage_percent = if validated_total > 0 {
        (validated_executed as f64 / validated_total as f64) * 100.0
    } else {
        0.0
    };
    let etat_conformite = if !referentiel.taux_juridique {
        String::new()
    } else if referentiel.retrograde_si_non_teste && audit_incomplet {
        "non conforme".to_string()
    } else if taux_global >= referentiel.seuil_total {
        "totale".to_string()
    } else if taux_global >= referentiel.seuil_partiel {
        "partielle".to_string()
    } else {
        "non conforme".to_string()
    };

    SiteMetrics {
        taux_global,
        coverage_percent,
        etat_conformite,
        conformes,
        non_conformes,
        non_applicables,
        non_testes,
        audit_incomplet,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rgaa_core::Classification;

    fn result(id: &str, status: CriterionStatus) -> CriterionResult {
        CriterionResult {
            criterion_id: id.into(),
            title: "test".into(),
            classification: Classification::Deterministe,
            status,
            violations: Vec::new(),
            confidence: None,
            justification: None,
            source: "test".into(),
            citations: Vec::new(),
            considered_sources: vec![],
            tests: vec![],
        }
    }

    const UE: Referentiel = Referentiel {
        id: "ue-test",
        seuil_total: 100.0,
        seuil_partiel: 50.0,
        retrograde_si_non_teste: false,
        taux_juridique: false,
    };

    #[test]
    fn seuils_fr_100_50() {
        let full = vec![result("1.1", CriterionStatus::Pass)];
        let m = compute_metrics(&full, &RGAA_41);
        assert_eq!(m.taux_global, 100.0);
        assert_eq!(m.etat_conformite, "totale");

        let half = vec![
            result("1.1", CriterionStatus::Pass),
            result("1.2", CriterionStatus::Fail),
        ];
        let m = compute_metrics(&half, &RGAA_41);
        assert_eq!(m.taux_global, 50.0);
        assert_eq!(m.etat_conformite, "partielle");

        let low = vec![result("1.1", CriterionStatus::Fail)];
        let m = compute_metrics(&low, &RGAA_41);
        assert_eq!(m.etat_conformite, "non conforme");
    }

    #[test]
    fn na_nt_exclus_du_taux() {
        let criteria = vec![
            result("1.1", CriterionStatus::Pass),
            result("1.2", CriterionStatus::NotApplicable),
            result("1.4", CriterionStatus::Pass),
        ];
        let m = compute_metrics(&criteria, &UE);
        assert_eq!(m.taux_global, 100.0);
        assert_eq!(m.conformes, 2);
        assert_eq!(m.non_conformes, 0);
        assert!(!m.audit_incomplet);
    }

    #[test]
    fn nt_retrograde_fr_uniquement() {
        let criteria = vec![
            result("1.1", CriterionStatus::Pass),
            result("1.2", CriterionStatus::NotTested),
        ];
        let fr = compute_metrics(&criteria, &RGAA_41);
        assert_eq!(fr.taux_global, 100.0);
        assert!(fr.audit_incomplet);
        assert_eq!(fr.etat_conformite, "non conforme");

        let ue = compute_metrics(&criteria, &UE);
        assert!(ue.audit_incomplet);
        assert!(ue.etat_conformite.is_empty());
    }

    #[test]
    fn needs_review_ne_retrograde_pas() {
        let criteria = vec![
            result("1.1", CriterionStatus::Pass),
            result("1.2", CriterionStatus::NeedsReview),
        ];
        let m = compute_metrics(&criteria, &RGAA_41);
        assert!(!m.audit_incomplet);
        assert_eq!(m.etat_conformite, "totale");
    }

    #[test]
    fn vide_vaut_zero_non_conforme() {
        let m = compute_metrics(&[], &RGAA_41);
        assert_eq!(m.taux_global, 0.0);
        assert_eq!(m.coverage_percent, 0.0);
        assert_eq!(m.etat_conformite, "non conforme");
    }

    #[test]
    fn couverture_comptee_sur_automatisables() {
        // 1.1 and 1.2 are partially automatable, 1.4 is not: 2 tracked, 1 run.
        let criteria = vec![
            result("1.1", CriterionStatus::Pass),
            result("1.2", CriterionStatus::NotTested),
            result("1.4", CriterionStatus::Pass),
        ];
        let m = compute_metrics(&criteria, &UE);
        assert!((m.coverage_percent - 50.0).abs() < 0.01);
    }

    /// A criterion that went to review, or whose evaluation errored, has no
    /// verdict, so it must not count as covered. 1.1 and 1.2 are tracked.
    #[test]
    fn review_and_error_are_not_counted_as_covered() {
        for open in [
            CriterionStatus::NeedsReview,
            CriterionStatus::Error,
            CriterionStatus::NotTested,
        ] {
            let criteria = vec![
                result("1.1", CriterionStatus::Pass),
                result("1.2", open.clone()),
            ];
            let m = compute_metrics(&criteria, &UE);
            assert!(
                (m.coverage_percent - 50.0).abs() < 0.01,
                "{open:?} must not be credited as tested, got {}",
                m.coverage_percent
            );
        }
    }

    #[test]
    fn a_verdict_is_a_fail_or_all_pages_pass_or_not_applicable() {
        use CriterionStatus::{Error, Fail, NeedsReview, NotApplicable, NotTested, Pass};
        assert!(is_validated(&[Pass, Pass]));
        assert!(is_validated(&[Pass, NotApplicable]));
        assert!(is_validated(&[NotApplicable]));
        // One failing page decides the criterion for the whole site.
        assert!(is_validated(&[Fail, NotTested]));
        assert!(is_validated(&[Fail, NeedsReview, Error]));
        // No verdict: something is still open on a page.
        assert!(!is_validated(&[Pass, NeedsReview]));
        assert!(!is_validated(&[Pass, NotTested]));
        assert!(!is_validated(&[Pass, Error]));
        assert!(!is_validated(&[NeedsReview]));
    }

    /// #203 recommendation 4: the rate never travels without the coverage it rests on.
    #[test]
    fn the_conformance_statement_carries_both_numbers() {
        let m = SiteMetrics {
            taux_global: 34.285_714_285_714_285,
            coverage_percent: 76.744_186_046_511_63,
            etat_conformite: "non conforme".into(),
            conformes: 12,
            non_conformes: 23,
            non_applicables: 30,
            non_testes: 41,
            audit_incomplet: true,
        };

        let statement = m.conformance_statement();
        assert!(statement.contains("34.29"), "{statement}");
        assert!(statement.contains("76.74"), "{statement}");
        assert!(
            statement.contains("surface testable"),
            "the coverage must be named, not just printed: {statement}"
        );
    }

    /// The parisprivate baseline is the case this guards against: 81.08 % presented as a
    /// conformance claim over an audit that had never tested 48 criteria.
    #[test]
    fn an_incomplete_audit_rate_is_not_a_conformance_claim() {
        let mut m = SiteMetrics {
            taux_global: 81.08,
            coverage_percent: 100.0,
            etat_conformite: "partielle".into(),
            conformes: 30,
            non_conformes: 7,
            non_applicables: 20,
            non_testes: 49,
            audit_incomplet: true,
        };
        assert!(!m.rate_is_a_conformance_claim());

        m.audit_incomplet = false;
        m.non_testes = 0;
        assert!(m.rate_is_a_conformance_claim());
    }

    /// Sourcery on #209: `compute_metrics` raises `audit_incomplet` for a `NotTested`
    /// criterion but not for one awaiting review, so gating on that field alone let an
    /// all-`NeedsReview` audit read as a conformance claim.
    #[test]
    fn criteria_awaiting_review_are_not_a_conformance_claim_either() {
        let m = SiteMetrics {
            taux_global: 100.0,
            coverage_percent: 100.0,
            etat_conformite: "totale".into(),
            conformes: 20,
            non_conformes: 0,
            non_applicables: 10,
            non_testes: 7,
            audit_incomplet: false,
        };
        assert!(
            !m.rate_is_a_conformance_claim(),
            "7 criteria nobody closed is not a 100 % conformance claim"
        );
    }
}
