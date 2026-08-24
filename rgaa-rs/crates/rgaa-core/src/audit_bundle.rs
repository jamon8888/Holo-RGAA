use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::{
    AuditResult, CheckpointResult, CrawlConfig, CriterionResult, CriterionStatus, Finding,
    PageError, RgaaError, Violation,
};

pub const CURRENT_SCHEMA_VERSION: &str = "1.0";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AuditConfig {
    pub max_pages: usize,
    pub max_depth: u32,
    pub respect_robots: bool,
    pub sample_mode: bool,
}

impl Default for AuditConfig {
    fn default() -> Self {
        Self {
            max_pages: 50,
            max_depth: 5,
            respect_robots: true,
            sample_mode: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PageAudit {
    pub page_id: String,
    pub url: String,
    pub title: Option<String>,
    pub criteria: Vec<CriterionResult>,
    pub findings: Vec<Finding>,
    pub errors: Vec<PageError>,
    pub completed: bool,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct AuditSummary {
    pub total_pages: usize,
    pub completed_pages: usize,
    pub total_findings: usize,
    pub passed: usize,
    pub failed: usize,
    pub needs_review: usize,
    pub errors: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AuditBundle {
    pub schema_version: String,
    pub audit_id: String,
    pub url: String,
    pub config: AuditConfig,
    pub pages: Vec<PageAudit>,
    pub findings: Vec<Finding>,
    pub checkpoints: Vec<CheckpointResult>,
    pub summary: AuditSummary,
}

impl AuditBundle {
    pub fn new(audit_id: impl Into<String>, url: impl Into<String>, config: AuditConfig) -> Self {
        Self {
            schema_version: CURRENT_SCHEMA_VERSION.to_owned(),
            audit_id: audit_id.into(),
            url: url.into(),
            config,
            pages: Vec::new(),
            findings: Vec::new(),
            checkpoints: Vec::new(),
            summary: AuditSummary::default(),
        }
    }

    /// Validates identifiers, statuses, finding uniqueness, and evidence completeness.
    pub fn validate(&self) -> Result<(), RgaaError> {
        if self.audit_id.trim().is_empty() {
            return Err(RgaaError::MissingId("audit_id".into()));
        }
        if self.url.trim().is_empty() {
            return Err(RgaaError::MissingId("url".into()));
        }
        if self.schema_version.trim().is_empty() {
            return Err(RgaaError::MissingId("schema_version".into()));
        }
        if self.schema_version != CURRENT_SCHEMA_VERSION {
            return Err(RgaaError::UnsupportedSchemaVersion(
                self.schema_version.clone(),
            ));
        }

        for page in &self.pages {
            if page.page_id.trim().is_empty() {
                return Err(RgaaError::MissingId("page_id".into()));
            }
            if page.url.trim().is_empty() {
                return Err(RgaaError::MissingId("page.url".into()));
            }
            for criterion in &page.criteria {
                if criterion.criterion_id.trim().is_empty() {
                    return Err(RgaaError::MissingId("criterion_id".into()));
                }
                validate_status(&criterion.status)?;
            }
        }

        let mut finding_ids: HashSet<String> = HashSet::with_capacity(
            self.findings.len()
                + self
                    .pages
                    .iter()
                    .map(|page| page.findings.len())
                    .sum::<usize>(),
        );
        for finding in &self.findings {
            validate_finding(finding, &mut finding_ids)?;
        }
        for page in &self.pages {
            for finding in &page.findings {
                validate_finding(finding, &mut finding_ids)?;
            }
        }

        for checkpoint in &self.checkpoints {
            if checkpoint.checkpoint_id.trim().is_empty() {
                return Err(RgaaError::MissingId("checkpoint_id".into()));
            }
            if checkpoint.criterion_id.trim().is_empty() {
                return Err(RgaaError::MissingId("criterion_id".into()));
            }
            validate_status(&checkpoint.status)?;
            if checkpoint.status == CriterionStatus::Pass
                && (checkpoint.evidence.is_empty()
                    || checkpoint.evidence.iter().any(|evidence| {
                        evidence.kind.trim().is_empty() || evidence.hash.trim().is_empty()
                    }))
            {
                return Err(RgaaError::IncompleteEvidence(
                    checkpoint.checkpoint_id.clone(),
                ));
            }
        }

        Ok(())
    }
}

fn validate_status(status: &CriterionStatus) -> Result<(), RgaaError> {
    match status {
        CriterionStatus::Pass
        | CriterionStatus::Fail
        | CriterionStatus::NotApplicable
        | CriterionStatus::Error
        | CriterionStatus::NeedsReview
        | CriterionStatus::NotTested => Ok(()),
    }
}

fn validate_finding(finding: &Finding, finding_ids: &mut HashSet<String>) -> Result<(), RgaaError> {
    if finding.id.trim().is_empty() {
        return Err(RgaaError::MissingId("finding.id".into()));
    }
    if !finding_ids.insert(finding.id.clone()) {
        return Err(RgaaError::DuplicateFindingId(finding.id.clone()));
    }
    for field in [
        ("finding.rule", finding.rule.as_str()),
        ("finding.url", finding.url.as_str()),
        ("finding.target", finding.target.as_str()),
    ] {
        if field.1.trim().is_empty() {
            return Err(RgaaError::MissingField(field.0.into()));
        }
    }
    validate_status(&finding.status)
}

impl From<&CrawlConfig> for AuditConfig {
    fn from(crawl: &CrawlConfig) -> Self {
        Self {
            max_pages: crawl.max_pages,
            max_depth: crawl.max_depth,
            respect_robots: crawl.respect_robots,
            sample_mode: crawl.sample_mode,
        }
    }
}

impl From<AuditResult> for AuditBundle {
    fn from(result: AuditResult) -> Self {
        let mut findings = Vec::new();
        let mut page_audits = Vec::with_capacity(result.pages.len());

        for (page_idx, page) in result.pages.into_iter().enumerate() {
            let page_id = format!("page-{page_idx}");
            let mut page_findings = Vec::new();

            for criterion in &page.criteria {
                for violation in &criterion.violations {
                    let finding = finding_from_violation(
                        violation,
                        criterion,
                        &page.url,
                        &page_id,
                        findings.len() + page_findings.len(),
                    );
                    page_findings.push(finding);
                }
            }

            findings.append(&mut page_findings.clone());

            page_audits.push(PageAudit {
                page_id,
                url: page.url,
                title: page.title,
                criteria: page.criteria,
                findings: page_findings,
                errors: Vec::new(),
                completed: true,
                duration_ms: 0,
            });
        }

        let page_count = page_audits.len();
        let total_findings =
            findings.len() + page_audits.iter().map(|p| p.findings.len()).sum::<usize>();

        let needs_review = page_audits
            .iter()
            .flat_map(|p| &p.criteria)
            .filter(|c| c.status == CriterionStatus::NeedsReview)
            .count();

        Self {
            schema_version: CURRENT_SCHEMA_VERSION.to_owned(),
            audit_id: result.audit_id,
            url: result.url,
            config: AuditConfig::default(),
            pages: page_audits,
            findings,
            checkpoints: Vec::new(),
            summary: AuditSummary {
                total_pages: page_count,
                completed_pages: page_count,
                total_findings,
                passed: result.passed,
                failed: result.failed,
                needs_review,
                errors: 0,
            },
        }
    }
}

fn finding_from_violation(
    violation: &Violation,
    criterion: &CriterionResult,
    page_url: &str,
    page_id: &str,
    index: usize,
) -> Finding {
    let id = format!("{page_id}-{}-{}", criterion.criterion_id, index);
    let mut finding = Finding::new(id);
    finding.rule = violation.rule_id.clone();
    finding.criterion_id = Some(criterion.criterion_id.clone());
    finding.url = page_url.to_string();
    finding.target = format!("[data-rgaa-criterion=\"{}\"]", criterion.criterion_id);
    finding.status = criterion.status.clone();
    finding.severity = Some(violation.impact.clone());
    finding.description = Some(violation.description.clone());
    finding.source = criterion.source.clone();
    finding
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundle_round_trips_and_defaults_to_schema_one() {
        let bundle = AuditBundle::new("audit-1", "https://example.test", AuditConfig::default());
        let json = serde_json::to_string(&bundle).unwrap();
        let decoded: AuditBundle = serde_json::from_str(&json).unwrap();

        assert_eq!(decoded.schema_version, "1.0");
        assert_eq!(decoded.audit_id, "audit-1");
    }

    #[test]
    fn unsupported_schema_versions_are_rejected() {
        let mut bundle =
            AuditBundle::new("audit-1", "https://example.test", AuditConfig::default());
        bundle.schema_version = "2.0".into();

        assert!(matches!(
            bundle.validate(),
            Err(RgaaError::UnsupportedSchemaVersion(_))
        ));
    }

    #[test]
    fn duplicate_finding_ids_are_rejected() {
        let mut bundle =
            AuditBundle::new("audit-1", "https://example.test", AuditConfig::default());
        bundle.findings = vec![valid_finding("finding-1"), valid_finding("finding-1")];

        assert!(matches!(
            bundle.validate(),
            Err(RgaaError::DuplicateFindingId(_))
        ));
    }

    #[test]
    fn missing_ids_are_rejected() {
        let bundle = AuditBundle::new("", "https://example.test", AuditConfig::default());

        assert!(matches!(bundle.validate(), Err(RgaaError::MissingId(_))));
    }

    #[test]
    fn duplicate_ids_are_rejected_across_page_and_top_level_findings() {
        let mut bundle =
            AuditBundle::new("audit-1", "https://example.test", AuditConfig::default());
        bundle.findings.push(valid_finding("finding-1"));
        bundle.pages.push(PageAudit {
            page_id: "page-1".into(),
            url: "https://example.test/page".into(),
            title: None,
            criteria: Vec::new(),
            findings: vec![valid_finding("finding-1")],
            errors: Vec::new(),
            completed: true,
            duration_ms: 1,
        });

        assert!(matches!(
            bundle.validate(),
            Err(RgaaError::DuplicateFindingId(_))
        ));
    }

    #[test]
    fn duplicate_ids_are_rejected_between_pages() {
        let mut bundle =
            AuditBundle::new("audit-1", "https://example.test", AuditConfig::default());
        for page_id in ["page-1", "page-2"] {
            bundle.pages.push(PageAudit {
                page_id: page_id.into(),
                url: format!("https://example.test/{page_id}"),
                title: None,
                criteria: Vec::new(),
                findings: vec![valid_finding("finding-1")],
                errors: Vec::new(),
                completed: true,
                duration_ms: 1,
            });
        }

        assert!(matches!(
            bundle.validate(),
            Err(RgaaError::DuplicateFindingId(_))
        ));
    }

    #[test]
    fn findings_require_rule_url_and_target() {
        let mut bundle =
            AuditBundle::new("audit-1", "https://example.test", AuditConfig::default());
        bundle.findings.push(Finding::new("finding-1"));

        assert!(matches!(bundle.validate(), Err(RgaaError::MissingField(_))));
    }

    fn valid_finding(id: &str) -> Finding {
        let mut finding = Finding::new(id);
        finding.rule = "rgaa-1.1".into();
        finding.url = "https://example.test".into();
        finding.target = "#main".into();
        finding
    }

    fn sample_audit_result() -> crate::AuditResult {
        crate::AuditResult {
            audit_id: "audit-1".into(),
            url: "https://example.test".into(),
            pages: vec![crate::PageResult {
                url: "https://example.test".into(),
                title: Some("Home".into()),
                criteria: vec![crate::CriterionResult {
                    criterion_id: "1.1".into(),
                    title: "Test".into(),
                    classification: crate::Classification::Deterministe,
                    status: CriterionStatus::Fail,
                    violations: vec![crate::Violation {
                        rule_id: "image-alt".into(),
                        impact: "critical".into(),
                        description: "Missing alt".into(),
                        nodes_affected: 2,
                    }],
                    confidence: None,
                    justification: None,
                    source: "axe".into(),
                }],
                compliance_rate: 0.0,
                crawl_depth: 0,
            }],
            total_criteria: 106,
            passed: 100,
            failed: 5,
            na: 1,
            overall_compliance: 95.0,
            duration_ms: 1000,
        }
    }

    #[test]
    fn from_audit_result_populates_bundle() {
        let result = sample_audit_result();
        let bundle = AuditBundle::from(result);

        assert_eq!(bundle.schema_version, CURRENT_SCHEMA_VERSION);
        assert_eq!(bundle.audit_id, "audit-1");
        assert_eq!(bundle.pages.len(), 1);
        assert_eq!(bundle.pages[0].criteria.len(), 1);
        assert_eq!(bundle.findings.len(), 1);
        assert_eq!(bundle.findings[0].rule, "image-alt");
        assert_eq!(bundle.findings[0].criterion_id.as_deref(), Some("1.1"));
        assert_eq!(bundle.summary.passed, 100);
        assert_eq!(bundle.summary.failed, 5);
        assert_eq!(bundle.summary.needs_review, 0);
    }

    #[test]
    fn from_crawl_config_converts_to_audit_config() {
        let crawl = crate::CrawlConfig {
            max_pages: 25,
            max_depth: 3,
            respect_robots: false,
            sample_mode: true,
        };
        let config = AuditConfig::from(&crawl);
        assert_eq!(config.max_pages, 25);
        assert_eq!(config.max_depth, 3);
        assert!(!config.respect_robots);
        assert!(config.sample_mode);
    }
}
