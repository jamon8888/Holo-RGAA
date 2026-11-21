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
    /// Print the active offer (Souverain / Confort) and LLM route.
    #[command(about = "Show the active offer and LLM route")]
    Status,
    #[command(about = "Audit commands")]
    Audit(AuditArgs),
    /// Serve the RGAA tools over MCP (HTTP + SSE, or stdio).
    ///
    /// The flags live in `rgaa_mcp_http::McpServerArgs` rather than here so
    /// that `rgaa mcp-server` and the standalone `rgaa-mcp-http` binary
    /// cannot disagree about what `--cors-origin` means (issue #93).
    #[command(name = "mcp-server", about = "Run the MCP tool server")]
    McpServer(rgaa_mcp_http::McpServerArgs),
}

#[derive(Debug, clap::Args)]
struct AuditArgs {
    #[command(subcommand)]
    command: AuditCommand,
}

#[tokio::main]
async fn main() {
    // Loads `.env` from the working directory (or any parent) so a local
    // checkout is configured by that one file. Variables already present in
    // the real environment always win, so a container or systemd unit keeps
    // precedence over a stray `.env`. Deliberately in the binary only: a
    // library must never reach for the filesystem behind its caller.
    let _ = dotenvy::dotenv();

    let cli = Cli::parse();
    let args = match cli.command {
        TopCommand::Status => match rgaa_cli::commands::status::run() {
            Ok(code) => std::process::exit(code),
            Err(error) => {
                eprintln!("{error}");
                std::process::exit(error.exit_code());
            }
        },
        TopCommand::Audit(args) => args,
        // Served before the audit monitoring is initialised: the MCP server
        // logs to stderr for its supervisor, and opening the audit JSON-lines
        // file for a long-lived server would leave a log nothing ever writes.
        TopCommand::McpServer(args) => {
            tracing_subscriber::fmt()
                .with_writer(std::io::stderr)
                .init();
            if let Err(error) = rgaa_mcp_http::run(args).await {
                eprintln!("{error}");
                std::process::exit(1);
            }
            return;
        }
    };

    // Ticket #440: the active offer is visible before any audit work starts.
    // Both the mode and the LLM route come from the existing configuration.
    rgaa_cli::commands::status::print_audit_header();

    let (log_path, monitoring_guard) =
        match rgaa_cli::monitoring::init(args.command.common().log_file.as_deref()) {
            Ok(initialized) => initialized,
            Err(error) => {
                eprintln!("{error}");
                std::process::exit(exit_code(&error));
            }
        };
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
