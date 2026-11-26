//! Dev-only `rgaa explore` command.
//!
//! Drives agent-browser with the repository-pinned Obscura binary and prints
//! **exploration hints**. Nothing here produces a verdict, a finding or an
//! evidence artifact; see `rgaa-explore` for the output contract.

use crate::CliError;
use clap::Args;
use rgaa_explore::{ExploreOptions, ObservationKind};
use std::path::PathBuf;

fn parse_viewport(value: &str) -> Result<(u32, u32), String> {
    let (width, height) = value
        .split_once('x')
        .ok_or("expected WIDTHxHEIGHT, e.g. 1920x1080")?;
    Ok((
        width.parse::<u32>().map_err(|error| error.to_string())?,
        height.parse::<u32>().map_err(|error| error.to_string())?,
    ))
}

/// Arguments for the exploratory study command.
#[derive(Debug, Args)]
pub struct ExploreArgs {
    /// Public http(s) URL to study.
    pub url: String,
    /// Path to the pinned Obscura binary (defaults to `RGAA_OBSCURA_BIN`).
    #[arg(long)]
    pub executable_path: Option<PathBuf>,
    /// Observations to capture: snapshot,screenshot,console,errors.
    #[arg(long, value_delimiter = ',')]
    pub include: Option<Vec<String>>,
    /// Per-command timeout in milliseconds (capped at 60000).
    #[arg(long, default_value_t = 25_000)]
    pub timeout_ms: u64,
    /// Maximum characters retained per textual observation.
    #[arg(long, default_value_t = 20_000)]
    pub max_output: usize,
    /// Leave the browser session open and report its run directory.
    #[arg(long)]
    pub keep_open: bool,
    /// Allow localhost/private-network URLs (off by default).
    #[arg(long)]
    pub allow_private_network: bool,
    /// Directory for persistent screenshots (defaults to the temporary run dir).
    #[arg(long)]
    pub artifact_dir: Option<PathBuf>,
    /// Viewport to apply after opening, e.g. 1920x1080.
    #[arg(long, value_parser = parse_viewport)]
    pub viewport: Option<(u32, u32)>,
    /// Isolated agent-browser namespace.
    #[arg(long)]
    pub namespace: Option<String>,
    /// Isolated agent-browser session name.
    #[arg(long)]
    pub session: Option<String>,
    /// Emit the hint contract as JSON.
    #[arg(long)]
    pub json: bool,
}

/// Run an exploratory study and print hints.
///
/// # Errors
///
/// Returns [`CliError::invalid_input`] for bad arguments and
/// [`CliError::execution`] when the browser study fails.
pub async fn run(args: ExploreArgs) -> Result<i32, CliError> {
    let mut options = ExploreOptions {
        url: args.url,
        timeout_ms: args.timeout_ms,
        max_output: args.max_output,
        keep_open: args.keep_open,
        allow_private_network: args.allow_private_network,
        artifact_dir: args.artifact_dir,
        viewport: args.viewport,
        namespace: args.namespace,
        session: args.session,
        ..ExploreOptions::default()
    };
    options.executable_path = args
        .executable_path
        .or_else(|| std::env::var_os("RGAA_OBSCURA_BIN").map(PathBuf::from))
        .ok_or_else(|| {
            CliError::invalid_input(
                "pass --executable-path or set RGAA_OBSCURA_BIN to the pinned obscura binary",
            )
        })?;
    if let Some(requested) = args.include {
        if requested.is_empty() {
            return Err(CliError::invalid_input("--include must not be empty"));
        }
        options.include = requested
            .iter()
            .map(|name| {
                name.parse::<ObservationKind>()
                    .map_err(|error| CliError::invalid_input(error))
            })
            .collect::<Result<Vec<_>, _>>()?;
    }

    match rgaa_explore::explore(options).await {
        Ok(result) => {
            if args.json {
                let json = result.to_json_pretty().map_err(|error| {
                    CliError::execution(format!("hint serialization failed: {error}"))
                })?;
                println!("{json}");
            } else {
                println!("{}", result.summary());
            }
            Ok(0)
        }
        Err(error) => Err(CliError::execution(error.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[derive(Debug, Parser)]
    struct TestCli {
        #[command(flatten)]
        explore: ExploreArgs,
    }

    #[test]
    fn explore_args_parse_flags() {
        let cli = TestCli::try_parse_from([
            "rgaa",
            "https://example.com",
            "--executable-path",
            "/tmp/obscura",
            "--include",
            "snapshot,console",
            "--keep-open",
            "--json",
        ])
        .expect("explore args parse");
        assert_eq!(cli.explore.url, "https://example.com");
        assert_eq!(
            cli.explore.include.as_deref(),
            Some(["snapshot", "console"].as_slice())
        );
        assert!(cli.explore.keep_open);
        assert!(cli.explore.json);
    }
}
