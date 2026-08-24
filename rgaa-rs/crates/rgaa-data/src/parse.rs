use anyhow::Result;

use crate::fetch::AxeRule;

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
