use rgaa_core::Classification;
use std::path::Path;

/// Whether a test page exercises a criterion the ordinary way, or is
/// specifically designed to probe evaluator robustness (ticket #131:
/// "cas adversariaux") — e.g. content that tries to manipulate an LLM
/// evaluator, or markup that looks compliant on casual inspection but
/// isn't semantically.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaseKind {
    Standard,
    Adversarial,
}

/// A test page for a specific RGAA criterion.
#[derive(Debug, Clone)]
pub struct TestPage {
    pub criterion_id: String,
    pub name: String,
    pub html_path: String,
    pub description: String,
    /// "Pass", "Fail", "NotApplicable", or "NotTested".
    pub expected_status: String,
    pub kind: CaseKind,
}

impl TestPage {
    /// This page's criterion's [`Classification`] from the RGAA catalog
    /// (`rgaa_core::RgaaCriteria`), when `criterion_id` matches a known
    /// criterion.
    pub fn classification(&self) -> Option<Classification> {
        rgaa_core::RgaaCriteria::all()
            .iter()
            .find(|c| c.id == self.criterion_id)
            .map(|c| c.classification)
    }
}

/// The test corpus containing HTML pages for RGAA criteria evaluation.
///
/// Filename convention: `{criterion_id}-{slug}-{status}[-adversarial].html`,
/// where `status` is `pass`, `fail`, or `na` (missing status defaults to
/// "NotTested"), and the optional `-adversarial` suffix sets
/// [`TestPage::kind`] to [`CaseKind::Adversarial`]. `status` and
/// `adversarial` are independent — an adversarial case can expect any
/// status.
pub struct TestCorpus {
    pages: Vec<TestPage>,
}

impl TestCorpus {
    /// Create a new test corpus
    pub fn new() -> Self {
        Self { pages: Vec::new() }
    }

    /// Load test pages from the criteria directory
    pub fn load(dir: &Path) -> Result<Self, String> {
        let mut corpus = Self::new();
        let entries = std::fs::read_dir(dir).map_err(|e| format!("Failed to read dir: {e}"))?;

        for entry in entries {
            let entry = entry.map_err(|e| format!("Failed to read entry: {e}"))?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("html") {
                continue;
            }

            let filename = path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string();

            let name = filename.trim_end_matches(".html").to_string();
            let parts: Vec<&str> = name.splitn(2, '-').collect();
            let criterion_id = parts.first().unwrap_or(&"unknown").to_string();

            let segments: Vec<&str> = name.split('-').collect();
            let expected_status = if segments.contains(&"na") {
                "NotApplicable".to_string()
            } else if segments.contains(&"pass") {
                "Pass".to_string()
            } else if segments.contains(&"fail") {
                "Fail".to_string()
            } else {
                "NotTested".to_string()
            };
            let kind = if segments.contains(&"adversarial") {
                CaseKind::Adversarial
            } else {
                CaseKind::Standard
            };

            corpus.pages.push(TestPage {
                criterion_id,
                name,
                html_path: path.to_string_lossy().to_string(),
                description: format!("Test page: {filename}"),
                expected_status,
                kind,
            });
        }

        Ok(corpus)
    }

    /// Get test pages for a specific criterion
    pub fn for_criterion(&self, criterion_id: &str) -> Vec<&TestPage> {
        self.pages
            .iter()
            .filter(|p| p.criterion_id == criterion_id)
            .collect()
    }

    /// Get all test pages
    pub fn all_pages(&self) -> &[TestPage] {
        &self.pages
    }

    /// Pages whose criterion is classified [`Classification::IaAssiste`] —
    /// the coverage ticket #131 requires beyond the deterministic
    /// axe-mappable criteria most of the original corpus exercises.
    pub fn ia_assiste_pages(&self) -> Vec<&TestPage> {
        self.pages
            .iter()
            .filter(|p| p.classification() == Some(Classification::IaAssiste))
            .collect()
    }

    /// Pages expected to be "not applicable" for their criterion.
    pub fn not_applicable_pages(&self) -> Vec<&TestPage> {
        self.pages
            .iter()
            .filter(|p| p.expected_status == "NotApplicable")
            .collect()
    }

    /// Pages specifically designed to probe evaluator robustness (see
    /// [`CaseKind::Adversarial`]).
    pub fn adversarial_pages(&self) -> Vec<&TestPage> {
        self.pages
            .iter()
            .filter(|p| p.kind == CaseKind::Adversarial)
            .collect()
    }
}

impl Default for TestCorpus {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load_corpus() -> TestCorpus {
        let criteria_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("criteria");
        TestCorpus::load(&criteria_dir).expect("load should succeed")
    }

    #[test]
    fn new_creates_empty_corpus() {
        let corpus = TestCorpus::new();
        assert!(corpus.all_pages().is_empty());
    }

    #[test]
    fn load_parses_html_files() {
        let criteria_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("criteria");
        if criteria_dir.exists() {
            let corpus = TestCorpus::load(&criteria_dir).expect("load should succeed");
            assert!(!corpus.all_pages().is_empty());
        }
    }

    // --- #131: coverage of IA-assistée, NA, and adversarial cases ---

    #[test]
    fn corpus_covers_at_least_two_ia_assiste_criteria_with_labels() {
        let corpus = load_corpus();
        let ia_pages = corpus.ia_assiste_pages();
        assert!(
            ia_pages.len() >= 2,
            "expected at least 2 IA-assistée pages, got {}",
            ia_pages.len()
        );
        for page in &ia_pages {
            assert_eq!(page.classification(), Some(Classification::IaAssiste));
        }
        assert!(
            ia_pages.iter().any(|p| p.criterion_id == "11.2"),
            "expected 11.2 (link purpose, IA-assistée) to be covered"
        );
    }

    #[test]
    fn corpus_covers_at_least_two_not_applicable_cases_with_labels() {
        let corpus = load_corpus();
        let na_pages = corpus.not_applicable_pages();
        assert!(
            na_pages.len() >= 2,
            "expected at least 2 NotApplicable pages, got {}",
            na_pages.len()
        );
        for page in &na_pages {
            assert_eq!(page.expected_status, "NotApplicable");
        }
    }

    #[test]
    fn corpus_covers_at_least_two_adversarial_cases_with_labels() {
        let corpus = load_corpus();
        let adversarial_pages = corpus.adversarial_pages();
        assert!(
            adversarial_pages.len() >= 2,
            "expected at least 2 adversarial pages, got {}",
            adversarial_pages.len()
        );
        for page in &adversarial_pages {
            assert_eq!(page.kind, CaseKind::Adversarial);
            // Every adversarial case still carries a definite expected
            // status — "adversarial" is a difficulty tag, not an excuse
            // to skip labeling the expected outcome.
            assert_ne!(page.expected_status, "NotTested");
        }
    }

    #[test]
    fn adversarial_and_standard_cases_are_independent_of_expected_status() {
        let corpus = load_corpus();
        // Both a Fail-adversarial and a Fail-standard case exist — kind
        // and status vary independently, not conflated.
        assert!(corpus
            .adversarial_pages()
            .iter()
            .any(|p| p.expected_status == "Fail"));
        assert!(corpus
            .all_pages()
            .iter()
            .any(|p| p.kind == CaseKind::Standard && p.expected_status == "Fail"));
    }

    #[test]
    fn na_page_status_never_collides_with_pass_or_fail_parsing() {
        // Filenames containing "na" as a full segment must not be
        // misparsed by a naive substring check against "pass"/"fail" (e.g.
        // "no-video-na" must not spuriously match anything else).
        let corpus = load_corpus();
        let page = corpus
            .all_pages()
            .iter()
            .find(|p| p.name == "4.7-no-video-na")
            .expect("4.7-no-video-na.html must be loaded");
        assert_eq!(page.expected_status, "NotApplicable");
    }
}
