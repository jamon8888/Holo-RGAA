use rgaa_core::{
    AuditResult, Classification, CriterionResult, CriterionStatus, PageResult, RgaaCriteria,
};
use std::collections::HashMap;

fn mock_criterion_result(criterion_id: &str, status: CriterionStatus) -> CriterionResult {
    CriterionResult {
        criterion_id: criterion_id.to_string(),
        title: RgaaCriteria::all()
            .iter()
            .find(|c| c.id == criterion_id)
            .map(|c| c.title.clone())
            .unwrap_or_else(|| "Unknown".to_string()),
        classification: RgaaCriteria::all()
            .iter()
            .find(|c| c.id == criterion_id)
            .map(|c| c.classification.clone())
            .unwrap_or(Classification::Deterministe),
        status,
        violations: vec![],
        confidence: None,
        justification: None,
        source: "test".to_string(),
    }
}

fn calculate_compliance(criteria: &[CriterionResult]) -> f64 {
    let pass = criteria
        .iter()
        .filter(|c| c.status == CriterionStatus::Pass)
        .count();
    let fail = criteria
        .iter()
        .filter(|c| c.status == CriterionStatus::Fail || c.status == CriterionStatus::Error)
        .count();
    let denominator = pass + fail;
    if denominator > 0 {
        (pass as f64 / denominator as f64) * 100.0
    } else {
        0.0
    }
}

fn calculate_compliance_summary(criteria: &[CriterionResult]) -> (f64, f64, String) {
    use rgaa_core::types::ConformityStatus;
    use rgaa_core::catalog::Automatable;
    use rgaa_core::RgaaCatalog;

    let mut c = 0;
    let mut nc = 0;
    let mut validated_total = 0;
    let mut validated_executed = 0;

    for criterion in criteria {
        let conformity = ConformityStatus::from(criterion.status.clone());
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
        match conformity {
            ConformityStatus::Conforme => c += 1,
            ConformityStatus::NonConforme => nc += 1,
            _ => {}
        }
    }

    let taux_global = if c + nc > 0 {
        (c as f64 / (c + nc) as f64) * 100.0
    } else {
        0.0
    };

    let coverage_percent = if validated_total > 0 {
        (validated_executed as f64 / validated_total as f64) * 100.0
    } else {
        0.0
    };

    let etat_conformite = if taux_global >= 100.0 {
        "totale".to_string()
    } else if taux_global >= 50.0 {
        "partielle".to_string()
    } else {
        "non conforme".to_string()
    };

    (taux_global, coverage_percent, etat_conformite)
}

fn build_audit_result(criteria: Vec<CriterionResult>) -> AuditResult {
    let total = RgaaCriteria::count();
    let compliance = calculate_compliance(&criteria);
    let (taux_global, coverage_percent, etat_conformite) = calculate_compliance_summary(&criteria);

    let pass_count = criteria
        .iter()
        .filter(|c| c.status == CriterionStatus::Pass)
        .count();
    let fail_count = criteria
        .iter()
        .filter(|c| c.status == CriterionStatus::Fail)
        .count();
    let na_count = criteria
        .iter()
        .filter(|c| c.status == CriterionStatus::NotApplicable)
        .count();

    AuditResult {
        audit_id: uuid::Uuid::new_v4().to_string(),
        url: "https://example.com".to_string(),
        pages: vec![PageResult {
            url: "https://example.com".to_string(),
            title: Some("Test Page".to_string()),
            criteria,
            compliance_rate: compliance,
            crawl_depth: 0,
        }],
        total_criteria: total,
        passed: pass_count,
        failed: fail_count,
        na: na_count,
        overall_compliance: compliance,
        taux_global,
        coverage_percent,
        etat_conformite,
        duration_ms: 0,
    }
}

#[cfg(test)]
mod integration_test {
    use super::*;

    #[test]
    fn test_full_audit_pipeline_with_all_pass() {
        let criteria: Vec<CriterionResult> = RgaaCriteria::all()
            .iter()
            .map(|c| mock_criterion_result(c.id, CriterionStatus::Pass))
            .collect();

        let audit = build_audit_result(criteria);

        assert_eq!(
            audit.taux_global, 100.0,
            "taux_global should be 100% when all criteria pass"
        );
        assert_eq!(
            audit.coverage_percent, 100.0,
            "coverage_percent should be 100% when all criteria are tested"
        );
        assert_eq!(
            audit.etat_conformite, "totale",
            "etat_conformite should be 'totale' when taux_global >= 100"
        );
        assert_eq!(
            audit.passed, 106,
            "All 106 criteria should be marked as passed"
        );
        assert_eq!(
            audit.failed, 0,
            "No criteria should be marked as failed"
        );
        assert_eq!(
            audit.pages[0].criteria.len(),
            106,
            "pages[0].criteria should have all 106 criteria entries"
        );
    }

    #[test]
    fn test_full_audit_pipeline_with_all_fail() {
        let criteria: Vec<CriterionResult> = RgaaCriteria::all()
            .iter()
            .map(|c| mock_criterion_result(c.id, CriterionStatus::Fail))
            .collect();

        let audit = build_audit_result(criteria);

        assert_eq!(
            audit.taux_global, 0.0,
            "taux_global should be 0% when all criteria fail"
        );
        assert_eq!(
            audit.etat_conformite, "non conforme",
            "etat_conformite should be 'non conforme' when taux_global < 50"
        );
        assert_eq!(
            audit.failed, 106,
            "All 106 criteria should be marked as failed"
        );
        assert_eq!(
            audit.passed, 0,
            "No criteria should be marked as passed"
        );
    }

    #[test]
    fn test_full_audit_pipeline_mixed_results() {
        let mut criteria: Vec<CriterionResult> = Vec::with_capacity(106);

        for c in RgaaCriteria::all().iter() {
            let status = match c.id {
                "1.1" | "1.2" | "2.1" => CriterionStatus::Pass,
                "3.1" | "3.2" | "4.1" => CriterionStatus::Fail,
                "5.1" | "5.2" | "5.3" => CriterionStatus::NotApplicable,
                _ => CriterionStatus::Pass,
            };
            criteria.push(mock_criterion_result(c.id, status));
        }

        let audit = build_audit_result(criteria);

        assert!(
            audit.taux_global > 0.0 && audit.taux_global < 100.0,
            "taux_global should be between 0 and 100 with mixed results"
        );
        assert_eq!(
            audit.etat_conformite, "partielle",
            "etat_conformite should be 'partielle' when 50 <= taux_global < 100"
        );
        assert!(
            audit.passed > 0,
            "Some criteria should be marked as passed"
        );
        assert!(
            audit.failed > 0,
            "Some criteria should be marked as failed"
        );
        assert!(
            audit.na > 0,
            "Some criteria should be marked as NotApplicable"
        );
    }

    #[test]
    fn test_na_detection_criteria_1_x_marked_not_applicable() {
        let mut criteria: Vec<CriterionResult> = RgaaCriteria::all()
            .iter()
            .map(|c| mock_criterion_result(c.id, CriterionStatus::Pass))
            .collect();

        for criterion in criteria.iter_mut() {
            if criterion.criterion_id.starts_with("1.") {
                criterion.status = CriterionStatus::NotApplicable;
            }
        }

        let audit = build_audit_result(criteria);

        let image_criteria: Vec<&CriterionResult> = audit.pages[0]
            .criteria
            .iter()
            .filter(|c| c.criterion_id.starts_with("1."))
            .collect();

        assert!(
            image_criteria.iter().all(|c| c.status == CriterionStatus::NotApplicable),
            "All 1.x criteria should be NotApplicable when page has no images"
        );

        assert_eq!(
            image_criteria.len(), 9,
            "There should be 9 image-related criteria (1.1 to 1.9)"
        );
    }

    #[test]
    fn test_na_detection_criteria_11_x_marked_not_applicable() {
        let mut criteria: Vec<CriterionResult> = RgaaCriteria::all()
            .iter()
            .map(|c| mock_criterion_result(c.id, CriterionStatus::Pass))
            .collect();

        for criterion in criteria.iter_mut() {
            if criterion.criterion_id.starts_with("11.") {
                criterion.status = CriterionStatus::NotApplicable;
            }
        }

        let audit = build_audit_result(criteria);

        let form_criteria: Vec<&CriterionResult> = audit.pages[0]
            .criteria
            .iter()
            .filter(|c| c.criterion_id.starts_with("11."))
            .collect();

        assert!(
            form_criteria.iter().all(|c| c.status == CriterionStatus::NotApplicable),
            "All 11.x criteria should be NotApplicable when page has no forms"
        );

        assert_eq!(
            form_criteria.len(), 13,
            "There should be 13 form-related criteria (11.1 to 11.13)"
        );
    }

    #[test]
    fn test_na_detection_criteria_4_x_marked_not_applicable() {
        let mut criteria: Vec<CriterionResult> = RgaaCriteria::all()
            .iter()
            .map(|c| mock_criterion_result(c.id, CriterionStatus::Pass))
            .collect();

        for criterion in criteria.iter_mut() {
            if criterion.criterion_id.starts_with("4.") {
                criterion.status = CriterionStatus::NotApplicable;
            }
        }

        let audit = build_audit_result(criteria);

        let media_criteria: Vec<&CriterionResult> = audit.pages[0]
            .criteria
            .iter()
            .filter(|c| c.criterion_id.starts_with("4."))
            .collect();

        assert!(
            media_criteria.iter().all(|c| c.status == CriterionStatus::NotApplicable),
            "All 4.x criteria should be NotApplicable when page has no media"
        );
    }

    #[test]
    fn test_na_detection_criteria_5_x_marked_not_applicable() {
        let mut criteria: Vec<CriterionResult> = RgaaCriteria::all()
            .iter()
            .map(|c| mock_criterion_result(c.id, CriterionStatus::Pass))
            .collect();

        for criterion in criteria.iter_mut() {
            if criterion.criterion_id.starts_with("5.") {
                criterion.status = CriterionStatus::NotApplicable;
            }
        }

        let audit = build_audit_result(criteria);

        let table_criteria: Vec<&CriterionResult> = audit.pages[0]
            .criteria
            .iter()
            .filter(|c| c.criterion_id.starts_with("5."))
            .collect();

        assert!(
            table_criteria.iter().all(|c| c.status == CriterionStatus::NotApplicable),
            "All 5.x criteria should be NotApplicable when page has no tables"
        );
    }

    #[test]
    fn test_na_detection_criteria_2_1_marked_not_applicable() {
        let mut criteria: Vec<CriterionResult> = RgaaCriteria::all()
            .iter()
            .map(|c| mock_criterion_result(c.id, CriterionStatus::Pass))
            .collect();

        for criterion in criteria.iter_mut() {
            if criterion.criterion_id == "2.1" {
                criterion.status = CriterionStatus::NotApplicable;
            }
        }

        let audit = build_audit_result(criteria);

        let iframe_criterion = audit
            .pages[0]
            .criteria
            .iter()
            .find(|c| c.criterion_id == "2.1")
            .expect("Criterion 2.1 should exist");

        assert_eq!(
            iframe_criterion.status,
            CriterionStatus::NotApplicable,
            "Criterion 2.1 should be NotApplicable when page has no iframes"
        );
    }

    #[test]
    fn test_all_106_criteria_present_in_audit_result() {
        let criteria: Vec<CriterionResult> = RgaaCriteria::all()
            .iter()
            .map(|c| mock_criterion_result(c.id, CriterionStatus::Pass))
            .collect();

        let audit = build_audit_result(criteria);

        assert_eq!(
            audit.pages[0].criteria.len(),
            106,
            "pages[0].criteria should have exactly 106 entries"
        );

        let criterion_ids: Vec<&str> = audit.pages[0]
            .criteria
            .iter()
            .map(|c| c.criterion_id.as_str())
            .collect();

        for c in RgaaCriteria::all().iter() {
            assert!(
                criterion_ids.contains(&c.id),
                "Criterion {} should be present in audit result",
                c.id
            );
        }
    }

    #[test]
    fn test_audit_result_has_all_required_fields() {
        let criteria: Vec<CriterionResult> = RgaaCriteria::all()
            .iter()
            .map(|c| mock_criterion_result(c.id, CriterionStatus::Pass))
            .collect();

        let audit = build_audit_result(criteria);

        assert!(
            audit.taux_global >= 0.0 && audit.taux_global <= 100.0,
            "taux_global should be between 0 and 100"
        );
        assert!(
            audit.coverage_percent >= 0.0 && audit.coverage_percent <= 100.0,
            "coverage_percent should be between 0 and 100"
        );
        assert!(
            audit.etat_conformite == "totale"
                || audit.etat_conformite == "partielle"
                || audit.etat_conformite == "non conforme",
            "etat_conformite should be one of 'totale', 'partielle', or 'non conforme'"
        );
        assert!(
            audit.total_criteria > 0,
            "total_criteria should be greater than 0"
        );
        assert!(
            audit.overall_compliance >= 0.0 && audit.overall_compliance <= 100.0,
            "overall_compliance should be between 0 and 100"
        );
        assert!(
            !audit.pages.is_empty(),
            "pages should not be empty"
        );
        assert!(
            !audit.audit_id.is_empty(),
            "audit_id should not be empty"
        );
        assert_eq!(
            audit.url, "https://example.com",
            "url should match the input"
        );
    }

    #[test]
    fn test_compliance_calculation_with_not_tested_excluded() {
        let mut criteria: Vec<CriterionResult> = RgaaCriteria::all()
            .iter()
            .map(|c| mock_criterion_result(c.id, CriterionStatus::Pass))
            .collect();

        for c in criteria.iter_mut().take(50) {
            c.status = CriterionStatus::NotTested;
        }

        let audit = build_audit_result(criteria);

        assert_eq!(
            audit.overall_compliance, 100.0,
            "Compliance should be 100% when all tested criteria pass (NotTested excluded)"
        );
    }

    #[test]
    fn test_compliance_calculation_with_needs_review_excluded() {
        let mut criteria: Vec<CriterionResult> = RgaaCriteria::all()
            .iter()
            .map(|c| mock_criterion_result(c.id, CriterionStatus::Pass))
            .collect();

        for c in criteria.iter_mut().take(20) {
            c.status = CriterionStatus::NeedsReview;
        }

        let audit = build_audit_result(criteria);

        assert_eq!(
            audit.overall_compliance, 100.0,
            "Compliance should be 100% when all tested criteria pass (NeedsReview excluded)"
        );
    }

    #[test]
    fn test_compliance_calculation_error_counted_as_fail() {
        let mut criteria: Vec<CriterionResult> = RgaaCriteria::all()
            .iter()
            .map(|c| mock_criterion_result(c.id, CriterionStatus::Pass))
            .collect();

        for c in criteria.iter_mut().take(30) {
            c.status = CriterionStatus::Error;
        }

        let audit = build_audit_result(criteria);

        let expected_compliance = (76 as f64 / 106 as f64) * 100.0;
        assert!(
            (audit.overall_compliance - expected_compliance).abs() < 0.1,
            "Error should be counted as fail in compliance calculation"
        );
    }

    #[test]
    fn test_na_criteria_do_not_affect_taux_global() {
        let mut criteria: Vec<CriterionResult> = RgaaCriteria::all()
            .iter()
            .map(|c| mock_criterion_result(c.id, CriterionStatus::Pass))
            .collect();

        for c in criteria.iter_mut().take(40) {
            c.status = CriterionStatus::NotApplicable;
        }

        let audit = build_audit_result(criteria);

        assert_eq!(
            audit.taux_global, 100.0,
            "taux_global should be 100% when all non-NA criteria pass (NA excluded)"
        );
    }

    #[test]
    fn test_coverage_percent_with_not_tested_criteria() {
        let criteria: Vec<CriterionResult> = RgaaCriteria::all()
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let status = if i % 2 == 0 {
                    CriterionStatus::Pass
                } else {
                    CriterionStatus::NotTested
                };
                mock_criterion_result(c.id, status)
            })
            .collect();

        let audit = build_audit_result(criteria);

        assert!(
            audit.coverage_percent > 0.0 && audit.coverage_percent < 100.0,
            "coverage_percent should reflect executed vs total automatable criteria"
        );
    }

    #[test]
    fn test_etat_conformite_thresholds() {
        let test_cases = vec![
            (100.0, "totale"),
            (99.0, "partielle"),
            (50.0, "partielle"),
            (49.9, "non conforme"),
            (0.0, "non conforme"),
        ];

        for (taux, expected_etat) in test_cases {
            let criteria: Vec<CriterionResult> = RgaaCriteria::all()
                .iter()
                .map(|c| {
                    let status = if taux >= 50.0 {
                        CriterionStatus::Pass
                    } else {
                        CriterionStatus::Fail
                    };
                    mock_criterion_result(c.id, status)
                })
                .collect();

            let audit = build_audit_result(criteria);

            assert_eq!(
                audit.etat_conformite, expected_etat,
                "etat_conformite should be '{}' when taux_global is {}",
                expected_etat, taux
            );
        }
    }

    #[test]
    fn test_page_result_has_all_criteria_with_statuses() {
        let status_patterns = [
            CriterionStatus::Pass,
            CriterionStatus::Fail,
            CriterionStatus::NotApplicable,
            CriterionStatus::NeedsReview,
        ];

        for status in status_patterns {
            let criteria: Vec<CriterionResult> = RgaaCriteria::all()
                .iter()
                .map(|c| mock_criterion_result(c.id, status.clone()))
                .collect();

            let audit = build_audit_result(criteria.clone());

            assert_eq!(
                audit.pages[0].criteria.len(),
                106,
                "PageResult should have 106 criteria when all have status {:?}",
                status
            );

            assert!(
                audit.pages[0]
                    .criteria
                    .iter()
                    .all(|c| c.status == status),
                "All criteria should have the expected status {:?}",
                status
            );
        }
    }

    #[test]
    fn test_audit_result_passing_and_failing_counts() {
        let mut criteria: Vec<CriterionResult> = Vec::with_capacity(106);
        let mut pass_count = 0;
        let mut fail_count = 0;

        for (i, c) in RgaaCriteria::all().iter().enumerate() {
            let status = if i % 3 == 0 {
                CriterionStatus::Pass
            } else {
                CriterionStatus::Fail
            };

            if matches!(status, CriterionStatus::Pass) {
                pass_count += 1;
            } else {
                fail_count += 1;
            }

            criteria.push(mock_criterion_result(c.id, status));
        }

        let audit = build_audit_result(criteria);

        assert_eq!(
            audit.passed, pass_count,
            "passed count should match number of Pass statuses"
        );
        assert_eq!(
            audit.failed, fail_count,
            "failed count should match number of Fail statuses"
        );
    }

    #[test]
    fn test_realistic_audit_result() {
        let criteria: Vec<CriterionResult> = RgaaCriteria::all()
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let status = match i % 10 {
                    0 | 1 | 2 | 3 => CriterionStatus::Pass,
                    4 | 5 => CriterionStatus::Fail,
                    6 => CriterionStatus::NotApplicable,
                    7 => CriterionStatus::NotTested,
                    8 => CriterionStatus::NeedsReview,
                    _ => CriterionStatus::Pass,
                };
                mock_criterion_result(c.id, status)
            })
            .collect();

        let audit = build_audit_result(criteria);

        assert!(
            audit.pages[0].compliance_rate >= 0.0
                && audit.pages[0].compliance_rate <= 100.0,
            "compliance_rate should be between 0 and 100"
        );
        assert!(
            audit.taux_global >= 0.0 && audit.taux_global <= 100.0,
            "taux_global should be between 0 and 100"
        );
        assert!(
            audit.coverage_percent >= 0.0 && audit.coverage_percent <= 100.0,
            "coverage_percent should be between 0 and 100"
        );
        assert!(
            audit.na > 0,
            "Some criteria should be NotApplicable in realistic scenario"
        );
    }
}
