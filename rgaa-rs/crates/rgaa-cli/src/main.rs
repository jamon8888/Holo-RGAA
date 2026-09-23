use clap::{Parser, Subcommand};
use rgaa_cli::commands::AuditCommand;
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
}

#[derive(Debug, clap::Args)]
struct AuditArgs {
    #[command(subcommand)]
    command: AuditCommand,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let TopCommand::Audit(args) = cli.command;

    let (log_path, monitoring_guard) =
        rgaa_cli::monitoring::init(args.command.common().log_file.as_deref());
    eprintln!("Monitoring log: {}", log_path.display());

    let result = rgaa_cli::commands::dispatch(args.command).await;
    let code = match &result {
        Ok(code) => *code,
        Err(error) => {
            eprintln!("{error}");
            exit_code(error)
        }
    };

    // `std::process::exit` skips destructors, so the monitoring guard's
    // flush-on-drop (which pushes buffered JSON-lines to disk) must run
    // explicitly first — otherwise a fast-failing run can exit before the
    // async writer ever gets scheduled, silently losing every log line.
    drop(monitoring_guard);
    std::process::exit(code);
}

fn exit_code(error: &CliError) -> i32 {
    error.exit_code()
}
