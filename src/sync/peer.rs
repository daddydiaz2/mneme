use std::str::FromStr;
use std::sync::{Arc, Mutex};

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{MnemeError, Result};

/// Tipo de transporte para sincronizacion.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TransportType {
    Http,
    Ssh,
    File,
}

impl std::fmt::Display for TransportType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            TransportType::Http => "http",
            TransportType::Ssh => "ssh",
            TransportType::File => "file",
        };
        write!(f, "{}", s)
    }
}

impl std::str::FromStr for TransportType {
    type Err = MnemeError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "http" => Ok(TransportType::Http),
            "ssh" => Ok(TransportType::Ssh),
            "file" => Ok(TransportType::File),
            other => Err(MnemeError::UnsupportedTransport(other.to_string())),
        }
    }
}

/// Representa un peer de sincronizacion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Peer {
    /// ID unico del peer.
    pub id: Uuid,
    /// Nombre descriptivo del peer.
    pub name: String,
    /// Tipo de transporte (http, ssh, file).
    pub transport: TransportType,
    /// Direccion del peer (URL o ruta).
    pub address: String,
    /// Proyecto asociado.
    pub project: String,
    /// Fecha del ultimo sync.
    pub last_sync: Option<DateTime<Utc>>,
    /// Estado del ultimo sync.
    pub last_status: Option<String>,
    /// Habilitar auto-sync.
    pub auto_sync: bool,
    /// Fecha de creacion.
    pub created_at: DateTime<Utc>,
}

/// Store para operaciones de peers.
pub struct PeerStore {
    conn: Arc<Mutex<Connection>>,
}

impl PeerStore {
    /// Crea un nuevo PeerStore.
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }

    /// Agrega un nuevo peer.
    pub fn add(&self, peer: &Peer) -> Result<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| MnemeError::Config("mutex poisoned".into()))?;
        conn.execute(
            "INSERT INTO sync_peers (id, name, transport, address, project, last_sync, last_status, auto_sync, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
             ON CONFLICT(id) DO UPDATE SET
                 name = excluded.name,
                 transport = excluded.transport,
                 address = excluded.address,
                 project = excluded.project,
                 auto_sync = excluded.auto_sync",
            params![
                peer.id.to_string(),
                &peer.name,
                peer.transport.to_string(),
                &peer.address,
                &peer.project,
                peer.last_sync.map(|d| d.to_rfc3339()),
                peer.last_status.as_deref(),
                peer.auto_sync as i32,
                peer.created_at.to_rfc3339(),
            ],
        )?;
        tracing::info!("added peer: {} ({})", peer.id, peer.name);
        Ok(())
    }

    /// Elimina un peer por ID.
    pub fn remove(&self, id: Uuid) -> Result<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| MnemeError::Config("mutex poisoned".into()))?;
        let rows = conn.execute(
            "DELETE FROM sync_peers WHERE id = ?1",
            params![id.to_string()],
        )?;
        if rows == 0 {
            return Err(MnemeError::PeerNotFound(id));
        }
        tracing::info!("removed peer: {}", id);
        Ok(())
    }

    /// Lista todos los peers de un proyecto.
    pub fn list(&self, project: &str) -> Result<Vec<Peer>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| MnemeError::Config("mutex poisoned".into()))?;
        let mut stmt = conn.prepare(
            "SELECT id, name, transport, address, project, last_sync, last_status, auto_sync, created_at
             FROM sync_peers WHERE project = ?1 ORDER BY created_at DESC",
        )?;
        let rows = stmt.query_map(params![project], |row| {
            Ok(Peer {
                id: Uuid::parse_str(&row.get::<_, String>(0)?).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        0,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                })?,
                name: row.get(1)?,
                transport: TransportType::from_str(&row.get::<_, String>(2)?).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        2,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                })?,
                address: row.get(3)?,
                project: row.get(4)?,
                last_sync: row
                    .get::<_, Option<String>>(5)?
                    .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                    .map(|d| d.with_timezone(&Utc)),
                last_status: row.get(6)?,
                auto_sync: row.get::<_, i32>(7)? != 0,
                created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(8)?)
                    .map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            8,
                            rusqlite::types::Type::Text,
                            Box::new(e),
                        )
                    })?
                    .with_timezone(&Utc),
            })
        })?;

        let mut peers = Vec::new();
        for row in rows {
            peers.push(row?);
        }
        Ok(peers)
    }

    /// Obtiene un peer por ID.
    pub fn get(&self, id: Uuid) -> Result<Option<Peer>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| MnemeError::Config("mutex poisoned".into()))?;
        let mut stmt = conn.prepare(
            "SELECT id, name, transport, address, project, last_sync, last_status, auto_sync, created_at
             FROM sync_peers WHERE id = ?1",
        )?;
        let result = stmt.query_row(params![id.to_string()], |row| {
            Ok(Peer {
                id: Uuid::parse_str(&row.get::<_, String>(0)?).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        0,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                })?,
                name: row.get(1)?,
                transport: TransportType::from_str(&row.get::<_, String>(2)?).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        2,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                })?,
                address: row.get(3)?,
                project: row.get(4)?,
                last_sync: row
                    .get::<_, Option<String>>(5)?
                    .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                    .map(|d| d.with_timezone(&Utc)),
                last_status: row.get(6)?,
                auto_sync: row.get::<_, i32>(7)? != 0,
                created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(8)?)
                    .map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            8,
                            rusqlite::types::Type::Text,
                            Box::new(e),
                        )
                    })?
                    .with_timezone(&Utc),
            })
        });

        match result {
            Ok(peer) => Ok(Some(peer)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Actualiza el estado del ultimo sync de un peer.
    pub fn update_status(&self, id: Uuid, status: &str) -> Result<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| MnemeError::Config("mutex poisoned".into()))?;
        let rows = conn.execute(
            "UPDATE sync_peers SET last_sync = ?1, last_status = ?2 WHERE id = ?3",
            params![Utc::now().to_rfc3339(), status, id.to_string()],
        )?;
        if rows == 0 {
            return Err(MnemeError::PeerNotFound(id));
        }
        tracing::info!("updated peer status: {} -> {}", id, status);
        Ok(())
    }

    /// Registra el resultado de un sync en el log.
    pub fn record_sync(
        &self,
        result: &crate::sync::protocol::SyncResult,
        project: &str,
    ) -> Result<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| MnemeError::Config("mutex poisoned".into()))?;
        let id = Uuid::new_v4();
        let direction = match result.direction {
            crate::sync::protocol::SyncDirection::Push => "push",
            crate::sync::protocol::SyncDirection::Pull => "pull",
            crate::sync::protocol::SyncDirection::Bidirectional => "bidirectional",
        };
        let status = match result.status {
            crate::sync::protocol::SyncStatus::Ok => "ok",
            crate::sync::protocol::SyncStatus::Error => "error",
            crate::sync::protocol::SyncStatus::Partial => "partial",
        };
        conn.execute(
            "INSERT INTO sync_log (id, peer_id, direction, project, memories_sent, memories_received, conflicts_resolved, duration_ms, status, error, started_at, finished_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                id.to_string(),
                result.peer_name,
                direction,
                project,
                result.memories_sent as i32,
                result.memories_received as i32,
                result.conflicts_resolved as i32,
                result.duration_ms as i32,
                status,
                result.error.as_deref(),
                Utc::now().to_rfc3339(),
                Utc::now().to_rfc3339(),
            ],
        )?;
        Ok(())
    }
}
