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
    #[error("unsupported analysis configuration: {0}")]
    UnsupportedConfiguration(String),
    #[error("analysis policy denied: {0}")]
    PolicyDenied(String),
}

impl ObscuraError {
    pub fn page_error(&self) -> PageError {
        let code = match self {
            Self::Navigation(_) => "navigation",
            Self::Evaluation(_) => "evaluation",
            Self::Timeout(_) => "timeout",
            Self::Evidence(_) => "evidence",
            Self::CdpTransport(_) => "cdp_transport",
            Self::ProcessStartup(_) => "process_startup",
            Self::Validation(_) => "validation",
            Self::Json(_) => "json",
            Self::UnsupportedConfiguration(_) => "unsupported_configuration",
            Self::PolicyDenied(_) => "policy_denied",
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

    pub fn is_clean_complete(&self) -> bool {
        self.completed && self.errors.is_empty() && !self.evidence.is_empty()
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

    #[test]
    fn all_typed_errors_keep_distinct_serialized_codes() {
        let errors = [
            (ObscuraError::ProcessStartup("x".into()), "process_startup"),
            (ObscuraError::CdpTransport("x".into()), "cdp_transport"),
            (ObscuraError::Navigation("x".into()), "navigation"),
            (ObscuraError::Json("x".into()), "json"),
            (ObscuraError::Evaluation("x".into()), "evaluation"),
            (ObscuraError::Timeout("x".into()), "timeout"),
            (ObscuraError::Evidence("x".into()), "evidence"),
            (
                ObscuraError::UnsupportedConfiguration("x".into()),
                "unsupported_configuration",
            ),
            (ObscuraError::PolicyDenied("x".into()), "policy_denied"),
        ];
        for (error, code) in errors {
            assert_eq!(error.page_error().code, code);
        }
    }

    #[test]
    fn clean_result_requires_evidence() {
        let result = AnalyzePageResult {
            url: "https://example.test".into(),
            findings: Vec::new(),
            evidence: Vec::new(),
            errors: Vec::new(),
            completed: true,
            duration_ms: 1,
        };
        assert!(!result.is_clean_complete());
    }
}
