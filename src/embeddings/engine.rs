use std::sync::Arc;

/// Motor de embeddings multi-proveedor.
/// Soporta: ONNX local (fastembed), OpenAI, Ollama, Google.
pub struct EmbeddingEngine {
    backend: EmbeddingBackend,
    model_name: String,
    dimensions: usize,
    cache_dir: std::path::PathBuf,
}

enum EmbeddingBackend {
    /// ONNX local via fastembed.
    Onnx {
        model: Arc<tokio::sync::Mutex<fastembed::TextEmbedding>>,
    },
    /// OpenAI-compatible API.
    OpenAI {
        client: reqwest::Client,
        api_key: String,
        model: String,
        base_url: String,
    },
    /// Ollama local server.
    Ollama {
        client: reqwest::Client,
        host: String,
        model: String,
    },
    /// Google Gemini API.
    Google {
        client: reqwest::Client,
        api_key: String,
        model: String,
    },
}

impl std::fmt::Debug for EmbeddingBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EmbeddingBackend::Onnx { .. } => f.debug_struct("Onnx").finish_non_exhaustive(),
            EmbeddingBackend::OpenAI { model, .. } => f.debug_struct("OpenAI").field("model", model).finish_non_exhaustive(),
            EmbeddingBackend::Ollama { model, .. } => f.debug_struct("Ollama").field("model", model).finish_non_exhaustive(),
            EmbeddingBackend::Google { model, .. } => f.debug_struct("Google").field("model", model).finish_non_exhaustive(),
        }
    }
}

impl std::fmt::Debug for EmbeddingEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EmbeddingEngine")
            .field("model_name", &self.model_name)
            .field("dimensions", &self.dimensions)
            .field("backend", &self.backend)
            .finish_non_exhaustive()
    }
}

impl EmbeddingEngine {
    /// Inicializa el motor con el proveedor y configuración especificados.
    pub async fn new(
        cache_dir: &std::path::Path,
        provider: &crate::config::settings::EmbeddingProvider,
        model: &str,
    ) -> crate::error::Result<Self> {
        match provider {
            crate::config::settings::EmbeddingProvider::Onnx => {
                Self::new_onnx(cache_dir, model).await
            }
            crate::config::settings::EmbeddingProvider::OpenAI => {
                let api_key = std::env::var("OPENAI_API_KEY")
                    .map_err(|_| crate::error::MnemeError::Config(
                        "OPENAI_API_KEY environment variable required for OpenAI embeddings".into()
                    ))?;
                Self::new_openai(model, &api_key)
            }
            crate::config::settings::EmbeddingProvider::Ollama => {
                let host = std::env::var("OLLAMA_HOST")
                    .unwrap_or_else(|_| "http://localhost:11434".to_string());
                Self::new_ollama(model, &host)
            }
            crate::config::settings::EmbeddingProvider::Google => {
                let api_key = std::env::var("GOOGLE_API_KEY")
                    .map_err(|_| crate::error::MnemeError::Config(
                        "GOOGLE_API_KEY environment variable required for Google embeddings".into()
                    ))?;
                Self::new_google(model, &api_key)
            }
        }
    }

    /// Inicializa con ONNX local (fastembed).
    async fn new_onnx(cache_dir: &std::path::Path, model: &str) -> crate::error::Result<Self> {
        let model_cache = cache_dir.join("models");
        std::fs::create_dir_all(&model_cache)?;

        let embedding_model = match model.to_lowercase().as_str() {
            "bge-small-en-v1.5" | "baai/bge-small-en-v1.5" => fastembed::EmbeddingModel::BGESmallENV15,
            "bge-base-en-v1.5" | "baai/bge-base-en-v1.5" => fastembed::EmbeddingModel::BGEBaseENV15,
            "bge-large-en-v1.5" | "baai/bge-large-en-v1.5" => fastembed::EmbeddingModel::BGELargeENV15,
            "all-minilm-l6-v2" | "sentence-transformers/all-minilm-l6-v2" => fastembed::EmbeddingModel::AllMiniLML6V2,
            _ => {
                tracing::warn!("Unknown ONNX model '{}', falling back to BGE-Small", model);
                fastembed::EmbeddingModel::BGESmallENV15
            }
        };

        let model_name = format!("{:?}", embedding_model);
        let dimensions = match embedding_model {
            fastembed::EmbeddingModel::BGESmallENV15 => 384,
            fastembed::EmbeddingModel::BGEBaseENV15 => 768,
            fastembed::EmbeddingModel::BGELargeENV15 => 1024,
            fastembed::EmbeddingModel::AllMiniLML6V2 => 384,
            _ => 384,
        };

        let ft_model = tokio::task::spawn_blocking(move || {
            fastembed::TextEmbedding::try_new(
                fastembed::InitOptions::new(embedding_model)
                    .with_cache_dir(model_cache)
                    .with_show_download_progress(false),
            )
        })
        .await
        .map_err(|e| crate::error::MnemeError::Embeddings(e.to_string()))?
        .map_err(|e| crate::error::MnemeError::Embeddings(e.to_string()))?;

        Ok(Self {
            backend: EmbeddingBackend::Onnx {
                model: Arc::new(tokio::sync::Mutex::new(ft_model)),
            },
            model_name,
            dimensions,
            cache_dir: cache_dir.to_path_buf(),
        })
    }

    /// Inicializa con OpenAI-compatible API.
    fn new_openai(model: &str, api_key: &str) -> crate::error::Result<Self> {
        let client = reqwest::Client::new();
        let (base_url, model_name) = if model.contains('/') {
            // Format: provider/model (e.g., "openai/text-embedding-3-small", "azure/...")
            let parts: Vec<&str> = model.splitn(2, '/').collect();
            match parts[0] {
                "azure" => (
                    std::env::var("AZURE_OPENAI_ENDPOINT")
                        .unwrap_or_else(|_| "https://api.openai.com/v1".to_string()),
                    parts[1].to_string(),
                ),
                _ => ("https://api.openai.com/v1".to_string(), model.to_string()),
            }
        } else {
            ("https://api.openai.com/v1".to_string(), model.to_string())
        };

        Ok(Self {
            backend: EmbeddingBackend::OpenAI {
                client,
                api_key: api_key.to_string(),
                model: model_name.clone(),
                base_url,
            },
            model_name,
            dimensions: 1536, // default for text-embedding-3-small
            cache_dir: std::path::PathBuf::new(),
        })
    }

    /// Inicializa con Ollama.
    fn new_ollama(model: &str, host: &str) -> crate::error::Result<Self> {
        Ok(Self {
            backend: EmbeddingBackend::Ollama {
                client: reqwest::Client::new(),
                host: host.to_string(),
                model: model.to_string(),
            },
            model_name: model.to_string(),
            dimensions: 768, // default for nomic-embed-text
            cache_dir: std::path::PathBuf::new(),
        })
    }

    /// Inicializa con Google Gemini.
    fn new_google(model: &str, api_key: &str) -> crate::error::Result<Self> {
        Ok(Self {
            backend: EmbeddingBackend::Google {
                client: reqwest::Client::new(),
                api_key: api_key.to_string(),
                model: model.to_string(),
            },
            model_name: model.to_string(),
            dimensions: 768, // default for embedding-001
            cache_dir: std::path::PathBuf::new(),
        })
    }

    /// Genera el embedding de un texto.
    pub async fn embed(&self, text: &str) -> crate::error::Result<Vec<f32>> {
        match &self.backend {
            EmbeddingBackend::Onnx { model } => {
                let text_owned = text.to_string();
                let model = model.clone();
                let embeddings = tokio::task::spawn_blocking(move || {
                    let model_guard = model.blocking_lock();
                    model_guard.embed(vec![&text_owned], None)
                })
                .await
                .map_err(|e| crate::error::MnemeError::Embeddings(e.to_string()))?
                .map_err(|e| crate::error::MnemeError::Embeddings(e.to_string()))?;
                Ok(embeddings.into_iter().next().unwrap_or_default())
            }
            EmbeddingBackend::OpenAI { client, api_key, model, base_url } => {
                let url = format!("{}/embeddings", base_url.trim_end_matches('/'));
                let resp = client
                    .post(&url)
                    .header("Authorization", format!("Bearer {}", api_key))
                    .json(&serde_json::json!({
                        "input": text,
                        "model": model,
                    }))
                    .send()
                    .await
                    .map_err(|e| crate::error::MnemeError::Http(e.to_string()))?;

                let body: serde_json::Value = resp
                    .json()
                    .await
                    .map_err(|e| crate::error::MnemeError::Http(e.to_string()))?;

                let embedding: Vec<f32> = body["data"][0]["embedding"]
                    .as_array()
                    .ok_or_else(|| crate::error::MnemeError::Embeddings("invalid OpenAI response".into()))?
                    .iter()
                    .map(|v| v.as_f64().unwrap_or(0.0) as f32)
                    .collect();

                Ok(embedding)
            }
            EmbeddingBackend::Ollama { client, host, model } => {
                let url = format!("{}/api/embeddings", host.trim_end_matches('/'));
                let resp = client
                    .post(&url)
                    .json(&serde_json::json!({
                        "model": model,
                        "prompt": text,
                    }))
                    .send()
                    .await
                    .map_err(|e| crate::error::MnemeError::Http(e.to_string()))?;

                let body: serde_json::Value = resp
                    .json()
                    .await
                    .map_err(|e| crate::error::MnemeError::Http(e.to_string()))?;

                let embedding: Vec<f32> = body["embedding"]
                    .as_array()
                    .ok_or_else(|| crate::error::MnemeError::Embeddings("invalid Ollama response".into()))?
                    .iter()
                    .map(|v| v.as_f64().unwrap_or(0.0) as f32)
                    .collect();

                Ok(embedding)
            }
            EmbeddingBackend::Google { client, api_key, model } => {
                let url = format!(
                    "https://generativelanguage.googleapis.com/v1/models/{}:embedContent?key={}",
                    model, api_key
                );
                let resp = client
                    .post(&url)
                    .json(&serde_json::json!({
                        "content": {
                            "parts": [{"text": text}]
                        }
                    }))
                    .send()
                    .await
                    .map_err(|e| crate::error::MnemeError::Http(e.to_string()))?;

                let body: serde_json::Value = resp
                    .json()
                    .await
                    .map_err(|e| crate::error::MnemeError::Http(e.to_string()))?;

                let embedding: Vec<f32> = body["embedding"]["values"]
                    .as_array()
                    .ok_or_else(|| crate::error::MnemeError::Embeddings("invalid Google response".into()))?
                    .iter()
                    .map(|v| v.as_f64().unwrap_or(0.0) as f32)
                    .collect();

                Ok(embedding)
            }
        }
    }

    /// Genera embeddings para un batch de textos.
    pub async fn embed_batch(&self, texts: &[String]) -> crate::error::Result<Vec<Vec<f32>>> {
        match &self.backend {
            EmbeddingBackend::Onnx { model } => {
                let texts_owned: Vec<String> = texts.to_vec();
                let model = model.clone();
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
            // Other providers: call embed() sequentially for each text
            _ => {
                let mut results = Vec::with_capacity(texts.len());
                for text in texts {
                    results.push(self.embed(text).await?);
                }
                Ok(results)
            }
        }
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
