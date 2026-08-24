use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomatableCriterion {
    pub criterion_id: String,
    pub title: String,
    pub classification: String,
    pub automatable_test_count: usize,
    pub total_test_count: usize,
    pub test_keys: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomatabilityReport {
    pub total_criteria: usize,
    pub fully_automatable: usize,
    pub partially_automatable: usize,
    pub not_automatable: usize,
    pub criteria: Vec<AutomatableCriterion>,
}

#[derive(Debug, Clone, Deserialize)]
struct CriteresRoot {
    topics: Vec<Topic>,
}

#[derive(Debug, Clone, Deserialize)]
struct Topic {
    criteria: Vec<CriterionWrapper>,
}

#[derive(Debug, Clone, Deserialize)]
struct CriterionWrapper {
    criterium: Criterium,
}

#[derive(Debug, Clone, Deserialize)]
struct Criterium {
    number: serde_json::Value,
    title: String,
    tests: std::collections::HashMap<String, Vec<String>>,
}

const MANUAL_CLASSIFICATION_KEYWORDS: &[&str] = &[
    "pertinent",
    "compréhensib",
    "perceptib",
    "interprétab",
    "significatif",
    "suffisant",
    "correctement restitué",
    "correctement identifié",
    "intention",
    "objectif",
    "but",
    "finalité",
    "visib",
    "audib",
    "évident",
    "clairement",
    "cohérent",
    "pertinence",
    "sens",
];

fn has_manual_keyword(text: &str) -> bool {
    MANUAL_CLASSIFICATION_KEYWORDS
        .iter()
        .any(|kw| text.contains(kw))
}

fn classify_criterion(
    title: &str,
    tests: &std::collections::HashMap<String, Vec<String>>,
) -> (String, usize, usize) {
    let mut manual_test_count = 0usize;
    let total_test_count: usize = tests.values().map(|v| v.len()).sum();

    for test_list in tests.values() {
        for test in test_list {
            let lower = test.to_lowercase();
            if has_manual_keyword(&lower) {
                manual_test_count += 1;
            }
        }
    }

    // Also check title
    let title_lower = title.to_lowercase();
    let title_has_manual = has_manual_keyword(&title_lower);

    let automatable_count = total_test_count.saturating_sub(manual_test_count);

    let classification = if manual_test_count == 0 && !title_has_manual {
        "FullyAutomatable".to_string()
    } else if automatable_count > 0 && manual_test_count > 0 {
        "PartiallyAutomatable".to_string()
    } else {
        "NotAutomatable".to_string()
    };

    (classification, automatable_count, total_test_count)
}

pub fn analyze_automatability(criteres_path: &std::path::Path) -> Result<AutomatabilityReport> {
    let path = criteres_path.to_path_buf();
    let data =
        fs::read_to_string(&path).with_context(|| format!("Failed to read {}", path.display()))?;
    let root: CriteresRoot =
        serde_json::from_str(&data).context("Failed to parse criteres.json")?;

    let mut criteria = Vec::new();
    let mut fully = 0usize;
    let mut partial = 0usize;
    let mut not_auto = 0usize;

    for (topic_idx, topic) in root.topics.iter().enumerate() {
        let topic_num = topic_idx + 1;
        for wrapper in &topic.criteria {
            let c = &wrapper.criterium;
            let criterion_id = match &c.number {
                serde_json::Value::Number(n) => format!("{topic_num}.{n}"),
                serde_json::Value::String(s) => format!("{topic_num}.{s}"),
                _ => format!("{topic_num}.{}", c.number),
            };

            let test_keys: Vec<String> = c.tests.keys().cloned().collect();
            let (classification, auto_count, total) = classify_criterion(&c.title, &c.tests);

            match classification.as_str() {
                "FullyAutomatable" => fully += 1,
                "PartiallyAutomatable" => partial += 1,
                "NotAutomatable" => not_auto += 1,
                _ => {}
            }

            criteria.push(AutomatableCriterion {
                criterion_id,
                title: c.title.clone(),
                classification,
                automatable_test_count: auto_count,
                total_test_count: total,
                test_keys,
            });
        }
    }

    Ok(AutomatabilityReport {
        total_criteria: criteria.len(),
        fully_automatable: fully,
        partially_automatable: partial,
        not_automatable: not_auto,
        criteria,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_criteres_path() -> std::path::PathBuf {
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
        std::path::Path::new(&manifest_dir)
            .join("../../crates/rgaa-core/data/rgaa-4.1.2/criteres.json")
    }

    #[test]
    fn test_criterion_id_formatting() {
        let report = analyze_automatability(&test_criteres_path()).unwrap();
        assert_eq!(report.total_criteria, 106);
        // Check first criterion ID
        assert_eq!(report.criteria[0].criterion_id, "1.1");
        // Check a criterion from topic 13
        let last = report.criteria.last().unwrap();
        assert!(last.criterion_id.starts_with("13."));
    }

    #[test]
    fn test_classifications_sum() {
        let report = analyze_automatability(&test_criteres_path()).unwrap();
        assert_eq!(
            report.fully_automatable + report.partially_automatable + report.not_automatable,
            report.total_criteria
        );
    }

    #[test]
    fn test_all_criteria_have_valid_classification() {
        let report = analyze_automatability(&test_criteres_path()).unwrap();
        for c in &report.criteria {
            assert!(
                c.classification == "FullyAutomatable"
                    || c.classification == "PartiallyAutomatable"
                    || c.classification == "NotAutomatable",
                "Invalid classification for {}: {}",
                c.criterion_id,
                c.classification
            );
            assert!(c.total_test_count > 0, "No tests for {}", c.criterion_id);
            assert!(
                c.automatable_test_count <= c.total_test_count,
                "Automatable > total for {}",
                c.criterion_id
            );
        }
    }

    #[test]
    fn test_criterion_8_1_is_fully_automatable() {
        let report = analyze_automatability(&test_criteres_path()).unwrap();
        let c8_1 = report
            .criteria
            .iter()
            .find(|c| c.criterion_id == "8.1")
            .expect("8.1 not found");
        assert_eq!(c8_1.classification, "FullyAutomatable");
    }

    #[test]
    fn test_criterion_3_2_is_fully_automatable() {
        let report = analyze_automatability(&test_criteres_path()).unwrap();
        let c3_2 = report
            .criteria
            .iter()
            .find(|c| c.criterion_id == "3.2")
            .expect("3.2 not found");
        assert_eq!(c3_2.classification, "FullyAutomatable");
    }
}
