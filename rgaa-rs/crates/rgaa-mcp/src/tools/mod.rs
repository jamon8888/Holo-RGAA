pub mod analyze;
pub mod igt;
pub mod remediate;

pub use analyze::*;
pub use igt::*;
pub use remediate::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    InvalidInput,
    PolicyDenied,
    UnsupportedConfiguration,
    ExecutionFailed,
    IncompleteResult,
    EmptyResult,
}

impl ErrorCode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InvalidInput => "INVALID_INPUT",
            Self::PolicyDenied => "POLICY_DENIED",
            Self::UnsupportedConfiguration => "UNSUPPORTED_CONFIGURATION",
            Self::ExecutionFailed => "EXECUTION_FAILED",
            Self::IncompleteResult => "INCOMPLETE_RESULT",
            Self::EmptyResult => "EMPTY_RESULT",
        }
    }
}
