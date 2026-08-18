use std::hash::{Hash, Hasher};

use serde::{Deserialize, Serialize};

use crate::EvidenceRef;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Finding {
    pub id: String,
    pub rule: String,
    pub criterion_id: Option<String>,
    pub url: String,
    pub target: String,
    pub component_path: Option<String>,
    pub evidence: Vec<EvidenceRef>,
    pub status: crate::CriterionStatus,
    pub severity: Option<String>,
    pub description: Option<String>,
    pub remediation: Option<String>,
}

impl Finding {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            rule: String::new(),
            criterion_id: None,
            url: String::new(),
            target: String::new(),
            component_path: None,
            evidence: Vec::new(),
            status: crate::CriterionStatus::NotTested,
            severity: None,
            description: None,
            remediation: None,
        }
    }
}

pub struct FindingFingerprint;

impl FindingFingerprint {
    pub fn from_finding(finding: &Finding) -> String {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        finding.rule.hash(&mut hasher);
        finding.url.hash(&mut hasher);
        finding.target.hash(&mut hasher);
        finding.component_path.hash(&mut hasher);
        for evidence in &finding.evidence {
            evidence.kind.hash(&mut hasher);
            evidence.hash.hash(&mut hasher);
        }
        format!("{:016x}", hasher.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_findings_have_identical_fingerprints() {
        let mut left = Finding::new("finding-1");
        left.rule = "rgaa-1.1".into();
        left.url = "https://example.test".into();
        left.target = "#main".into();
        left.component_path = Some("App > Main".into());
        left.evidence = vec![EvidenceRef::new("screenshot", "sha256:abc")];
        let right = left.clone();

        assert_eq!(
            FindingFingerprint::from_finding(&left),
            FindingFingerprint::from_finding(&right)
        );
    }
}
