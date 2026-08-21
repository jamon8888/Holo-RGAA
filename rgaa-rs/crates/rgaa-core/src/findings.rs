use serde::{Deserialize, Serialize};

use crate::EvidenceRef;

/// An accessibility finding representing a specific violation.
///
/// Findings are the primary output of accessibility audits, containing
/// information about what was found, where, and how to fix it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Finding {
    /// Unique identifier for this finding.
    pub id: String,
    /// The RGAA rule that was violated (e.g., "rgaa-1.1").
    pub rule: String,
    /// Optional criterion identifier (e.g., "1.3").
    pub criterion_id: Option<String>,
    /// URL where the finding was discovered.
    pub url: String,
    /// CSS selector or AX reference to the problematic element.
    pub target: String,
    /// Optional component path in the codebase.
    pub component_path: Option<String>,
    /// Evidence collected for this finding.
    pub evidence: Vec<EvidenceRef>,
    /// Status of the finding (Fail, NeedsReview, etc.).
    pub status: crate::CriterionStatus,
    /// Severity level (critical, serious, moderate, minor).
    pub severity: Option<String>,
    /// Human-readable description of the issue.
    pub description: Option<String>,
    /// Recommended remediation steps.
    pub remediation: Option<String>,
    /// HTML snippet of the problematic element.
    #[serde(default)]
    pub html: Option<String>,
    /// Additional details about the finding.
    #[serde(default)]
    pub details: Option<String>,
    /// Source of the finding (axe, agent, manual).
    #[serde(default)]
    pub source: String,
}

impl Finding {
    /// Creates a new finding with the given ID.
    ///
    /// # Arguments
    ///
    /// * `id` - Unique identifier for the finding.
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
            html: None,
            details: None,
            source: String::new(),
        }
    }
}

/// Stable persisted fingerprint format: `rgaa-fp-v1-` followed by a 16-digit
/// lowercase hexadecimal FNV-1a hash of length-prefixed UTF-8 fields.
///
/// Fingerprints are used for deduplication and baseline comparison.
pub struct FindingFingerprint;

impl FindingFingerprint {
    /// Generates a stable fingerprint for a finding.
    ///
    /// The fingerprint is based on the rule, URL, target, component path,
    /// and evidence, making it suitable for deduplication across audits.
    ///
    /// # Arguments
    ///
    /// * `finding` - The finding to generate a fingerprint for.
    ///
    /// # Returns
    ///
    /// A string in the format `rgaa-fp-v1-{16 hex digits}`.
    pub fn from_finding(finding: &Finding) -> String {
        let mut hash = 0xcbf29ce484222325_u64;
        hash = hash_field(hash, Some(&finding.rule));
        hash = hash_field(hash, Some(&finding.url));
        hash = hash_field(hash, Some(&finding.target));
        hash = hash_field(hash, finding.component_path.as_deref());
        hash = hash_field(hash, Some(&finding.evidence.len().to_string()));
        for evidence in &finding.evidence {
            hash = hash_field(hash, Some(&evidence.kind));
            hash = hash_field(hash, Some(&evidence.hash));
            hash = hash_field(hash, evidence.location.as_deref());
        }
        format!("rgaa-fp-v1-{hash:016x}")
    }
}

fn hash_field(mut hash: u64, field: Option<&str>) -> u64 {
    match field {
        None => fnv_byte(hash, 0),
        Some(value) => {
            hash = fnv_byte(hash, 1);
            for byte in (value.len() as u64).to_le_bytes() {
                hash = fnv_byte(hash, byte);
            }
            for &byte in value.as_bytes() {
                hash = fnv_byte(hash, byte);
            }
            hash
        }
    }
}

fn fnv_byte(hash: u64, byte: u8) -> u64 {
    (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3)
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

    #[test]
    fn fingerprint_changes_when_evidence_location_changes() {
        let mut finding = valid_finding();
        finding.evidence = vec![EvidenceRef {
            kind: "screenshot".into(),
            hash: "sha256:abc".into(),
            location: Some("before.png".into()),
        }];
        let first = FindingFingerprint::from_finding(&finding);
        finding.evidence[0].location = Some("after.png".into());

        assert_ne!(first, FindingFingerprint::from_finding(&finding));
    }

    #[test]
    fn fingerprint_uses_stable_persistence_format() {
        let fingerprint = FindingFingerprint::from_finding(&valid_finding());

        assert!(fingerprint.starts_with("rgaa-fp-v1-"));
        assert_eq!(fingerprint.len(), 27);
    }

    fn valid_finding() -> Finding {
        let mut finding = Finding::new("finding-1");
        finding.rule = "rgaa-1.1".into();
        finding.url = "https://example.test".into();
        finding.target = "#main".into();
        finding
    }
}
