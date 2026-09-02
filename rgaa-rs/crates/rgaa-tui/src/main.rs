use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "rgaa", version, about = "RGAA accessibility audit CLI + TUI")]
struct Cli {
    #[command(subcommand)]
    command: Option<TopCommand>,
}

#[derive(Debug, Subcommand)]
enum TopCommand {
    Tui,
    Audit {
        #[arg(long)]
        url: Option<String>,
    },
    Config,
    Install,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    match cli.command {
        None | Some(TopCommand::Tui) => rgaa_tui::tui::run().await,
        Some(TopCommand::Audit { url }) => rgaa_tui::commands::audit(url).await,
        Some(TopCommand::Config) => rgaa_tui::commands::config().await,
        Some(TopCommand::Install) => rgaa_tui::commands::install().await,
    }
}
