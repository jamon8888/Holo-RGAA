//! Types for dev-only exploration hints.
//!
//! Nothing in this module is an evidence or verdict type. The "not audit
//! evidence" fields are constants injected by [`super::explore`], not values a
//! caller can supply.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::str::FromStr;

/// Optional observations an exploratory run may capture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObservationKind {
    /// Accessibility tree snapshot with refs.
    Snapshot,
    /// Screenshot preview, never an evidence artifact.
    Screenshot,
    /// Browser console output.
    Console,
    /// Page errors.
    Errors,
}

impl FromStr for ObservationKind {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "snapshot" => Ok(Self::Snapshot),
            "screenshot" => Ok(Self::Screenshot),
            "console" => Ok(Self::Console),
            "errors" => Ok(Self::Errors),
            other => Err(format!(
                "unknown observation '{other}'; expected snapshot, screenshot, console or errors"
            )),
        }
    }
}

/// Inputs for an exploratory run.
#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ExploreOptions {
    /// Public http(s) URL to open.
    pub url: String,
    /// Path to the pinned `obscura` executable.
    pub executable_path: PathBuf,
    /// Which observations to capture.
    pub include: Vec<ObservationKind>,
    /// Per-command timeout, capped at 60_000 ms by the runner.
    pub timeout_ms: u64,
    /// Maximum characters retained per textual observation.
    pub max_output: usize,
    /// Keep the browser daemon and tab alive when the run finishes.
    pub keep_open: bool,
    /// Explicit opt-in for localhost/private-network URLs.
    pub allow_private_network: bool,
    /// Where to persist screenshots. Defaults to the temporary run directory,
    /// which is removed when `keep_open` is false.
    pub artifact_dir: Option<std::path::PathBuf>,
    /// Viewport to apply after opening, for comparable screenshots.
    pub viewport: Option<(u32, u32)>,
    /// Isolated agent-browser namespace. Generated when absent.
    pub namespace: Option<String>,
    /// Isolated agent-browser session name. Generated when absent.
    pub session: Option<String>,
}

impl Default for ExploreOptions {
    fn default() -> Self {
        Self {
            url: String::new(),
            executable_path: PathBuf::new(),
            include: vec![
                ObservationKind::Snapshot,
                ObservationKind::Screenshot,
                ObservationKind::Console,
                ObservationKind::Errors,
            ],
            timeout_ms: 25_000,
            max_output: 20_000,
            keep_open: false,
            allow_private_network: false,
            artifact_dir: None,
            viewport: None,
            namespace: None,
            session: None,
        }
    }
}

/// Text retained with an explicit truncation flag (never silently cut).
#[derive(Debug, Clone, Serialize)]
pub struct RawText {
    /// Retained characters.
    pub text: String,
    /// True when the source exceeded `max_output`.
    pub truncated: bool,
}

/// Accessibility snapshot with the refs agent-browser issued.
#[derive(Debug, Clone, Serialize)]
pub struct SnapshotObservation {
    /// Readable accessibility tree.
    pub text: String,
    /// Ref-to-target map returned by agent-browser.
    pub refs: serde_json::Value,
    /// True when `text` was capped.
    pub truncated: bool,
}

/// Preview screenshot. Not a content-addressed evidence artifact.
#[derive(Debug, Clone, Serialize)]
pub struct ScreenshotObservation {
    /// Absolute path inside the run directory.
    pub path: String,
    /// Always true: this image is a preview only.
    pub preview_only: bool,
}

/// Observations collected during the study.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Observations {
    /// Accessibility snapshot, if requested and captured.
    pub snapshot: Option<SnapshotObservation>,
    /// Screenshot preview, if requested and captured.
    pub screenshot: Option<ScreenshotObservation>,
    /// Console output, if requested and captured.
    pub console: Option<RawText>,
    /// Page errors, if requested and captured.
    pub errors: Option<RawText>,
}

/// A possible issue to investigate. Advisory only.
#[derive(Debug, Clone, Serialize)]
pub struct Candidate {
    /// Stable-in-run identifier.
    pub id: String,
    /// What was noticed.
    pub kind: String,
    /// RGAA criterion to look at, never a settled classification.
    pub criterion_hint: Option<String>,
    /// Human-readable explanation.
    pub note: String,
    /// Always true: a candidate cannot resolve a criterion.
    pub review_required: bool,
}

/// Engine provenance for the run.
#[derive(Debug, Clone, Serialize)]
pub struct EngineInfo {
    /// Browser engine name.
    pub name: &'static str,
    /// Version required by the repository pin.
    pub requested_version: &'static str,
    /// Version actually reported by the binary.
    pub observed_version: String,
    /// Pinned agent-browser CLI version.
    pub cli_version: &'static str,
    /// Executable used for this run.
    pub executable_path: String,
}

/// Session provenance for the run.
#[derive(Debug, Clone, Serialize)]
pub struct SessionInfo {
    /// Isolated session name.
    pub name: String,
    /// Isolated namespace.
    pub namespace: String,
    /// Socket directory, inside the run directory.
    pub socket_dir: String,
    /// Whether the daemon and tab were left alive.
    pub left_open: bool,
    /// Run directory holding all downloaded and generated state.
    pub run_dir: String,
}

/// Target provenance for the run.
#[derive(Debug, Clone, Serialize)]
pub struct TargetInfo {
    /// Final URL, including a trailing slash if the server added one.
    pub url: String,
    /// Scheme/host/port, without path or query.
    pub origin: String,
    /// Whether private-network access was explicitly enabled.
    pub private_network: bool,
}

/// The complete result of an exploratory run.
///
/// `evidence`, `not_audit_evidence` and `audit_verdict` are written by
/// [`ExplorationResult::new`] only; there is no public constructor that can set
/// them, and no field that can carry a criterion status.
#[derive(Debug, Clone, Serialize)]
pub struct ExplorationResult {
    schema_version: &'static str,
    kind: &'static str,
    evidence: bool,
    not_audit_evidence: bool,
    audit_verdict: Option<()>,
    engine: EngineInfo,
    session: SessionInfo,
    target: TargetInfo,
    observations: Observations,
    candidates: Vec<Candidate>,
    limitations: Vec<String>,
    next_steps: Vec<String>,
}

impl ExplorationResult {
    pub(crate) fn new(
        engine: EngineInfo,
        session: SessionInfo,
        target: TargetInfo,
        observations: Observations,
    ) -> Self {
        Self {
            schema_version: "0.1",
            kind: "exploration_hints",
            evidence: false,
            not_audit_evidence: true,
            audit_verdict: None,
            engine,
            session,
            target,
            observations,
            candidates: Vec::new(),
            limitations: vec![
                "agent-browser Obscura adapter is experimental".to_owned(),
                "AX naming, hidden-element, iframe and screenshot fidelity gaps are documented"
                    .to_owned(),
                "No Chrome parity claim".to_owned(),
                "Not ingested by the rgaa-core status model".to_owned(),
            ],
            next_steps: vec![
                "Reproduce each candidate through ObscuraBridge with a typed GuidedAction"
                    .to_owned(),
                "Only the resulting AnalyzePageResult may become a finding".to_owned(),
            ],
        }
    }

    /// Observations captured during the run.
    #[must_use]
    pub fn observations(&self) -> &Observations {
        &self.observations
    }

    /// Serialize to indented JSON using the hint contract.
    ///
    /// # Errors
    ///
    /// Returns a serde error if serialization fails (it should not for this type).
    pub fn to_json_pretty(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Short human-readable summary for terminal use.
    #[must_use]
    pub fn summary(&self) -> String {
        let mut out = format!(
            "Exploration hints for {} (not audit evidence)\nengine: {} {}\nsession: {} (left open: {})",
            self.target.url,
            self.engine.name,
            self.engine.observed_version,
            self.session.name,
            self.session.left_open
        );
        if let Some(snapshot) = &self.observations.snapshot {
            out.push_str("\n--- snapshot ---\n");
            out.push_str(&snapshot.text);
            if snapshot.truncated {
                out.push_str("\n[snapshot truncated]");
            }
        }
        out.push_str("\n--- next steps ---\n");
        for step in &self.next_steps {
            out.push_str("- ");
            out.push_str(step);
            out.push('\n');
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> ExplorationResult {
        ExplorationResult::new(
            EngineInfo {
                name: "obscura",
                requested_version: "obscura 0.2.4",
                observed_version: "obscura 0.2.4".to_owned(),
                cli_version: "0.45.0",
                executable_path: "/tmp/obscura".to_owned(),
            },
            SessionInfo {
                name: "s".to_owned(),
                namespace: "n".to_owned(),
                socket_dir: "/tmp/n/run".to_owned(),
                left_open: false,
                run_dir: "/tmp".to_owned(),
            },
            TargetInfo {
                url: "https://example.com/".to_owned(),
                origin: "https://example.com".to_owned(),
                private_network: false,
            },
            Observations::default(),
        )
    }

    #[test]
    fn serialization_always_marks_output_as_non_evidence() {
        let value = serde_json::to_value(sample()).expect("serialize hints");
        assert_eq!(value["kind"], "exploration_hints");
        assert_eq!(value["evidence"], false);
        assert_eq!(value["not_audit_evidence"], true);
        assert!(value["audit_verdict"].is_null());
        for forbidden in ["status", "verdict", "criterion_id", "finding"] {
            assert!(
                value.get(forbidden).is_none(),
                "hint output must not carry {forbidden}"
            );
        }
    }

    #[test]
    fn observation_kind_parses_only_known_values() {
        assert_eq!(
            "snapshot".parse::<ObservationKind>().unwrap(),
            ObservationKind::Snapshot
        );
        assert!("verdict".parse::<ObservationKind>().is_err());
    }

    #[test]
    fn options_reject_unknown_fields() {
        let parsed =
            serde_json::from_str::<ExploreOptions>(r#"{"url":"https://x","verdict":"pass"}"#);
        assert!(parsed.is_err());
    }
}
