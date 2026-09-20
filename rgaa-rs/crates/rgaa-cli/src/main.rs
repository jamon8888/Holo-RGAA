use clap::{Parser, Subcommand};
use rgaa_cli::commands::{schedule, AuditCommand};
use rgaa_cli::CliError;

#[derive(Debug, Parser)]
#[command(name = "rgaa", version, about = "RGAA accessibility audit CLI")]
struct Cli {
    #[command(subcommand)]
    command: TopCommand,
}

#[derive(Debug, Subcommand)]
enum TopCommand {
    #[command(about = "Audit commands")]
    Audit(AuditArgs),
    #[command(about = "Run the recurring RGAA + SEO/GEO/AEO jobs from .rgaa/config.yaml")]
    Schedule(schedule::ScheduleArgs),
}

#[derive(Debug, clap::Args)]
struct AuditArgs {
    #[command(subcommand)]
    command: AuditCommand,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let result = match cli.command {
        TopCommand::Audit(args) => rgaa_cli::commands::dispatch(args.command).await,
        TopCommand::Schedule(args) => schedule::run(args).await,
    };
    match result {
        Ok(code) => std::process::exit(code),
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(exit_code(&error));
        }
    }
}

fn exit_code(error: &CliError) -> i32 {
    error.exit_code()
}
