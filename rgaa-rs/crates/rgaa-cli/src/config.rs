use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct ViewportProfile {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct UrlProfile {
    pub url: String,
    #[serde(default)]
    pub viewport: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PolicyConfig {
    #[serde(default = "default_min_compliance")]
    pub min_compliance: f64,
    #[serde(default)]
    pub required_criteria: Vec<String>,
}

impl Default for PolicyConfig {
    fn default() -> Self {
        Self {
            min_compliance: default_min_compliance(),
            required_criteria: Vec::new(),
        }
    }
}

fn default_min_compliance() -> f64 {
    80.0
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct Config {
    #[serde(default)]
    pub url_profiles: HashMap<String, UrlProfile>,
    #[serde(default)]
    pub viewport_profiles: HashMap<String, ViewportProfile>,
    #[serde(default)]
    pub guided_tests: Vec<String>,
    #[serde(default)]
    pub standards: Vec<String>,
    #[serde(default)]
    pub policy: PolicyConfig,
    #[serde(default)]
    pub evidence_dir: Option<String>,
    #[serde(default)]
    pub remote_endpoint: Option<String>,
    #[serde(default)]
    pub upload_consent: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("failed to read config file {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("invalid config: {0}")]
    Parse(String),
    #[error("invalid config: {0}")]
    Validation(String),
}

pub fn default_config_path() -> PathBuf {
    PathBuf::from(".rgaa").join("config.yaml")
}

impl Config {
    pub fn load(path: Option<&Path>) -> Result<Self, ConfigError> {
        let path = path
            .map(Path::to_path_buf)
            .unwrap_or_else(default_config_path);
        if !path.exists() {
            return Ok(Self::default());
        }
        let raw = std::fs::read_to_string(&path).map_err(|source| ConfigError::Io {
            path: path.display().to_string(),
            source,
        })?;
        let config: Config =
            serde_yaml::from_str(&raw).map_err(|error| ConfigError::Parse(error.to_string()))?;
        config.validate()?;
        Ok(config)
    }

    pub fn validate(&self) -> Result<(), ConfigError> {
        if !(0.0..=100.0).contains(&self.policy.min_compliance) {
            return Err(ConfigError::Validation(
                "policy.min_compliance must be between 0 and 100".into(),
            ));
        }
        for (name, viewport) in &self.viewport_profiles {
            if viewport.width == 0 || viewport.height == 0 {
                return Err(ConfigError::Validation(format!(
                    "viewport profile '{name}' must have non-zero dimensions"
                )));
            }
        }
        for (name, profile) in &self.url_profiles {
            if profile.url.trim().is_empty() {
                return Err(ConfigError::Validation(format!(
                    "url profile '{name}' must have a non-empty url"
                )));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_config_falls_back_to_defaults() {
        let config = Config::load(Some(Path::new("/nonexistent/config.yaml"))).expect("defaults");
        assert_eq!(config.policy.min_compliance, 80.0);
        assert!(!config.upload_consent);
    }

    #[test]
    fn invalid_min_compliance_is_rejected() {
        let mut config = Config::default();
        config.policy.min_compliance = 120.0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn zero_viewport_is_rejected() {
        let mut config = Config::default();
        config.viewport_profiles.insert(
            "broken".into(),
            ViewportProfile {
                width: 0,
                height: 10,
            },
        );
        assert!(config.validate().is_err());
    }
}
