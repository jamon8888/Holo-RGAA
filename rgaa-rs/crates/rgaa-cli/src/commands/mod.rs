pub mod analyze;
pub mod igt;
pub mod policy;
pub mod report;
pub mod verify;

use clap::Subcommand;

/// Common CLI arguments shared across all audit commands.
#[derive(Debug, Clone, clap::Args)]
pub struct CommonArgs {
    /// Path to the configuration file.
    #[clap(long)]
    pub config: Option<std::path::PathBuf>,
    /// Path to write output to (defaults to stdout).
    #[clap(long)]
    pub output: Option<std::path::PathBuf>,
    /// Output format (json, markdown, sarif, junit).
    #[clap(long)]
    pub format: Option<String>,
    /// Audit ID for operations that require it.
    #[clap(long)]
    pub audit_id: Option<String>,
}

/// Available audit commands.
#[derive(Debug, Subcommand)]
pub enum AuditCommand {
    /// Run an RGAA audit against a URL.
    Analyze(analyze::AnalyzeArgs),
    /// Run a guided accessibility test.
    Igt(igt::IgtArgs),
    /// Verify remediation proposals.
    Verify(verify::VerifyArgs),
    /// Render an audit bundle as a report.
    Report(report::ReportArgs),
    /// Check compliance against the configured policy.
    Policy(policy::PolicyArgs),
}

/// Dispatches a command to the appropriate handler.
///
/// # Arguments
///
/// * `command` - The audit command to execute.
///
/// # Returns
///
/// Exit code (0 for success, non-zero for failure).
///
/// # Errors
///
/// Returns `CliError` if the command fails.
pub async fn dispatch(command: AuditCommand) -> Result<i32, crate::CliError> {
    match command {
        AuditCommand::Analyze(args) => analyze::run(args).await,
        AuditCommand::Igt(args) => igt::run(args).await,
        AuditCommand::Verify(args) => verify::run(args),
        AuditCommand::Report(args) => report::run(args),
        AuditCommand::Policy(args) => policy::run(args),
    }
}

pub(crate) fn parse_format(value: Option<String>) -> Result<crate::ReportFormat, crate::CliError> {
    match value {
        Some(format) => format
            .parse()
            .map_err(|error: String| crate::CliError::invalid_input(error)),
        None => Ok(crate::ReportFormat::default()),
    }
}

pub(crate) fn write_output(
    output: &Option<std::path::PathBuf>,
    content: &str,
) -> Result<(), crate::CliError> {
    match output {
        Some(path) => std::fs::write(path, content).map_err(|error| {
            crate::CliError::execution(format!("failed to write output: {error}"))
        }),
        None => {
            println!("{content}");
            Ok(())
        }
    }
}
