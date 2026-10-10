pub mod merge;
pub mod pipeline;
pub(crate) mod site_comparison;

pub use merge::{merge_candidates, merge_results};
pub use pipeline::{AuditEvent, AuditPhase, BatchObserver, Orchestrator};
