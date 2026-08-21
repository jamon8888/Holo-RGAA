use std::str::FromStr;

/// Supported output formats for audit reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ReportFormat {
    /// JSON format (default).
    #[default]
    Json,
    /// Markdown format.
    Markdown,
    /// SARIF 2.1.0 format (Static Analysis Results Interchange Format).
    Sarif,
    /// JUnit XML format.
    Junit,
}

impl ReportFormat {
    /// List of all supported format names.
    pub const ALL: [&'static str; 4] = ["json", "markdown", "sarif", "junit"];

    /// Returns the format as a static string.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Json => "json",
            Self::Markdown => "markdown",
            Self::Sarif => "sarif",
            Self::Junit => "junit",
        }
    }
}

impl FromStr for ReportFormat {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.to_ascii_lowercase().as_str() {
            "json" => Ok(Self::Json),
            "markdown" | "md" => Ok(Self::Markdown),
            "sarif" => Ok(Self::Sarif),
            "junit" | "xml" => Ok(Self::Junit),
            other => Err(format!("unsupported format '{other}'")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_all_supported_formats() {
        for format in ReportFormat::ALL {
            assert_eq!(format.parse::<ReportFormat>().unwrap().as_str(), format);
        }
        assert_eq!(
            "md".parse::<ReportFormat>().unwrap(),
            ReportFormat::Markdown
        );
        assert_eq!("XML".parse::<ReportFormat>().unwrap(), ReportFormat::Junit);
    }

    #[test]
    fn rejects_unknown_formats() {
        assert!("pdf".parse::<ReportFormat>().is_err());
    }
}
