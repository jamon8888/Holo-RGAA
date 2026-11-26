//! Dev-only exploration hints driven by agent-browser and the pinned Obscura
//! binary.
//!
//! This crate is intentionally not a dependency of any audit crate. Its output
//! is labeled `exploration_hints` and carries no verdict, status or evidence
//! fields; anything found here must be reproduced through `ObscuraBridge`
//! before it can affect an RGAA result.

pub mod pin;
pub mod runner;
pub mod types;

pub use runner::{run, ExploreError};
pub use types::{
    Candidate, EngineInfo, ExplorationResult, ExploreOptions, ObservationKind, Observations,
    RawText, ScreenshotObservation, SessionInfo, SnapshotObservation, TargetInfo,
};

/// Convenience wrapper around [`run`].
///
/// # Errors
///
/// See [`ExploreError`].
pub async fn explore(options: ExploreOptions) -> Result<ExplorationResult, ExploreError> {
    run(options).await
}
