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

/// One recurring job for `rgaa schedule`.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct JobConfig {
    pub name: String,
    /// Explicit URLs to audit.
    #[serde(default)]
    pub urls: Vec<String>,
    /// Names of `url_profiles` entries to audit (merged with `urls`).
    #[serde(default)]
    pub profiles: Vec<String>,
    /// Daily local time, `HH:MM`. Exclusive with `every`.
    #[serde(default)]
    pub at: Option<String>,
    /// Fixed interval such as `6h`, `90m`, `3600s`. Exclusive with `at`.
    #[serde(default)]
    pub every: Option<String>,
    /// Source file that renders the document `<head>`; enables patch proposals.
    #[serde(default)]
    pub head_template: Option<PathBuf>,
}

/// Google Business Profile facts for NAP consistency rules.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct BusinessProfileConfig {
    pub name: String,
    pub phone: String,
    pub address: String,
}

/// `schedule:` section of the config.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct ScheduleConfig {
    #[serde(default)]
    pub jobs: Vec<JobConfig>,
    #[serde(default)]
    pub business_profile: Option<BusinessProfileConfig>,
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
    /// Recurring jobs for `rgaa schedule`.
    #[serde(default)]
    pub schedule: ScheduleConfig,
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
        let mut seen = std::collections::HashSet::new();
        for job in &self.schedule.jobs {
            let name = job.name.trim();
            if name.is_empty() {
                return Err(ConfigError::Validation(
                    "schedule job must have a name".into(),
                ));
            }
            if !seen.insert(name) {
                return Err(ConfigError::Validation(format!(
                    "schedule job '{name}' is defined twice"
                )));
            }
            if job.urls.is_empty() && job.profiles.is_empty() {
                return Err(ConfigError::Validation(format!(
                    "schedule job '{name}' needs urls or profiles"
                )));
            }
            for profile in &job.profiles {
                if !self.url_profiles.contains_key(profile) {
                    return Err(ConfigError::Validation(format!(
                        "schedule job '{name}' references unknown url profile '{profile}'"
                    )));
                }
            }
            match (&job.at, &job.every) {
                (Some(_), Some(_)) => {
                    return Err(ConfigError::Validation(format!(
                        "schedule job '{name}': set either at or every, not both"
                    )))
                }
                (Some(at), None) => {
                    parse_daily_time(at).map_err(|e| {
                        ConfigError::Validation(format!("schedule job '{name}': {e}"))
                    })?;
                }
                (None, Some(every)) => {
                    parse_interval(every).map_err(|e| {
                        ConfigError::Validation(format!("schedule job '{name}': {e}"))
                    })?;
                }
                (None, None) => {}
            }
        }
        if let Some(profile) = &self.schedule.business_profile {
            if profile.name.trim().is_empty()
                || profile.phone.trim().is_empty()
                || profile.address.trim().is_empty()
            {
                return Err(ConfigError::Validation(
                    "schedule.business_profile needs name, phone and address".into(),
                ));
            }
        }
        Ok(())
    }
}

/// Parses `HH:MM` (24h) into `(hour, minute)`.
pub fn parse_daily_time(value: &str) -> Result<(u32, u32), String> {
    let (h, m) = value
        .trim()
        .split_once(':')
        .ok_or_else(|| format!("invalid time '{value}', expected HH:MM"))?;
    let hour: u32 = h
        .parse()
        .map_err(|_| format!("invalid hour in '{value}'"))?;
    let minute: u32 = m
        .parse()
        .map_err(|_| format!("invalid minute in '{value}'"))?;
    if hour > 23 || minute > 59 {
        return Err(format!("time '{value}' out of range"));
    }
    Ok((hour, minute))
}

/// Parses `<n>s`, `<n>m`, `<n>h` or `<n>d` into a duration of at least one minute.
pub fn parse_interval(value: &str) -> Result<std::time::Duration, String> {
    let value = value.trim();
    let (digits, unit) = value.split_at(value.trim_end_matches(char::is_alphabetic).len());
    let n: u64 = digits
        .parse()
        .map_err(|_| format!("invalid interval '{value}', expected e.g. 6h, 90m, 3600s"))?;
    let secs = match unit {
        "s" => n,
        "m" => n * 60,
        "h" => n * 3600,
        "d" => n * 86_400,
        _ => {
            return Err(format!(
                "invalid interval unit in '{value}' (use s, m, h or d)"
            ))
        }
    };
    if secs < 60 {
        return Err(format!("interval '{value}' is below one minute"));
    }
    Ok(std::time::Duration::from_secs(secs))
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
    fn schedule_jobs_are_validated() {
        let mut config = Config::default();
        config.url_profiles.insert(
            "home".into(),
            UrlProfile {
                url: "https://a.test".into(),
                viewport: None,
            },
        );
        let ok = JobConfig {
            name: "nightly".into(),
            profiles: vec!["home".into()],
            at: Some("02:00".into()),
            ..Default::default()
        };
        config.schedule.jobs = vec![ok.clone()];
        assert!(config.validate().is_ok());

        let cases = [
            JobConfig {
                name: " ".into(),
                ..ok.clone()
            },
            JobConfig {
                profiles: vec![],
                ..ok.clone()
            },
            JobConfig {
                profiles: vec!["missing".into()],
                ..ok.clone()
            },
            JobConfig {
                every: Some("6h".into()),
                ..ok.clone()
            },
            JobConfig {
                at: Some("25:00".into()),
                ..ok.clone()
            },
            JobConfig {
                at: None,
                every: Some("30s".into()),
                ..ok.clone()
            },
        ];
        for bad in cases {
            config.schedule.jobs = vec![bad.clone()];
            assert!(config.validate().is_err(), "{bad:?}");
        }
        config.schedule.jobs = vec![ok.clone(), ok];
        assert!(config.validate().is_err(), "duplicate names");
    }

    #[test]
    fn schedule_section_parses_from_yaml() {
        let yaml = r#"
url_profiles:
  home: { url: "https://a.test" }
schedule:
  business_profile: { name: "Dupont", phone: "04 12 34 56 78", address: "12 rue X" }
  jobs:
    - name: nightly
      profiles: [home]
      urls: ["https://a.test/contact"]
      at: "02:30"
      head_template: src/layout.html
    - name: hourly
      urls: ["https://a.test"]
      every: 1h
"#;
        let config: Config = serde_yaml::from_str(yaml).unwrap();
        config.validate().unwrap();
        assert_eq!(config.schedule.jobs.len(), 2);
        assert_eq!(parse_daily_time("02:30").unwrap(), (2, 30));
        assert_eq!(
            parse_interval("1h").unwrap(),
            std::time::Duration::from_secs(3600)
        );
        assert_eq!(
            parse_interval("2d").unwrap(),
            std::time::Duration::from_secs(172_800)
        );
        assert!(parse_interval("10x").is_err());
        assert!(parse_daily_time("2h").is_err());
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
