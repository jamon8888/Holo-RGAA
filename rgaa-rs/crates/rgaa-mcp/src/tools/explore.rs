//! Dev-only `rgaa_explore` MCP tool.
//!
//! Returns exploration hints, never findings, verdicts or evidence. Kept
//! behind the `dev-explore` feature and off by default so it cannot become
//! part of the audited MCP surface by accident.

use rgaa_explore::{ExploreOptions, ObservationKind};
use schemars::JsonSchema;
use serde::Deserialize;
use std::path::PathBuf;
use std::str::FromStr;

/// Input for the exploratory study tool.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct ExploreRequest {
    /// Public http(s) URL to study.
    pub url: String,
    /// Path to the pinned Obscura binary (defaults to `RGAA_OBSCURA_BIN`).
    pub executable_path: Option<String>,
    /// Observations to capture: snapshot, screenshot, console, errors.
    pub include: Option<Vec<String>>,
    /// Per-command timeout in milliseconds (capped at 60000).
    pub timeout_ms: Option<u64>,
    /// Maximum characters kept per textual observation.
    pub max_output: Option<usize>,
    /// Leave the browser session open and report its run directory.
    pub keep_open: Option<bool>,
    /// Allow localhost/private-network URLs (off by default).
    pub allow_private_network: Option<bool>,
    /// Directory for persistent screenshots.
    pub artifact_dir: Option<String>,
    /// Optional `[width, height]` viewport.
    pub viewport: Option<Vec<u32>>,
    /// Isolated agent-browser namespace.
    pub namespace: Option<String>,
    /// Isolated agent-browser session name.
    pub session: Option<String>,
}

impl Default for ExploreRequest {
    fn default() -> Self {
        Self {
            url: String::new(),
            executable_path: None,
            include: None,
            timeout_ms: None,
            max_output: None,
            keep_open: None,
            allow_private_network: None,
            artifact_dir: None,
            viewport: None,
            namespace: None,
            session: None,
        }
    }
}

impl ExploreRequest {
    /// Convert the MCP request into runner options.
    ///
    /// # Errors
    ///
    /// Returns a message when the URL, observation names or viewport are
    /// invalid, or when no Obscura executable path can be resolved.
    pub fn into_options(self) -> Result<ExploreOptions, String> {
        let executable_path = self
            .executable_path
            .or_else(|| std::env::var("RGAA_OBSCURA_BIN").ok())
            .filter(|path| !path.is_empty())
            .map(PathBuf::from)
            .ok_or_else(|| {
                "pass executable_path or set RGAA_OBSCURA_BIN to the pinned obscura binary"
                    .to_owned()
            })?;

        let mut options = ExploreOptions {
            url: self.url,
            executable_path,
            keep_open: self.keep_open.unwrap_or(false),
            allow_private_network: self.allow_private_network.unwrap_or(false),
            artifact_dir: self.artifact_dir.map(PathBuf::from),
            namespace: self.namespace,
            session: self.session,
            ..ExploreOptions::default()
        };
        if let Some(timeout_ms) = self.timeout_ms {
            options.timeout_ms = timeout_ms;
        }
        if let Some(max_output) = self.max_output {
            options.max_output = max_output;
        }
        if let Some(requested) = self.include {
            if requested.is_empty() {
                return Err("include must not be empty".to_owned());
            }
            options.include = requested
                .into_iter()
                .map(|name| ObservationKind::from_str(&name))
                .collect::<Result<Vec<_>, _>>()?;
        }
        if let Some(viewport) = self.viewport {
            if viewport.len() != 2 {
                return Err("viewport must be [width, height]".to_owned());
            }
            options.viewport = Some((viewport[0], viewport[1]));
        }
        Ok(options)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_keep_the_session_closed_and_private_network_off() {
        let options = ExploreRequest {
            url: "https://example.com".to_owned(),
            executable_path: Some("/tmp/obscura".to_owned()),
            ..ExploreRequest::default()
        }
        .into_options()
        .expect("options");
        assert!(!options.keep_open);
        assert!(!options.allow_private_network);
        assert!(options.viewport.is_none());
    }

    #[test]
    fn rejects_bad_viewport_and_unknown_observation() {
        let base = || ExploreRequest {
            url: "https://example.com".to_owned(),
            executable_path: Some("/tmp/obscura".to_owned()),
            ..ExploreRequest::default()
        };
        let mut bad_viewport = base();
        bad_viewport.viewport = Some(vec![1920]);
        assert!(bad_viewport.into_options().is_err());

        let mut bad_include = base();
        bad_include.include = Some(vec!["verdict".to_owned()]);
        assert!(bad_include.into_options().is_err());
    }
}
