//! RGAA CLI library for accessibility auditing.

pub mod commands;
pub mod config;
pub mod format;
pub mod report;

pub use config::{Config, ConfigError};
pub use format::ReportFormat;

/// Errors that can occur during CLI operations.
#[derive(Debug, thiserror::Error)]
pub enum CliError {
    /// Policy compliance check failed.
    #[error("{0}")]
    PolicyFailure(String),
    /// Invalid input provided by the user.
    #[error("{0}")]
    InvalidInput(String),
    /// Execution error (browser, network, file system).
    #[error("{0}")]
    Execution(String),
}

impl CliError {
    /// Creates a policy failure error.
    pub fn policy(message: impl Into<String>) -> Self {
        Self::PolicyFailure(message.into())
    }

    /// Creates an invalid input error.
    pub fn invalid_input(message: impl Into<String>) -> Self {
        Self::InvalidInput(message.into())
    }

    /// Creates an execution error.
    pub fn execution(message: impl Into<String>) -> Self {
        Self::Execution(message.into())
    }

    /// Returns the exit code for this error type.
    ///
    /// - 1: Policy failure
    /// - 2: Invalid input
    /// - 3: Execution error
    pub fn exit_code(&self) -> i32 {
        match self {
            Self::PolicyFailure(_) => 1,
            Self::InvalidInput(_) => 2,
            Self::Execution(_) => 3,
        }
    }
}
