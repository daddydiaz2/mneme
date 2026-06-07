use automerge::transaction::Transactable;
use mneme::store::db::Database;
use mneme::store::memory::{CreateMemoryInput, Importance, MemoryType, Scope};
use mneme::sync::crdt;
use mneme::sync::peer::{Peer, TransportType};
use mneme::sync::protocol::SyncResponse;
use mneme::sync::transport::file::FileTransport;
use std::path::PathBuf;
use uuid::Uuid;

fn setup_db() -> Database {
    let path = PathBuf::from(format!("/tmp/mneme_test_{}.db", Uuid::new_v4()));
    Database::open(&path).unwrap()
}

#[test]
fn test_crdt_merge_is_commutative() {
    let db = setup_db();
    let store = db.memories();

    let mem = store
        .save(
            CreateMemoryInput {
                encrypt: false,
                project: "crdt-test".to_string(),
                scope: Some(Scope::Project),
                title: "Test Memory".to_string(),
                content: "Initial content".to_string(),
                what: None,
                why: None,
                context: None,
                learned: None,
                memory_type: MemoryType::Note,
                importance: Importance::Medium,
                tags: vec!["tag1".to_string()],
                topic_key: None,
                capture_prompt: None,
            },
            None,
            None,
        )
        .unwrap();

    let mem2 = store
        .save(
            CreateMemoryInput {
                encrypt: false,
                project: "crdt-test".to_string(),
                scope: Some(Scope::Project),
                title: "Test Memory".to_string(),
                content: "Modified content".to_string(),
                what: None,
                why: None,
                context: None,
                learned: None,
                memory_type: MemoryType::Note,
                importance: Importance::Medium,
                tags: vec!["tag1".to_string()],
                topic_key: None,
                capture_prompt: None,
            },
            None,
            None,
        )
        .unwrap();

    let doc1 = crdt::memory_to_doc(&mem).unwrap();
    let bytes1 = crdt::doc_to_bytes(&mut doc1.clone()).unwrap();

    let doc2 = crdt::memory_to_doc(&mem2).unwrap();
    let bytes2 = crdt::doc_to_bytes(&mut doc2.clone()).unwrap();

    let merged_ab = crdt::merge_docs(&bytes1, &bytes2).unwrap();
    let merged_ba = crdt::merge_docs(&bytes2, &bytes1).unwrap();

    // Automerge merge is semantically commutative but not byte-level commutative
    let mem_ab = crdt::doc_to_memory(&merged_ab).unwrap();
    let mem_ba = crdt::doc_to_memory(&merged_ba).unwrap();
    assert_eq!(mem_ab.title, mem_ba.title, "merge should be commutative");
    assert_eq!(
        mem_ab.content, mem_ba.content,
        "merge should be commutative"
    );
}

#[test]
fn test_crdt_merge_is_idempotent() {
    let db = setup_db();
    let store = db.memories();

    let mem = store
        .save(
            CreateMemoryInput {
                encrypt: false,
                project: "crdt-test".to_string(),
                scope: Some(Scope::Project),
                title: "Test Memory".to_string(),
                content: "Content".to_string(),
                what: None,
                why: None,
                context: None,
                learned: None,
                memory_type: MemoryType::Note,
                importance: Importance::Medium,
                tags: vec![],
                topic_key: None,
                capture_prompt: None,
            },
            None,
            None,
        )
        .unwrap();

    let doc = crdt::memory_to_doc(&mem).unwrap();
    let bytes = crdt::doc_to_bytes(&mut doc.clone()).unwrap();

    let merged = crdt::merge_docs(&bytes, &bytes).unwrap();
    // Merging identical docs may produce different bytes due to re-encoding,
    // but should parse to semantically equivalent documents
    let original_mem = crdt::doc_to_memory(&bytes).unwrap();
    let merged_mem = crdt::doc_to_memory(&merged).unwrap();
    assert_eq!(
        original_mem.title, merged_mem.title,
        "merging identical docs should preserve content"
    );
    assert_eq!(
        original_mem.content, merged_mem.content,
        "merging identical docs should preserve content"
    );
}

#[test]
fn test_crdt_doc_roundtrip() {
    let db = setup_db();
    let store = db.memories();

    let mem = store
        .save(
            CreateMemoryInput {
                encrypt: false,
                project: "roundtrip-test".to_string(),
                scope: Some(Scope::Project),
                title: "Roundtrip".to_string(),
                content: "Test content".to_string(),
                what: Some("what".to_string()),
                why: Some("why".to_string()),
                context: Some("context".to_string()),
                learned: Some("learned".to_string()),
                memory_type: MemoryType::Decision,
                importance: Importance::High,
                tags: vec!["tag1".to_string(), "tag2".to_string()],
                topic_key: Some("test/key".to_string()),
                capture_prompt: None,
            },
            None,
            None,
        )
        .unwrap();

    let doc = crdt::memory_to_doc(&mem).unwrap();
    let bytes = crdt::doc_to_bytes(&mut doc.clone()).unwrap();
    let restored = crdt::doc_to_memory(&bytes).unwrap();

    assert_eq!(restored.id, mem.id);
    assert_eq!(restored.project, mem.project);
    assert_eq!(restored.title, mem.title);
    assert_eq!(restored.content, mem.content);
    assert_eq!(restored.what, mem.what);
    assert_eq!(restored.why, mem.why);
    assert_eq!(restored.memory_type, mem.memory_type);
    assert_eq!(restored.importance, mem.importance);
    assert_eq!(restored.tags, mem.tags);
    assert_eq!(restored.topic_key, mem.topic_key);
}

#[test]
fn test_peer_store_crud() {
    let db = setup_db();
    let peers = db.peers();

    let peer = Peer {
        id: Uuid::new_v4(),
        name: "test-peer".to_string(),
        transport: TransportType::Http,
        address: "http://localhost:8080".to_string(),
        project: "test-project".to_string(),
        last_sync: None,
        last_status: None,
        auto_sync: true,
        created_at: chrono::Utc::now(),
    };

    peers.add(&peer).unwrap();

    let list = peers.list("test-project").unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].name, "test-peer");

    let fetched = peers.get(peer.id).unwrap().unwrap();
    assert_eq!(fetched.name, "test-peer");

    peers.update_status(peer.id, "ok").unwrap();
    let updated = peers.get(peer.id).unwrap().unwrap();
    assert_eq!(updated.last_status, Some("ok".to_string()));

    peers.remove(peer.id).unwrap();
    let list = peers.list("test-project").unwrap();
    assert!(list.is_empty());
}

#[test]
fn test_zstd_compression_roundtrip() {
    let data = b"Hello, this is a test string for compression. It should compress and decompress correctly.";
    let compressed = zstd::encode_all(&data[..], 3).unwrap();
    let decompressed = zstd::decode_all(&compressed[..]).unwrap();
    assert_eq!(data.to_vec(), decompressed);
}

#[test]
fn test_file_transport_export_import() {
    let dir = PathBuf::from(format!("/tmp/mneme_sync_test_{}", Uuid::new_v4()));
    let transport = FileTransport::new(dir.clone()).unwrap();

    let changes = vec![mneme::sync::protocol::MemoryChangeset {
        automerge_id: "test-123".to_string(),
        payload: vec![1, 2, 3, 4, 5],
        is_full_doc: true,
    }];

    let (path, stats) = transport.export("test-project", &changes).unwrap();
    assert!(path.exists());
    assert_eq!(stats.memories_exported, 1);

    let (imported, _) = transport.import_pending("test-project").unwrap();
    assert_eq!(imported.len(), 1);
    assert_eq!(imported[0].automerge_id, "test-123");

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn test_sync_engine_build_hello() {
    let db = setup_db();
    let store = db.memories();

    store
        .save(
            CreateMemoryInput {
                encrypt: false,
                project: "sync-test".to_string(),
                scope: Some(Scope::Project),
                title: "Sync Test".to_string(),
                content: "Content".to_string(),
                what: None,
                why: None,
                context: None,
                learned: None,
                memory_type: MemoryType::Note,
                importance: Importance::Medium,
                tags: vec![],
                topic_key: None,
                capture_prompt: None,
            },
            None,
            None,
        )
        .unwrap();

    let config = mneme::config::settings::SyncConfig {
        enabled: true,
        peer_id: Uuid::new_v4().to_string(),
        peer_name: "test-peer".to_string(),
        auto_sync_interval: 0,
        compress: false,
    };

    let engine = mneme::sync::engine::SyncEngine::new(std::sync::Arc::new(db), config).unwrap();

    let hello = engine.build_hello("sync-test").unwrap();
    assert_eq!(hello.project, "sync-test");
    assert_eq!(hello.memory_count, 1);
}

#[test]
fn test_apply_response_creates_memory() {
    let db = setup_db();
    let store = db.memories();

    let mem = store
        .save(
            CreateMemoryInput {
                encrypt: false,
                project: "apply-test".to_string(),
                scope: Some(Scope::Project),
                title: "Apply Test".to_string(),
                content: "Original".to_string(),
                what: None,
                why: None,
                context: None,
                learned: None,
                memory_type: MemoryType::Note,
                importance: Importance::Medium,
                tags: vec![],
                topic_key: None,
                capture_prompt: None,
            },
            None,
            None,
        )
        .unwrap();

    let doc = crdt::memory_to_doc(&mem).unwrap();
    let bytes = crdt::doc_to_bytes(&mut doc.clone()).unwrap();

    let config = mneme::config::settings::SyncConfig {
        enabled: true,
        peer_id: Uuid::new_v4().to_string(),
        peer_name: "test-peer".to_string(),
        auto_sync_interval: 0,
        compress: false,
    };

    let engine =
        mneme::sync::engine::SyncEngine::new(std::sync::Arc::new(db.clone()), config).unwrap();

    let response = SyncResponse {
        project: "apply-test".to_string(),
        changes: vec![mneme::sync::protocol::MemoryChangeset {
            automerge_id: mem.id.to_string(),
            payload: bytes,
            is_full_doc: true,
        }],
        tombstones: vec![],
    };

    let stats = engine.apply_response(&response).unwrap();
    assert_eq!(stats.memories_applied, 1);
    assert_eq!(stats.conflicts_resolved, 0);
}
