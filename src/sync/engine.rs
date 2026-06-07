use std::collections::HashMap;
use std::sync::Arc;

use chrono::Utc;
use rusqlite::params;
use uuid::Uuid;

use crate::config::settings::SyncConfig;
use crate::error::{MnemeError, Result};
use crate::store::db::Database;
use crate::sync::crdt;
use crate::sync::peer::{Peer, TransportType};
use crate::sync::protocol::{
    ApplyStats, ExportStats, MemoryChangeset, SyncDirection, SyncHello, SyncRequest, SyncResponse,
    SyncResult, SyncStatus,
};
use crate::sync::transport::file::FileTransport;
use crate::sync::transport::http::HttpTransport;

/// Motor de sincronizacion CRDT.
pub struct SyncEngine {
    db: Arc<Database>,
    config: SyncConfig,
}

impl SyncEngine {
    /// Crea un nuevo SyncEngine.
    pub fn new(db: Arc<Database>, config: SyncConfig) -> Result<Self> {
        if !config.enabled {
            return Err(MnemeError::SyncDisabled);
        }
        Ok(Self { db, config })
    }

    /// Sincroniza con un peer especifico.
    pub async fn sync_with_peer(&self, peer: &Peer) -> Result<SyncResult> {
        let start = std::time::Instant::now();
        let project = peer.project.clone();

        let result = match peer.transport {
            TransportType::Http => self.sync_http(peer, &project).await,
            TransportType::File => self.sync_file(peer, &project).await,
            TransportType::Ssh => Err(MnemeError::UnsupportedTransport("ssh".to_string())),
        };

        let duration_ms = start.elapsed().as_millis() as u64;

        match result {
            Ok(mut r) => {
                r.duration_ms = duration_ms;
                let peer_store = self.db.peers();
                peer_store.update_status(peer.id, "ok")?;
                Ok(r)
            }
            Err(e) => {
                let peer_store = self.db.peers();
                let _ = peer_store.update_status(peer.id, "error");
                Ok(SyncResult {
                    peer_name: peer.name.clone(),
                    direction: SyncDirection::Bidirectional,
                    memories_sent: 0,
                    memories_received: 0,
                    conflicts_resolved: 0,
                    duration_ms,
                    status: SyncStatus::Error,
                    error: Some(e.to_string()),
                })
            }
        }
    }

    /// Sincroniza automaticamente todos los peers auto_sync de un proyecto.
    pub async fn sync_auto(&self, project: &str) -> Result<Vec<SyncResult>> {
        let peers = self.db.peers().list(project)?;
        let mut results = Vec::new();

        for peer in peers {
            if peer.auto_sync {
                match self.sync_with_peer(&peer).await {
                    Ok(r) => results.push(r),
                    Err(e) => {
                        tracing::warn!("auto sync failed for {}: {}", peer.name, e);
                        results.push(SyncResult {
                            peer_name: peer.name.clone(),
                            direction: SyncDirection::Bidirectional,
                            memories_sent: 0,
                            memories_received: 0,
                            conflicts_resolved: 0,
                            duration_ms: 0,
                            status: SyncStatus::Error,
                            error: Some(e.to_string()),
                        });
                    }
                }
            }
        }

        Ok(results)
    }

    /// Exporta un proyecto a archivo.
    pub fn export_project(
        &self,
        project: &str,
        output: Option<std::path::PathBuf>,
    ) -> Result<ExportStats> {
        let conn_arc = self.db.get_conn();
        let conn = conn_arc
            .lock()
            .map_err(|_| MnemeError::Config("mutex poisoned".into()))?;

        let mut stmt = conn.prepare(
            "SELECT automerge_id, doc_bytes FROM sync_state
             JOIN memories ON sync_state.memory_id = memories.id
             WHERE memories.project = ?1 AND sync_state.is_tombstone = 0",
        )?;

        let rows = stmt.query_map(params![project], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, Vec<u8>>(1)?))
        })?;

        let mut changes = Vec::new();
        for row in rows {
            let (automerge_id, doc_bytes) = row?;
            changes.push(MemoryChangeset {
                automerge_id,
                payload: doc_bytes,
                is_full_doc: true,
            });
        }

        let dir = output.unwrap_or_else(|| {
            let mut path =
                std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
            path.push("sync_exports");
            path
        });

        let transport = FileTransport::new(dir)?;
        let (_, stats) = transport.export(project, &changes)?;

        tracing::info!(
            "exported project {}: {} memories",
            project,
            stats.memories_exported
        );
        Ok(ExportStats {
            memories_exported: stats.memories_exported,
            bytes_written: stats.bytes_written,
        })
    }

    /// Construye mensaje de saludo.
    pub fn build_hello(&self, project: &str) -> Result<SyncHello> {
        let conn_arc = self.db.get_conn();
        let conn = conn_arc
            .lock()
            .map_err(|_| MnemeError::Config("mutex poisoned".into()))?;

        let count: u32 = conn.query_row(
            "SELECT COUNT(*) FROM memories WHERE project = ?1 AND deleted_at IS NULL",
            params![project],
            |row| row.get(0),
        )?;

        let mut heads = HashMap::new();
        let mut stmt = conn.prepare(
            "SELECT automerge_id, doc_bytes FROM sync_state
             JOIN memories ON sync_state.memory_id = memories.id
             WHERE memories.project = ?1",
        )?;
        let rows = stmt.query_map(params![project], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, Vec<u8>>(1)?))
        })?;

        for row in rows {
            let (automerge_id, doc_bytes) = row?;
            match crdt::get_heads(&doc_bytes) {
                Ok(h) => {
                    heads.insert(automerge_id, h);
                }
                Err(e) => {
                    tracing::warn!("failed to get heads: {}", e);
                }
            }
        }

        let peer_id = if self.config.peer_id.is_empty() {
            Uuid::new_v4()
        } else {
            Uuid::parse_str(&self.config.peer_id).unwrap_or_else(|_| Uuid::new_v4())
        };

        Ok(SyncHello {
            peer_id,
            peer_name: self.config.peer_name.clone(),
            project: project.to_string(),
            mneme_version: env!("CARGO_PKG_VERSION").to_string(),
            memory_count: count,
            heads,
        })
    }

    /// Aplica una respuesta de sync entrante.
    pub fn apply_response(&self, response: &SyncResponse) -> Result<ApplyStats> {
        let mut applied = 0u32;
        let mut conflicts = 0u32;

        for change in &response.changes {
            if let Err(e) = self.apply_changeset(change) {
                tracing::warn!("failed to apply changeset: {}", e);
                conflicts += 1;
            } else {
                applied += 1;
            }
        }

        Ok(ApplyStats {
            memories_applied: applied,
            conflicts_resolved: conflicts,
        })
    }

    /// Construye respuesta a una solicitud de sync (placeholder).
    pub fn build_response(&self, request: &SyncRequest) -> Result<SyncResponse> {
        Ok(SyncResponse {
            project: request.project.clone(),
            changes: Vec::new(),
            tombstones: Vec::new(),
        })
    }

    async fn sync_http(&self, peer: &Peer, project: &str) -> Result<SyncResult> {
        let transport = HttpTransport::new(peer.address.clone())?;
        let hello = self.build_hello(project)?;
        let _remote_hello = transport.hello(&hello).await?;

        let request = SyncRequest {
            project: project.to_string(),
            have: hello.heads,
        };

        let response = transport.pull(&request).await?;
        let apply_stats = self.apply_response(&response)?;

        let push_response = self.build_response(&request)?;
        transport.push(&push_response).await?;

        Ok(SyncResult {
            peer_name: peer.name.clone(),
            direction: SyncDirection::Bidirectional,
            memories_sent: push_response.changes.len() as u32,
            memories_received: response.changes.len() as u32,
            conflicts_resolved: apply_stats.conflicts_resolved,
            duration_ms: 0,
            status: SyncStatus::Ok,
            error: None,
        })
    }

    async fn sync_file(&self, peer: &Peer, project: &str) -> Result<SyncResult> {
        let dir = std::path::PathBuf::from(&peer.address);
        let transport = FileTransport::new(dir)?;
        let (changes, _) = transport.import_pending(project)?;

        let mut applied = 0u32;
        let mut conflicts = 0u32;

        for change in changes {
            if let Err(e) = self.apply_changeset(&change) {
                tracing::warn!("failed to apply file changeset: {}", e);
                conflicts += 1;
            } else {
                applied += 1;
            }
        }

        Ok(SyncResult {
            peer_name: peer.name.clone(),
            direction: SyncDirection::Pull,
            memories_sent: 0,
            memories_received: applied + conflicts,
            conflicts_resolved: conflicts,
            duration_ms: 0,
            status: if conflicts > 0 {
                SyncStatus::Partial
            } else {
                SyncStatus::Ok
            },
            error: None,
        })
    }

    fn apply_changeset(&self, change: &MemoryChangeset) -> Result<()> {
        let conn_arc = self.db.get_conn();
        let conn = conn_arc
            .lock()
            .map_err(|_| MnemeError::Config("mutex poisoned".into()))?;

        if change.is_full_doc {
            let memory = crdt::doc_to_memory(&change.payload)?;
            let exists: bool = conn
                .query_row(
                    "SELECT 1 FROM sync_state WHERE automerge_id = ?1",
                    params![&change.automerge_id],
                    |_| Ok(true),
                )
                .unwrap_or(false);

            if exists {
                let existing_bytes: Vec<u8> = conn.query_row(
                    "SELECT doc_bytes FROM sync_state WHERE automerge_id = ?1",
                    params![&change.automerge_id],
                    |row| row.get(0),
                )?;
                let merged = crdt::merge_docs(&existing_bytes, &change.payload)?;
                conn.execute(
                    "UPDATE sync_state SET doc_bytes = ?1 WHERE automerge_id = ?2",
                    params![merged, &change.automerge_id],
                )?;
            } else {
                conn.execute(
                    "INSERT INTO sync_state (memory_id, automerge_id, doc_bytes, last_synced, is_tombstone)
                     VALUES (?1, ?2, ?3, ?4, 0)
                     ON CONFLICT(automerge_id) DO UPDATE SET
                         doc_bytes = excluded.doc_bytes,
                         last_synced = excluded.last_synced",
                    params![
                        memory.id.to_string(),
                        &change.automerge_id,
                        &change.payload,
                        Utc::now().to_rfc3339(),
                    ],
                )?;
            }
        }

        Ok(())
    }

    /// Crea o actualiza estado sync para una memoria existente.
    pub fn ensure_sync_state(&self, memory_id: Uuid, automerge_id: &str) -> Result<()> {
        let conn_arc = self.db.get_conn();
        let conn = conn_arc
            .lock()
            .map_err(|_| MnemeError::Config("mutex poisoned".into()))?;

        let exists: bool = conn
            .query_row(
                "SELECT 1 FROM sync_state WHERE memory_id = ?1",
                params![memory_id.to_string()],
                |_| Ok(true),
            )
            .unwrap_or(false);

        if !exists {
            let memory = match self.db.memories().get(memory_id)? {
                Some(m) => m,
                None => return Err(MnemeError::NotFound(memory_id)),
            };

            let mut doc = crdt::memory_to_doc(&memory)?;
            let doc_bytes = crdt::doc_to_bytes(&mut doc)?;

            conn.execute(
                "INSERT INTO sync_state (memory_id, automerge_id, doc_bytes, last_synced, is_tombstone)
                 VALUES (?1, ?2, ?3, ?4, 0)
                 ON CONFLICT(memory_id) DO UPDATE SET
                     automerge_id = excluded.automerge_id,
                     doc_bytes = excluded.doc_bytes,
                     last_synced = excluded.last_synced",
                params![
                    memory_id.to_string(),
                    automerge_id,
                    doc_bytes,
                    Utc::now().to_rfc3339(),
                ],
            )?;
        }

        Ok(())
    }
}
