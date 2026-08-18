pub mod analyze;
pub mod igt;
pub mod remediate;

pub use analyze::*;
pub use igt::*;
pub use remediate::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    InvalidInput,
    ExecutionFailed,
    EmptyResult,
}

impl ErrorCode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InvalidInput => "INVALID_INPUT",
            Self::ExecutionFailed => "EXECUTION_FAILED",
            Self::EmptyResult => "EMPTY_RESULT",
        }
    }
}
