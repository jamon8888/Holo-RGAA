//! Behaviour of the static linter on realistic TSX, Vue and HTML sources.
//!
//! The three fixtures under `tests/fixtures/` are written as ordinary component
//! code with deliberate defects planted in it, and each test below pins the
//! exact `(rule, line)` set the linter must produce. Pinning the whole set —
//! rather than "at least one img-alt" — is what makes these tests fail on a
//! regression in either direction: a rule that stops firing and a rule that
//! starts firing on correct markup both break the assertion.

use rgaa_linter::{
    lint_paths, lint_sources, Language, LintConfig, LintError, LintOptions, Profile, RuleId,
    Severity, Source,
};
use std::path::{Path, PathBuf};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn default_options() -> LintOptions {
    LintOptions {
        profile: Profile::Rgaa41,
        config: LintConfig::default(),
        config_source: "test defaults".into(),
    }
}

fn source(name: &str) -> Source {
    Source {
        path: name.to_string(),
        content: std::fs::read_to_string(fixture(name)).expect("fixture readable"),
    }
}

/// `(rule, line)` for every finding, sorted, so assertions read as a table.
fn located(report: &rgaa_linter::LintReport) -> Vec<(&'static str, usize)> {
    let mut rows: Vec<(&'static str, usize)> = report
        .findings
        .iter()
        .map(|f| (f.rule.as_str(), f.line))
        .collect();
    rows.sort_unstable();
    rows
}

#[test]
fn a_tsx_component_reports_every_planted_defect_and_nothing_else() {
    let report = lint_sources(&[source("sample.tsx")], &default_options()).expect("lint");
    assert_eq!(
        located(&report),
        vec![
            ("button-name", 45), // icon button whose only child is alt=""
            ("button-name", 74), // self-closing button with no children at all
            ("form-label", 34),  // <input type="text"> with only a placeholder
            ("form-label", 35),  // <select> with no label
            ("img-alt", 65),     // <img> with className but no alt
            ("img-alt", 69),     // icon <img> inside a link
            ("link-name", 68),   // the link wrapping that unnamed icon
            ("link-name", 84),   // <a href> with empty content
        ]
    );
}

#[test]
fn a_vue_single_file_component_reports_every_planted_defect_and_nothing_else() {
    let report = lint_sources(&[source("sample.vue")], &default_options()).expect("lint");
    assert_eq!(
        located(&report),
        vec![
            ("button-name", 26),
            ("form-label", 12),
            ("form-label", 14),
            ("img-alt", 22),
            ("img-alt", 35),
            ("link-name", 35),
        ]
    );
}

#[test]
fn an_html_page_reports_every_planted_defect_and_nothing_else() {
    let report = lint_sources(&[source("sample.html")], &default_options()).expect("lint");
    assert_eq!(
        located(&report),
        vec![
            ("button-name", 34),
            ("form-label", 24),
            ("form-label", 31),
            ("img-alt", 13),
            ("img-alt", 39),
            ("link-name", 13),
            ("link-name", 46),
        ]
    );
}

/// The single most expensive mistake this linter could make. A dynamic value is
/// not a missing value, and a spread may supply anything; reporting either would
/// bury the real defects in noise and get the tool switched off.
#[test]
fn dynamic_values_and_spreads_never_produce_a_finding() {
    let content = r#"
<div>
  <img src={a} alt={caption} />
  <img {...imageProps} />
  <button {...buttonProps} />
  <button aria-label={label} />
  <a href={url}>{linkText}</a>
  <input id={fieldId} type="text" />
  <input type={kind} />
  <button><Icon /></button>
</div>"#;
    let report = lint_sources(
        &[Source {
            path: "dynamic.tsx".into(),
            content: content.into(),
        }],
        &default_options(),
    )
    .expect("lint");
    assert_eq!(report.findings, vec![]);
}

/// A blank name is worse than no name: it looks deliberate. Whitespace-only
/// values must not satisfy a naming rule.
#[test]
fn a_whitespace_only_accessible_name_does_not_satisfy_a_rule() {
    let content = "<button aria-label=\"   \"></button><a href=\"/x\" title=\" \"></a>";
    let report = lint_sources(
        &[Source {
            path: "blank.html".into(),
            content: content.into(),
        }],
        &default_options(),
    )
    .expect("lint");
    let mut rules: Vec<&str> = report.findings.iter().map(|f| f.rule.as_str()).collect();
    rules.sort_unstable();
    assert_eq!(rules, vec!["button-name", "link-name"]);
}

/// `alt=""` is the specified way to mark an image decorative. A linter that
/// flags it teaches authors to write a meaningless alt instead, which is worse
/// for a screen reader than silence.
#[test]
fn an_empty_alt_is_accepted_as_a_decorative_image() {
    let report = lint_sources(
        &[Source {
            path: "decorative.html".into(),
            content: "<img src=\"/spacer.gif\" alt=\"\">".into(),
        }],
        &default_options(),
    )
    .expect("lint");
    assert_eq!(report.findings, vec![]);
}

/// Markup inside comments, `<script>` and `<style>` is not markup.
#[test]
fn comments_scripts_and_styles_are_not_scanned_for_defects() {
    let content = concat!(
        "<!-- <img src=x> <button></button> -->\n",
        "<script>var s = \"<img src=y>\"; if (a < b) {}</script>\n",
        "<style>.x { content: \"<button></button>\"; }</style>\n",
        "<img src=z>\n"
    );
    let report = lint_sources(
        &[Source {
            path: "noise.html".into(),
            content: content.into(),
        }],
        &default_options(),
    )
    .expect("lint");
    assert_eq!(report.findings.len(), 1);
    assert_eq!(report.findings[0].rule, RuleId::ImgAlt);
    assert_eq!(report.findings[0].line, 4);
}

/// Columns are counted in characters. Byte counting would put the caret past
/// the element on any line containing accented text, which is most of this
/// codebase's own fixtures.
#[test]
fn a_column_is_counted_in_characters_not_bytes() {
    let content = "<p>Référence à l'élément</p><img src=x>";
    let report = lint_sources(
        &[Source {
            path: "accents.html".into(),
            content: content.into(),
        }],
        &default_options(),
    )
    .expect("lint");
    let finding = &report.findings[0];
    assert_eq!((finding.line, finding.column), (1, 29));
    assert!(content.chars().nth(28) == Some('<'));
}

#[test]
fn switching_a_rule_off_in_the_config_removes_only_that_rule() {
    let mut config = LintConfig::default();
    config.rules.insert(RuleId::ImgAlt, Severity::Off);
    let options = LintOptions {
        profile: Profile::Rgaa41,
        config,
        config_source: "test".into(),
    };
    let report = lint_sources(&[source("sample.html")], &options).expect("lint");
    assert!(!report.findings.iter().any(|f| f.rule == RuleId::ImgAlt));
    assert!(report.findings.iter().any(|f| f.rule == RuleId::LinkName));
}

#[test]
fn a_warning_severity_is_reported_but_not_counted_as_an_error() {
    let mut config = LintConfig::default();
    for rule in RuleId::ALL {
        config.rules.insert(rule, Severity::Warning);
    }
    let options = LintOptions {
        profile: Profile::Rgaa41,
        config,
        config_source: "test".into(),
    };
    let report = lint_sources(&[source("sample.html")], &options).expect("lint");
    assert_eq!(report.error_count, 0);
    assert_eq!(report.warning_count, report.findings.len());
    assert!(report.warning_count > 0);
}

/// The reason #166 asks for profiles: the same defect is filed against a
/// different framework, and Section 508 must not be told about success criteria
/// WCAG 2.1 added after the standard it incorporates.
#[test]
fn each_profile_cites_its_own_framework_for_the_same_defect() {
    let content = "<img src=x>";
    let cited = |profile: Profile| {
        let options = LintOptions {
            profile,
            config: LintConfig::default(),
            config_source: "test".into(),
        };
        let report = lint_sources(
            &[Source {
                path: "p.html".into(),
                content: content.into(),
            }],
            &options,
        )
        .expect("lint");
        report.findings[0]
            .references
            .iter()
            .map(|r| format!("{} {}", r.framework, r.id))
            .collect::<Vec<_>>()
    };
    assert_eq!(cited(Profile::Rgaa41), vec!["RGAA 4.1.2 1.1"]);
    assert_eq!(cited(Profile::Wcag21Aa), vec!["WCAG 2.1 AA 1.1.1"]);
    assert_eq!(
        cited(Profile::Section508),
        vec!["Section 508 (Revised) E205.4", "WCAG 2.0 AA 1.1.1"]
    );
}

/// Every rule must name a criterion that exists in the shipped catalog, so a
/// renumbering there cannot leave a finding citing a criterion nobody can look
/// up.
#[test]
fn every_rule_resolves_to_a_real_rgaa_criterion_and_cites_it() {
    for rule in RuleId::ALL {
        let criterion = rgaa_core::RgaaCriteria::all()
            .iter()
            .find(|c| c.id == rule.rgaa_criterion())
            .unwrap_or_else(|| panic!("{} cites unknown criterion", rule.as_str()));
        assert!(!criterion.title.is_empty());
        for profile in Profile::ALL {
            assert!(
                !profile.references(rule).is_empty(),
                "{} has no reference under {}",
                rule.as_str(),
                profile.as_str()
            );
        }
    }
}

/// `source_files` is the whole point of source mapping: findings must point at
/// the real path on disk, not at a label the caller invented.
#[test]
fn reading_files_from_disk_maps_findings_onto_their_real_paths() {
    let path = fixture("sample.vue");
    let report = lint_paths(&[&path], &default_options()).expect("lint");
    assert!(report.source_mapped);
    assert_eq!(report.files.len(), 1);
    assert_eq!(report.files[0].language, Language::Vue);
    let expected = path.display().to_string();
    assert!(report.findings.iter().all(|f| f.file == expected));

    // The mapped line/column must actually address the offending tag.
    let text = std::fs::read_to_string(&path).expect("read");
    let lines: Vec<&str> = text.lines().collect();
    for finding in &report.findings {
        assert!(lines[finding.line - 1].contains('<'), "{finding:?}");
        assert_eq!(&text[finding.offset..finding.offset + 1], "<");
    }
}

/// A missing file must abort the run. Skipping it would produce a report with
/// fewer files than the caller asked for, and a short report reads as "clean".
#[test]
fn a_missing_source_file_aborts_the_run_instead_of_shrinking_the_report() {
    let error = lint_paths(&[fixture("does-not-exist.tsx")], &default_options())
        .expect_err("must fail loudly");
    assert!(matches!(error, LintError::SourceNotFound(_)), "{error:?}");
}

#[test]
fn a_file_the_scanner_has_no_dialect_for_is_refused_rather_than_guessed_at() {
    let error = lint_sources(
        &[Source {
            path: "styles.css".into(),
            content: "body{}".into(),
        }],
        &default_options(),
    )
    .expect_err("must fail loudly");
    assert!(
        matches!(error, LintError::UnsupportedLanguage { .. }),
        "{error:?}"
    );
}

#[test]
fn every_finding_carries_a_fix_hint_that_names_something_to_write() {
    let report = lint_sources(
        &[
            source("sample.tsx"),
            source("sample.vue"),
            source("sample.html"),
        ],
        &default_options(),
    )
    .expect("lint");
    assert!(report.findings.len() >= 20);
    for finding in &report.findings {
        assert!(!finding.fix_hint.action.is_empty());
        assert!(!finding.fix_hint.suggestion.is_empty());
        assert!(!finding.snippet.is_empty());
        assert!(finding.line >= 1 && finding.column >= 1);
    }
}
