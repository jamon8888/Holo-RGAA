# Task 2: Implement axe-core rule fetcher

**Files:**
- Create: `rgaa-rs/crates/rgaa-data/src/fetch.rs`
- Create: `rgaa-rs/crates/rgaa-data/src/parse.rs`

**Interfaces:**
- Produces: `fetch::axe_core_rules() -> Result<Vec<AxeRule>>`
- Produces: `AxeRule { id, description, impact, tags, help, help_url }`

## Steps

- [ ] **Step 1: Write fetch.rs with AxeRule struct**

```rust
use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AxeRule {
    pub id: String,
    pub description: String,
    pub impact: String,
    pub tags: Vec<String>,
    pub help: String,
    pub help_url: String,
}

pub async fn axe_core_rules() -> Result<Vec<AxeRule>> {
    let url = "https://raw.githubusercontent.com/dequelabs/axe-core/develop/doc/rule-descriptions.md";
    let body = reqwest::get(url).await?.text().await?;
    parse::parse_rule_descriptions(&body)
}
```

- [ ] **Step 2: Write parse.rs with markdown parser**

Parse the axe-core rule descriptions markdown table into `Vec<AxeRule>`.

```rust
use super::AxeRule;
use anyhow::{Context, Result};

pub fn parse_rule_descriptions(markdown: &str) -> Result<Vec<AxeRule>> {
    let mut rules = Vec::new();
    for line in markdown.lines() {
        if !line.starts_with("|") || line.starts_with("| Rule ID") || line.starts_with("|---") {
            continue;
        }
        let cols: Vec<&str> = line.split('|').collect();
        if cols.len() < 5 {
            continue;
        }
        let id = cols[1].trim().to_string();
        if id.is_empty() || id == "Rule ID" {
            continue;
        }
        rules.push(AxeRule {
            id,
            description: cols[2].trim().to_string(),
            impact: cols[3].trim().to_string(),
            tags: cols[4].trim().split(',').map(|s| s.trim().to_string()).collect(),
            help: String::new(),
            help_url: String::new(),
        });
    }
    Ok(rules)
}
```

- [ ] **Step 3: Wire fetch into main.rs**

Replace the `todo!()` in fetch.rs call with actual fetch. Update main.rs to use the fetch module.

- [ ] **Step 4: Run the fetcher**

Run: `cargo run -p rgaa-data`
Expected: creates `crates/rgaa-core/data/rgaa-4.1.2/axe_rules.json`

- [ ] **Step 5: Verify output**

Run: `cat crates/rgaa-core/data/rgaa-4.1.2/axe_rules.json | python3 -m json.tool | head -20`
Expected: valid JSON array of axe rule objects

- [ ] **Step 6: Commit**

```bash
git add rgaa-rs/crates/rgaa-data/src/
git commit -m "Add axe-core rule fetcher to rgaa-data crate"
```
