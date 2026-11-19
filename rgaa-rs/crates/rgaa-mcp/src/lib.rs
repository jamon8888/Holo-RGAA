//! Three-tool MCP facade for the local RGAA engine.

pub mod license;
pub mod server;
pub mod tools;

pub use license::{
    capture_license_outcome, denial_error_data, tool_license, upgrade_url_from_env, LicenseOutcome,
    ToolLicense, UPGRADE_URL_ENV,
};
pub use server::*;
pub use tools::*;
