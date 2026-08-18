use thiserror::Error;

#[derive(Error, Debug)]
pub enum RgaaError {
    #[error("Crawl error: {0}")]
    Crawl(String),
    #[error("Browser error: {0}")]
    Browser(String),
    #[error("Axe-core error: {0}")]
    AxeCore(String),
    #[error("Holo3 API error: {0}")]
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
    #[error("Timeout after {0}ms")]
    Timeout(u64),
}

pub type Result<T> = std::result::Result<T, RgaaError>;
