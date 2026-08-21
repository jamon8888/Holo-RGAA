use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::{CheckpointResult, CriterionResult, CriterionStatus, Finding, PageError, RgaaError};

/// Current schema version for audit bundles.
pub const CURRENT_SCHEMA_VERSION: &str = "1.0";

/// Configuration for an accessibility audit.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AuditConfig {
    /// Maximum number of pages to audit.
    pub max_pages: usize,
    /// Maximum crawl depth from the starting URL.
    pub max_depth: u32,
    /// Whether to respect robots.txt directives.
    pub respect_robots: bool,
    /// Whether to use sampling mode (audit a subset of pages).
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

/// Audit results for a single page.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PageAudit {
    /// Unique identifier for the page.
    pub page_id: String,
    /// URL of the page.
    pub url: String,
    /// Page title, if available.
    pub title: Option<String>,
    /// Criterion evaluation results for this page.
    pub criteria: Vec<CriterionResult>,
    /// Accessibility findings for this page.
    pub findings: Vec<Finding>,
    /// Errors encountered during auditing.
    pub errors: Vec<PageError>,
    /// Whether the page audit completed successfully.
    pub completed: bool,
    /// Duration of the page audit in milliseconds.
    pub duration_ms: u64,
}

/// Summary statistics for an audit.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct AuditSummary {
    /// Total number of pages audited.
    pub total_pages: usize,
    /// Number of pages that completed successfully.
    pub completed_pages: usize,
    /// Total number of findings across all pages.
    pub total_findings: usize,
    /// Number of criteria that passed.
    pub passed: usize,
    /// Number of criteria that failed.
    pub failed: usize,
    /// Number of criteria that need review.
    pub needs_review: usize,
    /// Number of errors encountered.
    pub errors: usize,
}

/// Complete audit bundle containing all results and metadata.
///
/// This is the primary output format for RGAA audits, containing
/// page-level results, findings, checkpoints, and summary statistics.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AuditBundle {
    /// Schema version (currently "1.0").
    pub schema_version: String,
    /// Unique identifier for this audit.
    pub audit_id: String,
    /// Starting URL for the audit.
    pub url: String,
    /// Configuration used for this audit.
    pub config: AuditConfig,
    /// Results for each audited page.
    pub pages: Vec<PageAudit>,
    /// Aggregate findings across all pages.
    pub findings: Vec<Finding>,
    /// Checkpoint results for criterion verification.
    pub checkpoints: Vec<CheckpointResult>,
    /// Summary statistics.
    pub summary: AuditSummary,
}

impl AuditBundle {
    /// Creates a new empty audit bundle.
    ///
    /// # Arguments
    ///
    /// * `audit_id` - Unique identifier for this audit.
    /// * `url` - Starting URL for the audit.
    /// * `config` - Audit configuration.
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
}
