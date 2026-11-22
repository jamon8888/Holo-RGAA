use clap::{Parser, Subcommand};
use rgaa_cli::commands::metrics::ReouvertureArgs;
use rgaa_cli::commands::{AuditCommand, ExportCommand};
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
    /// Indicateurs internes au pilote, jamais inclus dans le rapport client.
    Metrics(MetricsArgs),
    #[command(about = "Export commands")]
    Export(ExportArgs),
    /// Serve the RGAA tools over MCP (HTTP + SSE, or stdio).
    ///
    /// The flags live in `rgaa_mcp_http::McpServerArgs` rather than here so
    /// that `rgaa mcp-server` and the standalone `rgaa-mcp-http` binary
    /// cannot disagree about what `--cors-origin` means (issue #93).
    #[command(name = "mcp-server", about = "Run the MCP tool server")]
    McpServer(rgaa_mcp_http::McpServerArgs),
    /// Inspect analysis plugins and their local models.
    #[command(name = "plugins", about = "List analysis plugins")]
    Plugins(rgaa_cli::commands::plugins::PluginsArgs),
    /// Manage the local model cache used by analysis plugins.
    #[command(name = "models", about = "Manage analysis models")]
    Models(rgaa_cli::commands::models::ModelsArgs),
}

#[derive(Debug, clap::Args)]
struct AuditArgs {
    #[command(subcommand)]
    command: AuditCommand,
}

#[derive(Debug, clap::Args)]
struct MetricsArgs {
    #[command(subcommand)]
    command: MetricsCommand,
}

#[derive(Debug, Subcommand)]
enum MetricsCommand {
    /// Taux de non-conformités posées là où l'outil n'avait rien signalé (#438).
    Reouverture(ReouvertureArgs),
}

#[derive(Debug, clap::Args)]
struct ExportArgs {
    #[command(subcommand)]
    command: ExportCommand,
}

/// A dispatchable command, with the audit and export groups sharing setup.
enum Action {
    Audit(AuditCommand),
    Export(ExportCommand),
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
    let log_file = match &cli.command {
        TopCommand::Status => None,
        TopCommand::Audit(args) => args.command.common().log_file.clone(),
        TopCommand::Export(args) => args.command.common().log_file.clone(),
        // Internal pilot metrics do not open an audit monitoring log.
        TopCommand::Metrics(_) => None,
        TopCommand::McpServer(_) => None,
    };
    let action = match cli.command {
        TopCommand::Status => match rgaa_cli::commands::status::run() {
            Ok(code) => std::process::exit(code),
            Err(error) => {
                eprintln!("{error}");
                std::process::exit(error.exit_code());
            }
        },
        TopCommand::Audit(args) => Action::Audit(args.command),
        TopCommand::Export(args) => Action::Export(args.command),
        TopCommand::Metrics(args) => {
            let result = match args.command {
                MetricsCommand::Reouverture(reouverture) => {
                    rgaa_cli::commands::metrics::run(reouverture)
                }
            };
            match result {
                Ok(code) => std::process::exit(code),
                Err(error) => {
                    eprintln!("{error}");
                    std::process::exit(error.exit_code());
                }
            }
        }
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
        TopCommand::Plugins(args) => {
            exit_with(rgaa_cli::commands::plugins::run(args).await);
        }
        TopCommand::Models(args) => {
            exit_with(rgaa_cli::commands::models::run(args).await);
        }
    };

    // Ticket #440: the active offer is visible before any audit work starts.
    // Both the mode and the LLM route come from the existing configuration.
    rgaa_cli::commands::status::print_audit_header();

    let (log_path, monitoring_guard) = match rgaa_cli::monitoring::init(log_file.as_deref()) {
        Ok(initialized) => initialized,
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(exit_code(&error));
        }
    };
    eprintln!("Monitoring log: {}", log_path.display());

    let result = match action {
        Action::Audit(command) => rgaa_cli::commands::dispatch(command).await,
        Action::Export(command) => rgaa_cli::commands::dispatch_export(command).await,
    };
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

fn exit_with(result: Result<i32, CliError>) -> ! {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn metrics_reouverture_est_une_commande_de_premier_niveau() {
        let cli = Cli::try_parse_from([
            "rgaa",
            "metrics",
            "reouverture",
            "bundle.json",
            "--output",
            "export.json",
        ])
        .expect("metrics reouverture est reconnue");
        match cli.command {
            TopCommand::Metrics(MetricsArgs {
                command: MetricsCommand::Reouverture(args),
            }) => {
                assert_eq!(args.audit, Some(PathBuf::from("bundle.json")));
                assert_eq!(args.output, Some(PathBuf::from("export.json")));
                assert!(!args.all);
            }
            _ => panic!("attendu : rgaa metrics reouverture"),
        }
    }

    #[test]
    fn metrics_reouverture_all_accepte_un_dossier_et_la_recursion() {
        let cli = Cli::try_parse_from([
            "rgaa",
            "metrics",
            "reouverture",
            "--all",
            "--input",
            "bundles",
            "--recursive",
        ])
        .expect("--all, --input et --recursive sont reconnus");
        match cli.command {
            TopCommand::Metrics(MetricsArgs {
                command: MetricsCommand::Reouverture(args),
            }) => {
                assert!(args.all);
                assert!(args.recursive);
                assert_eq!(args.input, Some(PathBuf::from("bundles")));
            }
            _ => panic!("attendu : rgaa metrics reouverture"),
        }
    }

    #[test]
    fn export_grille_is_a_top_level_command() {
        let cli =
            Cli::try_parse_from(["rgaa", "export", "grille", "audit.json", "--format", "csv"])
                .expect("rgaa export grille <audit> --format csv must parse");
        match cli.command {
            TopCommand::Export(args) => match args.command {
                rgaa_cli::commands::ExportCommand::Grille(args) => {
                    assert_eq!(args.audit, PathBuf::from("audit.json"));
                    assert_eq!(args.common.format.as_deref(), Some("csv"));
                }
            },
            other => panic!("expected export grille, got {other:?}"),
        }
    }
}
