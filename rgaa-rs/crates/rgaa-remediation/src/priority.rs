//! Priority ordering of the remediation plan.
//!
//! The plan is ordered by the impact and quick-win flag the reviewer set on
//! the non-conformities (ticket #431, R5). A machine impact proposal never
//! enters this order (R3).

use indexmap::IndexMap;
use rgaa_core::{impact_rank, NonConformity, UserImpact};

use crate::{
    remediate, FrameworkAdapter, RemediationError, RemediationIssue, RemediationOutcome,
    RemediationPolicy,
};

/// Impact and quick_win aggregated for one criterion from reviewed
/// non-conformities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ImpactPriority {
    /// Most severe reviewed impact among the criterion's non-conformities.
    pub user_impact: Option<UserImpact>,
    /// True when at least one non-conformity of the criterion is a quick win.
    pub quick_win: bool,
}

/// Builds a priority index keyed by canonical criterion id. Several
/// non-conformities can share a criterion: the most severe reviewed impact
/// wins, and the criterion is a quick win when any of its items is.
#[must_use]
pub fn priority_index(non_conformities: &[NonConformity]) -> IndexMap<String, ImpactPriority> {
    let mut index: IndexMap<String, ImpactPriority> = IndexMap::new();
    for item in non_conformities {
        let entry = index.entry(item.criterion_id.clone()).or_default();
        let impact = item.authoritative_impact();
        if impact_rank(impact) > impact_rank(entry.user_impact) {
            entry.user_impact = impact;
        }
        entry.quick_win |= item.quick_win;
    }
    index
}

fn outcome_id(outcome: &RemediationOutcome) -> &str {
    match outcome {
        RemediationOutcome::Ok(guidance) => &guidance.issue_id,
        RemediationOutcome::Error(error) => &error.issue_id,
    }
}

fn outcome_criteria(outcome: &RemediationOutcome) -> &[String] {
    match outcome {
        RemediationOutcome::Ok(guidance) => &guidance.criteria,
        RemediationOutcome::Error(_) => &[],
    }
}

fn outcome_priority(
    outcome: &RemediationOutcome,
    priorities: &IndexMap<String, ImpactPriority>,
) -> ImpactPriority {
    outcome_criteria(outcome)
        .iter()
        .filter_map(|criterion_id| priorities.get(criterion_id).copied())
        .max_by_key(|priority| (impact_rank(priority.user_impact), priority.quick_win))
        .unwrap_or_default()
}

/// Sorts a remediation plan in place: reviewed impact first (bloquant →
/// majeur → mineur → none), then quick win, then issue id for determinism.
pub fn sort_remediation_outcomes(
    plan: &mut [RemediationOutcome],
    priorities: &IndexMap<String, ImpactPriority>,
) {
    plan.sort_by(|left, right| {
        let left_priority = outcome_priority(left, priorities);
        let right_priority = outcome_priority(right, priorities);
        impact_rank(right_priority.user_impact)
            .cmp(&impact_rank(left_priority.user_impact))
            .then_with(|| right_priority.quick_win.cmp(&left_priority.quick_win))
            .then_with(|| outcome_id(left).cmp(outcome_id(right)))
    });
}

/// Generates a remediation plan and orders it by reviewed impact, then quick
/// win. Existing callers of [`remediate`] keep their current behaviour; this
/// is the entry point for a prioritised plan (ticket #431).
///
/// # Errors
/// Propagates [`RemediationError`] from [`remediate`] for an invalid batch or
/// a policy refusal.
pub fn remediate_prioritized(
    issues: &[RemediationIssue],
    policy: &RemediationPolicy,
    adapter: &dyn FrameworkAdapter,
    non_conformities: &[NonConformity],
) -> Result<Vec<RemediationOutcome>, RemediationError> {
    let mut plan = remediate(issues, policy, adapter)?;
    let priorities = priority_index(non_conformities);
    sort_remediation_outcomes(&mut plan, &priorities);
    Ok(plan)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Framework, PatchProposal, ReactAdapter, RemediationGuidance, SourceLocation};

    fn non_conformity(
        id: &str,
        criterion_id: &str,
        impact: Option<UserImpact>,
        quick_win: bool,
    ) -> NonConformity {
        let mut item =
            NonConformity::new(id, criterion_id, "https://example.test", "Défaut constaté");
        if let Some(impact) = impact {
            item.record_review(impact, quick_win, "auditrice", "2026-11-21T10:00:00Z")
                .expect("valid review");
        }
        item
    }

    fn issue(id: &str, criterion_id: &str) -> RemediationIssue {
        RemediationIssue {
            id: id.into(),
            rule: "image-alt".into(),
            element_html: "import React from \"react\"; <img src=\"hero.png\">".into(),
            page_url: "https://example.test".into(),
            source_locations: vec![SourceLocation {
                file: "src/App.tsx".into(),
                line: 10,
                column: Some(4),
            }],
            summary: "Image has no alternative text".into(),
            remediation: "Add an alt attribute".into(),
            criteria: vec![criterion_id.into()],
            framework: Some(Framework::React),
        }
    }

    fn guidance(issue_id: &str, criteria: &[&str]) -> RemediationOutcome {
        RemediationOutcome::Ok(RemediationGuidance {
            issue_id: issue_id.into(),
            explanation: "explication".into(),
            steps: vec!["étape".into()],
            confidence: "high".into(),
            criteria: criteria
                .iter()
                .map(|criterion| (*criterion).into())
                .collect(),
            proposal: PatchProposal::new(
                format!("proposal-{issue_id}"),
                vec![issue_id.into()],
                "diff",
                vec!["src/App.tsx".into()],
                "rationale",
                Vec::new(),
                Vec::new(),
                "fixes alt",
            ),
        })
    }

    #[test]
    fn priority_index_uses_the_most_severe_reviewed_impact_per_criterion() {
        let index = priority_index(&[
            non_conformity("a", "1.1", Some(UserImpact::Mineur), false),
            non_conformity("b", "1.1", Some(UserImpact::Bloquant), true),
        ]);
        let priority = index.get("1.1").expect("criterion indexed");
        assert_eq!(priority.user_impact, Some(UserImpact::Bloquant));
        assert!(priority.quick_win);
    }

    /// R3: a machine proposal must not drive the remediation plan.
    #[test]
    fn priority_index_ignores_machine_proposals() {
        let mut item = non_conformity("a", "12.9", None, false);
        item.set_proposed_impact(UserImpact::Bloquant);
        let index = priority_index(&[item]);
        let priority = index.get("12.9").expect("criterion indexed");
        assert_eq!(priority.user_impact, None);
        assert!(!priority.quick_win);
    }

    #[test]
    fn plan_sorts_by_impact_then_quick_win() {
        let index = priority_index(&[
            non_conformity("a", "1.1", Some(UserImpact::Mineur), true),
            non_conformity("b", "2.1", Some(UserImpact::Bloquant), false),
            non_conformity("c", "3.1", Some(UserImpact::Majeur), true),
            non_conformity("d", "4.1", Some(UserImpact::Bloquant), true),
        ]);
        let mut plan = vec![
            guidance("minor", &["1.1"]),
            guidance("blocking", &["2.1"]),
            guidance("major", &["3.1"]),
            guidance("blocking-quick", &["4.1"]),
        ];
        sort_remediation_outcomes(&mut plan, &index);

        let order: Vec<&str> = plan.iter().map(outcome_id).collect();
        assert_eq!(order, ["blocking-quick", "blocking", "major", "minor"]);
    }

    #[test]
    fn remediate_prioritized_returns_a_sorted_plan() {
        let issues = [
            issue("minor", "1.1"),
            issue("blocking", "2.1"),
            issue("blocking-quick", "4.1"),
        ];
        let non_conformities = [
            non_conformity("a", "1.1", Some(UserImpact::Mineur), false),
            non_conformity("b", "2.1", Some(UserImpact::Bloquant), false),
            non_conformity("c", "4.1", Some(UserImpact::Bloquant), true),
        ];
        let plan = remediate_prioritized(
            &issues,
            &RemediationPolicy::default(),
            &ReactAdapter,
            &non_conformities,
        )
        .expect("valid plan");

        let order: Vec<&str> = plan.iter().map(outcome_id).collect();
        assert_eq!(order, ["blocking-quick", "blocking", "minor"]);
    }
}
