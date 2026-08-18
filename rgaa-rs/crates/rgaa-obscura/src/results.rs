use rgaa_core::{EvidenceRef, Finding, PageError};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ObscuraError {
    #[error("failed to start obscura: {0}")]
    ProcessStartup(String),
    #[error("CDP transport failed: {0}")]
    CdpTransport(String),
    #[error("navigation failed: {0}")]
    Navigation(String),
    #[error("page evaluation failed: {0}")]
    Evaluation(String),
    #[error("invalid analysis request: {0}")]
    Validation(String),
    #[error("operation timed out: {0}")]
    Timeout(String),
    #[error("evidence capture failed: {0}")]
    Evidence(String),
    #[error("invalid JSON: {0}")]
    Json(String),
}

impl ObscuraError {
    pub fn page_error(&self) -> PageError {
        let code = match self {
            Self::Navigation(_) => "navigation",
            Self::Evaluation(_) | Self::Json(_) => "evaluation",
            Self::Timeout(_) => "timeout",
            Self::Evidence(_) => "evidence",
            Self::CdpTransport(_) => "cdp_transport",
            Self::ProcessStartup(_) => "process_startup",
            Self::Validation(_) => "validation",
        };
        PageError {
            code: code.into(),
            message: self.to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AnalyzePageResult {
    pub url: String,
    pub findings: Vec<Finding>,
    pub evidence: Vec<EvidenceRef>,
    pub errors: Vec<PageError>,
    pub completed: bool,
    pub duration_ms: u64,
}

impl AnalyzePageResult {
    pub fn failed(url: impl Into<String>, error: ObscuraError, duration_ms: u64) -> Self {
        Self {
            url: url.into(),
            findings: Vec::new(),
            evidence: Vec::new(),
            errors: vec![error.page_error()],
            completed: false,
            duration_ms,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failed_page_is_not_a_clean_result() {
        let result = AnalyzePageResult::failed(
            "https://bad.test",
            ObscuraError::Navigation("unreachable".into()),
            12,
        );
        assert!(!result.completed);
        assert!(result.findings.is_empty());
        assert_eq!(result.errors[0].code, "navigation");
        let json = serde_json::to_string(&result).expect("result serializes");
        assert!(json.contains("unreachable"));
    }
}
