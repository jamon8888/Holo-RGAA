pub mod audit_bundle;
pub mod audit_type;
pub mod catalog;
pub mod checkpoints;
pub mod citations;
pub mod completion;
pub mod criteria;
pub mod engine_plan;
pub mod error;
pub mod evidence;
pub mod findings;
pub mod methodologies;
pub mod na_detection;
pub mod non_conformity;
pub mod provider;
pub mod registry;
pub mod test_plan;
pub mod types;

pub use audit_bundle::*;
pub use audit_type::AuditType;
pub use catalog::RgaaCatalog;
pub use checkpoints::*;
pub use citations::Citation;
pub use completion::{
    CompletionParams, LlmProvenance, ReasoningEffort, ResponseFormat, DEFAULT_MAX_TOKENS,
    DEFAULT_TEMPERATURE,
};
pub use criteria::{Criterion, RgaaCriteria};
pub use engine_plan::{EnginePlan, EnginePlanEntry, PlanEngine};
pub use error::{Result, RgaaError};
pub use evidence::*;
pub use findings::*;
pub use methodologies::{
    Methodology, RgaaMethodologies, METHODOLOGIES_COMMIT, METHODOLOGIES_SHA256,
};
pub use non_conformity::{
    compare_non_conformities, impact_rank, propose_impact_for_criterion, sort_non_conformities,
    ImpactReview, NonConformity, UserImpact,
};
pub use provider::{
    endpoint_host, is_agent_publisher_endpoint, provider, LlmMode, LlmSettings, ModeStatus,
    Provider, FORBIDDEN_AGENT_HOSTS, PROVIDERS,
};
pub use registry::{Mechanism, MechanismKind, MechanismRegistry};
pub use test_plan::{CoverageLevel, TestRoute, TestRoutePlan};
pub use types::{
    is_deterministic_source, reduce_test_outcomes, AuditResult, AutomatedVerdict, Classification,
    ConformityStatus, CrawlConfig, CriterionResult, CriterionStatus, PageResult, ReviewEvent,
    TestOutcome, VerdictBasis, Violation, TRANSVERSE_PAGE_URL, TRANSVERSE_REFERENCE_SOURCE,
};
