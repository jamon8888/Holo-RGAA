use rgaa_remediation::{
    detect_framework, remediate, Framework, RemediationIssue, RemediationOutcome,
    RemediationPolicy, SourceLocation,
};
use rig_core::tool::PortableTool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Errors that can occur when using the remediate tool.
#[derive(Debug, thiserror::Error)]
pub enum RemediateError {
    #[error("remediation failed: {0}")]
    RemediationFailed(String),

    #[error("remediation returned no outcome for the issue")]
    NoOutcome,
}

/// Arguments for the remediate tool.
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct RemediateArgs {
    /// The finding ID to remediate
    pub finding_id: String,
    /// The axe-core rule ID (e.g., "image-alt", "button-name")
    pub rule: String,
    /// The HTML source of the offending element
    pub element_html: String,
    /// The page URL where the finding was detected
    pub page_url: String,
    /// Source file locations for the fix
    pub source_locations: Vec<SourceLocation>,
    /// Optional human-readable summary of the finding, carried through to the
    /// generated remediation context.
    pub summary: Option<String>,
    /// Optional suggested remediation steps, carried through to the generated
    /// remediation context.
    pub remediation: Option<String>,
    /// Optional related RGAA criteria IDs, carried through to the generated
    /// remediation context.
    pub criteria: Option<Vec<String>>,
    /// Optional explicit framework hint (e.g., "react", "vue", "angular", "html").
    /// If provided, skips framework detection from element_html.
    pub framework: Option<String>,
}

/// Tool that generates remediation proposals for accessibility findings.
pub struct RemediateTool {
    policy: RemediationPolicy,
}

impl RemediateTool {
    pub fn new(policy: RemediationPolicy) -> Self {
        Self { policy }
    }
}

impl PortableTool for RemediateTool {
    const NAME: &str = "remediate";
    type Error = RemediateError;
    type Args = RemediateArgs;
    type Output = RemediationOutcome;

    fn description(&self) -> String {
        "Generate a remediation patch proposal for an accessibility finding".to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::to_value(schemars::schema_for!(RemediateArgs)).expect("valid schema")
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let framework = if let Some(fw) = args.framework {
            parse_framework(&fw).ok_or_else(|| {
                RemediateError::RemediationFailed(format!("unsupported framework: {fw}"))
            })?
        } else {
            detect_framework(&args.element_html).ok_or_else(|| {
                RemediateError::RemediationFailed(
                    "could not detect a supported UI framework from the element HTML".into(),
                )
            })?
        };
        let adapter = rgaa_remediation::adapter_for(framework);

        let issue = RemediationIssue {
            id: args.finding_id,
            rule: args.rule,
            element_html: args.element_html,
            page_url: args.page_url,
            source_locations: args.source_locations,
            summary: args.summary.unwrap_or_default(),
            remediation: args.remediation.unwrap_or_default(),
            criteria: args.criteria.unwrap_or_default(),
            framework: Some(framework),
        };

        let outcomes = remediate(&[issue], &self.policy, adapter)
            .map_err(|e| RemediateError::RemediationFailed(e.to_string()))?;

        outcomes.into_iter().next().ok_or(RemediateError::NoOutcome)
    }
}

/// Parses a framework string into a [`Framework`] enum.
///
/// Supported values: "react", "vue", "angular", "html" (case-insensitive).
fn parse_framework(s: &str) -> Option<Framework> {
    match s.to_lowercase().as_str() {
        "react" => Some(Framework::React),
        "vue" => Some(Framework::Vue),
        "angular" => Some(Framework::Angular),
        "next" => Some(Framework::Next),
        _ => None,
    }
}
