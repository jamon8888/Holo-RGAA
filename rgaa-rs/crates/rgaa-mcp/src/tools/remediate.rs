use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RemediationIssueInput {
    pub id: String,
    pub rule: String,
    pub element_html: String,
    pub page_url: String,
    #[serde(default)]
    pub source_locations: Vec<SourceLocationInput>,
    pub summary: String,
    pub remediation: String,
    #[serde(default)]
    pub criteria: Vec<String>,
    #[serde(default)]
    pub framework: Option<FrameworkInput>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum FrameworkInput {
    React,
    Next,
    Vue,
    Angular,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SourceLocationInput {
    pub file: String,
    pub line: u32,
    #[serde(default)]
    pub column: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RemediationResponse {
    pub outcomes: Vec<serde_json::Value>,
}
