use rgaa_core::{Classification, CriterionResult, CriterionStatus, Violation};
use std::collections::HashMap;

pub struct AxeMapper;

impl AxeMapper {
    /// Map axe-core violations JSON to RGAA criterion results.
    /// Input: JSON array of axe violations from axe.run()
    /// Output: HashMap of criterion_id → CriterionResult
    pub fn map(violations_json: &str) -> HashMap<String, CriterionResult> {
        let mapping = Self::rgaa_to_axe_map();
        let violations: Vec<AxeViolation> =
            serde_json::from_str(violations_json).unwrap_or_default();

        let mut results: HashMap<String, CriterionResult> = HashMap::new();

        // Initialize all axe-mapped criteria as PASS
        for rgaa_id in mapping.keys() {
            results.insert(
                rgaa_id.clone(),
                CriterionResult {
                    criterion_id: rgaa_id.clone(),
                    title: String::new(),
                    classification: Classification::Deterministe,
                    status: CriterionStatus::Pass,
                    violations: vec![],
                    confidence: None,
                    justification: None,
                    source: "axe-core".to_string(),
                },
            );
        }

        // Map violations to criteria
        for violation in &violations {
            for (rgaa_id, axe_rules) in &mapping {
                if axe_rules.iter().any(|rule| rule == &violation.id) {
                    if let Some(result) = results.get_mut(rgaa_id) {
                        result.status = CriterionStatus::Fail;
                        result.violations.push(Violation {
                            rule_id: violation.id.clone(),
                            impact: violation.impact.clone(),
                            description: violation.description.clone(),
                            nodes_affected: violation.nodes.len(),
                        });
                    }
                }
            }
        }

        results
    }

    fn rgaa_to_axe_map() -> HashMap<String, Vec<String>> {
        let mut m: HashMap<String, Vec<String>> = HashMap::new();
        // From existing poc.js — 77 criteria mapped
        m.insert(
            "1.1".into(),
            vec!["image-alt".into(), "input-image-alt".into()],
        );
        m.insert(
            "1.2".into(),
            vec!["image-alt".into(), "image-redundant-alt".into()],
        );
        m.insert("1.5".into(), vec!["image-alt".into()]);
        m.insert("1.6".into(), vec!["image-alt".into(), "longdesc".into()]);
        m.insert("1.8".into(), vec!["image-text".into()]);
        m.insert("1.9".into(), vec!["figure-caption".into()]);
        m.insert("2.1".into(), vec!["iframe-title".into()]);
        m.insert("3.2".into(), vec!["color-contrast".into()]);
        m.insert("3.3".into(), vec!["color-contrast".into()]);
        m.insert(
            "4.1".into(),
            vec!["audio-description".into(), "video-description".into()],
        );
        m.insert("4.3".into(), vec!["video-caption".into()]);
        m.insert(
            "4.5".into(),
            vec!["audio-description".into(), "video-description".into()],
        );
        m.insert(
            "4.7".into(),
            vec!["video-description".into(), "audio-description".into()],
        );
        m.insert(
            "4.8".into(),
            vec!["video-description".into(), "audio-description".into()],
        );
        m.insert("4.10".into(), vec!["audio-control".into()]);
        m.insert(
            "4.11".into(),
            vec!["keyboard".into(), "keyboard-trap".into()],
        );
        m.insert(
            "4.12".into(),
            vec!["keyboard".into(), "keyboard-trap".into()],
        );
        m.insert(
            "4.13".into(),
            vec!["video-description".into(), "audio-description".into()],
        );
        m.insert("5.1".into(), vec!["table-header".into()]);
        m.insert("5.4".into(), vec!["table-header".into()]);
        m.insert(
            "5.6".into(),
            vec!["table-header".into(), "td-headers-attr".into()],
        );
        m.insert(
            "5.7".into(),
            vec!["td-headers-attr".into(), "th-has-data-cells".into()],
        );
        m.insert("5.8".into(), vec!["layout-table".into()]);
        m.insert(
            "6.1".into(),
            vec!["link-name".into(), "link-purpose-in-context".into()],
        );
        m.insert("6.2".into(), vec!["link-name".into()]);
        m.insert(
            "7.1".into(),
            vec![
                "keyboard".into(),
                "keyboard-trap".into(),
                "focus-order".into(),
            ],
        );
        m.insert(
            "7.3".into(),
            vec![
                "keyboard".into(),
                "keyboard-trap".into(),
                "focus-visible".into(),
            ],
        );
        m.insert("7.4".into(), vec!["on-focus".into(), "on-input".into()]);
        m.insert("8.1".into(), vec!["doctype".into()]);
        m.insert(
            "8.2".into(),
            vec!["html-has-lang".into(), "html-lang-valid".into()],
        );
        m.insert("8.3".into(), vec!["html-has-lang".into()]);
        m.insert("8.5".into(), vec!["page-title".into()]);
        m.insert("8.7".into(), vec!["lang".into()]);
        m.insert(
            "8.9".into(),
            vec!["layout-table".into(), "deprecated-element".into()],
        );
        m.insert(
            "8.10".into(),
            vec!["focus-order".into(), "meaningful-sequence".into()],
        );
        m.insert(
            "9.1".into(),
            vec![
                "heading-order".into(),
                "landmark-one-main".into(),
                "region".into(),
            ],
        );
        m.insert("9.3".into(), vec!["list".into(), "listitem".into()]);
        m.insert("9.4".into(), vec!["blockquote".into()]);
        m.insert("10.1".into(), vec!["deprecated-element".into()]);
        m.insert(
            "10.2".into(),
            vec!["color-contrast".into(), "image-alt".into()],
        );
        m.insert("10.4".into(), vec!["resize-text".into()]);
        m.insert("10.5".into(), vec!["color-contrast".into()]);
        m.insert("10.6".into(), vec!["link-in-text-block".into()]);
        m.insert("10.7".into(), vec!["focus-visible".into()]);
        m.insert(
            "10.8".into(),
            vec!["aria-hidden-focus".into(), "hidden-content".into()],
        );
        m.insert(
            "10.9".into(),
            vec!["color-contrast".into(), "image-alt".into()],
        );
        m.insert("10.11".into(), vec!["reflow".into()]);
        m.insert("10.12".into(), vec!["text-spacing".into()]);
        m.insert(
            "10.13".into(),
            vec!["focus-visible".into(), "keyboard".into()],
        );
        m.insert("10.14".into(), vec!["keyboard".into()]);
        m.insert(
            "11.1".into(),
            vec![
                "label".into(),
                "label-title-only".into(),
                "input-image-alt".into(),
            ],
        );
        m.insert("11.4".into(), vec!["label".into()]);
        m.insert("11.5".into(), vec!["fieldset".into()]);
        m.insert("11.6".into(), vec!["fieldset".into()]);
        m.insert("11.11".into(), vec!["error-suggestion".into()]);
        m.insert("11.12".into(), vec!["error-prevention".into()]);
        m.insert("11.13".into(), vec!["autocomplete".into()]);
        m.insert(
            "12.1".into(),
            vec!["landmark-one-main".into(), "region".into()],
        );
        m.insert("12.2".into(), vec!["consistent-navigation".into()]);
        m.insert(
            "12.4".into(),
            vec!["landmark-one-main".into(), "region".into()],
        );
        m.insert("12.5".into(), vec!["consistent-navigation".into()]);
        m.insert(
            "12.6".into(),
            vec!["landmark-one-main".into(), "region".into(), "bypass".into()],
        );
        m.insert("12.7".into(), vec!["bypass".into(), "skip-link".into()]);
        m.insert("12.9".into(), vec!["keyboard-trap".into()]);
        m.insert("12.10".into(), vec!["character-key-shortcuts".into()]);
        m.insert("12.11".into(), vec!["keyboard".into()]);
        m.insert(
            "13.1".into(),
            vec!["timing-adjustable".into(), "pause-stop-hide".into()],
        );
        m.insert("13.2".into(), vec!["on-focus".into()]);
        m.insert("13.3".into(), vec!["document-title".into(), "pdf".into()]);
        m.insert("13.4".into(), vec!["document-title".into(), "pdf".into()]);
        m.insert(
            "13.5".into(),
            vec!["image-alt".into(), "non-text-content".into()],
        );
        m.insert("13.7".into(), vec!["three-flashes".into()]);
        m.insert(
            "13.8".into(),
            vec!["pause-stop-hide".into(), "timing-adjustable".into()],
        );
        m.insert("13.9".into(), vec!["orientation".into()]);
        m.insert("13.10".into(), vec!["pointer-gestures".into()]);
        m.insert("13.11".into(), vec!["pointer-cancellation".into()]);
        m.insert("13.12".into(), vec!["motion-actuation".into()]);
        m
    }
}

#[derive(serde::Deserialize)]
struct AxeViolation {
    id: String,
    impact: String,
    description: String,
    nodes: Vec<serde_json::Value>,
}
