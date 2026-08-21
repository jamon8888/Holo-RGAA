use crate::{Framework, RemediationError, RemediationIssue};
use rgaa_core::FindingFingerprint;
use serde::{Deserialize, Serialize};

/// Policy configuration for remediation operations.
///
/// Controls which frameworks are allowed, whether remote AI is permitted,
/// batch size limits, and approval requirements.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RemediationPolicy {
    /// Whether remote AI remediation is allowed at all.
    pub allow_remote_ai: bool,
    /// Whether to actually use remote AI for remediation.
    pub use_remote_ai: bool,
    /// List of frameworks allowed for automated remediation.
    pub allowed_frameworks: Vec<Framework>,
    /// Maximum number of issues to remediate in a single batch.
    pub max_batch_size: usize,
    /// Whether human approval is required before applying patches.
    pub require_approval: bool,
}

impl Default for RemediationPolicy {
    fn default() -> Self {
        Self {
            allow_remote_ai: false,
            use_remote_ai: false,
            allowed_frameworks: vec![
                Framework::React,
                Framework::Next,
                Framework::Vue,
                Framework::Angular,
            ],
            max_batch_size: 25,
            require_approval: true,
        }
    }
}

impl RemediationPolicy {
    /// Checks if a remediation issue passes policy constraints.
    ///
    /// # Arguments
    ///
    /// * `issue` - The remediation issue to validate.
    ///
    /// # Returns
    ///
    /// `Ok(())` if the issue passes all policy checks.
    ///
    /// # Errors
    ///
    /// Returns `RemediationError::PolicyDenied` if the issue violates policy,
    /// `RemediationError::InvalidIssue` if required fields are missing,
    /// or `RemediationError::MissingSourceLocation` if no source locations exist.
    pub fn check(&self, issue: &RemediationIssue) -> Result<(), RemediationError> {
        if self.use_remote_ai && !self.allow_remote_ai {
            return Err(RemediationError::PolicyDenied {
                issue_id: issue.id.clone(),
                reason: "remote remediation is disabled by policy".into(),
            });
        }
        if issue.id.trim().is_empty()
            || issue.rule.trim().is_empty()
            || issue.page_url.trim().is_empty()
        {
            return Err(RemediationError::InvalidIssue {
                issue_id: issue.id.clone(),
                message: "id, rule, and page_url are required".into(),
            });
        }
        if issue.source_locations.is_empty() {
            return Err(RemediationError::MissingSourceLocation {
                issue_id: issue.id.clone(),
            });
        }
        if let Some(framework) = issue.framework {
            if !self.allowed_frameworks.contains(&framework) {
                return Err(RemediationError::PolicyDenied {
                    issue_id: issue.id.clone(),
                    reason: "framework is not allowed".into(),
                });
            }
        }
        Ok(())
    }
}

/// Result of evaluating a policy against audit bundles.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct PolicyResult {
    pub passed: bool,
    pub failures: Vec<PolicyFailure>,
    pub warnings: Vec<PolicyWarning>,
    pub counts: PolicyCounts,
}

/// Individual policy failure with context.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PolicyFailure {
    pub rule: String,
    pub message: String,
    pub finding_ids: Vec<String>,
}

/// Policy warning (does not fail the build but is reported).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PolicyWarning {
    pub rule: String,
    pub message: String,
    pub finding_ids: Vec<String>,
}

/// Aggregate counts for reporting.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct PolicyCounts {
    pub total_findings: usize,
    pub new_failures: usize,
    pub resolved: usize,
    pub unchanged: usize,
    pub regressions: usize,
    pub suppressed: usize,
    pub expired_suppressions: usize,
}

impl RemediationPolicy {
    /// Evaluate the policy against a current audit bundle and optional baseline.
    /// Returns PolicyResult with pass/fail determination.
    pub fn evaluate(
        &self,
        current: &rgaa_core::AuditBundle,
        baseline: Option<&rgaa_core::AuditBundle>,
    ) -> PolicyResult {
        let mut result = PolicyResult::default();
        result.counts.total_findings = current.findings.len()
            + current
                .pages
                .iter()
                .map(|p| p.findings.len())
                .sum::<usize>();

        // If no baseline, treat all findings as new
        let previous_findings = baseline.map(Self::collect_findings).unwrap_or_default();
        let current_findings = Self::collect_findings(current);

        // Build lookup maps
        let prev_map: std::collections::HashMap<_, _> = previous_findings
            .iter()
            .map(|f| (FindingFingerprint::from_finding(f), f))
            .collect();
        let curr_map: std::collections::HashMap<_, _> = current_findings
            .iter()
            .map(|f| (FindingFingerprint::from_finding(f), f))
            .collect();

        // Check each current finding by iterating over curr_map
        for (fp, finding) in &curr_map {
            match prev_map.get(fp) {
                Some(prev_f) => {
                    if prev_f.status == rgaa_core::CriterionStatus::Fail
                        && finding.status == rgaa_core::CriterionStatus::Pass
                    {
                        result.counts.resolved += 1;
                    } else if prev_f.status == finding.status {
                        result.counts.unchanged += 1;
                        if finding.status == rgaa_core::CriterionStatus::Fail {
                            // Still failing - check if suppressed
                            if finding
                                .details
                                .as_deref()
                                .is_some_and(|d| d.contains("suppressed:"))
                            {
                                result.counts.suppressed += 1;
                            } else {
                                // Unsuppressed failure counts as policy failure
                                result.failures.push(PolicyFailure {
                                    rule: "unchanged_failure".into(),
                                    message: format!(
                                        "Unresolved failure: {} ({})",
                                        finding.rule, finding.target
                                    ),
                                    finding_ids: vec![finding.id.clone()],
                                });
                            }
                        }
                    } else if prev_f.status == rgaa_core::CriterionStatus::Pass
                        && finding.status == rgaa_core::CriterionStatus::Fail
                    {
                        // Regression
                        result.counts.regressions += 1;
                        result.failures.push(PolicyFailure {
                            rule: "regression".into(),
                            message: format!("Finding {} regressed from pass to fail", finding.id),
                            finding_ids: vec![finding.id.clone()],
                        });
                    }
                }
                None => {
                    // New finding
                    if finding.status == rgaa_core::CriterionStatus::Fail {
                        result.counts.new_failures += 1;
                        result.failures.push(PolicyFailure {
                            rule: "new_failure".into(),
                            message: format!("New failure: {} ({})", finding.rule, finding.target),
                            finding_ids: vec![finding.id.clone()],
                        });
                    }
                }
            }
        }

        // Check for expired suppressions in baseline
        if let Some(_baseline) = baseline {
            for prev_f in previous_findings {
                if prev_f
                    .details
                    .as_deref()
                    .is_some_and(|d| d.contains("suppressed:"))
                    && !curr_map.contains_key(&FindingFingerprint::from_finding(prev_f))
                {
                    // Suppressed finding disappeared - could be expired or genuinely resolved
                    // We check if the suppression had an expiry in details
                    if prev_f
                        .details
                        .as_deref()
                        .is_some_and(|d| d.contains("expires:"))
                    {
                        result.counts.expired_suppressions += 1;
                        result.warnings.push(PolicyWarning {
                            rule: "expired_suppression".into(),
                            message: format!("Suppression for {} may have expired", prev_f.id),
                            finding_ids: vec![prev_f.id.clone()],
                        });
                    }
                }
            }
        }

        // Determine overall pass/fail
        result.passed = result.failures.is_empty();

        result
    }

    fn collect_findings(bundle: &rgaa_core::AuditBundle) -> Vec<&rgaa_core::Finding> {
        bundle
            .findings
            .iter()
            .chain(bundle.pages.iter().flat_map(|p| p.findings.iter()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rgaa_core::{AuditBundle, AuditConfig, CriterionStatus, EvidenceRef, Finding};

    fn make_bundle(findings: Vec<Finding>) -> AuditBundle {
        let mut bundle =
            AuditBundle::new("test-audit", "https://example.test", AuditConfig::default());
        bundle.findings = findings;
        bundle
    }

    fn finding(id: &str, status: CriterionStatus, fingerprint_suffix: &str) -> Finding {
        let mut f = Finding::new(id);
        f.rule = "rgaa-1.1".into();
        f.url = "https://example.test".into();
        f.target = "#main".into();
        f.status = status;
        f.evidence = vec![EvidenceRef::new(
            "screenshot",
            format!("sha256:{}", fingerprint_suffix),
        )];
        f
    }

    #[test]
    fn new_failure_fails_policy() {
        let policy = RemediationPolicy::default();
        let current = make_bundle(vec![finding("f1", CriterionStatus::Fail, "a")]);
        let result = policy.evaluate(&current, None);
        assert!(!result.passed);
        assert_eq!(result.counts.new_failures, 1);
    }

    #[test]
    fn no_baseline_treats_all_as_new() {
        let policy = RemediationPolicy::default();
        let current = make_bundle(vec![
            finding("f1", CriterionStatus::Fail, "a"),
            finding("f2", CriterionStatus::Pass, "b"),
        ]);
        let result = policy.evaluate(&current, None);
        assert!(!result.passed);
        assert_eq!(result.counts.new_failures, 1);
    }

    #[test]
    fn resolved_finding_is_counted() {
        let policy = RemediationPolicy::default();
        let baseline = make_bundle(vec![finding("f1", CriterionStatus::Fail, "a")]);
        let current = make_bundle(vec![finding("f1", CriterionStatus::Pass, "a")]);
        let result = policy.evaluate(&current, Some(&baseline));
        assert!(result.passed);
        assert_eq!(result.counts.resolved, 1);
    }

    #[test]
    fn regression_is_failure() {
        let policy = RemediationPolicy::default();
        let baseline = make_bundle(vec![finding("f1", CriterionStatus::Pass, "a")]);
        let current = make_bundle(vec![finding("f1", CriterionStatus::Fail, "a")]);
        let result = policy.evaluate(&current, Some(&baseline));
        assert!(!result.passed);
        assert_eq!(result.counts.regressions, 1);
    }

    #[test]
    fn unchanged_findings_are_tracked() {
        let policy = RemediationPolicy::default();
        let baseline = make_bundle(vec![finding("f1", CriterionStatus::Fail, "a")]);
        let current = make_bundle(vec![finding("f1", CriterionStatus::Fail, "a")]);
        let result = policy.evaluate(&current, Some(&baseline));
        assert!(!result.passed);
        assert_eq!(result.counts.unchanged, 1);
    }
}
