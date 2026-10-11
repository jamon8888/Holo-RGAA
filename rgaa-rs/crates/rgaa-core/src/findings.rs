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
    #[serde(default)]
    pub html: Option<String>,
    #[serde(default)]
    pub details: Option<String>,
    #[serde(default)]
    pub source: String,
    /// Set to true when this finding was generated from axe-core's `incomplete`
    /// results array rather than a definite `violations` entry. Indicates that
    /// axe-core could not determine accessibility — often because the element
    /// was inside Shadow DOM or used ElementInternals in a way that was not
    /// yet resolvable. The criterion status is `NeedsReview`.
    #[serde(default)]
    pub previously_incomplete: bool,
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
            html: None,
            details: None,
            source: String::new(),
            previously_incomplete: false,
        }
    }
}

/// Stable persisted fingerprint format: `rgaa-fp-v2-` followed by a 16-digit
/// lowercase hexadecimal FNV-1a hash of length-prefixed UTF-8 fields.
///
/// The fingerprint is the *identity* of a finding — which rule failed, under
/// which criterion, on which element of which page — and deliberately nothing
/// else. In particular it does not hash [`Finding::evidence`], and must not:
///
/// * Scanners attach evidence per *scan*, not per finding. `rgaa-obscura`
///   hashes `document.documentElement.outerHTML` once and clones that one
///   `EvidenceRef` onto every finding of the scan, so any change anywhere on
///   the page moved every fingerprint at once. A re-scan after a fix is the
///   only reason to re-scan, so `compare` saw two bundles sharing no
///   fingerprints: every reference finding read as resolved and every current
///   one as new, with nothing ever `unchanged`.
/// * The evidence *count* is no more stable than the hashes:
///   `ScreenshotPolicy::OnFailure` appends a screenshot only while the page
///   still has violations, so fixing the last violation changes the vector's
///   length too.
/// * Hashing the finding's own element instead (`Finding::html`) would trade
///   one instability for a worse one: the markup under repair changes exactly
///   when it is repaired, so a *partial* fix would be reported as one finding
///   resolved plus one new finding rather than as still outstanding — and
///   "still outstanding after my fix" is the answer callers need most.
///
/// `criterion_id` is part of the identity because a single axe rule maps to
/// several RGAA criteria: `image-alt` on one `<img>` decides both 1.1 and 1.2
/// and yields two findings agreeing on every other field here. Without it the
/// two collapse to one fingerprint and the `HashMap`s in
/// `rgaa_remediation::compare` keep only whichever was inserted last.
///
/// Bumped from `v1` because `findings.fingerprint` is persisted (Postgres, and
/// baseline bundles on disk): the prefix makes a stored fingerprint that
/// predates this change fail to match loudly rather than silently.
pub struct FindingFingerprint;

impl FindingFingerprint {
    #[must_use]
    pub fn from_finding(finding: &Finding) -> String {
        let mut hash = 0xcbf29ce484222325_u64;
        hash = hash_field(hash, Some(&finding.rule));
        hash = hash_field(hash, finding.criterion_id.as_deref());
        hash = hash_field(hash, Some(&finding.url));
        hash = hash_field(hash, Some(&finding.target));
        hash = hash_field(hash, finding.component_path.as_deref());
        format!("rgaa-fp-v2-{hash:016x}")
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

    /// Scan-level evidence must not reach the fingerprint. `rgaa-obscura`
    /// hashes the whole page once per scan and clones that `EvidenceRef` onto
    /// every finding, so hashing it made every fingerprint move whenever any
    /// part of the page changed — which is precisely the situation `compare`
    /// exists to analyse.
    #[test]
    fn fingerprint_ignores_evidence() {
        let mut finding = valid_finding();
        finding.evidence = vec![EvidenceRef {
            kind: "dom_snapshot".into(),
            hash: "sha256:before".into(),
            location: Some("before.json".into()),
        }];
        let before = FindingFingerprint::from_finding(&finding);

        // The whole evidence vector moves on a re-scan: a fresh page hash, a
        // fresh location, and one more item once `OnFailure` starts (or stops)
        // attaching a screenshot.
        finding.evidence = vec![
            EvidenceRef {
                kind: "dom_snapshot".into(),
                hash: "sha256:after".into(),
                location: Some("after.json".into()),
            },
            EvidenceRef::new("screenshot", "sha256:shot"),
        ];

        assert_eq!(before, FindingFingerprint::from_finding(&finding));
    }

    /// One axe rule maps to several RGAA criteria, so the same element yields
    /// several findings differing only here. They must not collapse into one
    /// entry of `compare`'s maps.
    #[test]
    fn fingerprint_distinguishes_criteria_on_the_same_element() {
        let mut first = valid_finding();
        first.criterion_id = Some("1.1".into());
        let mut second = first.clone();
        second.criterion_id = Some("1.2".into());

        assert_ne!(
            FindingFingerprint::from_finding(&first),
            FindingFingerprint::from_finding(&second)
        );
    }

    #[test]
    fn fingerprint_changes_when_the_element_changes() {
        let mut finding = valid_finding();
        let first = FindingFingerprint::from_finding(&finding);
        finding.target = "#other".into();

        assert_ne!(first, FindingFingerprint::from_finding(&finding));
    }

    #[test]
    fn fingerprint_uses_stable_persistence_format() {
        let fingerprint = FindingFingerprint::from_finding(&valid_finding());

        assert!(fingerprint.starts_with("rgaa-fp-v2-"));
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
