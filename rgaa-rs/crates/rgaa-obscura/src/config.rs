use serde::{Deserialize, Serialize};

use crate::ObscuraError;

const MAX_SELECTOR_LENGTH: usize = 1024;
const MAX_PRE_SCAN_ACTIONS: usize = 20;
pub const MAX_WAITFOR_TIMEOUT_MS: u64 = 30_000;
const MIN_VIEWPORT_WIDTH: u32 = 320;
const MAX_VIEWPORT_WIDTH: u32 = 7680;
const MIN_VIEWPORT_HEIGHT: u32 = 240;
const MAX_VIEWPORT_HEIGHT: u32 = 4320;
const MIN_TIMEOUT_MS: u64 = 100;
const MAX_TIMEOUT_MS: u64 = 300_000;
const MAX_RETRY_LIMIT: u8 = 5;
const MAX_CONCURRENCY: usize = 32;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Viewport {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PreScanAction {
    Click { selector: String },
    Fill { selector: String, value: String },
    WaitFor { selector: String, state: WaitForState },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum WaitForState {
    #[default]
    Visible,
    Attached,
    Hidden,
    Detached,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CookieReference {
    pub name: String,
    pub value: Option<String>,
    pub domain: Option<String>,
    pub path: Option<String>,
    pub same_site: Option<CookieSameSite>,
    pub r#secure: Option<bool>,
    pub http_only: Option<bool>,
    pub expires: Option<i64>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum CookieSameSite {
    #[default]
    Lax,
    Strict,
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ScreenshotPolicy {
    #[default]
    None,
    OnFailure,
    Always,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ScreenshotFormat {
    #[default]
    Png,
    Jpeg,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct ScreenshotConfig {
    pub policy: ScreenshotPolicy,
    pub format: ScreenshotFormat,
    pub save_to: Option<String>,
    pub inline: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum AdvancedRulePolicy {
    #[default]
    Disabled,
    Enabled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum NeedsReviewPolicy {
    #[default]
    Record,
    Fail,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AnalyzeConfig {
    pub profile: String,
    pub viewport: Viewport,
    pub selector: Option<String>,
    pub pre_scan_actions: Vec<PreScanAction>,
    pub cookie_references: Vec<CookieReference>,
    pub screenshot: ScreenshotConfig,
    pub advanced_rule_policy: AdvancedRulePolicy,
    pub needs_review_policy: NeedsReviewPolicy,
    pub igt_tools: Vec<String>,
    pub timeout_ms: u64,
    pub retry_limit: u8,
    pub concurrency: usize,
}

impl Default for AnalyzeConfig {
    fn default() -> Self {
        Self {
            profile: "default".into(),
            viewport: Viewport {
                width: 1000,
                height: 1080,
            },
            selector: None,
            pre_scan_actions: Vec::new(),
            cookie_references: Vec::new(),
            screenshot: ScreenshotConfig::default(),
            advanced_rule_policy: AdvancedRulePolicy::Disabled,
            needs_review_policy: NeedsReviewPolicy::Record,
            igt_tools: Vec::new(),
            timeout_ms: 30_000,
            retry_limit: 0,
            concurrency: 1,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AnalyzeRequest {
    pub url: String,
    pub config: AnalyzeConfig,
}

impl AnalyzeRequest {
    pub fn validate(&self) -> Result<(), ObscuraError> {
        let parsed = reqwest::Url::parse(&self.url)
            .map_err(|_| ObscuraError::Validation("url must be an absolute URL".into()))?;
        if !matches!(parsed.scheme(), "http" | "https") || parsed.host_str().is_none() {
            return Err(ObscuraError::Validation(
                "url must use http or https and include a host".into(),
            ));
        }

        let viewport = &self.config.viewport;
        if !(MIN_VIEWPORT_WIDTH..=MAX_VIEWPORT_WIDTH).contains(&viewport.width)
            || !(MIN_VIEWPORT_HEIGHT..=MAX_VIEWPORT_HEIGHT).contains(&viewport.height)
        {
            return Err(ObscuraError::Validation(
                "viewport dimensions are outside the supported range".into(),
            ));
        }
        if let Some(selector) = &self.config.selector {
            if selector.trim().is_empty() || selector.len() > MAX_SELECTOR_LENGTH {
                return Err(ObscuraError::Validation(
                    "selector must be non-empty and at most 1024 bytes".into(),
                ));
            }
        }
        if self.config.pre_scan_actions.len() > MAX_PRE_SCAN_ACTIONS {
            return Err(ObscuraError::Validation(
                "pre-scan action count exceeds the limit of 20".into(),
            ));
        }
        for action in &self.config.pre_scan_actions {
            let (selector, value) = match action {
                PreScanAction::Click { selector } => (selector, None),
                PreScanAction::Fill { selector, value } => (selector, Some(value)),
                PreScanAction::WaitFor { selector, .. } => (selector, None),
            };
            if selector.trim().is_empty() || selector.len() > MAX_SELECTOR_LENGTH {
                return Err(ObscuraError::Validation(
                    "pre-scan action selector is invalid".into(),
                ));
            }
            if value.is_some_and(|value| value.len() > MAX_SELECTOR_LENGTH) {
                return Err(ObscuraError::Validation(
                    "pre-scan fill value is too long".into(),
                ));
            }
        }
        if !(MIN_TIMEOUT_MS..=MAX_TIMEOUT_MS).contains(&self.config.timeout_ms) {
            return Err(ObscuraError::Validation(
                "timeout must be between 100 and 300000 milliseconds".into(),
            ));
        }
        if self.config.retry_limit > MAX_RETRY_LIMIT {
            return Err(ObscuraError::Validation("retry limit exceeds 5".into()));
        }
        if !(1..=MAX_CONCURRENCY).contains(&self.config.concurrency) {
            return Err(ObscuraError::Validation(
                "concurrency must be between 1 and 32".into(),
            ));
        }
        for cookie in &self.config.cookie_references {
            if cookie.name.trim().is_empty() {
                return Err(ObscuraError::Validation(
                    "cookie reference name must not be empty".into(),
                ));
            }
            if let (Some(same_site), Some(secure)) = (cookie.same_site, cookie.r#secure) {
                if same_site == CookieSameSite::None && !secure {
                    return Err(ObscuraError::Validation(
                        "cookie with SameSite=None requires Secure=true".into(),
                    ));
                }
            }
        }
        Ok(())
    }

    pub fn validate_supported(&self) -> Result<(), ObscuraError> {
        self.validate()?;
        if !matches!(
            self.config.profile.as_str(),
            "default" | "desktop" | "mobile"
        ) {
            return Err(ObscuraError::UnsupportedConfiguration(format!(
                "unknown profile '{}'",
                self.config.profile
            )));
        }
        if self.config.concurrency != 1 {
            return Err(ObscuraError::UnsupportedConfiguration(
                "analyze handles one page; concurrency greater than 1 requires a batch API".into(),
            ));
        }
        if self.config.advanced_rule_policy == AdvancedRulePolicy::Enabled {
            return Err(ObscuraError::UnsupportedConfiguration(
                "advanced rules are not available in the local axe runner".into(),
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_to_desktop_viewport() {
        assert_eq!(
            AnalyzeConfig::default().viewport,
            Viewport {
                width: 1000,
                height: 1080
            }
        );
    }

    #[test]
    fn accepts_mobile_viewport_override() {
        let config = AnalyzeConfig {
            viewport: Viewport {
                width: 375,
                height: 812,
            },
            ..Default::default()
        };
        assert!(AnalyzeRequest {
            url: "https://example.test".into(),
            config
        }
        .validate()
        .is_ok());
    }

    #[test]
    fn rejects_malformed_inputs_and_unbounded_actions() {
        let mut request = AnalyzeRequest {
            url: "not-a-url".into(),
            config: AnalyzeConfig::default(),
        };
        assert!(request.validate().is_err());
        request.url = "https://example.test".into();
        request.config.selector = Some(" ".into());
        assert!(request.validate().is_err());
        request.config.selector = None;
        request.config.pre_scan_actions = (0..21)
            .map(|_| PreScanAction::Click {
                selector: "#ok".into(),
            })
            .collect();
        assert!(request.validate().is_err());
        // WaitFor variant is valid
        request.config.pre_scan_actions = vec![PreScanAction::WaitFor {
            selector: "#app".into(),
            state: WaitForState::Visible,
        }];
        assert!(request.validate().is_ok());
    }

    #[test]
    fn cookie_serialization_contains_name_and_domain() {
        let config = AnalyzeConfig {
            cookie_references: vec![CookieReference {
                name: "session".into(),
                value: Some("abc123".into()),
                domain: Some("example.test".into()),
                path: Some("/".into()),
                same_site: Some(CookieSameSite::Lax),
                r#secure: Some(false),
                http_only: Some(true),
                expires: None,
            }],
            ..Default::default()
        };
        let json = serde_json::to_string(&config).expect("config serializes");
        assert!(json.contains("session"));
        assert!(json.contains("example"));
        assert!(json.contains("abc123"));
    }

    #[test]
    fn screenshot_is_opt_in() {
        assert_eq!(
            AnalyzeConfig::default().screenshot.policy,
            ScreenshotPolicy::None
        );
    }

    #[test]
    fn rejects_configuration_that_analyze_cannot_execute() {
        let mut request = AnalyzeRequest {
            url: "https://example.test".into(),
            config: AnalyzeConfig::default(),
        };
        request.config.advanced_rule_policy = AdvancedRulePolicy::Enabled;
        assert!(matches!(
            request.validate_supported(),
            Err(ObscuraError::UnsupportedConfiguration(_))
        ));

        request.config.advanced_rule_policy = AdvancedRulePolicy::Disabled;
        request.config.concurrency = 2;
        assert!(matches!(
            request.validate_supported(),
            Err(ObscuraError::UnsupportedConfiguration(_))
        ));
    }
}
