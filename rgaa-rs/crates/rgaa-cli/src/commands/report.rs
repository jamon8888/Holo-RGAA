use std::path::Path;

use rgaa_core::AuditBundle;

use crate::commands::{parse_format, write_output, CommonArgs};
use crate::CliError;

/// CLI arguments for the report command.
///
/// Renders an audit bundle as a formatted report (JSON, Markdown, SARIF, or JUnit).
#[derive(Debug, clap::Args)]
pub struct ReportArgs {
    /// Common CLI arguments shared across all commands.
    #[clap(flatten)]
    pub common: CommonArgs,
    /// Path to the audit bundle JSON file.
    #[clap(long, value_name = "BUNDLE")]
    pub input: std::path::PathBuf,
}

/// Executes the report command.
///
/// Loads an audit bundle, validates it, and renders it in the specified format.
///
/// # Errors
///
/// Returns `CliError` if the bundle is invalid or rendering fails.
pub fn run(args: ReportArgs) -> Result<i32, CliError> {
    let raw = std::fs::read_to_string(&args.input)
        .map_err(|error| CliError::execution(format!("failed to read bundle: {error}")))?;
    let bundle: AuditBundle = serde_json::from_str(&raw)
        .map_err(|error| CliError::invalid_input(format!("invalid audit bundle: {error}")))?;
    bundle
        .validate()
        .map_err(|error| CliError::invalid_input(error.to_string()))?;
    let format = parse_format(args.common.format)?;
    let rendered = crate::report::render(&bundle, format)?;
    write_output(&args.common.output, &rendered)?;
    Ok(0)
}

/// Loads an audit bundle from a JSON file.
///
/// # Arguments
///
/// * `path` - Path to the JSON file.
///
/// # Returns
///
/// The parsed `AuditBundle`.
///
/// # Errors
///
/// Returns `CliError` if the file cannot be read or parsed.
pub fn load_bundle(path: &Path) -> Result<AuditBundle, CliError> {
    let raw = std::fs::read_to_string(path)
        .map_err(|error| CliError::execution(format!("failed to read bundle: {error}")))?;
    serde_json::from_str(&raw)
        .map_err(|error| CliError::invalid_input(format!("invalid audit bundle: {error}")))
}
