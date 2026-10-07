pub mod audit_bundle;
pub mod catalog;
pub mod checkpoints;
pub mod citations;
pub mod completion;
pub mod criteria;
pub mod engine_plan;
pub mod error;
pub mod evidence;
pub mod findings;
pub mod na_detection;
pub mod provider;
pub mod registry;
pub mod types;

pub use audit_bundle::*;
pub use catalog::RgaaCatalog;
pub use checkpoints::*;
pub use citations::Citation;
pub use completion::{
    CompletionParams, LlmProvenance, ResponseFormat, DEFAULT_MAX_TOKENS, DEFAULT_TEMPERATURE,
};
pub use criteria::{Criterion, RgaaCriteria};
pub use engine_plan::{EnginePlan, EnginePlanEntry, PlanEngine};
pub use error::{Result, RgaaError};
pub use evidence::*;
pub use findings::*;
pub use provider::{provider, LlmSettings, Provider, PROVIDERS};
pub use registry::{Mechanism, MechanismKind, MechanismRegistry};
pub use types::{
    is_deterministic_source, reduce_test_outcomes, AuditResult, AutomatedVerdict, Classification,
    ConformityStatus, CrawlConfig, CriterionResult, CriterionStatus, PageResult, ReviewEvent,
    TestOutcome, VerdictBasis, Violation,
};
