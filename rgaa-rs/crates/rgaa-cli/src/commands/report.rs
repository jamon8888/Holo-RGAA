use std::path::Path;

use rgaa_core::AuditBundle;

use crate::commands::{parse_format, write_output, CommonArgs};
use crate::CliError;

/// CLI arguments for the report command.
///
/// Renders an audit bundle as a formatted report (JSON, Markdown, SARIF, JUnit, or HTML).
///
/// # Examples
///
/// ```bash
/// # Generate HTML report from a bundle file
/// rgaa audit report --input bundle.json --format html --output report.html
///
/// # Generate JSON report
/// rgaa audit report --input bundle.json --format json
///
/// # Use audit ID to load from storage (requires --config)
/// rgaa audit report --audit-id abc123 --format html
/// ```
#[derive(Debug, clap::Args)]
pub struct ReportArgs {
    /// Common CLI arguments shared across all commands.
    #[clap(flatten)]
    pub common: CommonArgs,
    /// Path to the audit bundle JSON file.
    #[clap(long, value_name = "BUNDLE", help = "Path to audit bundle JSON file")]
    pub input: Option<std::path::PathBuf>,
    /// Audit ID to load from storage (requires configured storage).
    #[clap(long, value_name = "AUDIT-ID", help = "Audit ID to load from storage")]
    pub audit_id: Option<String>,
}

/// Executes the report command.
///
/// Loads an audit bundle, validates it, and renders it in the specified format.
///
/// # Errors
///
/// Returns `CliError` if the bundle is invalid or rendering fails.
pub fn run(args: ReportArgs) -> Result<i32, CliError> {
    let bundle = load_bundle(&args)?;
    let format = parse_format(args.common.format)?;
    let rendered = crate::report::render(&bundle, format)?;
    write_output(&args.common.output, &rendered)?;
    Ok(0)
}

fn load_bundle(args: &ReportArgs) -> Result<AuditBundle, CliError> {
    match (&args.input, &args.audit_id) {
        (Some(path), None) => load_from_file(path),
        (None, Some(_id)) => Err(CliError::invalid_input(
            "loading from storage not yet implemented",
        )),
        (None, None) => Err(CliError::invalid_input(
            "provide either --input <bundle> or --audit-id <id>",
        )),
        (Some(_), Some(_)) => Err(CliError::invalid_input(
            "cannot use both --input and --audit-id",
        )),
    }
}

fn load_from_file(path: &Path) -> Result<AuditBundle, CliError> {
    let raw = std::fs::read_to_string(path)
        .map_err(|error| CliError::execution(format!("failed to read bundle: {error}")))?;
    let bundle: AuditBundle = serde_json::from_str(&raw)
        .map_err(|error| CliError::invalid_input(format!("invalid audit bundle: {error}")))?;
    bundle
        .validate()
        .map_err(|error| CliError::invalid_input(error.to_string()))?;
    Ok(bundle)
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
pub fn load_bundle_from_path(path: &Path) -> Result<AuditBundle, CliError> {
    load_from_file(path)
}
