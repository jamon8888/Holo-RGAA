//! RGAA CLI library for accessibility auditing.

pub mod commands;
pub mod config;
pub mod license;
pub mod monitoring;
pub mod sitemap;

pub use config::{Config, ConfigError};
pub use rgaa_report::ReportFormat;

/// Moved to [`rgaa_report`]: kept so downstream imports keep compiling.
#[deprecated(note = "import from rgaa_report instead")]
pub use rgaa_report::format;
/// Moved to [`rgaa_report`]: kept so downstream imports keep compiling.
#[deprecated(note = "import from rgaa_report instead")]
pub use rgaa_report::report;

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
    /// Refused by the licence gate (tier, expiry, invalid licence). The
    /// message is already user-facing French copy from [`license::messages`].
    #[error("{0}")]
    License(String),
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
    /// - 4: Refused by the licence gate ([`license::LICENSE_EXIT_CODE`])
    pub fn exit_code(&self) -> i32 {
        match self {
            Self::PolicyFailure(_) => 1,
            Self::InvalidInput(_) => 2,
            Self::Execution(_) => 3,
            Self::License(_) => license::LICENSE_EXIT_CODE,
        }
    }
}
