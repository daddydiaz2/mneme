use std::path::PathBuf;
use std::sync::Arc;
use uuid::Uuid;

use mneme::http::router::create_router;
use mneme::store::db::Database;

fn setup_db() -> Database {
    let path = PathBuf::from(format!("/tmp/mneme_http_test_{}.db", Uuid::new_v4()));
    Database::open(&path).unwrap()
}

#[tokio::test]
async fn test_router_creation_succeeds() {
    let db = Arc::new(setup_db());
    let _router = create_router(db, None);
}

#[tokio::test]
async fn test_router_creation_without_embeddings_succeeds() {
    let db = Arc::new(setup_db());
    let _router = create_router(db, None);
}

#[tokio::test]
async fn test_store_create_memory_is_accessible() {
    let db = setup_db();
    let store = db.memories();

    let input = mneme::store::memory::CreateMemoryInput {
        encrypt: false,
        project: "http-test".to_string(),
        scope: Some(mneme::store::memory::Scope::Project),
        title: "HTTP Test Memory".to_string(),
        content: "Test content for HTTP testing".to_string(),
        what: None,
        why: None,
        context: None,
        learned: None,
        memory_type: mneme::store::memory::MemoryType::Note,
        importance: mneme::store::memory::Importance::Medium,
        tags: vec![],
        topic_key: None,
        capture_prompt: None,
    };

    let memory = store.save(input, None, None).unwrap();
    assert_eq!(memory.project, "http-test");
    assert_eq!(memory.title, "HTTP Test Memory");

    let list = store.list("http-test", None, None, None, 10, 0).unwrap();
    assert_eq!(list.len(), 1);
}

#[tokio::test]
async fn test_health_via_store() {
    let db = setup_db();

    let store = db.memories();
    let health = store.health(None).unwrap();

    assert_eq!(health.version, env!("CARGO_PKG_VERSION"));
    assert_eq!(health.total_memories, 0);
    assert!(health.db_size_mb >= 0.0);
    assert!(!health.version.is_empty());
}

#[tokio::test]
async fn test_graph_empty_project() {
    let db = setup_db();
    let store = db.memories();

    let graph = store.get_graph("empty-project").unwrap();
    assert!(graph.nodes.is_empty());
    assert!(graph.edges.is_empty());
}

#[tokio::test]
async fn test_stats_new_project() {
    let db = setup_db();
    let store = db.memories();

    let stats = store.stats("fresh-project").unwrap();
    assert_eq!(stats.total_memories, 0);
    assert_eq!(stats.total_relations, 0);
    assert_eq!(stats.total_sessions, 0);
}

#[tokio::test]
async fn test_projects_empty() {
    let db = setup_db();
    let store = db.memories();

    let projects = store.list_projects().unwrap();
    assert!(projects.is_empty());
}
