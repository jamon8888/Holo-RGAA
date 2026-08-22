pub mod fastembed;

pub use fastembed::FastEmbedModel;

use rig_core::embeddings::{Embedding, EmbeddingError, EmbeddingModel};
use rig_core::wasm_compat::WasmCompatSend;

#[derive(Clone)]
pub enum EmbeddingBackend {
    FastEmbed(FastEmbedModel),
}

#[derive(Clone)]
pub struct HybridEmbeddingProvider {
    primary: EmbeddingBackend,
    fallback: Option<EmbeddingBackend>,
}

impl HybridEmbeddingProvider {
    pub fn new(config: &crate::config::AgentConfig) -> Result<Self, crate::error::AgentError> {
        let primary = match &config.embedding_backend {
            crate::config::EmbeddingBackendConfig::FastEmbed { model_name } => {
                EmbeddingBackend::FastEmbed(FastEmbedModel::new(model_name)?)
            }
            _ => {
                return Err(crate::error::AgentError::Config(
                    "unsupported embedding backend".into(),
                ))
            }
        };

        Ok(Self {
            primary,
            fallback: None,
        })
    }

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
