use std::fmt::Write;
use std::sync::Arc;

use chrono::Local;
use rgaa_orchestrator::{
    BusinessProfile, Orchestrator, Schedule, ScheduledJob, ScheduledRun, Scheduler, SeoStage,
};

use crate::commands::{write_output, CommonArgs};
use crate::config::{parse_daily_time, parse_interval, Config, JobConfig};
use crate::CliError;

#[derive(Debug, clap::Args)]
pub struct ScheduleArgs {
    #[clap(flatten)]
    pub common: CommonArgs,
    /// Only these job names (repeatable). Default: every configured job.
    #[clap(long = "job")]
    pub jobs: Vec<String>,
    /// Print the resolved jobs and their next run, then exit.
    #[clap(long, conflicts_with = "once")]
    pub list: bool,
    /// Run the selected jobs once now instead of looping.
    #[clap(long, conflicts_with = "list")]
    pub once: bool,
    /// Output format for --list / --once: json or table.
    #[clap(long, default_value = "table")]
    pub format: String,
}

pub async fn run(args: ScheduleArgs) -> Result<i32, CliError> {
    let config = Config::load(args.common.config.as_deref())
        .map_err(|error| CliError::invalid_input(error.to_string()))?;
    let jobs = resolve_jobs(&config, &args.jobs)?;
    let format = args.format.to_lowercase();
    if format != "json" && format != "table" {
        return Err(CliError::invalid_input(format!(
            "unsupported format '{}'",
            args.format
        )));
    }

    if args.list {
        let rendered = render_list(&jobs, &format)?;
        write_output(&args.common.output, &rendered)?;
        return Ok(0);
    }

    let mut seo = SeoStage::default();
    if let Some(profile) = &config.schedule.business_profile {
        seo = seo.with_business_profile(BusinessProfile {
            name: profile.name.clone(),
            phone: profile.phone.clone(),
            address: profile.address.clone(),
        });
    }
    let scheduler = Scheduler::new(Arc::new(Orchestrator::new().with_seo(seo)));

    if args.once {
        let mut runs = Vec::new();
        let mut failed = 0usize;
        for job in &jobs {
            match scheduler.run_once(job).await {
                Ok(run) => runs.push(run),
                Err(error) => {
                    failed += 1;
                    eprintln!("job '{}' failed: {error}", job.name);
                }
            }
        }
        let rendered = render_runs(&runs, &format)?;
        write_output(&args.common.output, &rendered)?;
        return if failed == 0 {
            Ok(0)
        } else {
            Err(CliError::execution(format!(
                "{failed} of {} job(s) failed",
                jobs.len()
            )))
        };
    }

    eprintln!("{}", render_list(&jobs, "table")?);
    eprintln!("Scheduler running; press Ctrl-C to stop.");
    let (stop_tx, stop_rx) = tokio::sync::watch::channel(false);
    tokio::spawn(async move {
        if tokio::signal::ctrl_c().await.is_ok() {
            let _ = stop_tx.send(true);
        }
    });
    scheduler
        .run_until(jobs, stop_rx)
        .await
        .map_err(CliError::execution)?;
    eprintln!("Scheduler stopped.");
    Ok(0)
}

fn resolve_jobs(config: &Config, selected: &[String]) -> Result<Vec<ScheduledJob>, CliError> {
    if config.schedule.jobs.is_empty() {
        return Err(CliError::invalid_input(
            "no schedule jobs configured (add a `schedule.jobs` section to .rgaa/config.yaml)",
        ));
    }
    for name in selected {
        if !config.schedule.jobs.iter().any(|j| &j.name == name) {
            return Err(CliError::invalid_input(format!(
                "unknown schedule job '{name}'"
            )));
        }
    }
    config
        .schedule
        .jobs
        .iter()
        .filter(|j| selected.is_empty() || selected.contains(&j.name))
        .map(|j| job_from_config(config, j))
        .collect()
}

fn job_from_config(config: &Config, job: &JobConfig) -> Result<ScheduledJob, CliError> {
    let mut urls: Vec<String> = job
        .profiles
        .iter()
        .map(|p| {
            config
                .url_profiles
                .get(p)
                .map(|entry| entry.url.clone())
                .ok_or_else(|| CliError::invalid_input(format!("unknown url profile '{p}'")))
        })
        .collect::<Result<_, _>>()?;
    urls.extend(job.urls.iter().cloned());
    urls.dedup();

    let schedule = match (&job.at, &job.every) {
        (Some(at), _) => {
            let (hour, minute) = parse_daily_time(at).map_err(CliError::invalid_input)?;
            Schedule::Daily { hour, minute }
        }
        (None, Some(every)) => {
            Schedule::Every(parse_interval(every).map_err(CliError::invalid_input)?)
        }
        (None, None) => Schedule::NIGHTLY,
    };

    let mut scheduled = ScheduledJob::nightly(job.name.clone(), urls).with_schedule(schedule);
    if let Some(path) = &job.head_template {
        scheduled = scheduled.with_head_template(path.clone());
    }
    scheduled.validate().map_err(CliError::invalid_input)?;
    Ok(scheduled)
}

fn describe_schedule(schedule: &Schedule) -> String {
    match schedule {
        Schedule::Daily { hour, minute } => format!("daily at {hour:02}:{minute:02}"),
        Schedule::Every(d) => format!("every {}s", d.as_secs()),
    }
}

fn render_list(jobs: &[ScheduledJob], format: &str) -> Result<String, CliError> {
    let now = Local::now();
    if format == "json" {
        let value: Vec<serde_json::Value> = jobs
            .iter()
            .map(|j| {
                serde_json::json!({
                    "name": j.name,
                    "urls": j.urls,
                    "schedule": describe_schedule(&j.schedule),
                    "next_run": j.schedule.next_run(now).to_rfc3339(),
                    "head_template": j.head_template.as_ref().map(|p| p.display().to_string()),
                })
            })
            .collect();
        return serde_json::to_string_pretty(&value)
            .map_err(|error| CliError::execution(error.to_string()));
    }
    let mut out = String::new();
    let _ = writeln!(&mut out, "=== Scheduled jobs ===");
    for j in jobs {
        let _ = writeln!(&mut out);
        let _ = writeln!(&mut out, "{}", j.name);
        let _ = writeln!(
            &mut out,
            "  schedule:      {}",
            describe_schedule(&j.schedule)
        );
        let _ = writeln!(
            &mut out,
            "  next run:      {}",
            j.schedule.next_run(now).format("%Y-%m-%d %H:%M %Z")
        );
        let _ = writeln!(&mut out, "  urls:          {}", j.urls.join(", "));
        let _ = writeln!(
            &mut out,
            "  head template: {}",
            j.head_template.as_ref().map_or_else(
                || "none (no patch proposals)".to_string(),
                |p| p.display().to_string()
            )
        );
    }
    Ok(out)
}

fn render_runs(runs: &[ScheduledRun], format: &str) -> Result<String, CliError> {
    if format == "json" {
        let value: Vec<serde_json::Value> = runs.iter().map(ScheduledRun::summary_json).collect();
        return serde_json::to_string_pretty(&value)
            .map_err(|error| CliError::execution(error.to_string()));
    }
    let mut out = String::new();
    let _ = writeln!(&mut out, "=== Scheduled runs ===");
    for run in runs {
        let s = run.summary_json();
        let _ = writeln!(&mut out);
        let _ = writeln!(&mut out, "{}", run.job);
        let _ = writeln!(&mut out, "  audits:            {}", s["audits"]);
        let _ = writeln!(&mut out, "  findings:          {}", s["findings"]);
        let _ = writeln!(
            &mut out,
            "  by criticality:    P0={} P1={} P2={} P3={}",
            s["by_criticality"]["P0"],
            s["by_criticality"]["P1"],
            s["by_criticality"]["P2"],
            s["by_criticality"]["P3"]
        );
        let _ = writeln!(&mut out, "  awaiting approval: {}", s["awaiting_approval"]);
        let _ = writeln!(&mut out, "  needs review:      {}", s["needs_review"]);
        let _ = writeln!(&mut out, "  triaged:           {}", s["triaged"]);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ScheduleConfig, UrlProfile};

    fn config() -> Config {
        let mut config = Config::default();
        config.url_profiles.insert(
            "home".into(),
            UrlProfile {
                url: "https://a.test".into(),
                viewport: None,
            },
        );
        config.schedule = ScheduleConfig {
            business_profile: None,
            jobs: vec![
                JobConfig {
                    name: "nightly".into(),
                    profiles: vec!["home".into()],
                    urls: vec!["https://a.test/contact".into()],
                    at: Some("02:30".into()),
                    head_template: Some("src/layout.html".into()),
                    ..Default::default()
                },
                JobConfig {
                    name: "hourly".into(),
                    urls: vec!["https://a.test".into()],
                    every: Some("1h".into()),
                    ..Default::default()
                },
                JobConfig {
                    name: "implicit".into(),
                    urls: vec!["https://a.test".into()],
                    ..Default::default()
                },
            ],
        };
        config
    }

    #[test]
    fn resolves_profiles_and_urls_into_jobs() {
        let jobs = resolve_jobs(&config(), &[]).unwrap();
        assert_eq!(jobs.len(), 3);
        assert_eq!(
            jobs[0].urls,
            vec!["https://a.test", "https://a.test/contact"]
        );
        assert_eq!(
            jobs[0].schedule,
            Schedule::Daily {
                hour: 2,
                minute: 30
            }
        );
        assert_eq!(
            jobs[0].head_template.as_deref(),
            Some(std::path::Path::new("src/layout.html"))
        );
        assert_eq!(
            jobs[1].schedule,
            Schedule::Every(std::time::Duration::from_secs(3600))
        );
        assert_eq!(jobs[2].schedule, Schedule::NIGHTLY, "no at/every → nightly");
    }

    #[test]
    fn job_selection_filters_and_rejects_unknown_names() {
        let jobs = resolve_jobs(&config(), &["hourly".into()]).unwrap();
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].name, "hourly");
        let err = resolve_jobs(&config(), &["nope".into()]).unwrap_err();
        assert_eq!(err.exit_code(), 2);
        let err = resolve_jobs(&Config::default(), &[]).unwrap_err();
        assert_eq!(err.exit_code(), 2);
    }

    #[test]
    fn list_renders_table_and_json() {
        let jobs = resolve_jobs(&config(), &[]).unwrap();
        let table = render_list(&jobs, "table").unwrap();
        assert!(table.contains("nightly"));
        assert!(table.contains("daily at 02:30"));
        assert!(table.contains("every 3600s"));
        assert!(table.contains("no patch proposals"));
        let json: Vec<serde_json::Value> =
            serde_json::from_str(&render_list(&jobs, "json").unwrap()).unwrap();
        assert_eq!(json.len(), 3);
        assert!(json[0]["next_run"].as_str().unwrap().contains('T'));
    }
}
