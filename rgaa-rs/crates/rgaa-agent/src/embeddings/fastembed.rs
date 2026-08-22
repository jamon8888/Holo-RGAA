use rig_core::embeddings::{Embedding, EmbeddingError, EmbeddingModel};
use rig_core::wasm_compat::WasmCompatSend;
use fastembed::{EmbeddingModel as FastEmbedTrait, InitOptions, TextEmbedding};
use std::sync::Arc;

#[derive(Clone)]
pub struct FastEmbedModel {
    model: Arc<TextEmbedding>,
    dimensions: usize,
}

impl FastEmbedModel {
    pub fn new(_model_name: &str) -> Result<Self, crate::error::AgentError> {
        let model = TextEmbedding::try_new(InitOptions::new(FastEmbedTrait::AllMiniLML6V2))
            .map_err(|e| {
                crate::error::AgentError::Embedding(format!("Failed to initialize FastEmbed: {}", e))
            })?;
        Ok(Self {
            model: Arc::new(model),
            dimensions: 384,
        })
    }

    pub fn dimensions(&self) -> usize {
        self.dimensions
    }
}

impl EmbeddingModel for FastEmbedModel {
    const MAX_DOCUMENTS: usize = 256;
    type Client = ();

    fn make(_client: &(), _model: impl Into<String>, _dims: Option<usize>) -> Self {
        Self::new("all-MiniLM-L6-v2")
            .expect("failed to build default FastEmbed embedding model")
    }

    fn ndims(&self) -> usize {
        self.dimensions
    }

    fn embed_texts(
        &self,
        documents: impl IntoIterator<Item = String> + WasmCompatSend,
    ) -> impl std::future::Future<Output = Result<Vec<Embedding>, EmbeddingError>> {
        let docs: Vec<String> = documents.into_iter().collect();
        let model = self.model.clone();
        async move {
            let vectors = model
                .embed(docs.clone(), None)
                .map_err(|e| EmbeddingError::ResponseError(format!("fastembed embed failed: {}", e)))?;
            let embeddings = docs
                .into_iter()
                .zip(vectors.into_iter())
                .map(|(document, vec)| Embedding {
                    document,
                    vec: vec.into_iter().map(|v| v as f64).collect(),
                })
                .collect();
            Ok(embeddings)
        }
    }
}
