//! Commande `rgaa metrics` : indicateurs internes au pilote, jamais dans le rapport client.

use std::fs;
use std::path::{Path, PathBuf};

use rgaa_core::AuditBundle;

use crate::CliError;

/// Arguments de `rgaa metrics reouverture`.
#[derive(Debug, clap::Args)]
pub struct ReouvertureArgs {
    /// Bundle JSON d'un seul audit.
    #[clap(value_name = "AUDIT")]
    pub audit: Option<PathBuf>,
    /// Agrège tous les `*.json` d'un dossier.
    #[clap(long)]
    pub all: bool,
    /// Dossier parcouru avec `--all` (par défaut le dossier courant).
    #[clap(long, value_name = "DIR")]
    pub input: Option<PathBuf>,
    /// Descend dans les sous-dossiers avec `--all`.
    #[clap(long)]
    pub recursive: bool,
    /// Fichier de l'export JSON (par défaut la sortie standard).
    #[clap(long, value_name = "PATH")]
    pub output: Option<PathBuf>,
}

impl ReouvertureArgs {
    fn validate(&self) -> Result<(), CliError> {
        if self.audit.is_some() && self.all {
            return Err(CliError::invalid_input(
                "fournissez un bundle ou --all, pas les deux",
            ));
        }
        if self.audit.is_none() && !self.all {
            return Err(CliError::invalid_input("fournissez un bundle ou --all"));
        }
        if self.input.is_some() && !self.all {
            return Err(CliError::invalid_input("--input ne sert qu'avec --all"));
        }
        if self.recursive && !self.all {
            return Err(CliError::invalid_input("--recursive ne sert qu'avec --all"));
        }
        Ok(())
    }
}

fn read_bundle(path: &Path) -> Result<AuditBundle, CliError> {
    let raw = fs::read_to_string(path).map_err(|error| {
        CliError::execution(format!(
            "failed to read bundle '{}': {error}",
            path.display()
        ))
    })?;
    let bundle: AuditBundle = serde_json::from_str(&raw).map_err(|error| {
        CliError::invalid_input(format!(
            "invalid audit bundle '{}': {error}",
            path.display()
        ))
    })?;
    bundle
        .validate()
        .map_err(|error| CliError::invalid_input(error.to_string()))?;
    Ok(bundle)
}

/// Liste les `*.json` d'un dossier, triés pour un export reproductible.
fn json_files(dir: &Path, recursive: bool) -> Result<Vec<PathBuf>, CliError> {
    let mut files = Vec::new();
    let entries = fs::read_dir(dir).map_err(|error| {
        CliError::execution(format!(
            "failed to read directory '{}': {error}",
            dir.display()
        ))
    })?;
    for entry in entries {
        let entry = entry.map_err(|error| {
            CliError::execution(format!(
                "failed to read an entry of '{}': {error}",
                dir.display()
            ))
        })?;
        let path = entry.path();
        if path.is_dir() {
            if recursive {
                files.extend(json_files(&path, true)?);
            }
        } else if path.is_file() && path.extension().is_some_and(|ext| ext == "json") {
            files.push(path);
        }
    }
    files.sort();
    Ok(files)
}

/// Calcule et exporte le taux de réouverture.
///
/// # Errors
/// Returns [`CliError::InvalidInput`] for incompatible arguments and
/// [`CliError::Execution`] when a bundle cannot be read or the export written.
pub fn run(args: ReouvertureArgs) -> Result<i32, CliError> {
    args.validate()?;

    let bundles = if args.all {
        let dir = args.input.clone().unwrap_or_else(|| PathBuf::from("."));
        let mut loaded = Vec::new();
        for path in json_files(&dir, args.recursive)? {
            match read_bundle(&path) {
                Ok(bundle) => loaded.push(bundle),
                Err(error) => eprintln!(
                    "avis : '{}' ignoré ({}), ce n'est pas un bundle d'audit",
                    path.display(),
                    error
                ),
            }
        }
        loaded
    } else {
        let path = args
            .audit
            .as_deref()
            .ok_or_else(|| CliError::invalid_input("fournissez un bundle ou --all"))?;
        vec![read_bundle(path)?]
    };

    let references: Vec<&AuditBundle> = bundles.iter().collect();
    let json = rgaa_report::reouverture::export_reouverture_json(&references)
        .map_err(|error| CliError::execution(error.to_string()))?;
    match &args.output {
        Some(path) => fs::write(path, json).map_err(|error| {
            CliError::execution(format!(
                "failed to write export '{}': {error}",
                path.display()
            ))
        })?,
        None => println!("{json}"),
    }
    Ok(0)
}
