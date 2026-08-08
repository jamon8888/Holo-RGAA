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
    #[error("Timeout after {0}ms")]
    Timeout(u64),
}

pub type Result<T> = std::result::Result<T, RgaaError>;
