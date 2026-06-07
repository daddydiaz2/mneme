#[cfg(feature = "embeddings")]
pub mod engine;
#[cfg(feature = "embeddings")]
pub mod store;
#[cfg(feature = "embeddings")]
pub mod similarity;

// Stubs para compilar sin el feature embeddings
#[cfg(not(feature = "embeddings"))]
pub mod engine {
    use std::path::Path;

    /// Stub de EmbeddingEngine para compilacion sin feature embeddings.
    pub struct EmbeddingEngine;

    impl std::fmt::Debug for EmbeddingEngine {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("EmbeddingEngine")
                .field("model_name", &"disabled")
                .field("dimensions", &0)
                .finish_non_exhaustive()
        }
    }

    impl EmbeddingEngine {
        /// Inicializa el motor (stub — retorna error).
        pub async fn new(_cache_dir: &Path) -> crate::error::Result<Self> {
            Err(crate::error::MnemeError::EmbeddingsDisabled)
        }

        /// Genera el embedding de un texto (stub — retorna error).
        pub async fn embed(&self, _text: &str) -> crate::error::Result<Vec<f32>> {
            Err(crate::error::MnemeError::EmbeddingsDisabled)
        }

        /// Genera embeddings para un batch de textos (stub — retorna error).
        pub async fn embed_batch(
            &self,
            _texts: &[String],
        ) -> crate::error::Result<Vec<Vec<f32>>> {
            Err(crate::error::MnemeError::EmbeddingsDisabled)
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
            "disabled"
        }

        /// Retorna las dimensiones del embedding.
        pub fn dimensions(&self) -> usize {
            0
        }
    }
}

#[cfg(not(feature = "embeddings"))]
pub mod store {
    use std::sync::{Arc, Mutex};
    use rusqlite::Connection;
    use uuid::Uuid;

    /// Stub de EmbeddingStore para compilacion sin feature embeddings.
    #[derive(Clone)]
    pub struct EmbeddingStore {
        _conn: Arc<Mutex<Connection>>,
    }

    impl EmbeddingStore {
        /// Crea un nuevo EmbeddingStore.
        pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
            Self { _conn: conn }
        }

        /// Guarda embedding de una memoria (stub — retorna error).
        pub fn save(
            &self,
            _memory_id: Uuid,
            _embedding: &[f32],
            _model_name: &str,
        ) -> crate::error::Result<()> {
            Err(crate::error::MnemeError::EmbeddingsDisabled)
        }

        /// Carga embedding de una memoria (stub — retorna None).
        pub fn load(&self, _memory_id: Uuid) -> crate::error::Result<Option<Vec<f32>>> {
            Ok(None)
        }

        /// Carga todos los embeddings de un proyecto (stub — retorna vacio).
        pub fn load_all_for_project(
            &self,
            _project: &str,
        ) -> crate::error::Result<Vec<(Uuid, Vec<f32>)>> {
            Ok(Vec::new())
        }

        /// Elimina el embedding de una memoria (stub — retorna error).
        pub fn delete(&self, _memory_id: Uuid) -> crate::error::Result<()> {
            Err(crate::error::MnemeError::EmbeddingsDisabled)
        }

        /// Lista IDs de memorias sin embedding (stub — retorna vacio).
        pub fn find_unindexed(&self, _project: &str) -> crate::error::Result<Vec<Uuid>> {
            Ok(Vec::new())
        }

        /// Serializa un vector de f32 a bytes little-endian.
        pub fn serialize(v: &[f32]) -> Vec<u8> {
            v.iter().flat_map(|f| f.to_le_bytes()).collect()
        }

        /// Deserializa bytes little-endian a un vector de f32.
        pub fn deserialize(bytes: &[u8]) -> Vec<f32> {
            bytes
                .chunks_exact(4)
                .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
                .collect()
        }
    }
}

#[cfg(not(feature = "embeddings"))]
pub mod similarity {
    use uuid::Uuid;

    /// Resultado de coincidencia semantica.
    #[derive(Debug, Clone)]
    pub struct SemanticMatch {
        /// ID de la memoria.
        pub memory_id: Uuid,
        /// Score de similitud coseno.
        pub cosine_score: f32,
        /// Score combinado (coseno * boost * decaimiento).
        pub combined_score: f64,
    }

    /// Calcula similitud coseno entre dos vectores f32 (stub — retorna 0.0).
    pub fn cosine_similarity(_a: &[f32], _b: &[f32]) -> f32 {
        0.0
    }

    /// Ordena coincidencias semanticas por score combinado descendente.
    pub fn rank_by_combined_score(matches: &mut [SemanticMatch]) {
        matches.sort_by(|a, b| {
            b.combined_score
                .partial_cmp(&a.combined_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    }
}
