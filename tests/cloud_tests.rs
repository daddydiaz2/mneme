use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

fn test_db_path() -> PathBuf {
    let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    std::env::temp_dir().join(format!("mneme_cloud_test_{}_{}.db", std::process::id(), id))
}

use mneme::cloud::CloudConfig;
use mneme::config::settings::SyncConfig;
use mneme::store::db::Database;
use mneme::sync::peer::{Peer, TransportType};
use uuid::Uuid;

fn make_db() -> Database {
    Database::open(&test_db_path()).unwrap()
}

#[test]
fn test_cloud_config_default() {
    let config = CloudConfig {
        server_url: "https://cloud.example.com".to_string(),
        token: "test-token".to_string(),
        project: "test".to_string(),
        auto_sync_interval: 60,
        last_sync: None,
        state: mneme::cloud::CloudState::Disabled,
    };
    assert_eq!(config.server_url, "https://cloud.example.com");
    assert_eq!(config.project, "test");
    assert_eq!(config.auto_sync_interval, 60);
}

#[test]
fn test_cloud_state_variants() {
    use mneme::cloud::CloudState;
    let states = vec![
        CloudState::Disabled,
        CloudState::Connecting,
        CloudState::Syncing,
        CloudState::Error("test error".to_string()),
    ];
    // All states should be constructible
    for state in &states {
        let serialized = serde_json::to_string(state).unwrap();
        assert!(!serialized.is_empty());
    }
}

#[test]
fn test_cloud_state_serialization() {
    use mneme::cloud::CloudState;
    use serde_json;

    let state = CloudState::Syncing;
    let json = serde_json::to_string(&state).unwrap();
    assert!(json.contains("syncing"));

    let error_state = CloudState::Error("connection refused".to_string());
    let json = serde_json::to_string(&error_state).unwrap();
    assert!(json.contains("connection refused"));
}

#[test]
fn test_cloud_orchestrator_creation() {
    let db = make_db();
    let config = SyncConfig::default();
    let orch = mneme::cloud::CloudOrchestrator::new(std::sync::Arc::new(db), config);
    // Just verify construction succeeds
    drop(orch);
}

#[test]
fn test_cloud_status_empty_peers() {
    let db = make_db();
    let config = SyncConfig::default();
    let orch = mneme::cloud::CloudOrchestrator::new(std::sync::Arc::new(db), config);
    let status = orch.cloud_status("nonexistent_project").unwrap();
    assert_eq!(status["project"], "nonexistent_project");
    assert_eq!(status["cloud_peers"], 0);
}

#[test]
fn test_cloud_status_with_http_peer() {
    let db = make_db();
    let peer_store = db.peers();
    let config = SyncConfig::default();
    let orch = mneme::cloud::CloudOrchestrator::new(std::sync::Arc::new(db.clone()), config);

    let peer = Peer {
        id: Uuid::new_v4(),
        name: "cloud-test".to_string(),
        transport: TransportType::Http,
        address: "https://cloud.example.com".to_string(),
        project: "test-project".to_string(),
        last_sync: None,
        last_status: Some("ok".to_string()),
        auto_sync: true,
        created_at: chrono::Utc::now(),
    };
    peer_store.add(&peer).unwrap();

    let status = orch.cloud_status("test-project").unwrap();
    assert_eq!(status["project"], "test-project");
    assert_eq!(status["cloud_peers"], 1);
    assert!(status["peers"].is_array());
    assert_eq!(status["peers"].as_array().unwrap().len(), 1);
}

#[test]
fn test_cloud_status_ignores_file_peers() {
    let db = make_db();
    let peer_store = db.peers();
    let config = SyncConfig::default();
    let orch = mneme::cloud::CloudOrchestrator::new(std::sync::Arc::new(db.clone()), config);

    let http_peer = Peer {
        id: Uuid::new_v4(),
        name: "cloud-peer".to_string(),
        transport: TransportType::Http,
        address: "https://cloud.example.com".to_string(),
        project: "p".to_string(),
        last_sync: None,
        last_status: None,
        auto_sync: true,
        created_at: chrono::Utc::now(),
    };
    let file_peer = Peer {
        id: Uuid::new_v4(),
        name: "file-peer".to_string(),
        transport: TransportType::File,
        address: "/tmp/sync".to_string(),
        project: "p".to_string(),
        last_sync: None,
        last_status: None,
        auto_sync: true,
        created_at: chrono::Utc::now(),
    };
    peer_store.add(&http_peer).unwrap();
    peer_store.add(&file_peer).unwrap();

    let status = orch.cloud_status("p").unwrap();
    // Only HTTP peers are cloud peers
    assert_eq!(status["cloud_peers"], 1);
}

#[test]
fn test_cloud_result_fields() {
    use mneme::cloud::CloudResult;
    let result = CloudResult {
        success: true,
        message: "OK".to_string(),
        project: "p".to_string(),
        memories_synced: 10,
        conflicts_resolved: 2,
        duration_ms: 100,
    };
    assert!(result.success);
    assert_eq!(result.memories_synced, 10);
    assert_eq!(result.conflicts_resolved, 2);
}

#[test]
fn test_cloud_result_serialization() {
    use mneme::cloud::CloudResult;
    let result = CloudResult {
        success: true,
        message: "Synced".to_string(),
        project: "p".to_string(),
        memories_synced: 5,
        conflicts_resolved: 0,
        duration_ms: 50,
    };
    let json = serde_json::to_string(&result).unwrap();
    let parsed: CloudResult = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.memories_synced, 5);
}
