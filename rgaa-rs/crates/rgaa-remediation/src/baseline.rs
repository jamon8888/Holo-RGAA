use rgaa_core::CriterionStatus;
use rgaa_core::Finding;
use std::collections::HashMap;

/// Baseline comparison between previous and current audit bundles.
#[derive(Debug, Clone, Default)]
pub struct BaselineDiff {
    pub new_findings: Vec<Finding>,
    pub resolved_findings: Vec<Finding>,
    pub unresolved_findings: Vec<Finding>,
    pub regressions: Vec<Finding>,
    pub unchanged: Vec<Finding>,
    pub suppressed: Vec<Finding>,
    pub expired_suppressions: Vec<ExpiredSuppression>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpiredSuppression {
    pub finding_id: String,
    pub fingerprint: String,
    pub reason: String,
    pub expires_at: Option<String>,
}

/// Compare a baseline audit bundle against a current one.
pub fn compare(
    previous: &rgaa_core::AuditBundle,
    current: &rgaa_core::AuditBundle,
) -> BaselineDiff {
    let prev_findings = collect_findings(previous);
    let curr_findings = collect_findings(current);

    // Build maps with owned Findings to avoid lifetime issues
    let prev_map: HashMap<_, _> = prev_findings
        .iter()
        .map(|f| (fingerprint(f), f.to_owned()))
        .collect();
    let curr_map: HashMap<_, _> = curr_findings
        .iter()
        .map(|f| (fingerprint(f), f.to_owned()))
        .collect();

    let mut diff = BaselineDiff::default();

    // Collect owned current findings for iteration
    let curr_owned: Vec<Finding> = curr_findings.into_iter().cloned().collect();

    // Check current findings against baseline
    for curr in curr_owned {
        let fp = fingerprint(&curr);
        match prev_map.get(&fp) {
            Some(prev) => {
                if prev.status == CriterionStatus::Fail && curr.status == CriterionStatus::Pass {
                    diff.resolved_findings.push(curr);
                } else if prev.status == curr.status {
                    diff.unchanged.push(curr);
                } else if prev.status == CriterionStatus::Pass
                    && curr.status == CriterionStatus::Fail
                {
                    diff.regressions.push(curr);
                }
            }
            None => {
                diff.new_findings.push(curr);
            }
        }
    }

    // Check for expired suppressions
    for prev in prev_findings {
        let fp = fingerprint(prev);
        if !curr_map.contains_key(&fp) && is_suppressed(prev) {
            let expires_at = extract_expiry(prev);
            diff.expired_suppressions.push(ExpiredSuppression {
                finding_id: prev.id.clone(),
                fingerprint: fingerprint(prev),
                reason: prev.details.clone().unwrap_or_default(),
                expires_at,
            });
        }
    }

    // Check for suppressed findings in current
    // Need to re-collect since we moved curr_findings
    let curr_findings2 = collect_findings(current);
    for curr in curr_findings2 {
        if is_suppressed(curr) {
            diff.suppressed.push(curr.to_owned());
        }
    }

    diff
}

/// Generate fingerprint for a finding (mirrors FindingFingerprint::from_finding).
fn fingerprint(finding: &Finding) -> String {
    let hash = 0xcbf29ce484222325_u64;
    hash_field(hash, Some(&finding.rule));
    hash_field(hash, Some(&finding.url));
    hash_field(hash, Some(&finding.target));
    hash_field(hash, finding.component_path.as_deref());
    hash_field(hash, Some(&finding.evidence.len().to_string()));
    for evidence in &finding.evidence {
        hash_field(hash, Some(&evidence.kind));
        hash_field(hash, Some(&evidence.hash));
        hash_field(hash, evidence.location.as_deref());
    }
    format!("rgaa-fp-v1-{hash:016x}")
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

fn collect_findings(bundle: &rgaa_core::AuditBundle) -> Vec<&Finding> {
    bundle
        .findings
        .iter()
        .chain(bundle.pages.iter().flat_map(|p| p.findings.iter()))
        .collect()
}

fn is_suppressed(finding: &Finding) -> bool {
    finding
        .details
        .as_deref()
        .is_some_and(|d| d.contains("suppressed:"))
}

fn extract_expiry(finding: &Finding) -> Option<String> {
    finding.details.as_deref().and_then(|d| {
        d.split("expires:")
            .nth(1)
            .map(|s| s.split_whitespace().next().unwrap_or("").into())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rgaa_core::{CriterionStatus, Finding};

    fn make_finding(id: &str, status: CriterionStatus, fp_suffix: &str) -> Finding {
        let mut f = Finding::new(id);
        f.rule = "rgaa-1.1".into();
        f.url = "https://example.test".into();
        f.target = "#main".into();
        f.status = status;
        f.details = Some(format!("fp-{}", fp_suffix));
        f
    }

    #[test]
    fn detects_new_findings() {
        let prev = rgaa_core::AuditBundle::new("p", "u", Default::default());
        let mut curr = rgaa_core::AuditBundle::new("c", "u", Default::default());
        curr.findings
            .push(make_finding("new", CriterionStatus::Fail, "a"));
        let diff = compare(&prev, &curr);
        assert_eq!(diff.new_findings.len(), 1);
        assert_eq!(diff.resolved_findings.len(), 0);
    }

    #[test]
    fn detects_resolved() {
        let mut prev = rgaa_core::AuditBundle::new("p", "u", Default::default());
        prev.findings
            .push(make_finding("f1", CriterionStatus::Fail, "a"));
        let mut curr = rgaa_core::AuditBundle::new("c", "u", Default::default());
        curr.findings
            .push(make_finding("f1", CriterionStatus::Pass, "a"));
        let diff = compare(&prev, &curr);
        assert_eq!(diff.resolved_findings.len(), 1);
        assert_eq!(diff.new_findings.len(), 0);
    }

    #[test]
    fn detects_regression() {
        let mut prev = rgaa_core::AuditBundle::new("p", "u", Default::default());
        prev.findings
            .push(make_finding("f1", CriterionStatus::Pass, "a"));
        let mut curr = rgaa_core::AuditBundle::new("c", "u", Default::default());
        curr.findings
            .push(make_finding("f1", CriterionStatus::Fail, "a"));
        let diff = compare(&prev, &curr);
        assert_eq!(diff.regressions.len(), 1);
    }

    #[test]
    fn tracks_unchanged() {
        let mut prev = rgaa_core::AuditBundle::new("p", "u", Default::default());
        prev.findings
            .push(make_finding("f1", CriterionStatus::Fail, "a"));
        let mut curr = rgaa_core::AuditBundle::new("c", "u", Default::default());
        curr.findings
            .push(make_finding("f1", CriterionStatus::Fail, "a"));
        let diff = compare(&prev, &curr);
        assert_eq!(diff.unchanged.len(), 1);
    }

    #[test]
    fn detects_expired_suppression() {
        let mut prev = rgaa_core::AuditBundle::new("p", "u", Default::default());
        let mut f = Finding::new("suppressed");
        f.rule = "rgaa-1.1".into();
        f.url = "https://example.test".into();
        f.target = "#main".into();
        f.status = CriterionStatus::Fail;
        f.details = Some("suppressed: temporary waiver expires: 2025-01-01".into());
        prev.findings.push(f);

        let curr = rgaa_core::AuditBundle::new("c", "u", Default::default());
        let diff = compare(&prev, &curr);
        assert_eq!(diff.expired_suppressions.len(), 1);
        assert!(diff.expired_suppressions[0].expires_at.is_some());
    }
}
