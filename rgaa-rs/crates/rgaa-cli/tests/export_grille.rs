use std::path::PathBuf;

use rgaa_cli::commands::export::{self, GrilleArgs};
use rgaa_cli::commands::CommonArgs;
use rgaa_core::{AuditResult, Classification, CriterionResult, CriterionStatus, PageResult};

fn temp_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("rgaa-export-grille-{name}-{}", std::process::id()))
}

fn result(id: &str, status: CriterionStatus, verified: Option<CriterionStatus>) -> CriterionResult {
    CriterionResult {
        criterion_id: id.into(),
        title: "titre de test".into(),
        classification: Classification::IaAssiste,
        status,
        violations: Vec::new(),
        confidence: None,
        raw_confidence: None,
        justification: None,
        source: "agent-estimate".into(),
        citations: Vec::new(),
        considered_sources: Vec::new(),
        tests: Vec::new(),
        automated_verdict: None,
        verdict_basis: Vec::new(),
        evidence: Vec::new(),
        confidence_calibration_version: None,
        review_required: verified.is_none(),
        review_reason: None,
        verified_status: verified,
        review_events: Vec::new(),
    }
}

fn audit() -> AuditResult {
    AuditResult {
        audit_id: "audit-1".into(),
        audit_type: rgaa_core::AuditType::Complet,
        url: "https://example.test".into(),
        pages: vec![
            PageResult {
                url: "https://example.test/".into(),
                title: Some("Accueil".into()),
                criteria: vec![
                    result("1.1", CriterionStatus::Fail, Some(CriterionStatus::Pass)),
                    result("1.2", CriterionStatus::Pass, None),
                ],
                compliance_rate: 0.0,
                crawl_depth: 0,
            },
            PageResult {
                url: "https://example.test/contact".into(),
                title: None,
                criteria: vec![result(
                    "1.1",
                    CriterionStatus::Pass,
                    Some(CriterionStatus::NotApplicable),
                )],
                compliance_rate: 0.0,
                crawl_depth: 0,
            },
        ],
        total_criteria: 106,
        passed: 0,
        failed: 0,
        na: 0,
        overall_compliance: 0.0,
        taux_global: 0.0,
        coverage_percent: 0.0,
        automatic_verdict_coverage_percent: 0.0,
        test_evidence_coverage_percent: 0.0,
        verified_compliance_percent: 0.0,
        etat_conformite: "non conforme".into(),
        duration_ms: 0,
        audit_complete: false,
    }
}

fn common(format: Option<&str>, output: Option<PathBuf>) -> CommonArgs {
    CommonArgs {
        config: None,
        output,
        format: format.map(str::to_string),
        audit_id: None,
        log_file: None,
    }
}

#[test]
fn export_grille_writes_the_ara_csv_grid() {
    let audit_path = temp_path("audit.json");
    let output = temp_path("grille.csv");
    std::fs::write(&audit_path, serde_json::to_string(&audit()).unwrap()).unwrap();

    let args = GrilleArgs {
        common: common(Some("csv"), Some(output.clone())),
        audit: audit_path,
    };
    assert_eq!(export::run(args).unwrap(), 0);

    let csv = std::fs::read_to_string(&output).unwrap();
    let lines: Vec<&str> = csv.lines().collect();
    assert_eq!(
        lines[0],
        "Critères,Éléments transverses,Accueil,https://example.test/contact"
    );
    // Human decisions only; the un-reviewed axe/agent pass stays NT.
    assert_eq!(lines[1], "1.1,NT,C,NA");
    assert_eq!(lines[2], "1.2,NT,NT,NT");
    assert_eq!(lines.len(), 107, "header + 106 criteria");
}

#[test]
fn export_grille_rejects_any_format_other_than_csv() {
    let args = GrilleArgs {
        common: common(Some("json"), None),
        audit: temp_path("unused.json"),
    };
    let error = export::run(args).expect_err("only csv is available");
    assert_eq!(error.exit_code(), 2);
    assert!(error.to_string().contains("csv"), "{error}");
}

#[test]
fn export_grille_missing_audit_is_an_execution_error() {
    let args = GrilleArgs {
        common: common(None, None),
        audit: PathBuf::from("/nonexistent/audit.json"),
    };
    let error = export::run(args).expect_err("missing audit");
    assert_eq!(error.exit_code(), 3);
}

#[test]
fn export_grille_invalid_json_is_an_invalid_input() {
    let path = temp_path("invalid.json");
    std::fs::write(&path, "{not json}").unwrap();
    let args = GrilleArgs {
        common: common(None, None),
        audit: path,
    };
    let error = export::run(args).expect_err("invalid audit");
    assert_eq!(error.exit_code(), 2);
}
