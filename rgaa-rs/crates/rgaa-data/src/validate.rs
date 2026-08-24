use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MappingEntry {
    pub criterion_id: String,
    pub axe_rules: Vec<String>,
    pub provenance: Provenance,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Provenance {
    pub source: String,
    pub validated_by: String,
    pub validated_at: String,
    pub notes: String,
}

pub fn validate_mapping(
    axe_rules: &[super::fetch::AxeRule],
    existing_mapping: &HashMap<String, Vec<String>>,
) -> Vec<MappingEntry> {
    let axe_ids: std::collections::HashSet<&str> =
        axe_rules.iter().map(|r| r.id.as_str()).collect();
    let mut entries = Vec::new();

    for (criterion_id, rule_ids) in existing_mapping {
        let valid_rules: Vec<String> = rule_ids
            .iter()
            .filter(|r| axe_ids.contains(r.as_str()))
            .cloned()
            .collect();
        let invalid_rules: Vec<&String> = rule_ids
            .iter()
            .filter(|r| !axe_ids.contains(r.as_str()))
            .collect();

        let notes = if invalid_rules.is_empty() {
            format!("All {} axe rules validated", valid_rules.len())
        } else {
            format!(
                "Invalid rules found: {:?}",
                invalid_rules.iter().map(|r| r.as_str()).collect::<Vec<_>>()
            )
        };

        entries.push(MappingEntry {
            criterion_id: criterion_id.clone(),
            axe_rules: valid_rules,
            provenance: Provenance {
                source: "axe-core 4.9.1 rule-descriptions.md".to_string(),
                validated_by: "automated cross-reference".to_string(),
                validated_at: "2026-08-24".to_string(),
                notes,
            },
        });
    }
    entries.sort_by(|a, b| a.criterion_id.cmp(&b.criterion_id));
    entries
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_filters_invalid_rules() {
        use crate::fetch::AxeRule;
        let axe_rules = vec![AxeRule {
            id: "image-alt".into(),
            description: "test".into(),
            impact: "Critical".into(),
            tags: vec![],
            help: String::new(),
            help_url: String::new(),
        }];
        let mut mapping = HashMap::new();
        mapping.insert("1.1".into(), vec!["image-alt".into(), "nonexistent".into()]);
        let entries = validate_mapping(&axe_rules, &mapping);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].axe_rules, vec!["image-alt"]);
        assert!(entries[0].provenance.notes.contains("Invalid rules found"));
    }

    #[test]
    fn validate_all_valid_rules() {
        use crate::fetch::AxeRule;
        let axe_rules = vec![AxeRule {
            id: "image-alt".into(),
            description: "test".into(),
            impact: "Critical".into(),
            tags: vec![],
            help: String::new(),
            help_url: String::new(),
        }];
        let mut mapping = HashMap::new();
        mapping.insert("1.1".into(), vec!["image-alt".into()]);
        let entries = validate_mapping(&axe_rules, &mapping);
        assert_eq!(entries.len(), 1);
        assert!(entries[0]
            .provenance
            .notes
            .contains("All 1 axe rules validated"));
    }

    #[test]
    fn validate_output_sorted() {
        use crate::fetch::AxeRule;
        let axe_rules: Vec<AxeRule> = vec![];
        let mut mapping = HashMap::new();
        mapping.insert("3.1".into(), vec![]);
        mapping.insert("1.1".into(), vec![]);
        let entries = validate_mapping(&axe_rules, &mapping);
        assert_eq!(entries[0].criterion_id, "1.1");
        assert_eq!(entries[1].criterion_id, "3.1");
    }
}
