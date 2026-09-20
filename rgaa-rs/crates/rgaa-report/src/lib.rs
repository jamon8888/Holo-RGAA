//! Single source of truth for accessibility compliance computation.
//!
//! Every caller (orchestrator, CLI, TUI) must go through this crate so one
//! audit always yields the same figures. Computation is parameterized by
//! [`Referentiel`]: one constant per national framework, no duplication.

use rgaa_core::catalog::Automatable;
use rgaa_core::{ConformityStatus, CriterionResult, CriterionStatus, RgaaCatalog};

pub mod declaration;
pub mod depot;
pub mod format;
pub mod guard;
pub mod packs;
pub mod pdf;
pub mod report;
pub mod ue;

pub use declaration::{render_declaration_fr, DeclarationFrInput, NcEntry};
pub use depot::url_canonique;
pub use format::ReportFormat;
pub use guard::{
    schema_export_pack, validate_export, Contact, ContenuNonSoumis, Derogation, ExportPack,
    PageEchantillon, ECHANTILLON_MIN,
};
pub use packs::{mention_fr, pack, PackPays, Pays};
pub use report::render;
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
    /// True when at least one criterion was never tested.
    pub audit_incomplet: bool,
}

/// Per-page rate: passing over passing plus failing, NA/NT excluded.
#[must_use]
pub fn compliance_rate(criteria: &[CriterionResult]) -> f64 {
    let pass = criteria
        .iter()
        .filter(|c| c.status == CriterionStatus::Pass)
        .count();
    let fail = criteria
        .iter()
        .filter(|c| c.status == CriterionStatus::Fail || c.status == CriterionStatus::Error)
        .count();
    if pass + fail > 0 {
        (pass as f64 / (pass + fail) as f64) * 100.0
    } else {
        0.0
    }
}

/// Site-wide metrics for `criteria` under `referentiel`.
#[must_use]
pub fn compute_metrics(criteria: &[CriterionResult], referentiel: &Referentiel) -> SiteMetrics {
    let mut conformes = 0;
    let mut non_conformes = 0;
    let mut validated_total = 0;
    let mut validated_executed = 0;
    let mut audit_incomplet = false;

    for criterion in criteria {
        if criterion.status == CriterionStatus::NotTested {
            audit_incomplet = true;
        }
        if let Some((_theme, cat)) = RgaaCatalog::by_id(&criterion.criterion_id) {
            if matches!(
                cat.automatable,
                Automatable::FullyAutomatable | Automatable::PartiallyAutomatable
            ) {
                validated_total += 1;
                if criterion.status != CriterionStatus::NotTested {
                    validated_executed += 1;
                }
            }
        }
        match ConformityStatus::from(criterion.status.clone()) {
            ConformityStatus::Conforme => conformes += 1,
            ConformityStatus::NonConforme => non_conformes += 1,
            ConformityStatus::NonApplicable | ConformityStatus::NonTeste => {}
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
}
