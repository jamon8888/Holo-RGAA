use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Classification {
    Deterministe,
    IaAssiste,
    Manuel,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CriterionStatus {
    Pass,
    Fail,
    NotApplicable,
    Error,
    NeedsReview,
    NotTested,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ConformityStatus {
    Conforme,
    NonConforme,
    NonApplicable,
    NonTeste,
}

impl From<CriterionStatus> for ConformityStatus {
    fn from(status: CriterionStatus) -> Self {
        match status {
            CriterionStatus::Pass => ConformityStatus::Conforme,
            CriterionStatus::Fail => ConformityStatus::NonConforme,
            CriterionStatus::NotApplicable => ConformityStatus::NonApplicable,
            CriterionStatus::NeedsReview | CriterionStatus::NotTested => ConformityStatus::NonTeste,
            CriterionStatus::Error => ConformityStatus::NonConforme,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CriterionResult {
    pub criterion_id: String,
    pub title: String,
    pub classification: Classification,
    pub status: CriterionStatus,
    pub violations: Vec<Violation>,
    pub confidence: Option<f64>,
    pub justification: Option<String>,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Violation {
    pub rule_id: String,
    pub impact: String,
    pub description: String,
    pub nodes_affected: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PageResult {
    pub url: String,
    pub title: Option<String>,
    pub criteria: Vec<CriterionResult>,
    pub compliance_rate: f64,
    pub crawl_depth: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AuditResult {
    pub audit_id: String,
    pub url: String,
    pub pages: Vec<PageResult>,
    pub total_criteria: usize,
    pub passed: usize,
    pub failed: usize,
    pub na: usize,
    pub overall_compliance: f64,
    pub taux_global: f64,
    pub coverage_percent: f64,
    pub etat_conformite: String,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrawlConfig {
    pub max_pages: usize,
    pub max_depth: u32,
    pub respect_robots: bool,
    pub sample_mode: bool,
}

impl Default for CrawlConfig {
    fn default() -> Self {
        Self {
            max_pages: 50,
            max_depth: 5,
            respect_robots: true,
            sample_mode: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn criterion_statuses_have_stable_json_names() {
        let statuses = [
            (CriterionStatus::Pass, "pass"),
            (CriterionStatus::Fail, "fail"),
            (CriterionStatus::NotApplicable, "not_applicable"),
            (CriterionStatus::Error, "error"),
            (CriterionStatus::NeedsReview, "needs_review"),
            (CriterionStatus::NotTested, "not_tested"),
        ];

        for (status, expected) in statuses {
            assert_eq!(
                serde_json::to_string(&status).unwrap(),
                format!("\"{expected}\"")
            );
        }

        assert!(serde_json::from_str::<CriterionStatus>("\"na\"").is_err());
    }

    #[test]
    fn test_status_mapping() {
        assert_eq!(
            ConformityStatus::from(CriterionStatus::Pass),
            ConformityStatus::Conforme
        );
        assert_eq!(
            ConformityStatus::from(CriterionStatus::Fail),
            ConformityStatus::NonConforme
        );
        assert_eq!(
            ConformityStatus::from(CriterionStatus::NotApplicable),
            ConformityStatus::NonApplicable
        );
        assert_eq!(
            ConformityStatus::from(CriterionStatus::NeedsReview),
            ConformityStatus::NonTeste
        );
        assert_eq!(
            ConformityStatus::from(CriterionStatus::NotTested),
            ConformityStatus::NonTeste
        );
        assert_eq!(
            ConformityStatus::from(CriterionStatus::Error),
            ConformityStatus::NonConforme
        );
    }
}
