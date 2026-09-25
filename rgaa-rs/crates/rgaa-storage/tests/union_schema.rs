//! Ticket 03 seam: `PostgresStorage::new` must provision every table and
//! column the live API paths touch — legacy `/audit` reads (self-created
//! `audits`/`audit_logs` shape) plus `/v1` bundle upsert, findings listing,
//! and Bearer auth — so a fresh derivable database serves `/v1` without 500s.
//!
//! Requires `DATABASE_URL` pointing at a **disposable** database (rows are
//! created and deleted). Skipped when unset.

use rgaa_core::{
    AuditBundle, AuditResult, CheckpointResult, Classification, CriterionResult, CriterionStatus,
    PageResult, Violation,
};
use rgaa_storage::{PostgresStorage, Repository, Storage};

#[tokio::test]
async fn union_schema_supports_every_live_api_path() {
    let url = match std::env::var("DATABASE_URL") {
        Ok(url) => url,
        Err(_) => {
            eprintln!("skipping union_schema test: DATABASE_URL not set");
            return;
        }
    };

    let storage = PostgresStorage::new(&url)
        .await
        .expect("connect and provision schema");

    // Legacy path: save_audit / get_audit / list_audits / logs (pipeline).
    let audit_id = uuid::Uuid::new_v4().to_string();
    let mut result = sample_audit_result(&audit_id);
    result.taux_global = 83.33;
    let saved_id = storage.save_audit(&result).await.expect("save_audit");
    assert_eq!(saved_id, audit_id);

    let loaded = storage
        .get_audit(&audit_id)
        .await
        .expect("get_audit")
        .expect("saved audit readable");
    assert_eq!(loaded.taux_global, 83.33);

    let listed = storage.list_audits(50, 0).await.expect("list_audits");
    assert!(listed.iter().any(|a| a.id == audit_id));

    storage
        .save_audit_log(&audit_id, "test", None)
        .await
        .expect("save_audit_log");

    // /v1 path: put_bundle (audits + findings + checkpoints + versions).
    let mut bundle = AuditBundle::from(sample_audit_result(&audit_id));
    bundle.checkpoints.push(CheckpointResult {
        checkpoint_id: "cp-1".into(),
        criterion_id: "1.1".into(),
        status: CriterionStatus::Fail,
        evidence: Vec::new(),
        summary: "manual check failed".into(),
    });
    storage.put_bundle(&bundle).await.expect("put_bundle");

    let fetched = storage
        .get_bundle_by_audit_id(&audit_id)
        .await
        .expect("get_bundle_by_audit_id")
        .expect("bundle readable by audit_id");
    assert_eq!(fetched.audit_id, audit_id);
    assert_eq!(fetched.findings.len(), 1);
    assert_eq!(fetched.checkpoints.len(), 1);

    // /v1/findings path: Repository::list_findings against the findings table.
    let repo = Repository::new(storage.pool());
    let findings = repo
        .list_findings(
            uuid::Uuid::parse_str(&audit_id).unwrap(),
            None,
            None,
            None,
            100,
            0,
        )
        .await
        .expect("list_findings");
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].rule, "image-alt");
    assert_eq!(findings[0].criterion_id.as_deref(), Some("1.1"));
    assert_eq!(findings[0].audit_id.to_string(), audit_id);

    // Auth path: api_keys provisioning + validation.
    let scopes = vec!["audit:write".to_string()];
    let (_, plain_key) = repo
        .create_api_key("union-schema-test", &scopes, None)
        .await
        .expect("create_api_key");
    let validated = repo
        .validate_api_key(&plain_key, "audit:write")
        .await
        .expect("validate_api_key");
    assert!(validated.is_some(), "created key must validate");
    assert!(repo
        .validate_api_key("not-a-real-key", "audit:write")
        .await
        .expect("validate_api_key")
        .is_none());

    // Cleanup so reruns against the same database stay deterministic.
    storage.delete_audit(&audit_id).await.expect("cleanup");
    let _ = storage.delete_audit(&saved_id).await;
}

fn sample_audit_result(audit_id: &str) -> AuditResult {
    AuditResult {
        audit_id: audit_id.to_string(),
        url: "https://example.test".to_string(),
        pages: vec![PageResult {
            url: "https://example.test".to_string(),
            title: Some("Home".into()),
            criteria: vec![CriterionResult {
                criterion_id: "1.1".into(),
                title: "Image alt".into(),
                classification: Classification::Deterministe,
                status: CriterionStatus::Fail,
                violations: vec![Violation {
                    rule_id: "image-alt".into(),
                    impact: "critical".into(),
                    description: "Missing alt".into(),
                    nodes_affected: 2,
                }],
                confidence: None,
                justification: None,
                source: "axe".into(),
                citations: vec![],
            }],
            compliance_rate: 0.0,
            crawl_depth: 0,
        }],
        total_criteria: 106,
        passed: 50,
        failed: 10,
        na: 46,
        overall_compliance: 83.33,
        taux_global: 83.33,
        coverage_percent: 56.6,
        etat_conformite: "partielle".into(),
        duration_ms: 1000,
    }
}
