use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::{CheckpointResult, CriterionResult, CriterionStatus, Finding, PageError, RgaaError};

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

        let mut finding_ids = HashSet::with_capacity(self.findings.len());
        for finding in &self.findings {
            if finding.id.trim().is_empty() {
                return Err(RgaaError::MissingId("finding.id".into()));
            }
            if !finding_ids.insert(&finding.id) {
                return Err(RgaaError::DuplicateFindingId(finding.id.clone()));
            }
            validate_status(&finding.status)?;
        }

        for checkpoint in &self.checkpoints {
            if checkpoint.checkpoint_id.trim().is_empty() {
                return Err(RgaaError::MissingId("checkpoint_id".into()));
            }
            if checkpoint.criterion_id.trim().is_empty() {
                return Err(RgaaError::MissingId("criterion_id".into()));
            }
            validate_status(&checkpoint.status)?;
            if checkpoint.status == CriterionStatus::Pass && checkpoint.evidence.is_empty() {
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
        | CriterionStatus::NotTested
        | CriterionStatus::Na => Ok(()),
    }
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
    fn duplicate_finding_ids_are_rejected() {
        let mut bundle =
            AuditBundle::new("audit-1", "https://example.test", AuditConfig::default());
        bundle.findings = vec![Finding::new("finding-1"), Finding::new("finding-1")];

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
}
