use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(name = "rgaa", version, about = "RGAA accessibility audit CLI + TUI")]
struct Cli {
    #[command(subcommand)]
    command: Option<TopCommand>,
    #[arg(long, global = true)]
    pub debug: bool,
}

#[derive(Debug, Subcommand)]
enum TopCommand {
    /// Launch the interactive TUI
    Tui,
    /// Audit a URL (headless + agentic evaluation)
    Audit {
        /// Target URL to audit
        url: Option<String>,
        /// Export results to file (format detected from extension: .json, .html, .pdf)
        #[arg(short, long)]
        export: Option<PathBuf>,
    },
    /// View audit history
    History {
        /// Number of results to show (default: 20)
        #[arg(default_value = "20")]
        limit: usize,
    },
    /// Manage configuration
    Config {
        #[command(subcommand)]
        sub: Option<ConfigCommand>,
    },
    /// Run the install wizard
    Install,
}

#[derive(Debug, Subcommand)]
enum ConfigCommand {
    Show,
    Set {
        #[command(subcommand)]
        what: ConfigSetTarget,
    },
}

#[derive(Debug, Subcommand)]
enum ConfigSetTarget {
    ApiKey {
        key: String,
    },
    BaseUrl {
        url: String,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    if cli.debug {
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::DEBUG)
            .init();
    }

    match cli.command {
        None | Some(TopCommand::Tui) => rgaa_tui::tui::run().await,
        Some(TopCommand::Audit { url, export }) => {
            if let Err(e) = rgaa_tui::commands::audit(url, export).await {
                eprintln!("Error: {e}");
                std::process::exit(1);
            }
        }
        Some(TopCommand::History { limit }) => {
            if let Err(e) = rgaa_tui::commands::history(limit).await {
                eprintln!("Error: {e}");
                std::process::exit(1);
            }
        }
        Some(TopCommand::Config { sub }) => {
            let result = match sub {
                None | Some(ConfigCommand::Show) => rgaa_tui::commands::config_show().await,
                Some(ConfigCommand::Set { what }) => match what {
                    ConfigSetTarget::ApiKey { key } => {
                        rgaa_tui::commands::config_set_api_key(key).await
                    }
                    ConfigSetTarget::BaseUrl { url } => {
                        rgaa_tui::commands::config_set_base_url(url).await
                    }
                },
            };
            if let Err(e) = result {
                eprintln!("Error: {e}");
                std::process::exit(1);
            }
        }
        Some(TopCommand::Install) => {
            if let Err(e) = rgaa_tui::commands::install().await {
                eprintln!("Error: {e}");
                std::process::exit(1);
            }
        }
    }
}
