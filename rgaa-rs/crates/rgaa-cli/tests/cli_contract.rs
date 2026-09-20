use std::path::PathBuf;

use rgaa_cli::commands::policy::PolicyArgs;
use rgaa_cli::commands::report::ReportArgs;
use rgaa_cli::commands::verify::VerifyArgs;
use rgaa_cli::commands::CommonArgs;
use rgaa_cli::commands::{policy, report, verify};
use rgaa_cli::CliError;
use rgaa_core::{AuditBundle, AuditConfig, CriterionStatus};

fn temp_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("rgaa-cli-test-{name}-{}", std::process::id()))
}

fn write_json(path: &PathBuf, value: &impl serde::Serialize) {
    std::fs::write(path, serde_json::to_string_pretty(value).unwrap()).unwrap();
}

fn sample_bundle(failed: usize, errors: usize) -> AuditBundle {
    let mut bundle = AuditBundle::new("audit-1", "https://example.test", AuditConfig::default());
    bundle.summary.total_pages = 1;
    bundle.summary.completed_pages = 1;
    for i in 0..failed {
        let mut finding = rgaa_core::Finding::new(format!("finding-{i}"));
        finding.rule = "rgaa-1.1".into();
        finding.url = "https://example.test".into();
        finding.target = "#main".into();
        finding.status = CriterionStatus::Fail;
        finding.severity = Some("critical".into());
        finding.description = Some("missing alternative text".into());
        bundle.findings.push(finding);
    }
    bundle.summary.total_findings = failed + errors;
    bundle.summary.failed = failed;
    bundle.summary.errors = errors;
    bundle
}

fn common() -> CommonArgs {
    CommonArgs {
        config: None,
        output: None,
        format: None,
        audit_id: None,
    }
}

#[test]
fn report_renders_each_format_without_failing() {
    let bundle_path = temp_path("bundle.json");
    write_json(&bundle_path, &sample_bundle(1, 0));

    for format in ["json", "markdown", "sarif", "junit"] {
        let args = ReportArgs {
            common: CommonArgs {
                format: Some(format.into()),
                ..common()
            },
            input: Some(bundle_path.clone()),
            audit_id: None,
        };
        assert_eq!(
            report::run(args).unwrap(),
            0,
            "format {format} should render"
        );
    }
}

#[test]
fn report_rejects_invalid_bundle_as_invalid_input() {
    let path = temp_path("invalid.json");
    std::fs::write(&path, "{not json}").unwrap();
    let args = ReportArgs {
        common: common(),
        input: Some(path),
        audit_id: None,
    };
    let error = report::run(args).expect_err("invalid bundle");
    assert_eq!(error.exit_code(), 2);
}

#[test]
fn report_missing_file_is_execution_error() {
    let args = ReportArgs {
        common: common(),
        input: Some(PathBuf::from("/nonexistent/bundle.json")),
        audit_id: None,
    };
    let error = report::run(args).expect_err("missing file");
    assert_eq!(error.exit_code(), 3);
}

#[test]
fn policy_passes_when_compliance_meets_threshold() {
    let path = temp_path("pass.json");
    write_json(&path, &sample_bundle(0, 0));
    let args = PolicyArgs {
        common: common(),
        input: path,
    };
    assert_eq!(policy::run(args).unwrap(), 0);
}

#[test]
fn policy_fails_when_compliance_is_below_threshold() {
    let path = temp_path("fail.json");
    write_json(&path, &sample_bundle(1, 0));
    let args = PolicyArgs {
        common: common(),
        input: path,
    };
    assert_eq!(policy::run(args).unwrap(), 1);
}

#[test]
fn verify_requires_one_to_twenty_five_issues() {
    let empty = temp_path("empty.json");
    write_json(&empty, &Vec::<rgaa_remediation::RemediationIssue>::new());
    let args = VerifyArgs {
        common: common(),
        issues: empty,
    };
    assert_eq!(verify::run(args).expect_err("empty").exit_code(), 2);
}

#[test]
fn verify_reports_success_for_valid_issues() {
    let issue = rgaa_remediation::RemediationIssue {
        id: "a".into(),
        rule: "image-alt".into(),
        element_html: "import React from \"react\"; <img src=\"hero.png\">".into(),
        page_url: "https://example.test".into(),
        source_locations: vec![rgaa_remediation::SourceLocation {
            file: "src/App.tsx".into(),
            line: 1,
            column: None,
        }],
        summary: "missing alternative text".into(),
        remediation: "add alt".into(),
        criteria: vec!["RGAA-1.1".into()],
        framework: Some(rgaa_remediation::Framework::React),
    };
    let path = temp_path("issues.json");
    write_json(&path, &vec![issue]);
    let args = VerifyArgs {
        common: common(),
        issues: path,
    };
    assert_eq!(verify::run(args).unwrap(), 0);
}

#[test]
fn exit_codes_match_the_contract() {
    assert_eq!(CliError::policy("x").exit_code(), 1);
    assert_eq!(CliError::invalid_input("x").exit_code(), 2);
    assert_eq!(CliError::execution("x").exit_code(), 3);
}

#[test]
fn config_loads_valid_yaml_and_rejects_invalid() {
    let dir = temp_path("config");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join(".rgaa")).unwrap();
    let config_path = dir.join(".rgaa").join("config.yaml");
    std::fs::write(
        &config_path,
        "policy:\n  min_compliance: 90.0\nupload_consent: true\n",
    )
    .unwrap();

    let config = rgaa_cli::Config::load(Some(&config_path)).expect("valid config");
    assert_eq!(config.policy.min_compliance, 90.0);
    assert!(config.upload_consent);

    std::fs::write(&config_path, "not: [valid").unwrap();
    assert!(rgaa_cli::Config::load(Some(&config_path)).is_err());
}

#[tokio::test]
async fn schedule_list_reads_jobs_from_config_and_needs_no_browser() {
    use rgaa_cli::commands::schedule::{self, ScheduleArgs};

    let dir = temp_path("schedule");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let config_path = dir.join("config.yaml");
    std::fs::write(
        &config_path,
        r#"
url_profiles:
  home: { url: "https://example.test" }
schedule:
  business_profile: { name: "Dupont", phone: "0412345678", address: "12 rue X" }
  jobs:
    - name: nightly
      profiles: [home]
      at: "02:00"
      head_template: src/layout.html
"#,
    )
    .unwrap();
    let output = dir.join("list.json");

    let args = ScheduleArgs {
        common: CommonArgs {
            config: Some(config_path.clone()),
            output: Some(output.clone()),
            ..common()
        },
        jobs: vec![],
        list: true,
        once: false,
        format: "json".into(),
    };
    assert_eq!(schedule::run(args).await.unwrap(), 0);
    let listed: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&output).unwrap()).unwrap();
    assert_eq!(listed[0]["name"], "nightly");
    assert_eq!(listed[0]["schedule"], "daily at 02:00");
    assert_eq!(listed[0]["urls"][0], "https://example.test");

    let unknown = ScheduleArgs {
        common: CommonArgs {
            config: Some(config_path.clone()),
            ..common()
        },
        jobs: vec!["missing".into()],
        list: true,
        once: false,
        format: "table".into(),
    };
    assert_eq!(schedule::run(unknown).await.unwrap_err().exit_code(), 2);

    let bad_format = ScheduleArgs {
        common: CommonArgs {
            config: Some(config_path),
            ..common()
        },
        jobs: vec![],
        list: true,
        once: false,
        format: "xml".into(),
    };
    assert_eq!(schedule::run(bad_format).await.unwrap_err().exit_code(), 2);

    let no_jobs = ScheduleArgs {
        common: CommonArgs {
            config: Some(PathBuf::from("/nonexistent/config.yaml")),
            ..common()
        },
        jobs: vec![],
        list: true,
        once: false,
        format: "table".into(),
    };
    assert_eq!(schedule::run(no_jobs).await.unwrap_err().exit_code(), 2);
}
