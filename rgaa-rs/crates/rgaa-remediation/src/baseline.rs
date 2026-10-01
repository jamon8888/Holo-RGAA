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

    // A baseline finding with no counterpart in the current bundle.
    //
    // This is the ordinary shape of a successful fix: the scanners report
    // violations, so a corrected page stops reporting the finding rather than
    // re-reporting it as `Pass`. The loop above only ever sees current
    // findings, so without this pass a fixed finding fell into no category at
    // all and `resolved_findings` stayed empty for every real remediation.
    for prev in prev_findings {
        let fp = fingerprint(prev);
        if curr_map.contains_key(&fp) {
            continue;
        }
        if is_suppressed(prev) {
            let expires_at = extract_expiry(prev);
            diff.expired_suppressions.push(ExpiredSuppression {
                finding_id: prev.id.clone(),
                fingerprint: fp,
                reason: prev.details.clone().unwrap_or_default(),
                expires_at,
            });
            continue;
        }
        // A disappeared `Pass` says nothing was fixed — the criterion simply
        // is not reported any more — so only previously-open findings count.
        if matches!(
            prev.status,
            CriterionStatus::Fail | CriterionStatus::NeedsReview
        ) {
            diff.resolved_findings.push(prev.clone());
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

/// Fingerprint a finding, delegating to the single canonical implementation.
///
/// This used to be a hand-copied duplicate of
/// [`rgaa_core::FindingFingerprint::from_finding`] that dropped every
/// `hash_field` return value, so the "hash" never moved off its seed and
/// every finding fingerprinted identically. Both maps below then collapsed
/// to one entry and the diff was meaningless. Calling the original removes
/// the copy that drifted rather than re-fixing it here.
fn fingerprint(finding: &Finding) -> String {
    rgaa_core::FindingFingerprint::from_finding(finding)
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

    fn targeted(id: &str, status: CriterionStatus, target: &str) -> Finding {
        let mut f = make_finding(id, status, "t");
        f.target = target.into();
        f
    }

    #[test]
    fn a_finding_that_disappears_from_the_rescan_counts_as_resolved() {
        let mut prev = rgaa_core::AuditBundle::new("p", "u", Default::default());
        prev.findings
            .push(make_finding("f1", CriterionStatus::Fail, "a"));
        let curr = rgaa_core::AuditBundle::new("c", "u", Default::default());
        let diff = compare(&prev, &curr);
        assert_eq!(diff.resolved_findings.len(), 1, "{diff:?}");
        assert_eq!(diff.new_findings.len(), 0);
    }

    #[test]
    fn a_disappeared_passing_finding_is_not_claimed_as_a_fix() {
        let mut prev = rgaa_core::AuditBundle::new("p", "u", Default::default());
        prev.findings
            .push(make_finding("f1", CriterionStatus::Pass, "a"));
        let curr = rgaa_core::AuditBundle::new("c", "u", Default::default());
        assert!(compare(&prev, &curr).resolved_findings.is_empty());
    }

    #[test]
    fn findings_differing_only_by_target_are_told_apart() {
        let mut prev = rgaa_core::AuditBundle::new("p", "u", Default::default());
        prev.findings
            .push(targeted("f1", CriterionStatus::Fail, "#first"));
        let mut curr = rgaa_core::AuditBundle::new("c", "u", Default::default());
        curr.findings
            .push(targeted("f2", CriterionStatus::Fail, "#second"));
        let diff = compare(&prev, &curr);
        assert_eq!(diff.new_findings.len(), 1, "{diff:?}");
        assert_eq!(diff.resolved_findings.len(), 1, "{diff:?}");
        assert!(diff.unchanged.is_empty());
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
