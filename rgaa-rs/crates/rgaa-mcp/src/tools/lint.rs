//! The `lint_static` tool: source-level accessibility linting, no browser.
//!
//! `analyze` needs a running page and costs seconds. An agent editing a
//! component wants an answer before it moves to the next file, and most of what
//! it can fix at that point — a missing `alt`, an unlabelled input — is already
//! visible in the source. This tool is that fast path, delegating to
//! `rgaa-linter`.
//!
//! The DTOs here exist only to keep the wire contract stable: the linter's own
//! report type is re-exported as the response, but the request is ours, because
//! the tool has to decide what "give me sources" means over MCP and refuse the
//! ambiguous shapes loudly.

use rgaa_linter::{LintOptions, LintReport, Profile, Source};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::server::McpFailure;

/// One in-memory source file.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LintSourceInput {
    /// Path used to pick the dialect and to label findings. It does not have to
    /// exist on disk, but its extension must be one the linter supports
    /// (html, htm, jsx, tsx, js, ts, mjs, vue).
    pub path: String,
    /// The file's text.
    pub content: String,
}

/// Arguments for `lint_static`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct LintStaticRequest {
    /// Reference framework to report against. Defaults to the profile in the
    /// user's `lint-rules.toml`, or RGAA 4.1 when there is none.
    #[serde(default)]
    pub profile: Option<Profile>,
    /// Sources the caller already holds, e.g. an unsaved editor buffer.
    #[serde(default)]
    pub sources: Vec<LintSourceInput>,
    /// Paths to read from disk. Findings are then mapped onto these real paths,
    /// so a fix can be applied without asking where the code lives.
    #[serde(default)]
    pub source_files: Vec<String>,
    /// Override the `lint-rules.toml` location. The file must exist.
    #[serde(default)]
    pub config_path: Option<String>,
}

impl LintStaticRequest {
    /// Validates the request and runs the lint.
    ///
    /// `sources` and `source_files` are mutually exclusive on purpose. Accepting
    /// both would force a single report to be half source-mapped and half not,
    /// and `source_mapped` would then be a lie in one direction or the other —
    /// which is exactly the flag a caller uses to decide whether it may edit the
    /// file a finding points at.
    pub fn run(self) -> Result<LintReport, McpFailure> {
        if self.sources.is_empty() && self.source_files.is_empty() {
            return Err(McpFailure::invalid(
                "provide sources (inline content) or source_files (paths to read)",
            ));
        }
        if !self.sources.is_empty() && !self.source_files.is_empty() {
            return Err(McpFailure::invalid(
                "provide either sources or source_files, not both: a report cannot be \
                 half source-mapped",
            ));
        }
        let config_path = self.config_path.as_ref().map(PathBuf::from);
        let options = LintOptions::resolve(self.profile, config_path.as_deref())
            .map_err(|error| McpFailure::invalid(error.to_string()))?;

        if !self.source_files.is_empty() {
            let paths: Vec<PathBuf> = self.source_files.iter().map(PathBuf::from).collect();
            return rgaa_linter::lint_paths(&paths, &options)
                .map_err(|error| McpFailure::invalid(error.to_string()));
        }
        let sources: Vec<Source> = self
            .sources
            .into_iter()
            .map(|input| Source {
                path: input.path,
                content: input.content,
            })
            .collect();
        rgaa_linter::lint_sources(&sources, &options)
            .map_err(|error| McpFailure::invalid(error.to_string()))
    }
}

/// `lint_static`'s response: the linter's report, unaltered.
pub type LintStaticResponse = LintReport;
