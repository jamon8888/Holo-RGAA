use rgaa_core::{EvidenceRef, Finding};
use std::collections::HashMap;

/// Groups repeated rule/target/component findings without merging distinct evidence.
#[derive(Debug, Clone, Default)]
pub struct Deduplicator {
    groups: HashMap<String, FindingGroup>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FindingGroup {
    pub representative: Finding,
    pub members: Vec<Finding>,
    pub evidence: Vec<EvidenceRef>,
}

impl Deduplicator {
    pub fn new() -> Self {
        Self::default()
    }

    /// Normalize findings into groups by (rule, target, component_path).
    /// Distinct evidence is preserved in the group's evidence list.
    pub fn normalize(&mut self, findings: &[Finding]) -> Vec<FindingGroup> {
        let mut groups: HashMap<String, FindingGroup> = HashMap::new();

        for finding in findings {
            let key = Self::group_key(finding);
            let group = groups.entry(key).or_insert_with(|| FindingGroup {
                representative: finding.clone(),
                members: Vec::new(),
                evidence: Vec::new(),
            });
            group.members.push(finding.clone());
            // Accumulate unique evidence
            for ev in &finding.evidence {
                if !group
                    .evidence
                    .iter()
                    .any(|e| e.kind == ev.kind && e.hash == ev.hash)
                {
                    group.evidence.push(ev.clone());
                }
            }
        }

        self.groups = groups;
        self.groups.values().cloned().collect()
    }

    fn group_key(finding: &Finding) -> String {
        format!(
            "{}|{}|{}",
            finding.rule,
            finding.target,
            finding.component_path.as_deref().unwrap_or("")
        )
    }

    /// Get all groups.
    pub fn groups(&self) -> Vec<FindingGroup> {
        self.groups.values().cloned().collect()
    }

    /// Get total finding count across all groups.
    pub fn total_count(&self) -> usize {
        self.groups.values().map(|g| g.members.len()).sum()
    }

    /// Get unique group count.
    pub fn group_count(&self) -> usize {
        self.groups.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rgaa_core::{EvidenceRef, Finding};

    fn finding(id: &str, rule: &str, target: &str, component: Option<&str>) -> Finding {
        let mut f = Finding::new(id);
        f.rule = rule.into();
        f.target = target.into();
        f.component_path = component.map(|s| s.into());
        f.evidence = vec![EvidenceRef::new("screenshot", "sha256:abc")];
        f
    }

    #[test]
    fn groups_by_rule_and_target() {
        let mut dedup = Deduplicator::new();
        let findings = vec![
            finding("f1", "rgaa-1.1", "#main", Some("App > Main")),
            finding("f2", "rgaa-1.1", "#main", Some("App > Main")),
            finding("f3", "rgaa-1.2", "#sidebar", Some("App > Sidebar")),
        ];
        let groups = dedup.normalize(&findings);
        eprintln!("groups.len() = {}", groups.len());
        eprintln!("groups = {:?}", groups);
        for (i, g) in groups.iter().enumerate() {
            eprintln!(
                "Group {}: len={}, rule={}, target={:?}, component={:?}",
                i,
                g.members.len(),
                g.representative.rule,
                g.representative.target,
                g.representative.component_path
            );
        }
        let len = groups.len();
        eprintln!("After len(): len = {}", len);
        if len != 2 {
            panic!("Expected 2 groups, got {}", len);
        }
        assert_eq!(groups[0].members.len(), 2);
        assert_eq!(groups[1].members.len(), 1);
    }

    #[test]
    fn preserves_distinct_evidence() {
        let mut f1 = Finding::new("f1");
        f1.rule = "rgaa-1.1".into();
        f1.target = "#main".into();
        f1.evidence = vec![EvidenceRef::new("screenshot", "sha256:abc")];

        let mut f2 = Finding::new("f2");
        f2.rule = "rgaa-1.1".into();
        f2.target = "#main".into();
        f2.evidence = vec![EvidenceRef::new("dom_snapshot", "sha256:def")];

        let mut dedup = Deduplicator::new();
        let groups = dedup.normalize(&[f1, f2]);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].evidence.len(), 2);
    }

    #[test]
    fn different_component_paths_are_separate_groups() {
        let mut dedup = Deduplicator::new();
        let findings = vec![
            finding("f1", "rgaa-1.1", "#main", Some("App > Main")),
            finding("f2", "rgaa-1.1", "#main", Some("App > Header")),
        ];
        let groups = dedup.normalize(&findings);
        assert_eq!(groups.len(), 2);
    }
}
