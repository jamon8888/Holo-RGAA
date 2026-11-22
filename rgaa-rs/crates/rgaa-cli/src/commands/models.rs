//! `rgaa models list|pull <plugin>|verify` (P-00 §6).
//!
//! This is the only place that downloads a model. An audit never does.

use crate::CliError;
use clap::Subcommand;

#[derive(Debug, clap::Args)]
pub struct ModelsArgs {
    #[command(subcommand)]
    pub command: ModelsCommand,
}

#[derive(Debug, Subcommand)]
pub enum ModelsCommand {
    /// List required models, their licence and whether they are cached.
    List,
    /// Download and verify the models required by one plugin.
    Pull { plugin: String },
    /// Verify the digests of every cached model.
    Verify,
}

pub async fn run(args: ModelsArgs) -> Result<i32, CliError> {
    match args.command {
        ModelsCommand::List => list(),
        ModelsCommand::Pull { plugin } => pull(&plugin).await,
        ModelsCommand::Verify => verify(),
    }
}

/// Registry of the plugins linked into this CLI, built once per command.
fn registry() -> Result<rgaa_analysis::Registry, CliError> {
    rgaa_plugins::default_registry().map_err(|error| CliError::execution(error.to_string()))
}

fn list() -> Result<i32, CliError> {
    println!("cache: {}", rgaa_analysis::models::models_dir().display());
    for plugin in registry()?.iter() {
        for model in plugin.models() {
            println!(
                "{} {} {} {} {}",
                plugin.id(),
                model.nom,
                model.licence,
                model.taille_octets,
                if rgaa_analysis::models::is_present(model) {
                    "present"
                } else {
                    "absent"
                }
            );
        }
    }
    Ok(0)
}

async fn pull(id: &str) -> Result<i32, CliError> {
    let registry = registry()?;
    let plugin = registry
        .get(id)
        .ok_or_else(|| CliError::invalid_input(format!("plugin d'analyse inconnu : {id}")))?;
    for model in plugin.models() {
        let response = reqwest::get(model.url)
            .await
            .map_err(|error| CliError::execution(format!("téléchargement {}: {error}", model.nom)))?
            .error_for_status()
            .map_err(|error| {
                CliError::execution(format!("téléchargement {}: {error}", model.nom))
            })?;
        let bytes = response
            .bytes()
            .await
            .map_err(|error| CliError::execution(format!("lecture {}: {error}", model.nom)))?;
        let path = rgaa_analysis::models::install(model, &bytes)
            .map_err(|error| CliError::execution(error.to_string()))?;
        println!("{} -> {}", model.nom, path.display());
    }
    Ok(0)
}

fn verify() -> Result<i32, CliError> {
    let mut failed = false;
    for plugin in registry()?.iter() {
        for model in plugin.models() {
            match rgaa_analysis::models::ensure_present(plugin.id(), model) {
                Ok(()) => println!("{} {}: ok", plugin.id(), model.nom),
                Err(error) => {
                    eprintln!("{error}");
                    failed = true;
                }
            }
        }
    }
    Ok(i32::from(failed))
}
