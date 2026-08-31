use thiserror::Error;

#[derive(Debug, Error)]
pub enum SpiderError {
    #[error("spider crawl failed: {0}")]
    CrawlFailed(String),

    #[error("channel receive error")]
    ChannelError,

    #[error("invalid URL: {0}")]
    InvalidUrl(String),

    #[error("page fetch failed: {0}")]
    PageFetchFailed(String),
}
