use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GuidedTestInput {
    pub id: String,
    pub version: u32,
    #[serde(default)]
    pub preconditions: Vec<String>,
    pub steps: Vec<serde_json::Value>,
    #[serde(default)]
    pub criterion_mapping: Vec<String>,
    #[serde(default)]
    pub evidence_requirements: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GuidedTestResponse {
    pub issues: Vec<String>,
    pub unanalyzed_elements: Vec<String>,
    pub terminated_reason: String,
    pub completed_steps: usize,
    pub evidence: Vec<serde_json::Value>,
    pub manual_review_required: bool,
}
