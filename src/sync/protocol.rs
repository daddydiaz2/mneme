use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Mensaje de saludo inicial en sync.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncHello {
    pub peer_id: Uuid,
    pub peer_name: String,
    pub project: String,
    pub mneme_version: String,
    pub memory_count: u32,
    pub heads: HashMap<String, Vec<String>>,
}

/// Solicitud de sync.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncRequest {
    pub project: String,
    pub have: HashMap<String, Vec<String>>,
}

/// Respuesta de sync.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResponse {
    pub project: String,
    pub changes: Vec<MemoryChangeset>,
    pub tombstones: Vec<String>,
}

/// Cambio individual de una memoria.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryChangeset {
    pub automerge_id: String,
    pub payload: Vec<u8>,
    pub is_full_doc: bool,
}

/// Resultado de una operacion de sync.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResult {
    pub peer_name: String,
    pub direction: SyncDirection,
    pub memories_sent: u32,
    pub memories_received: u32,
    pub conflicts_resolved: u32,
    pub duration_ms: u64,
    pub status: SyncStatus,
    pub error: Option<String>,
}

/// Direccion del sync.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SyncDirection {
    Push,
    Pull,
    Bidirectional,
}

/// Estado del sync.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SyncStatus {
    Ok,
    Error,
    Partial,
}

/// Estadisticas de aplicacion de cambios.
#[derive(Debug, Clone, Default)]
pub struct ApplyStats {
    pub memories_applied: u32,
    pub conflicts_resolved: u32,
}

/// Estadisticas de exportacion.
#[derive(Debug, Clone, Default)]
pub struct ExportStats {
    pub memories_exported: u32,
    pub bytes_written: u64,
}
