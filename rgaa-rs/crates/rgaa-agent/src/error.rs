use thiserror::Error;

#[derive(Debug, Error)]
pub enum AgentError {
    #[error("rig agent error: {0}")]
    RigAgent(String),

    #[error("lancedb error: {0}")]
    LanceDb(String),

    #[error("embedding error: {0}")]
    Embedding(String),

    #[error("memory error: {0}")]
    Memory(String),

    #[error("tool execution error: {0}")]
    ToolExecution(String),

    #[error("configuration error: {0}")]
    Config(String),

    #[error("holo3 api error: {0}")]
    Holo3Api(String),
}