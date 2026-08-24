use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::parse;

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
