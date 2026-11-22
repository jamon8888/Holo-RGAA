//! Export commands: audit artefacts in interoperable formats.

use std::path::PathBuf;

use rgaa_core::AuditResult;

use crate::commands::{write_output, CommonArgs};
use crate::CliError;

/// Arguments for the `export` command group.
#[derive(Debug, clap::Args)]
pub struct ExportArgs {
    #[command(subcommand)]
    pub command: ExportCommand,
}

/// Available exports.
#[derive(Debug, clap::Subcommand)]
pub enum ExportCommand {
    /// Export the criteria × pages grid in the Ara CSV format.
    Grille(GrilleArgs),
}

impl ExportCommand {
    /// Returns the [`CommonArgs`] flattened into the selected export.
    pub fn common(&self) -> &CommonArgs {
        match self {
            Self::Grille(args) => &args.common,
        }
    }
}

/// Arguments for `rgaa export grille <audit> --format csv`.
#[derive(Debug, clap::Args)]
pub struct GrilleArgs {
    /// Shared CLI options, including `--format` and `--output`.
    #[clap(flatten)]
    pub common: CommonArgs,
    /// Path to the saved audit JSON (as written by `rgaa audit analyze --format json`).
    pub audit: PathBuf,
}

/// Dispatches an export subcommand.
pub async fn dispatch(command: ExportCommand) -> Result<i32, CliError> {
    match command {
        ExportCommand::Grille(args) => run(args),
    }
}

/// Loads a saved audit and writes its grid as Ara-format CSV.
///
/// # Errors
///
/// Returns [`CliError::InvalidInput`] for an unsupported format or malformed
/// audit JSON, and [`CliError::Execution`] when the audit file cannot be read
/// or the output cannot be written.
pub fn run(args: GrilleArgs) -> Result<i32, CliError> {
    check_format(args.common.format.as_deref())?;

    let raw = std::fs::read_to_string(&args.audit).map_err(|error| {
        CliError::execution(format!(
            "failed to read audit '{}': {error}",
            args.audit.display()
        ))
    })?;
    let audit: AuditResult = serde_json::from_str(&raw)
        .map_err(|error| CliError::invalid_input(format!("invalid audit JSON: {error}")))?;

    let csv = rgaa_report::render_grille_csv(&audit);
    write_output(&args.common.output, &csv)?;
    Ok(0)
}

fn check_format(value: Option<&str>) -> Result<(), CliError> {
    match value {
        None => Ok(()),
        Some(format) if format.eq_ignore_ascii_case("csv") => Ok(()),
        Some(other) => Err(CliError::invalid_input(format!(
            "unsupported grid format '{other}'; only csv is available"
        ))),
    }
}
