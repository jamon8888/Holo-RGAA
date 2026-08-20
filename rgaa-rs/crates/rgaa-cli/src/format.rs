use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ReportFormat {
    #[default]
    Json,
    Markdown,
    Sarif,
    Junit,
}

impl ReportFormat {
    pub const ALL: [&'static str; 4] = ["json", "markdown", "sarif", "junit"];

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
