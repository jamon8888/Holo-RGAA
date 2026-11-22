use clap::{Parser, Subcommand};
use rgaa_cli::commands::metrics::ReouvertureArgs;
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
    /// Indicateurs internes au pilote, jamais inclus dans le rapport client.
    Metrics(MetricsArgs),
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
        TopCommand::Audit(args) => args,
        // Internal pilot metrics do not open an audit monitoring log.
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
    };

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
}
