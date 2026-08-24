use anyhow::Result;
use std::collections::HashMap;

use crate::fetch::AxeRule;

pub fn load_existing_mapping() -> Result<HashMap<String, Vec<String>>> {
    let mut m: HashMap<String, Vec<String>> = HashMap::new();
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
    Ok(m)
}

pub fn parse_rule_descriptions(markdown: &str) -> Result<Vec<AxeRule>> {
    let mut rules = Vec::new();
    for line in markdown.lines() {
        if !line.starts_with('|') {
            continue;
        }
        let trimmed = line.trim();
        if trimmed.contains("---") || trimmed.starts_with("| Rule ID") {
            continue;
        }
        let cols: Vec<&str> = line.split('|').collect();
        if cols.len() < 5 {
            continue;
        }
        let raw_id_col = cols[1].trim();
        if raw_id_col.is_empty() || raw_id_col.starts_with("Rule ID") {
            continue;
        }
        let (id, help_url) = extract_link(raw_id_col);
        let description = cols[2].trim().to_string();
        let impact = cols[3].trim().to_string();
        let tags = parse_tags(cols[4].trim());
        rules.push(AxeRule {
            id,
            description: description.clone(),
            impact,
            tags,
            help: description,
            help_url,
        });
    }
    Ok(rules)
}

fn extract_link(cell: &str) -> (String, String) {
    if let (Some(start), Some(mid)) = (cell.find('['), cell.find("](")) {
        if let Some(end) = cell.find(')') {
            let id = &cell[start + 1..mid];
            let url = &cell[mid + 2..end];
            return (id.to_string(), url.to_string());
        }
    }
    (cell.to_string(), String::new())
}

fn parse_tags(cell: &str) -> Vec<String> {
    cell.split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_table() {
        let md = "\
| Rule ID | Description | Impact | Tags |
|---|---|---|---|
| [foo](https://example.com/foo) | Do foo | Critical | cat.a, wcag2a |
| [bar](https://example.com/bar) | Do bar | Serious | cat.b, wcag21aa |
";
        let rules = parse_rule_descriptions(md).unwrap();
        assert_eq!(rules.len(), 2);
        assert_eq!(rules[0].id, "foo");
        assert_eq!(rules[0].impact, "Critical");
        assert_eq!(rules[0].tags, vec!["cat.a", "wcag2a"]);
        assert_eq!(rules[0].help_url, "https://example.com/foo");
        assert_eq!(rules[1].id, "bar");
    }

    #[test]
    fn skips_non_table_lines() {
        let md = "Some intro text\n\n| Rule ID | Description | Impact | Tags |\n|---|---|---|---|\n| [x](https://example.com/x) | Desc | Minor | cat.c |\n";
        let rules = parse_rule_descriptions(md).unwrap();
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].id, "x");
    }

    #[test]
    fn handle_plain_id_without_link() {
        let md = "| Rule ID | Description | Impact | Tags |\n|---|---|---|---|\n| plain-id | Desc | Serious | cat.d |\n";
        let rules = parse_rule_descriptions(md).unwrap();
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].id, "plain-id");
        assert!(rules[0].help_url.is_empty());
    }
}
