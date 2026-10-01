//! `lint-rules.toml`: the per-user severity configuration.
//!
//! The governing rule here is that this file never fails quietly. A team that
//! demotes `link-name` to a warning and then sees it reported as an error has
//! lost trust in the tool; a team whose config has a typo and silently gets the
//! built-in defaults has lost it twice over, because the tool looks like it is
//! obeying them. So: a config that is asked for and absent is an error, a config
//! that is present and unreadable is an error, and an unknown key or severity is
//! an error naming the offending key. Only the complete absence of any config —
//! the state of a user who never wrote one — yields defaults, and the report
//! then says so in [`crate::LintReport::config_source`].

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::profile::Profile;
use crate::rules::{RuleId, Severity};

/// The file name looked up in the user profile directory.
pub const CONFIG_FILE_NAME: &str = "lint-rules.toml";

/// Environment variable that overrides the config location outright.
pub const CONFIG_ENV_VAR: &str = "RGAA_LINT_CONFIG";

/// The only schema version this build understands.
pub const SUPPORTED_VERSION: u32 = 1;

/// Why a configuration could not be honoured.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    /// A path was named explicitly (argument or environment) and does not exist.
    #[error("lint config not found at {path} (named by {origin}); create it or drop the override")]
    NotFound { path: PathBuf, origin: String },
    /// The file exists but could not be read.
    #[error("lint config at {path} could not be read: {source}")]
    Unreadable {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    /// The file exists and is not valid for this schema.
    #[error("lint config at {path} is invalid: {message}")]
    Invalid { path: PathBuf, message: String },
    /// The file declares a schema version this build does not implement.
    #[error(
        "lint config at {path} declares version {found}, but this build implements version \
         {SUPPORTED_VERSION}"
    )]
    UnsupportedVersion { path: PathBuf, found: u32 },
}

/// The on-disk shape of `lint-rules.toml`.
///
/// `deny_unknown_fields` is what turns a misspelled key into the error the user
/// needs instead of a setting that does nothing.
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LintConfig {
    /// Schema version; must equal [`SUPPORTED_VERSION`].
    pub version: u32,
    /// Default profile when the caller does not name one.
    #[serde(default)]
    pub profile: Profile,
    /// Per-rule severity overrides. Rules absent here keep their default.
    #[serde(default)]
    pub rules: BTreeMap<RuleId, Severity>,
}

impl Default for LintConfig {
    fn default() -> Self {
        Self {
            version: SUPPORTED_VERSION,
            profile: Profile::default(),
            rules: RuleId::ALL
                .iter()
                .map(|rule| (*rule, rule.default_severity()))
                .collect(),
        }
    }
}

impl LintConfig {
    /// The configured severity for a rule, falling back to its default.
    pub fn severity(&self, rule: RuleId) -> Severity {
        self.rules
            .get(&rule)
            .copied()
            .unwrap_or_else(|| rule.default_severity())
    }

    /// A commented example, used by the docs and by the "no config yet" path so
    /// the tool can tell a user exactly what to write.
    pub fn example_toml() -> String {
        let mut out = String::from(
            "# rgaa-linter configuration. Place at $XDG_CONFIG_HOME/rgaa/lint-rules.toml\n\
             # (or ~/.config/rgaa/lint-rules.toml), or point RGAA_LINT_CONFIG at it.\n\
             version = 1\n\
             profile = \"rgaa-4.1\"  # rgaa-4.1 | wcag-2.1-aa | section-508\n\n\
             [rules]\n",
        );
        for rule in RuleId::ALL {
            out.push_str(&format!(
                "{} = \"{}\"  # error | warning | off\n",
                rule.as_str(),
                rule.default_severity().as_str()
            ));
        }
        out
    }

    /// Parses a config from TOML text, attributing errors to `path`.
    pub fn parse(path: &Path, text: &str) -> Result<Self, ConfigError> {
        let config: Self = toml::from_str(text).map_err(|error| ConfigError::Invalid {
            path: path.to_path_buf(),
            message: error.message().to_string(),
        })?;
        if config.version != SUPPORTED_VERSION {
            return Err(ConfigError::UnsupportedVersion {
                path: path.to_path_buf(),
                found: config.version,
            });
        }
        Ok(config)
    }

    /// Reads a config that the caller insists exists.
    fn load_required(path: &Path, origin: &str) -> Result<Self, ConfigError> {
        if !path.exists() {
            return Err(ConfigError::NotFound {
                path: path.to_path_buf(),
                origin: origin.to_string(),
            });
        }
        let text = std::fs::read_to_string(path).map_err(|source| ConfigError::Unreadable {
            path: path.to_path_buf(),
            source,
        })?;
        Self::parse(path, &text)
    }

    /// Resolves the configuration for a run.
    ///
    /// Precedence: explicit argument, then `RGAA_LINT_CONFIG`, then the user
    /// profile file. The first two must exist — the caller asked for them by
    /// name. The last is optional, but if it exists it must be valid: a user who
    /// has written a config is owed an error, not a silent reset to defaults.
    ///
    /// Returns the config and a label naming where it came from, which the
    /// report carries so the number of findings is always attributable.
    pub fn resolve(explicit: Option<&Path>) -> Result<(Self, String), ConfigError> {
        if let Some(path) = explicit {
            let config = Self::load_required(path, "the config_path argument")?;
            return Ok((config, path.display().to_string()));
        }
        if let Some(from_env) = std::env::var_os(CONFIG_ENV_VAR) {
            let path = PathBuf::from(from_env);
            let config = Self::load_required(&path, CONFIG_ENV_VAR)?;
            return Ok((config, path.display().to_string()));
        }
        match user_profile_config_path() {
            Some(path) if path.exists() => {
                let text =
                    std::fs::read_to_string(&path).map_err(|source| ConfigError::Unreadable {
                        path: path.clone(),
                        source,
                    })?;
                let config = Self::parse(&path, &text)?;
                let label = path.display().to_string();
                Ok((config, label))
            }
            _ => Ok((Self::default(), "built-in defaults".to_string())),
        }
    }
}

/// `$XDG_CONFIG_HOME/rgaa/lint-rules.toml`, or `$HOME/.config/rgaa/...`.
///
/// `None` when neither variable is set, which happens in sandboxes and CI; the
/// caller then runs on defaults rather than failing, because no user profile
/// exists to disagree with.
pub fn user_profile_config_path() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))?;
    Some(base.join("rgaa").join(CONFIG_FILE_NAME))
}
