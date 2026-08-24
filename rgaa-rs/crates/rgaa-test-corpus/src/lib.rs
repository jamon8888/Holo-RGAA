use std::path::Path;

/// A test page for a specific RGAA criterion
#[derive(Debug, Clone)]
pub struct TestPage {
    pub criterion_id: String,
    pub name: String,
    pub html_path: String,
    pub description: String,
    pub expected_status: String, // "Pass", "Fail", "NotTested"
}

/// The test corpus containing HTML pages for RGAA criteria evaluation
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

            let expected_status = if name.contains("-pass") {
                "Pass".to_string()
            } else if name.contains("-fail") {
                "Fail".to_string()
            } else {
                "NotTested".to_string()
            };

            corpus.pages.push(TestPage {
                criterion_id,
                name,
                html_path: path.to_string_lossy().to_string(),
                description: format!("Test page: {filename}"),
                expected_status,
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
}

impl Default for TestCorpus {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
