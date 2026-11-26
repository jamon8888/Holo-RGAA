pub mod analyze;
#[cfg(feature = "dev-explore")]
pub mod explore;
pub mod igt;
pub mod lint;
pub mod remediate;
pub mod source_map;
pub mod verify_fix;

pub use analyze::*;
#[cfg(feature = "dev-explore")]
pub use explore::*;
pub use igt::*;
pub use lint::*;
pub use remediate::*;
pub use source_map::*;
pub use verify_fix::*;

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
