pub mod analyze;
pub mod igt;
pub mod policy;
pub mod report;
pub mod verify;

use clap::Subcommand;

#[derive(Debug, Clone, clap::Args)]
pub struct CommonArgs {
    #[clap(long)]
    pub config: Option<std::path::PathBuf>,
    #[clap(long)]
    pub output: Option<std::path::PathBuf>,
    #[clap(long)]
    pub format: Option<String>,
    #[clap(long)]
    pub audit_id: Option<String>,
}

#[derive(Debug, Subcommand)]
pub enum AuditCommand {
    #[command(about = "Run an RGAA audit against a URL")]
    Analyze(analyze::AnalyzeArgs),
    #[command(about = "Run a guided accessibility test")]
    Igt(igt::IgtArgs),
    #[command(about = "Verify remediation proposals")]
    Verify(verify::VerifyArgs),
    #[command(about = "Render an audit bundle as a report")]
    Report(report::ReportArgs),
    #[command(about = "Check compliance against the configured policy")]
    Policy(policy::PolicyArgs),
}

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
