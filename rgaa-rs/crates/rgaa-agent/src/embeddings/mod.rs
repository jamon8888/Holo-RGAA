pub mod fastembed;

pub use fastembed::FastEmbedModel;

use rig_core::embeddings::{Embedding, EmbeddingError, EmbeddingModel};
use rig_core::wasm_compat::WasmCompatSend;

/// Concrete embedding backend implementations usable by the provider.
#[derive(Clone)]
pub enum EmbeddingBackend {
    /// On-device embeddings produced by `fastembed`.
    FastEmbed(FastEmbedModel),
}

/// Embedding provider that produces vectors for memory and vector retrieval.
///
/// Currently backed by a single [`EmbeddingBackend`]; the `fallback` slot was
/// removed until a secondary backend is implemented.
#[derive(Clone)]
pub struct HybridEmbeddingProvider {
    primary: EmbeddingBackend,
}

impl HybridEmbeddingProvider {
    /// Builds the provider from agent configuration.
    ///
    /// # Errors
    /// Returns [`crate::error::AgentError::Embedding`] if the configured
    /// backend cannot be initialized, or if `embedding_dimensions` in config
    /// does not match the resolved model width.
    pub fn new(config: &crate::config::AgentConfig) -> Result<Self, crate::error::AgentError> {
        let primary = match &config.embedding_backend {
            crate::config::EmbeddingBackendConfig::FastEmbed { model_name } => {
                // Validate model name and dimensions before construction
                let (_, model_dims) = fastembed::resolve_model(model_name)?;
                if config.embedding_dimensions != model_dims {
                    return Err(crate::error::AgentError::Embedding(format!(
                        "embedding_dimensions {} does not match model {} (expected {})",
                        config.embedding_dimensions, model_name, model_dims
                    )));
                }
                EmbeddingBackend::FastEmbed(FastEmbedModel::new(model_name)?)
            }
        };

        Ok(Self { primary })
    }

    /// Returns the embedding dimensionality of the active backend.
    pub fn dimensions(&self) -> usize {
        match &self.primary {
            EmbeddingBackend::FastEmbed(m) => m.dimensions(),
        }
    }
}

impl EmbeddingModel for HybridEmbeddingProvider {
    const MAX_DOCUMENTS: usize = 256;
    type Client = ();

    fn make(_client: &(), _model: impl Into<String>, _dims: Option<usize>) -> Self {
        panic!("HybridEmbeddingProvider must be built via `new`")
    }

    fn ndims(&self) -> usize {
        self.dimensions()
    }

    fn embed_texts(
        &self,
        documents: impl IntoIterator<Item = String> + WasmCompatSend,
    ) -> impl std::future::Future<Output = Result<Vec<Embedding>, EmbeddingError>> {
        match &self.primary {
            EmbeddingBackend::FastEmbed(m) => m.embed_texts(documents),
        }
    }
}
