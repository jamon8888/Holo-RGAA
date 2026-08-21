use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Viewport dimensions for browser automation.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct ViewportProfile {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
}

/// URL profile with optional viewport configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct UrlProfile {
    /// The URL to audit.
    pub url: String,
    /// Optional viewport profile name to use.
    #[serde(default)]
    pub viewport: Option<String>,
}

/// Policy configuration for compliance checks.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PolicyConfig {
    /// Minimum compliance percentage required (0-100).
    #[serde(default = "default_min_compliance")]
    pub min_compliance: f64,
    /// List of criteria that must pass.
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

/// CLI configuration loaded from `.rgaa/config.yaml`.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct Config {
    /// Named URL profiles for auditing.
    #[serde(default)]
    pub url_profiles: HashMap<String, UrlProfile>,
    /// Named viewport profiles for browser automation.
    #[serde(default)]
    pub viewport_profiles: HashMap<String, ViewportProfile>,
    /// List of guided tests available to run.
    #[serde(default)]
    pub guided_tests: Vec<String>,
    /// Applicable accessibility standards.
    #[serde(default)]
    pub standards: Vec<String>,
    /// Policy configuration for compliance checks.
    #[serde(default)]
    pub policy: PolicyConfig,
    /// Directory to store evidence artifacts.
    #[serde(default)]
    pub evidence_dir: Option<String>,
    /// Remote API endpoint for uploading results.
    #[serde(default)]
    pub remote_endpoint: Option<String>,
    /// Whether the user has consented to upload results.
    #[serde(default)]
    pub upload_consent: bool,
}

/// Errors that can occur when loading or validating configuration.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    /// Failed to read the config file.
    #[error("failed to read config file {path}: {source}")]
    Io {
        /// Path to the config file.
        path: String,
        /// The underlying I/O error.
        #[source]
        source: std::io::Error,
    },
    /// Failed to parse the config file.
    #[error("invalid config: {0}")]
    Parse(String),
    /// Configuration validation failed.
    #[error("invalid config: {0}")]
    Validation(String),
}

/// Returns the default config file path (`.rgaa/config.yaml`).
pub fn default_config_path() -> PathBuf {
    PathBuf::from(".rgaa").join("config.yaml")
}

impl Config {
    /// Loads configuration from a YAML file.
    ///
    /// If no path is provided, uses the default path (`.rgaa/config.yaml`).
    /// Returns default configuration if the file doesn't exist.
    ///
    /// # Arguments
    ///
    /// * `path` - Optional path to the config file.
    ///
    /// # Errors
    ///
    /// Returns `ConfigError` if the file cannot be read, parsed, or validated.
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

    /// Validates the configuration.
    ///
    /// Checks that:
    /// - `min_compliance` is between 0 and 100
    /// - Viewport profiles have non-zero dimensions
    /// - URL profiles have non-empty URLs
    ///
    /// # Errors
    ///
    /// Returns `ConfigError::Validation` if any validation check fails.
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
