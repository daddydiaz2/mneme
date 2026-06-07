use std::sync::{Arc, Mutex};

use rusqlite::Connection;
use uuid::Uuid;

/// Store para operaciones de embeddings en SQLite.
#[derive(Clone)]
pub struct EmbeddingStore {
    conn: Arc<Mutex<Connection>>,
}

impl EmbeddingStore {
    /// Crea un nuevo EmbeddingStore.
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }

    /// Guarda embedding de una memoria. Serializa Vec<f32> a BLOB little-endian.
    pub fn save(
        &self,
        memory_id: Uuid,
        embedding: &[f32],
        model_name: &str,
    ) -> crate::error::Result<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| crate::error::MnemeError::Config("mutex poisoned".into()))?;
        let bytes = Self::serialize(embedding);
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT OR REPLACE INTO memory_embeddings (memory_id, embedding, model_name, dimensions, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![
                memory_id.to_string(),
                bytes,
                model_name,
                embedding.len() as i32,
                now
            ],
        )?;
        Ok(())
    }

    /// Carga embedding de una memoria. Retorna None si no tiene.
    pub fn load(&self, memory_id: Uuid) -> crate::error::Result<Option<Vec<f32>>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| crate::error::MnemeError::Config("mutex poisoned".into()))?;
        let result: Result<Vec<u8>, _> = conn.query_row(
            "SELECT embedding FROM memory_embeddings WHERE memory_id = ?1",
            rusqlite::params![memory_id.to_string()],
            |row| row.get(0),
        );
        match result {
            Ok(bytes) => Ok(Some(Self::deserialize(&bytes))),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Carga todos los embeddings de un proyecto.
    pub fn load_all_for_project(
        &self,
        project: &str,
    ) -> crate::error::Result<Vec<(Uuid, Vec<f32>)>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| crate::error::MnemeError::Config("mutex poisoned".into()))?;
        let mut stmt = conn.prepare(
            "SELECT e.memory_id, e.embedding
             FROM memory_embeddings e
             JOIN memories m ON e.memory_id = m.id
             WHERE m.project = ?1 AND m.deleted_at IS NULL",
        )?;
        let rows = stmt.query_map(rusqlite::params![project], |row| {
            let id_str: String = row.get(0)?;
            let bytes: Vec<u8> = row.get(1)?;
            let id = Uuid::parse_str(&id_str).map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    0,
                    rusqlite::types::Type::Text,
                    Box::new(e),
                )
            })?;
            Ok((id, Self::deserialize(&bytes)))
        })?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        Ok(results)
    }

    /// Elimina el embedding de una memoria.
    pub fn delete(&self, memory_id: Uuid) -> crate::error::Result<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| crate::error::MnemeError::Config("mutex poisoned".into()))?;
        conn.execute(
            "DELETE FROM memory_embeddings WHERE memory_id = ?1",
            rusqlite::params![memory_id.to_string()],
        )?;
        Ok(())
    }

    /// Lista IDs de memorias sin embedding (para reindexacion).
    pub fn find_unindexed(&self, project: &str) -> crate::error::Result<Vec<Uuid>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| crate::error::MnemeError::Config("mutex poisoned".into()))?;
        let mut stmt = conn.prepare(
            "SELECT m.id FROM memories m
             LEFT JOIN memory_embeddings e ON m.id = e.memory_id
             WHERE m.project = ?1 AND m.deleted_at IS NULL AND e.memory_id IS NULL",
        )?;
        let rows = stmt.query_map(rusqlite::params![project], |row| {
            let id_str: String = row.get(0)?;
            Uuid::parse_str(&id_str).map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    0,
                    rusqlite::types::Type::Text,
                    Box::new(e),
                )
            })
        })?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        Ok(results)
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
