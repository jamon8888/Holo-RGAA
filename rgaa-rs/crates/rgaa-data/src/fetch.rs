use anyhow::Result;
use serde::Deserialize;

/// axe-core rule as returned by the GitHub API.
#[derive(Debug, Deserialize, serde::Serialize)]
pub struct AxeRule {
    pub id: String,
    pub description: String,
}

/// Fetch axe-core rule descriptions from the GitHub repository.
pub async fn axe_core_rules() -> Result<Vec<AxeRule>> {
    todo!("fetch axe-core rules from GitHub API")
}
