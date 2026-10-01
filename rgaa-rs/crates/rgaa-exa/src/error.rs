use thiserror::Error;

/// Everything that can go wrong talking to the Exa API.
///
/// Converted into [`rgaa_core::RgaaError`] at the crate boundary so callers
/// inside the workspace keep a single error type (see the `From` impl below).
#[derive(Error, Debug)]
pub enum ExaError {
    /// `EXA_API_KEY` was absent or empty in the environment.
    #[error("missing EXA_API_KEY")]
    MissingApiKey,

    /// The HTTP client could not be built, or the request never completed.
    #[error("Exa transport error: {0}")]
    Transport(String),

    /// Exa answered with a non-2xx status.
    #[error("Exa API error (HTTP {status}): {body}")]
    Api { status: u16, body: String },

    /// Exa answered 429 and we ran out of retries.
    #[error("Exa rate limited after {attempts} attempts")]
    RateLimited { attempts: u32 },

    /// The payload did not match the expected response shape.
    #[error("Exa response decode error: {0}")]
    Decode(String),
}

impl From<reqwest::Error> for ExaError {
    fn from(e: reqwest::Error) -> Self {
        if e.is_decode() {
            ExaError::Decode(e.to_string())
        } else {
            ExaError::Transport(e.to_string())
        }
    }
}

impl From<ExaError> for rgaa_core::RgaaError {
    fn from(e: ExaError) -> Self {
        match e {
            ExaError::RateLimited { .. } => rgaa_core::RgaaError::RateLimited { retry_after: 0 },
            other => rgaa_core::RgaaError::Crawl(other.to_string()),
        }
    }
}

pub type Result<T> = std::result::Result<T, ExaError>;
