use serde::{Deserialize, Serialize};

use crate::ObscuraError;

const MAX_SELECTOR_LENGTH: usize = 1024;
const MAX_PRE_SCAN_ACTIONS: usize = 20;
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
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CookieReference {
    pub name: String,
    pub domain: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ScreenshotPolicy {
    None,
    OnFailure,
    Always,
}

impl Default for ScreenshotPolicy {
    fn default() -> Self {
        Self::None
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AdvancedRulePolicy {
    Disabled,
    Enabled,
}

impl Default for AdvancedRulePolicy {
    fn default() -> Self {
        Self::Disabled
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NeedsReviewPolicy {
    Record,
    Fail,
}

impl Default for NeedsReviewPolicy {
    fn default() -> Self {
        Self::Record
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AnalyzeConfig {
    pub profile: String,
    pub viewport: Viewport,
    pub selector: Option<String>,
    pub pre_scan_actions: Vec<PreScanAction>,
    pub cookie_references: Vec<CookieReference>,
    pub screenshot_policy: ScreenshotPolicy,
    pub advanced_rule_policy: AdvancedRulePolicy,
    pub needs_review_policy: NeedsReviewPolicy,
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
            screenshot_policy: ScreenshotPolicy::None,
            advanced_rule_policy: AdvancedRulePolicy::Disabled,
            needs_review_policy: NeedsReviewPolicy::Record,
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
        let mut config = AnalyzeConfig::default();
        config.viewport = Viewport {
            width: 375,
            height: 812,
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
    }

    #[test]
    fn cookie_serialization_contains_references_only() {
        let config = AnalyzeConfig {
            cookie_references: vec![CookieReference {
                name: "session".into(),
                domain: Some("example.test".into()),
            }],
            ..Default::default()
        };
        let json = serde_json::to_string(&config).expect("config serializes");
        assert!(json.contains("session"));
        assert!(!json.contains("value"));
    }

    #[test]
    fn screenshot_is_opt_in() {
        assert_eq!(
            AnalyzeConfig::default().screenshot_policy,
            ScreenshotPolicy::None
        );
    }
}
