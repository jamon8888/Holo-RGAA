//! Three-tool MCP facade for the local RGAA engine.

pub mod server;
pub mod tools;

pub use server::{
    AnalyzeRequest, GuidedTestRequest, ObscuraAnalyzeService, ObscuraGuidedService,
    RemediationRequest, RemediationServiceImpl, ToolServer,
};
pub use tools::{AnalyzeConfigInput, CookieReferenceInput, ErrorCode, ScreenshotPolicyInput};
