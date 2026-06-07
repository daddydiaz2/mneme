use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Motor de embeddings con modelo ONNX local.
pub struct EmbeddingEngine {
    model: Arc<Mutex<TextEmbedding>>,
    model_name: String,
    dimensions: usize,
    #[allow(dead_code)]
    cache_dir: std::path::PathBuf,
}

impl std::fmt::Debug for EmbeddingEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EmbeddingEngine")
            .field("model_name", &self.model_name)
            .field("dimensions", &self.dimensions)
            .field("cache_dir", &self.cache_dir)
            .finish_non_exhaustive()
    }
}

impl EmbeddingEngine {
    /// Inicializa el motor. Descarga el modelo si no esta en cache.
    /// El modelo se guarda en cache_dir/models/.
    pub async fn new(cache_dir: &std::path::Path) -> crate::error::Result<Self> {
        let model_cache = cache_dir.join("models");
        std::fs::create_dir_all(&model_cache)?;

        let model = tokio::task::spawn_blocking(move || {
            TextEmbedding::try_new(
                InitOptions::new(EmbeddingModel::BGESmallENV15)
                    .with_cache_dir(model_cache)
                    .with_show_download_progress(true),
            )
        })
        .await
        .map_err(|e| crate::error::MnemeError::Embeddings(e.to_string()))?
        .map_err(|e| crate::error::MnemeError::Embeddings(e.to_string()))?;

        Ok(Self {
            model: Arc::new(Mutex::new(model)),
            model_name: "BAAI/bge-small-en-v1.5".to_string(),
            dimensions: 384,
            cache_dir: cache_dir.to_path_buf(),
        })
    }

    /// Genera el embedding de un texto.
    pub async fn embed(&self, text: &str) -> crate::error::Result<Vec<f32>> {
        let text_owned = text.to_string();
        let model = self.model.clone();
        let embeddings = tokio::task::spawn_blocking(move || {
            let model_guard = model.blocking_lock();
            model_guard.embed(vec![&text_owned], None)
        })
        .await
        .map_err(|e| crate::error::MnemeError::Embeddings(e.to_string()))?
        .map_err(|e| crate::error::MnemeError::Embeddings(e.to_string()))?;
        Ok(embeddings.into_iter().next().unwrap_or_default())
    }

    /// Genera embeddings para un batch de textos.
    pub async fn embed_batch(&self, texts: &[String]) -> crate::error::Result<Vec<Vec<f32>>> {
        let texts_owned: Vec<String> = texts.to_vec();
        let model = self.model.clone();
        let embeddings = tokio::task::spawn_blocking(move || {
            let model_guard = model.blocking_lock();
            let refs: Vec<&str> = texts_owned.iter().map(|s| s.as_str()).collect();
            model_guard.embed(refs, None)
        })
        .await
        .map_err(|e| crate::error::MnemeError::Embeddings(e.to_string()))?
        .map_err(|e| crate::error::MnemeError::Embeddings(e.to_string()))?;
        Ok(embeddings)
    }

    /// Convierte una memoria a texto para embedding.
    pub fn memory_to_text(memory: &crate::store::memory::Memory) -> String {
        let mut parts = vec![memory.title.clone(), memory.content.clone()];
        if let Some(w) = &memory.what {
            parts.push(w.clone());
        }
        if let Some(w) = &memory.why {
            parts.push(w.clone());
        }
        if let Some(l) = &memory.learned {
            parts.push(l.clone());
        }
        parts.join(" . ")
    }

    /// Retorna el nombre del modelo.
    pub fn model_name(&self) -> &str {
        &self.model_name
    }

    /// Retorna las dimensiones del embedding.
    pub fn dimensions(&self) -> usize {
        self.dimensions
    }
}
