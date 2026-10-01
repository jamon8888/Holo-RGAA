//! `lint-rules.toml` handling.
//!
//! Every test here is about the same property: a configuration problem must
//! surface as an error naming the file, never as a silent fallback to defaults.

use rgaa_linter::config::{ConfigError, SUPPORTED_VERSION};
use rgaa_linter::{LintConfig, Profile, RuleId, Severity};
use std::path::Path;

fn parse(text: &str) -> Result<LintConfig, ConfigError> {
    LintConfig::parse(Path::new("/tmp/lint-rules.toml"), text)
}

#[test]
fn a_valid_config_overrides_only_the_rules_it_names() {
    let config = parse(
        r#"
version = 1
profile = "wcag-2.1-aa"

[rules]
link-name = "warning"
img-alt = "off"
"#,
    )
    .expect("valid config");
    assert_eq!(config.profile, Profile::Wcag21Aa);
    assert_eq!(config.severity(RuleId::LinkName), Severity::Warning);
    assert_eq!(config.severity(RuleId::ImgAlt), Severity::Off);
    // Untouched rules keep their default rather than being reset.
    assert_eq!(config.severity(RuleId::ButtonName), Severity::Error);
}

/// The failure this whole module exists to prevent: a typo that silently does
/// nothing, leaving the user convinced the linter is obeying a setting it never
/// saw.
#[test]
fn a_misspelled_rule_name_is_an_error_not_a_setting_that_does_nothing() {
    let error = parse("version = 1\n[rules]\nimage-alt = \"off\"\n").expect_err("must reject");
    assert!(matches!(error, ConfigError::Invalid { .. }), "{error:?}");
    assert!(error.to_string().contains("image-alt"), "{error}");
}

#[test]
fn an_unknown_top_level_key_is_an_error() {
    let error = parse("version = 1\nstrictness = \"high\"\n").expect_err("must reject");
    assert!(matches!(error, ConfigError::Invalid { .. }), "{error:?}");
}

#[test]
fn an_unknown_severity_is_an_error() {
    let error = parse("version = 1\n[rules]\nimg-alt = \"fatal\"\n").expect_err("must reject");
    assert!(matches!(error, ConfigError::Invalid { .. }), "{error:?}");
}

#[test]
fn an_unknown_profile_is_an_error() {
    let error = parse("version = 1\nprofile = \"wcag-3\"\n").expect_err("must reject");
    assert!(matches!(error, ConfigError::Invalid { .. }), "{error:?}");
}

/// A config written for a future schema must not be half-applied by this build.
#[test]
fn a_config_from_a_future_schema_version_is_refused() {
    let error = parse("version = 99\n").expect_err("must reject");
    match error {
        ConfigError::UnsupportedVersion { found, .. } => assert_eq!(found, 99),
        other => panic!("expected a version error, got {other:?}"),
    }
    assert_eq!(SUPPORTED_VERSION, 1);
}

#[test]
fn a_config_path_the_caller_named_must_exist() {
    let error = LintConfig::resolve(Some(Path::new("/nonexistent/rgaa/lint-rules.toml")))
        .expect_err("must fail loudly");
    assert!(matches!(error, ConfigError::NotFound { .. }), "{error:?}");
    assert!(error.to_string().contains("config_path"), "{error}");
}

#[test]
fn an_explicit_config_is_loaded_and_reported_as_the_source() {
    let dir = std::env::temp_dir().join(format!("rgaa-linter-cfg-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("lint-rules.toml");
    std::fs::write(&path, "version = 1\n[rules]\nbutton-name = \"warning\"\n").expect("write");
    let (config, source) = LintConfig::resolve(Some(&path)).expect("load");
    assert_eq!(config.severity(RuleId::ButtonName), Severity::Warning);
    assert_eq!(source, path.display().to_string());
    std::fs::remove_dir_all(&dir).ok();
}

/// The documented example must itself be valid; a broken example is a support
/// ticket generator.
#[test]
fn the_documented_example_config_parses() {
    let config = parse(&LintConfig::example_toml()).expect("example must be valid");
    assert_eq!(config.version, SUPPORTED_VERSION);
    for rule in RuleId::ALL {
        assert_eq!(config.severity(rule), rule.default_severity());
    }
}
