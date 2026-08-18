use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AnalyzeConfigInput {
    #[serde(default = "default_profile")]
    pub profile: String,
    #[serde(default = "default_width")]
    pub viewport_width: u32,
    #[serde(default = "default_height")]
    pub viewport_height: u32,
    #[serde(default)]
    pub selector: Option<String>,
    #[serde(default)]
    pub pre_scan_actions: Vec<PreScanActionInput>,
    #[serde(default)]
    pub cookie_references: Vec<CookieReferenceInput>,
    #[serde(default)]
    pub screenshot_policy: ScreenshotPolicyInput,
    #[serde(default)]
    pub timeout_ms: Option<u64>,
    #[serde(default)]
    pub retry_limit: Option<u8>,
}

impl Default for AnalyzeConfigInput {
    fn default() -> Self {
        let config = rgaa_obscura::AnalyzeConfig::default();
        Self {
            profile: config.profile,
            viewport_width: config.viewport.width,
            viewport_height: config.viewport.height,
            selector: config.selector,
            pre_scan_actions: Vec::new(),
            cookie_references: Vec::new(),
            screenshot_policy: ScreenshotPolicyInput::None,
            timeout_ms: None,
            retry_limit: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum ScreenshotPolicyInput {
    #[default]
    None,
    OnFailure,
    Always,
}

fn default_profile() -> String {
    "default".into()
}
fn default_width() -> u32 {
    1000
}
fn default_height() -> u32 {
    1080
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PreScanActionInput {
    Click { selector: String },
    Fill { selector: String, value: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CookieReferenceInput {
    pub name: String,
    #[serde(default)]
    pub domain: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AnalyzeResponse {
    pub url: String,
    pub findings: Vec<serde_json::Value>,
    pub evidence: Vec<serde_json::Value>,
    pub errors: Vec<serde_json::Value>,
    pub completed: bool,
    pub duration_ms: u64,
}
