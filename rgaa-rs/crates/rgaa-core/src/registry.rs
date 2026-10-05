//! The single registry of audit mechanisms (spec `criteria-coverage-expansion` §5,
//! issue #261).
//!
//! One declarative table, `data/rgaa-4.1.2/mechanisms.toml`, says which mechanisms
//! decide which RGAA criteria, and how far. It replaces the former
//! `axe_mapping.json` (axe coverage) and `GapFixRules::COMPLETE_COVERAGE`.
//!
//! The registry is *data plus invariants*: [`MechanismRegistry::check`] returns every
//! violated invariant so CI can fail on any of them; nothing here touches a browser.

use crate::catalog::{AxeCoverage, AxeProvenance};
use crate::RgaaError;
use serde::Deserialize;
use std::collections::{BTreeSet, HashSet};
use std::path::Path;
use std::sync::OnceLock;

const MECHANISMS_TOML: &str = include_str!("../data/rgaa-4.1.2/mechanisms.toml");

/// How a mechanism reaches its verdict.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MechanismKind {
    /// An axe-core rule, mapped as is.
    AxeNative,
    /// A static JS probe over the rendered DOM.
    JsStatic,
    /// A JS probe that interacts with the page.
    JsBehavioural,
    /// Analysis of media content.
    MediaAnalysis,
    /// A verdict computed across pages.
    Site,
}

/// The browser engine a mechanism needs. Chrome is only admitted with proof that a
/// fixture fails under Obscura and passes under Chrome (spec §3.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Engine {
    Obscura,
    Chrome,
}

/// A verdict a mechanism may emit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    Pass,
    Fail,
    Review,
}

/// One registry entry.
#[derive(Debug, Clone, Deserialize)]
pub struct Mechanism {
    pub id: String,
    /// Criterion id, e.g. `"11.6"`.
    pub criterion: String,
    pub kind: MechanismKind,
    pub engine: Engine,
    pub coverage: AxeCoverage,
    /// RGAA tests covered, e.g. `["11.6.1"]`. May stay empty for legacy entries.
    #[serde(default)]
    pub tests: Vec<String>,
    pub outcomes: Vec<Outcome>,
    /// Fixture names (file stems in `rgaa-test-corpus/criteria/`).
    #[serde(default)]
    pub fixtures: Vec<String>,
    /// Inherited entry, exempt from the fixture requirement.
    pub legacy: bool,
    /// Axe rule ids, for `axe-native` mechanisms.
    #[serde(default)]
    pub axe_rules: Vec<String>,
    #[serde(default)]
    pub provenance: Option<AxeProvenance>,
}

#[derive(Debug, Deserialize)]
struct RegistryFile {
    mechanism: Vec<Mechanism>,
}

/// The parsed registry, in file order (deterministic).
#[derive(Debug, Clone)]
pub struct MechanismRegistry {
    mechanisms: Vec<Mechanism>,
}

impl MechanismRegistry {
    /// Parse a registry from TOML text.
    #[must_use = "le registre parsé doit être utilisé"]
    pub fn from_toml_str(text: &str) -> Result<Self, RgaaError> {
        let file: RegistryFile = toml::from_str(text)
            .map_err(|e| RgaaError::MissingField(format!("mechanism registry: {e}")))?;
        Ok(Self {
            mechanisms: file.mechanism,
        })
    }

    /// The registry shipped with the crate, parsed once per process.
    ///
    /// The data is embedded at compile time and `builtin_registry_parses` guards it in
    /// CI, so a parse failure is a build defect, not a runtime condition (same
    /// contract as the catalog's embedded JSON).
    pub fn builtin() -> &'static Self {
        static REGISTRY: OnceLock<MechanismRegistry> = OnceLock::new();
        REGISTRY.get_or_init(|| {
            Self::from_toml_str(MECHANISMS_TOML).expect("mechanisms.toml must parse")
        })
    }

    pub fn mechanisms(&self) -> &[Mechanism] {
        &self.mechanisms
    }

    /// The axe-native mechanism of a criterion, if it has one.
    pub fn axe_for(&self, criterion: &str) -> Option<&Mechanism> {
        self.mechanisms
            .iter()
            .find(|m| m.kind == MechanismKind::AxeNative && m.criterion == criterion)
    }

    /// The probe (non-axe) mechanism of a criterion, if it has one.
    pub fn probe_for(&self, criterion: &str) -> Option<&Mechanism> {
        self.mechanisms
            .iter()
            .find(|m| m.kind != MechanismKind::AxeNative && m.criterion == criterion)
    }

    /// Whether a `pass` from the probe of this criterion may stand as a criterion-level
    /// `Pass`. Unknown criteria are partial: fail closed.
    pub fn probe_is_complete(&self, criterion: &str) -> bool {
        self.probe_for(criterion).is_some_and(|m| {
            m.coverage == AxeCoverage::Complete && m.outcomes.contains(&Outcome::Pass)
        })
    }

    /// Every violated invariant, as human-readable lines; empty when the registry is
    /// sound.
    ///
    /// * ids are unique;
    /// * `pass` in `outcomes` implies `coverage = complete`;
    /// * `legacy = false` implies fixtures present on disk under `fixtures_dir`: a
    ///   `-pass` and a `-fail` one, or such a pair per test when `complete`;
    /// * every cited axe rule is in `known_axe_rules`;
    /// * an `axe-native` mechanism cites at least one rule.
    pub fn check(&self, fixtures_dir: &Path, known_axe_rules: &HashSet<String>) -> Vec<String> {
        let mut problems = Vec::new();
        let mut seen = BTreeSet::new();
        for m in &self.mechanisms {
            if !seen.insert(m.id.as_str()) {
                problems.push(format!("{}: duplicate mechanism id", m.id));
            }
            if m.outcomes.contains(&Outcome::Pass) && m.coverage != AxeCoverage::Complete {
                problems.push(format!(
                    "{}: outcomes contain \"pass\" but coverage is not \"complete\"",
                    m.id
                ));
            }
            if m.kind == MechanismKind::AxeNative && m.axe_rules.is_empty() {
                problems.push(format!("{}: axe-native mechanism cites no axe rule", m.id));
            }
            for rule in &m.axe_rules {
                if !known_axe_rules.contains(rule) {
                    problems.push(format!("{}: unknown axe rule \"{rule}\"", m.id));
                }
            }
            if !m.legacy {
                problems.extend(Self::check_fixtures(m, fixtures_dir));
            }
        }
        problems
    }

    fn check_fixtures(m: &Mechanism, dir: &Path) -> Vec<String> {
        let mut problems = Vec::new();
        for name in &m.fixtures {
            if !dir.join(format!("{name}.html")).is_file() {
                problems.push(format!("{}: fixture file missing: {name}.html", m.id));
            }
        }
        let has = |prefix: &str, suffix: &str| {
            m.fixtures
                .iter()
                .any(|f| f.starts_with(prefix) && f.ends_with(suffix))
        };
        if m.coverage == AxeCoverage::Complete && !m.tests.is_empty() {
            // One pair per RGAA test; the test is carried by the slug (`11.6-t1-...`).
            for test in &m.tests {
                let n = test.rsplit('.').next().unwrap_or(test);
                let prefix = format!("{}-t{n}-", m.criterion);
                if !has(&prefix, "-pass") || !has(&prefix, "-fail") {
                    problems.push(format!(
                        "{}: complete mechanism needs a -pass and a -fail fixture for test {test} \
                         (named {prefix}<slug>-pass|fail)",
                        m.id
                    ));
                }
            }
        } else {
            let prefix = format!("{}-", m.criterion);
            if !has(&prefix, "-pass") || !has(&prefix, "-fail") {
                problems.push(format!(
                    "{}: non-legacy mechanism needs a -pass and a -fail fixture (named {prefix}<slug>-pass|fail)",
                    m.id
                ));
            }
        }
        problems
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn known(rules: &[&str]) -> HashSet<String> {
        rules.iter().map(|s| (*s).to_string()).collect()
    }

    fn entry(extra: &str) -> String {
        format!(
            "[[mechanism]]\nid=\"probe-11-6-x\"\ncriterion=\"11.6\"\nkind=\"js-static\"\n\
             engine=\"obscura\"\n{extra}\n"
        )
    }

    fn check(toml: &str, dir: &Path) -> Vec<String> {
        MechanismRegistry::from_toml_str(toml)
            .expect("test registry parses")
            .check(dir, &known(&["image-alt"]))
    }

    fn tmp_dir(tag: &str, files: &[&str]) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("rgaa-registry-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("tmp dir");
        for f in files {
            std::fs::write(dir.join(format!("{f}.html")), "<html></html>").expect("fixture");
        }
        dir
    }

    #[test]
    fn builtin_registry_parses() {
        assert!(!MechanismRegistry::builtin().mechanisms().is_empty());
    }

    #[test]
    fn pass_outcome_requires_complete_coverage() {
        let t =
            entry("coverage=\"partial\"\noutcomes=[\"fail\",\"pass\"]\nlegacy=true\nfixtures=[]");
        let problems = check(&t, Path::new("."));
        assert!(
            problems.iter().any(|p| p.contains("\"pass\"")),
            "{problems:?}"
        );
    }

    #[test]
    fn non_legacy_mechanism_without_fixtures_fails() {
        let t = entry("coverage=\"partial\"\noutcomes=[\"fail\"]\nlegacy=false\nfixtures=[]");
        let problems = check(&t, Path::new("."));
        assert!(
            problems
                .iter()
                .any(|p| p.contains("needs a -pass and a -fail")),
            "{problems:?}"
        );
    }

    #[test]
    fn non_legacy_fixture_names_must_exist_on_disk() {
        let dir = tmp_dir("missing", &["11.6-x-pass"]);
        let t = entry(
            "coverage=\"partial\"\noutcomes=[\"fail\"]\nlegacy=false\n\
             fixtures=[\"11.6-x-pass\",\"11.6-x-fail\"]",
        );
        let problems = check(&t, &dir);
        assert_eq!(problems.len(), 1, "{problems:?}");
        assert!(problems[0].contains("11.6-x-fail.html"));
    }

    #[test]
    fn non_legacy_pair_on_disk_is_accepted() {
        let dir = tmp_dir("pair", &["11.6-x-pass", "11.6-x-fail"]);
        let t = entry(
            "coverage=\"partial\"\noutcomes=[\"fail\"]\nlegacy=false\n\
             fixtures=[\"11.6-x-pass\",\"11.6-x-fail\"]",
        );
        assert!(check(&t, &dir).is_empty());
    }

    #[test]
    fn complete_mechanism_needs_one_pair_per_test() {
        let dir = tmp_dir("complete", &["11.6-t1-x-pass", "11.6-t1-x-fail"]);
        let t = entry(
            "coverage=\"complete\"\ntests=[\"11.6.1\",\"11.6.2\"]\noutcomes=[\"fail\",\"pass\"]\n\
             legacy=false\nfixtures=[\"11.6-t1-x-pass\",\"11.6-t1-x-fail\"]",
        );
        let problems = check(&t, &dir);
        assert_eq!(problems.len(), 1, "{problems:?}");
        assert!(problems[0].contains("test 11.6.2"));
    }

    #[test]
    fn unknown_axe_rule_is_reported() {
        let t = "[[mechanism]]\nid=\"axe-1-1\"\ncriterion=\"1.1\"\nkind=\"axe-native\"\n\
                 engine=\"obscura\"\ncoverage=\"partial\"\noutcomes=[\"fail\"]\nlegacy=true\n\
                 axe_rules=[\"image-alt\",\"no-such-rule\"]\n";
        let problems = check(t, Path::new("."));
        assert_eq!(problems.len(), 1, "{problems:?}");
        assert!(problems[0].contains("no-such-rule"));
    }

    #[test]
    fn duplicate_ids_are_reported() {
        let one = entry("coverage=\"partial\"\noutcomes=[\"fail\"]\nlegacy=true");
        let problems = check(&format!("{one}\n{one}"), Path::new("."));
        assert!(
            problems.iter().any(|p| p.contains("duplicate")),
            "{problems:?}"
        );
    }

    #[test]
    fn unknown_criterion_probe_is_not_complete() {
        assert!(!MechanismRegistry::builtin().probe_is_complete("99.99"));
    }
}
