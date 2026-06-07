use std::sync::Arc;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::store::db::Database;
use crate::sync::engine::SyncEngine;
use crate::sync::peer::{Peer, TransportType};

/// Estado de enrollment del cloud.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CloudState {
    /// No configurado.
    Disabled,
    /// Conectando al servidor cloud.
    Connecting,
    /// Enrolled y sincronizando.
    Syncing,
    /// Error de conexión.
    Error(String),
}

/// Configuración del cloud para un proyecto.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudConfig {
    /// Servidor cloud URL.
    pub server_url: String,
    /// Token de autenticación.
    pub token: String,
    /// Proyecto en el cloud.
    pub project: String,
    /// Intervalo de autosync en segundos.
    pub auto_sync_interval: u64,
    /// Último sync exitoso.
    pub last_sync: Option<chrono::DateTime<Utc>>,
    /// Estado actual.
    pub state: CloudState,
}

/// Resultado de una operación cloud.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudResult {
    pub success: bool,
    pub message: String,
    pub project: String,
    pub memories_synced: u32,
    pub conflicts_resolved: u32,
    pub duration_ms: u64,
}

/// Orquestador de sync cloud.
pub struct CloudOrchestrator {
    db: Arc<Database>,
    config: crate::config::settings::SyncConfig,
}

impl CloudOrchestrator {
    pub fn new(db: Arc<Database>, config: crate::config::settings::SyncConfig) -> Self {
        Self { db, config }
    }

    /// Enrolla el proyecto con un servidor cloud.
    pub async fn enroll(
        &self,
        server_url: &str,
        token: &str,
        project: &str,
    ) -> crate::error::Result<CloudResult> {
        let start = std::time::Instant::now();
        let peer_id = Uuid::new_v4();

        // Save cloud peer
        let peer = Peer {
            id: peer_id,
            name: format!("cloud-{}", project),
            transport: TransportType::Http,
            address: server_url.to_string(),
            project: project.to_string(),
            last_sync: None,
            last_status: None,
            auto_sync: true,
            created_at: Utc::now(),
        };

        let peer_store = self.db.peers();
        peer_store.add(&peer)?;

        // Send hello to cloud server
        let engine = SyncEngine::new(self.db.clone(), self.config.clone())?;
        let hello = engine.build_hello(project)?;

        let client = reqwest::Client::new();
        let resp = client
            .post(format!("{}/api/v1/sync/hello", server_url.trim_end_matches('/')))
            .header("Authorization", format!("Bearer {}", token))
            .json(&hello)
            .send()
            .await;

        let duration_ms = start.elapsed().as_millis() as u64;

        match resp {
            Ok(r) if r.status().is_success() => {
                peer_store.update_status(peer_id, "enrolled")?;
                tracing::info!(project = %project, server = %server_url, "cloud enrollment successful");
                Ok(CloudResult {
                    success: true,
                    message: format!("Enrolled with cloud server at {}", server_url),
                    project: project.to_string(),
                    memories_synced: hello.memory_count,
                    conflicts_resolved: 0,
                    duration_ms,
                })
            }
            Ok(r) => {
                let status = r.status().as_u16();
                let body = r.text().await.unwrap_or_default();
                peer_store.update_status(peer_id, &format!("enroll_failed_{}", status))?;
                Err(crate::error::MnemeError::SyncFailed {
                    peer: server_url.to_string(),
                    message: format!("enrollment failed ({}): {}", status, body),
                })
            }
            Err(e) => {
                let msg = e.to_string();
                peer_store.update_status(peer_id, "enroll_error")?;
                Err(crate::error::MnemeError::SyncFailed {
                    peer: server_url.to_string(),
                    message: msg,
                })
            }
        }
    }

    /// Ejecuta un ciclo de sync cloud completo.
    pub async fn sync_cloud(&self, project: &str) -> crate::error::Result<CloudResult> {
        let start = std::time::Instant::now();
        let mut memories_synced = 0u32;
        let mut conflicts_resolved = 0u32;

        let peers = self.db.peers().list(project)?;
        let cloud_peers: Vec<&Peer> = peers
            .iter()
            .filter(|p| matches!(p.transport, TransportType::Http) && p.auto_sync)
            .collect();

        if cloud_peers.is_empty() {
            return Ok(CloudResult {
                success: true,
                message: "No cloud peers configured".to_string(),
                project: project.to_string(),
                memories_synced: 0,
                conflicts_resolved: 0,
                duration_ms: 0,
            });
        }

        let engine = SyncEngine::new(self.db.clone(), self.config.clone())?;

        for peer in cloud_peers {
            match engine.sync_with_peer(peer).await {
                Ok(result) => {
                    self.db.peers().record_sync(&result, project)?;
                    memories_synced += result.memories_sent + result.memories_received;
                    conflicts_resolved += result.conflicts_resolved;
                }
                Err(e) => {
                    tracing::warn!(peer = %peer.name, error = %e, "cloud sync failed");
                }
            }
        }

        let duration_ms = start.elapsed().as_millis() as u64;
        Ok(CloudResult {
            success: true,
            message: format!("Synced {} memories", memories_synced),
            project: project.to_string(),
            memories_synced,
            conflicts_resolved,
            duration_ms,
        })
    }

    /// Obtiene el estado del sync cloud.
    pub fn cloud_status(&self, project: &str) -> crate::error::Result<serde_json::Value> {
        let peers = self.db.peers().list(project)?;
        let cloud_peers: Vec<&Peer> = peers
            .iter()
            .filter(|p| matches!(p.transport, TransportType::Http))
            .collect();

        // Get latest sync log entries
        let conn = self.db.get_conn();
        let sync_log: Vec<serde_json::Value>;
        {
            let conn_guard = conn.lock().map_err(|_| {
                crate::error::MnemeError::Config("mutex poisoned".into())
            })?;
            let mut stmt = conn_guard
                .prepare(
                    "SELECT peer_id, direction, status, memories_sent, memories_received, 
                            conflicts_resolved, duration_ms, error, finished_at
                     FROM sync_log WHERE project = ?1
                     ORDER BY finished_at DESC LIMIT 10",
                )
                .ok();
            sync_log = if let Some(ref mut stmt) = stmt {
                stmt.query_map(rusqlite::params![project], |row| {
                    Ok(serde_json::json!({
                        "peer": row.get::<_, String>(0).unwrap_or_default(),
                        "direction": row.get::<_, String>(1).unwrap_or_default(),
                        "status": row.get::<_, String>(2).unwrap_or_default(),
                        "sent": row.get::<_, i32>(3).unwrap_or(0),
                        "received": row.get::<_, i32>(4).unwrap_or(0),
                        "conflicts": row.get::<_, i32>(5).unwrap_or(0),
                        "duration_ms": row.get::<_, i32>(6).unwrap_or(0),
                        "error": row.get::<_, Option<String>>(7).unwrap_or(None),
                        "at": row.get::<_, String>(8).unwrap_or_default(),
                    }))
                })
                .ok()
                .map(|rows| rows.filter_map(|r| r.ok()).collect())
                .unwrap_or_default()
            } else {
                vec![]
            };
        }

        Ok(serde_json::json!({
            "project": project,
            "cloud_peers": cloud_peers.len(),
            "peers": cloud_peers.iter().map(|p| serde_json::json!({
                "name": p.name,
                "address": p.address,
                "last_sync": p.last_sync,
                "last_status": p.last_status,
                "auto_sync": p.auto_sync,
            })).collect::<Vec<_>>(),
            "recent_syncs": sync_log,
        }))
    }

    /// Inicia el autosync background task.
    pub fn start_autosync(
        db: Arc<Database>,
        config: crate::config::settings::SyncConfig,
        interval_secs: u64,
    ) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(interval_secs));
            loop {
                interval.tick().await;
                let engine = match SyncEngine::new(db.clone(), config.clone()) {
                    Ok(e) => e,
                    Err(_) => continue,
                };
                // Auto-sync all projects
                if let Ok(projects) = db.memories().list_projects() {
                    for proj in projects {
                        if let Err(e) = engine.sync_auto(&proj.name).await {
                            tracing::warn!(project = %proj.name, error = %e, "autosync failed");
                        }
                    }
                }
            }
        })
    }
}
