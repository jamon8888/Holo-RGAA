use serde::{Deserialize, Serialize};

use crate::{CriterionStatus, EvidenceRef};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PageError {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CheckpointResult {
    pub checkpoint_id: String,
    pub criterion_id: String,
    pub status: CriterionStatus,
    pub evidence: Vec<EvidenceRef>,
    pub summary: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AuditBundle, AuditConfig, RgaaError};

    #[test]
    fn passing_checkpoint_requires_evidence() {
        let checkpoint = CheckpointResult {
            checkpoint_id: "checkpoint-1".into(),
            criterion_id: "1.1".into(),
            status: CriterionStatus::Pass,
            evidence: Vec::new(),
            summary: "verified".into(),
        };
        let mut bundle =
            AuditBundle::new("audit-1", "https://example.test", AuditConfig::default());
        bundle.checkpoints.push(checkpoint);

        assert!(matches!(
            bundle.validate(),
            Err(RgaaError::IncompleteEvidence(_))
        ));
    }

    #[test]
    fn page_errors_are_explicitly_serialized() {
        let error = PageError {
            code: "navigation".into(),
            message: "failed".into(),
        };

        assert_eq!(serde_json::to_value(error).unwrap()["code"], "navigation");
    }
}
