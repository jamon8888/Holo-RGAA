use std::fmt::Write;

use rgaa_core::CrawlConfig;
use rgaa_orchestrator::Orchestrator;

use crate::commands::{write_output, CommonArgs};
use crate::config::Config;
use crate::CliError;

#[derive(Debug, clap::Args)]
pub struct AnalyzeArgs {
    #[clap(flatten)]
    pub common: CommonArgs,
    #[clap(long, conflicts_with = "profile", help = "URL to audit")]
    pub url: Option<String>,
    #[clap(
        long,
        conflicts_with = "url",
        help = "Name of a configured URL profile"
    )]
    pub profile: Option<String>,
    #[clap(long, help = "Enable verbose output with detailed progress")]
    pub verbose: bool,
}

pub async fn run(args: AnalyzeArgs) -> Result<i32, CliError> {
    let config = Config::load(args.common.config.as_deref())
        .map_err(|error| CliError::invalid_input(error.to_string()))?;
    let url = resolve_url(&config, args.url, args.profile)?;

    if args.verbose {
        eprintln!("Starting audit for: {}", url);
    }

    let crawl_config = crawl_config(&config)?;
    let orchestrator = Orchestrator::new();

    if args.verbose {
        eprintln!("Running accessibility audit...");
    }

    let result = match std::env::var("RGAA_SITEMAP_URL")
        .ok()
        .filter(|s| !s.is_empty())
    {
        Some(sitemap_url) => {
            if args.verbose {
                eprintln!("Discovering pillar pages from sitemap: {sitemap_url}");
            }
            let pages =
                crate::sitemap::discover_pillar_pages(&sitemap_url, &url, crawl_config.max_pages)
                    .await?;
            if pages.is_empty() {
                return Err(CliError::execution(format!(
                    "sitemap {sitemap_url} yielded no top-level pages to audit"
                )));
            }
            if args.verbose {
                eprintln!("Auditing {} page(s): {}", pages.len(), pages.join(", "));
            }
            orchestrator
                .run_explicit_audit(&url, pages, &crawl_config)
                .await
        }
        None => orchestrator.run_crawl_and_audit(&url, &crawl_config).await,
    }
    .map_err(|error| CliError::execution(error.to_string()))?;

    let format = args.common.format.as_deref().unwrap_or("json");

    if args.verbose {
        eprintln!("Audit complete. Generating {format} report...");
    }

    let rendered = render_output(&result, format)?;
    write_output(&args.common.output, &rendered)?;
    Ok(0)
}

fn render_output(result: &rgaa_core::AuditResult, format: &str) -> Result<String, CliError> {
    match format.to_lowercase().as_str() {
        "json" => serde_json::to_string_pretty(result)
            .map_err(|error| CliError::execution(error.to_string())),
        "table" => Ok(render_table(result)),
        "html" => Ok(render_html(result)),
        other => Err(CliError::invalid_input(format!(
            "unsupported format '{other}'"
        ))),
    }
}

fn render_table(result: &rgaa_core::AuditResult) -> String {
    let mut out = String::new();
    let _ = writeln!(&mut out, "=== RGAA Audit Results ===");
    let _ = writeln!(&mut out);
    let _ = writeln!(&mut out, "Audit ID: {}", result.audit_id);
    let _ = writeln!(&mut out, "URL: {}", result.url);
    let _ = writeln!(&mut out, "Conformity: {}", result.etat_conformite);
    let _ = writeln!(&mut out, "Global Rate: {:.1}%", result.taux_global);
    let _ = writeln!(&mut out, "Coverage: {:.1}%", result.coverage_percent);
    let _ = writeln!(&mut out);
    let _ = writeln!(&mut out, "--- Results ---");
    let _ = writeln!(&mut out, "Passed:   {}", result.passed);
    let _ = writeln!(&mut out, "Failed:   {}", result.failed);
    let _ = writeln!(&mut out, "N/A:      {}", result.na);
    let _ = writeln!(&mut out, "Total:    {}", result.total_criteria);
    let _ = writeln!(&mut out);
    let _ = writeln!(&mut out, "--- Pages ---");
    for page in &result.pages {
        let _ = writeln!(&mut out, "{}", page.url);
        for criterion in &page.criteria {
            let status_symbol = match criterion.status {
                rgaa_core::CriterionStatus::Pass => "[PASS]",
                rgaa_core::CriterionStatus::Fail => "[FAIL]",
                rgaa_core::CriterionStatus::NotApplicable => "[N/A]",
                rgaa_core::CriterionStatus::NeedsReview => "[REVIEW]",
                _ => "[??]",
            };
            let _ = writeln!(
                &mut out,
                "  {} {} - {}",
                status_symbol, criterion.criterion_id, criterion.title
            );
        }
    }
    out
}

fn render_html(result: &rgaa_core::AuditResult) -> String {
    let bundle = rgaa_core::AuditBundle::from(result.clone());
    rgaa_report::report::html::generate_html_report(&bundle)
}

fn resolve_url(
    config: &Config,
    url: Option<String>,
    profile: Option<String>,
) -> Result<String, CliError> {
    let resolved = match (url, profile) {
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
    }?;
    Ok(with_scheme(resolved))
}

/// Prepends `https://` when the URL has no scheme, so bare domains like
/// `example.com` work the same as they do when typed into a browser.
///
/// Checks for an actual leading scheme via `Url::parse` rather than a
/// substring search for `"://"` — the latter is fooled by a bare domain
/// whose query string happens to contain `"://"`, e.g.
/// `example.test/search?next=https://other.test/`, which `contains` sees as
/// "already has a scheme" and leaves the real host without one.
fn with_scheme(url: String) -> String {
    if reqwest::Url::parse(&url).is_ok() {
        url
    } else {
        format!("https://{url}")
    }
}

fn crawl_config(_config: &Config) -> Result<CrawlConfig, CliError> {
    let mut crawl_config = CrawlConfig::default();
    if let Some(max_pages) = std::env::var("RGAA_MAX_PAGES")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
    {
        if max_pages == 0 {
            return Err(CliError::invalid_input(
                "RGAA_MAX_PAGES must be greater than 0",
            ));
        }
        crawl_config.max_pages = max_pages;
    }
    if let Some(max_depth) = std::env::var("RGAA_MAX_DEPTH")
        .ok()
        .and_then(|v| v.parse().ok())
    {
        crawl_config.max_depth = max_depth;
    }
    if let Some(sample_mode) = std::env::var("RGAA_SAMPLE_MODE")
        .ok()
        .and_then(|v| v.parse::<bool>().ok())
    {
        crawl_config.sample_mode = sample_mode;
    }
    Ok(crawl_config)
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
    fn adds_https_scheme_to_bare_domain() {
        let config = Config::default();
        assert_eq!(
            resolve_url(&config, Some("danijelagracner.com".into()), None).unwrap(),
            "https://danijelagracner.com"
        );
    }

    #[test]
    fn preserves_explicit_http_scheme() {
        let config = Config::default();
        assert_eq!(
            resolve_url(&config, Some("http://a.test".into()), None).unwrap(),
            "http://a.test"
        );
    }

    #[test]
    fn adds_scheme_even_when_query_string_contains_a_scheme_like_substring() {
        let config = Config::default();
        assert_eq!(
            resolve_url(
                &config,
                Some("example.test/search?next=https://other.test/".into()),
                None
            )
            .unwrap(),
            "https://example.test/search?next=https://other.test/"
        );
    }
}
