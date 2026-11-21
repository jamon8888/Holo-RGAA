use thiserror::Error;

#[derive(Error, Debug)]
pub enum RgaaError {
    #[error("LLM error: {message}")]
    Llm {
        message: String,
        code: Option<String>,
    },

    #[error("Rate limited: retry after {retry_after}s")]
    RateLimited { retry_after: u64 },

    #[error("Timeout after {duration}s")]
    Timeout { duration: u64 },

    #[error("Criterion not found: {0}")]
    CriterionNotFound(String),

    #[error("Crawl error: {0}")]
    Crawl(String),
    #[error("Browser error: {0}")]
    Browser(String),
    #[error("Axe-core error: {0}")]
    AxeCore(String),
    #[error("LLM API error: {0}")]
    Holo3(String),
    #[error("Media analysis error: {0}")]
    Media(String),
    #[error("Storage error: {0}")]
    Storage(String),
    #[error("Invalid criterion ID: {0}")]
    InvalidCriterion(String),
    #[error("missing required ID: {0}")]
    MissingId(String),
    #[error("missing required field: {0}")]
    MissingField(String),
    #[error("unsupported schema version: {0}")]
    UnsupportedSchemaVersion(String),
    #[error("duplicate finding ID: {0}")]
    DuplicateFindingId(String),
    #[error("invalid status: {0}")]
    InvalidStatus(String),
    #[error("incomplete evidence for {0}")]
    IncompleteEvidence(String),
    /// An analysis plugin failed, or could not produce a usable proof.
    ///
    /// `plugin` names the plugin (or sidecar) responsible so the orchestrator
    /// can put its tests in review « preuve manquante » without touching the
    /// status itself (R-P1).
    #[error("analysis plugin '{plugin}': {message}")]
    Analysis { plugin: String, message: String },
}

pub type Result<T> = std::result::Result<T, RgaaError>;
