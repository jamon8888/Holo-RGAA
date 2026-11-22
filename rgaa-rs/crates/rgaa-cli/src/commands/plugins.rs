//! `rgaa plugins list` — the compiled plugin catalog (P-00 §7).

use crate::CliError;
use clap::Subcommand;
use indexmap::IndexMap;

#[derive(Debug, clap::Args)]
pub struct PluginsArgs {
    #[command(subcommand)]
    pub command: PluginsCommand,
}

#[derive(Debug, Subcommand)]
pub enum PluginsCommand {
    /// List compiled analysis plugins, their tests and required models.
    List,
}

pub async fn run(args: PluginsArgs) -> Result<i32, CliError> {
    match args.command {
        PluginsCommand::List => list(),
    }
}

/// Prints one line per plugin, then flags every RGAA test served twice.
fn list() -> Result<i32, CliError> {
    let registry =
        rgaa_plugins::default_registry().map_err(|error| CliError::execution(error.to_string()))?;
    for plugin in registry.iter() {
        let info = plugin.info();
        println!("{} {}", info.id, info.version);
        println!("  tests: {}", info.tests.join(", "));
        if info.models.is_empty() {
            println!("  models: aucun");
        } else {
            for model in &info.models {
                println!(
                    "  model: {} ({}) — {}",
                    model.nom,
                    model.licence,
                    if model.present { "présent" } else { "absent" }
                );
            }
        }
        if let Some(command) = &info.commande {
            println!("  commande: {command}");
            if let Some(hash) = &info.sha256_executable {
                println!("  sha256: {hash}");
            }
        }
    }

    let duplicates = registry.duplicate_tests();
    if !duplicates.is_empty() {
        println!("tests servis par plusieurs plugins :");
        print_duplicates(&duplicates);
    }
    Ok(0)
}

fn print_duplicates(duplicates: &IndexMap<String, Vec<String>>) {
    for (test, providers) in duplicates {
        println!("  {test}: {}", providers.join(", "));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duplicate_reporting_keeps_the_registry_order() {
        let mut duplicates = IndexMap::new();
        duplicates.insert(
            "1.1.1".to_owned(),
            vec!["echo".to_owned(), "other".to_owned()],
        );
        assert_eq!(duplicates.keys().next(), Some(&"1.1.1".to_owned()));
    }
}
