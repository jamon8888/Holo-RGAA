use rgaa_obscura::{AnalyzeConfig, AnalyzeRequest, ObscuraBridge};

use crate::commands::{write_output, CommonArgs};
use crate::config::Config;
use crate::CliError;

#[derive(Debug, clap::Args)]
pub struct AnalyzeArgs {
    #[clap(flatten)]
    pub common: CommonArgs,
    #[clap(long, conflicts_with = "profile")]
    pub url: Option<String>,
    #[clap(long, conflicts_with = "url")]
    pub profile: Option<String>,
}

pub async fn run(args: AnalyzeArgs) -> Result<i32, CliError> {
    let config = Config::load(args.common.config.as_deref())
        .map_err(|error| CliError::invalid_input(error.to_string()))?;
    let url = resolve_url(&config, args.url, args.profile)?;
    let request = AnalyzeRequest {
        url,
        config: analyze_config(&config)?,
    };
    request
        .validate()
        .map_err(|error| CliError::invalid_input(error.to_string()))?;

    let mut bridge = ObscuraBridge::new();
    bridge
        .start_server()
        .await
        .map_err(|error| CliError::execution(format!("browser unavailable: {error}")))?;
    let result = bridge
        .analyze(&request)
        .await
        .map_err(|error| CliError::execution(error.to_string()))?;
    let rendered = serde_json::to_string_pretty(&result)
        .map_err(|error| CliError::execution(error.to_string()))?;
    write_output(&args.common.output, &rendered)?;
    Ok(0)
}

fn resolve_url(
    config: &Config,
    url: Option<String>,
    profile: Option<String>,
) -> Result<String, CliError> {
    match (url, profile) {
        (Some(url), None) => Ok(url),
        (None, Some(profile)) => config
            .url_profiles
            .get(&profile)
            .map(|entry| entry.url.clone())
            .ok_or_else(|| CliError::invalid_input(format!("unknown url profile '{profile}'"))),
        (None, None) => config
            .url_profiles
            .get("default")
            .map(|entry| entry.url.clone())
            .ok_or_else(|| CliError::invalid_input("provide --url or a configured url profile")),
        (Some(_), Some(_)) => unreachable!("clap enforces url/profile exclusivity"),
    }
}

fn analyze_config(config: &Config) -> Result<AnalyzeConfig, CliError> {
    let mut analyze = AnalyzeConfig::default();
    if let Some(profile) = config
        .url_profiles
        .get("default")
        .and_then(|entry| entry.viewport.as_deref())
    {
        apply_viewport(config, profile, &mut analyze)?;
    }
    Ok(analyze)
}

fn apply_viewport(
    config: &Config,
    name: &str,
    analyze: &mut AnalyzeConfig,
) -> Result<(), CliError> {
    let viewport = config
        .viewport_profiles
        .get(name)
        .ok_or_else(|| CliError::invalid_input(format!("unknown viewport profile '{name}'")))?;
    analyze.viewport.width = viewport.width;
    analyze.viewport.height = viewport.height;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_explicit_url_over_profiles() {
        let config = Config::default();
        assert_eq!(
            resolve_url(&config, Some("https://a.test".into()), None).unwrap(),
            "https://a.test"
        );
    }

    #[test]
    fn unknown_profile_is_rejected() {
        let config = Config::default();
        assert!(resolve_url(&config, None, Some("missing".into())).is_err());
    }

    #[test]
    fn default_profile_is_used_when_no_arguments() {
        let mut config = Config::default();
        config.url_profiles.insert(
            "default".into(),
            crate::config::UrlProfile {
                url: "https://default.test".into(),
                viewport: None,
            },
        );
        assert_eq!(
            resolve_url(&config, None, None).unwrap(),
            "https://default.test"
        );
    }

    #[test]
    fn viewport_profile_override_applies_dimensions() {
        let mut config = Config::default();
        config.viewport_profiles.insert(
            "mobile".into(),
            crate::config::ViewportProfile {
                width: 375,
                height: 812,
            },
        );
        let mut analyze = AnalyzeConfig::default();
        apply_viewport(&config, "mobile", &mut analyze).unwrap();
        assert_eq!(analyze.viewport.width, 375);
    }
}
