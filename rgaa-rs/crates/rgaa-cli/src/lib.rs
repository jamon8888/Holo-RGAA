pub mod commands;
pub mod config;
pub mod format;
pub mod report;

pub use config::{Config, ConfigError};
pub use format::ReportFormat;

#[derive(Debug, thiserror::Error)]
pub enum CliError {
    #[error("{0}")]
    PolicyFailure(String),
    #[error("{0}")]
    InvalidInput(String),
    #[error("{0}")]
    Execution(String),
}

impl CliError {
    pub fn policy(message: impl Into<String>) -> Self {
        Self::PolicyFailure(message.into())
    }
    pub fn invalid_input(message: impl Into<String>) -> Self {
        Self::InvalidInput(message.into())
    }
    pub fn execution(message: impl Into<String>) -> Self {
        Self::Execution(message.into())
    }

    pub fn exit_code(&self) -> i32 {
        match self {
            Self::PolicyFailure(_) => 1,
            Self::InvalidInput(_) => 2,
            Self::Execution(_) => 3,
        }
    }
}
