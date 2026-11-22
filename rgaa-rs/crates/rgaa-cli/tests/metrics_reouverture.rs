use std::path::{Path, PathBuf};

use rgaa_cli::commands::metrics::{run, ReouvertureArgs};
use rgaa_core::{AuditBundle, AuditConfig, CriterionStatus, NonConformity, PageAudit};

fn temp_dir(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("rgaa-reouverture-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("temp dir");
    path
}

fn result(criterion_id: &str, status: CriterionStatus) -> rgaa_core::CriterionResult {
    rgaa_core::CriterionResult {
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

fn bundle(id: &str, statuses: &[(&str, CriterionStatus)]) -> AuditBundle {
    let url = "https://example.test/a";
    let mut audit = AuditBundle::new(id, "https://example.test", AuditConfig::default());
    audit.pages.push(PageAudit {
        page_id: "page-0".into(),
        url: url.into(),
        title: None,
        criteria: statuses
            .iter()
            .map(|(criterion, status)| result(criterion, (*status).clone()))
            .collect(),
        findings: Vec::new(),
        errors: Vec::new(),
        completed: true,
        duration_ms: 0,
    });
    audit.non_conformities = statuses
        .iter()
        .enumerate()
        .map(|(index, (criterion, _))| {
            NonConformity::new(
                format!("nc-{index}"),
                *criterion,
                url,
                "constat du relecteur",
            )
        })
        .collect();
    audit
}

fn write_bundle(dir: &Path, name: &str, audit: &AuditBundle) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(
        &path,
        serde_json::to_string_pretty(audit).expect("serialize"),
    )
    .expect("write bundle");
    path
}

fn args(audit: Option<PathBuf>, output: PathBuf) -> ReouvertureArgs {
    ReouvertureArgs {
        audit,
        all: false,
        input: None,
        recursive: false,
        output: Some(output),
    }
}

#[test]
fn reouverture_d_un_bundle_sort_le_json_export() {
    let dir = temp_dir("single");
    let path = write_bundle(
        &dir,
        "audit.json",
        &bundle(
            "audit-1",
            &[
                ("1.1", CriterionStatus::Pass),
                ("1.2", CriterionStatus::Fail),
            ],
        ),
    );
    let output = dir.join("export.json");
    assert_eq!(run(args(Some(path), output.clone())).expect("run"), 0);

    let json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&output).expect("read export"))
            .expect("valid export");
    assert_eq!(json["schema"], "reouverture-1");
    // Un Pass = rouverte, un Fail = signalée => 50 %.
    assert_eq!(json["global"]["rouvertes"], 1);
    assert_eq!(json["global"]["signalees"], 1);
    assert_eq!(json["global"]["indeterminables"], 0);
    assert_eq!(json["global"]["taux_reouverture_percent"], 50.0);
    assert_eq!(json["par_audit"][0]["audit_id"], "audit-1");
    assert_eq!(json["par_critere"]["1.1"]["rouvertes"], 1);
    assert_eq!(json["par_plugin"].as_object().map_or(0, |o| o.len()), 0);
}

#[test]
fn all_agrege_les_bundles_et_ignore_les_autres_json() {
    let dir = temp_dir("all");
    write_bundle(
        &dir,
        "b.json",
        &bundle("audit-b", &[("2.1", CriterionStatus::Fail)]),
    );
    write_bundle(
        &dir,
        "a.json",
        &bundle("audit-a", &[("2.1", CriterionStatus::Pass)]),
    );
    std::fs::write(dir.join("not-a-bundle.json"), "{\"hello\":\"world\"}").expect("write junk");
    std::fs::write(dir.join("README.txt"), "ignored").expect("write text");

    let output = dir.join("export.json");
    let arguments = ReouvertureArgs {
        audit: None,
        all: true,
        input: Some(dir.clone()),
        recursive: false,
        output: Some(output.clone()),
    };
    assert_eq!(run(arguments).expect("run"), 0);

    let json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&output).expect("read export"))
            .expect("valid export");
    assert_eq!(
        json["par_audit"]
            .as_array()
            .map_or(0, |audits| audits.len()),
        2
    );
    assert_eq!(json["par_audit"][0]["audit_id"], "audit-a");
    assert_eq!(json["par_audit"][1]["audit_id"], "audit-b");
    // Un Pass et un Fail sur 2.1 => 50 %.
    assert_eq!(json["global"]["taux_reouverture_percent"], 50.0);
}

#[test]
fn arguments_incompatibles_sont_refuses() {
    let dir = temp_dir("args");
    let error = run(ReouvertureArgs {
        audit: Some(dir.join("a.json")),
        all: true,
        input: None,
        recursive: false,
        output: None,
    })
    .expect_err("audit et --all s'excluent");
    assert_eq!(error.exit_code(), 2);

    let error = run(ReouvertureArgs {
        audit: None,
        all: false,
        input: None,
        recursive: true,
        output: None,
    })
    .expect_err("--recursive sans --all");
    assert_eq!(error.exit_code(), 2);
}

#[test]
fn bundle_inexistant_est_une_erreur_dexecution() {
    let dir = temp_dir("missing");
    let error =
        run(args(Some(dir.join("absent.json")), dir.join("out.json"))).expect_err("fichier absent");
    assert_eq!(error.exit_code(), 3);
}
