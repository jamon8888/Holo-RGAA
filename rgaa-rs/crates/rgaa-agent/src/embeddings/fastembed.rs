use std::sync::{Arc, Mutex};

use fastembed::{TextEmbedding, TextInitOptions};
use rig_core::embeddings::{Embedding, EmbeddingError, EmbeddingModel as RigEmbeddingModel};
use rig_core::wasm_compat::WasmCompatSend;

use crate::error::AgentError;

/// Map a `fastembed` model name to its variant and known embedding width.
///
/// The default model (`all-MiniLM-L6-v2`) is unchanged between fastembed 4 and
/// 7, so the stored vector width (384) and the model served to existing
/// LanceDB indexes are unchanged by this upgrade.
pub fn resolve_model(model_name: &str) -> Result<(fastembed::EmbeddingModel, usize), AgentError> {
    match model_name {
        "all-MiniLM-L6-v2" => Ok((fastembed::EmbeddingModel::AllMiniLML6V2, 384)),
        other => Err(AgentError::Embedding(format!(
            "unsupported embedding model: {other}"
        ))),
    }
}

/// On-device embedding model wrapper around `fastembed`'s `TextEmbedding`.
///
/// `TextEmbedding::embed` takes `&mut self` in fastembed 7, so the model is
/// shared behind a `Mutex` instead of a plain `Arc`.
#[derive(Clone)]
pub struct FastEmbedModel {
    model: Arc<Mutex<TextEmbedding>>,
    dimensions: usize,
}

impl FastEmbedModel {
    /// Loads the requested embedding model and reports its dimensionality.
    ///
    /// # Errors
    /// Returns [`AgentError::Embedding`] if the model is unknown or fails to
    /// initialize (e.g. model download or backend startup failure).
    pub fn new(model_name: &str) -> Result<Self, AgentError> {
        let (variant, dims) = resolve_model(model_name)?;
        let model = TextEmbedding::try_new(TextInitOptions::new(variant))
            .map_err(|e| AgentError::Embedding(format!("failed to init fastembed: {e}")))?;
        Ok(Self {
            model: Arc::new(Mutex::new(model)),
            dimensions: dims,
        })
    }

    /// Embedding width produced by this model.
    pub fn dimensions(&self) -> usize {
        self.dimensions
    }
}

impl RigEmbeddingModel for FastEmbedModel {
    const MAX_DOCUMENTS: usize = 256;
    type Client = ();

    fn make(_client: &(), model: impl Into<String>, dims: Option<usize>) -> Self {
        let model_name: String = model.into();
        let (variant, model_dims) =
            resolve_model(&model_name).expect("unsupported embedding model in make");
        if let Some(requested) = dims {
            if requested != model_dims {
                panic!(
                    "requested embedding dimensions {} mismatch resolved model {}",
                    requested, model_dims
                );
            }
        }
        let model = TextEmbedding::try_new(TextInitOptions::new(variant))
            .expect("failed to init fastembed in make");
        Self {
            model: Arc::new(Mutex::new(model)),
            dimensions: model_dims,
        }
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
            let (docs, vectors) = tokio::task::spawn_blocking(move || {
                let mut guard = model.lock().map_err(|e| {
                    EmbeddingError::ResponseError(format!("fastembed lock poisoned: {e}"))
                })?;
                let vectors = guard.embed(docs.clone(), None).map_err(|e| {
                    EmbeddingError::ResponseError(format!("fastembed embed failed: {e}"))
                })?;
                Ok::<_, EmbeddingError>((docs, vectors))
            })
            .await
            .map_err(|e| EmbeddingError::ResponseError(format!("embed task panicked: {e}")))?
            .map_err(|e| EmbeddingError::ResponseError(format!("fastembed embed failed: {e}")))?;

            let embeddings = docs
                .into_iter()
                .zip(vectors)
                .map(|(document, vec)| Embedding {
                    document,
                    vec: vec.into_iter().map(|v| v as f64).collect(),
                })
                .collect();

            Ok(embeddings)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_minilm_l6_v2_resolves_to_384_dims() {
        // Regression guard: the default model and its width must not change
        // across the fastembed 4 → 7 upgrade, else existing LanceDB vectors
        // and the configured `embedding_dimensions` would silently mismatch.
        let (variant, dims) = resolve_model("all-MiniLM-L6-v2").unwrap();
        assert_eq!(variant, fastembed::EmbeddingModel::AllMiniLML6V2);
        assert_eq!(dims, 384);
    }
}
