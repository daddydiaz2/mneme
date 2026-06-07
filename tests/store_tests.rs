use std::path::PathBuf;
use uuid::Uuid;

use mneme::store::db::Database;
use mneme::store::memory::{
    CreateMemoryInput, Importance, MemoryType, Scope, SearchQuery, UpdateMemoryInput,
};

fn setup_db() -> Database {
    let path = PathBuf::from(format!("/tmp/mneme_test_{}.db", Uuid::new_v4()));
    Database::open(&path).unwrap()
}

#[test]
fn test_save_and_get_memory() {
    let db = setup_db();
    let store = db.memories();

    let input = CreateMemoryInput {
        encrypt: false,
        project: "test-project".to_string(),
        scope: Some(Scope::Project),
        title: "Test Memory".to_string(),
        content: "This is a test memory".to_string(),
        what: None,
        why: None,
        context: None,
        learned: None,
        memory_type: MemoryType::Note,
        importance: Importance::Medium,
        tags: vec!["test".to_string()],
        topic_key: None,
        capture_prompt: None,
    };

    let memory = store.save(input, None, None).unwrap();
    assert_eq!(memory.title, "Test Memory");
    assert_eq!(memory.project, "test-project");

    let retrieved = store.get(memory.id).unwrap().unwrap();
    assert_eq!(retrieved.id, memory.id);
    assert_eq!(retrieved.access_count, 2); // save() calls get() internally + 1 explicit get
}

#[test]
fn test_soft_delete() {
    let db = setup_db();
    let store = db.memories();

    let input = CreateMemoryInput {
        encrypt: false,
        project: "test".to_string(),
        scope: Some(Scope::Project),
        title: "To Delete".to_string(),
        content: "Delete me".to_string(),
        what: None,
        why: None,
        context: None,
        learned: None,
        memory_type: MemoryType::Note,
        importance: Importance::Low,
        tags: vec![],
        topic_key: None,
        capture_prompt: None,
    };

    let memory = store.save(input, None, None).unwrap();
    store.delete(memory.id, false).unwrap();

    assert!(store.get(memory.id).unwrap().is_none());
}

#[test]
fn test_dedupe_detection() {
    let db = setup_db();
    let store = db.memories();

    let input = CreateMemoryInput {
        encrypt: false,
        project: "test".to_string(),
        scope: Some(Scope::Project),
        title: "Duplicate Title".to_string(),
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
    };

    let first = store.save(input.clone(), None, None).unwrap();
    let second = store.save(input, None, None).unwrap();

    assert_eq!(first.id, second.id);
    assert_eq!(second.duplicate_count, 1);
}

#[test]
fn test_topic_key_upsert() {
    let db = setup_db();
    let store = db.memories();

    let input1 = CreateMemoryInput {
        encrypt: false,
        project: "test".to_string(),
        scope: Some(Scope::Project),
        title: "Auth v1".to_string(),
        content: "JWT auth".to_string(),
        what: None,
        why: None,
        context: None,
        learned: None,
        memory_type: MemoryType::Architecture,
        importance: Importance::High,
        tags: vec![],
        topic_key: Some("architecture/auth".to_string()),
        capture_prompt: None,
    };

    let input2 = CreateMemoryInput {
        encrypt: false,
        project: "test".to_string(),
        scope: Some(Scope::Project),
        title: "Auth v2".to_string(),
        content: "OAuth2 auth".to_string(),
        what: None,
        why: None,
        context: None,
        learned: None,
        memory_type: MemoryType::Architecture,
        importance: Importance::High,
        tags: vec![],
        topic_key: Some("architecture/auth".to_string()),
        capture_prompt: None,
    };

    let first = store.save(input1, None, None).unwrap();
    let second = store.save(input2, None, None).unwrap();

    assert_eq!(first.id, second.id);
    assert_eq!(second.revision_count, 2);
    assert_eq!(second.content, "OAuth2 auth");
}

#[test]
fn test_search_fts5() {
    let db = setup_db();
    let store = db.memories();

    store
        .save(
            CreateMemoryInput {
                encrypt: false,
                project: "test".to_string(),
                scope: Some(Scope::Project),
                title: "Rust Performance".to_string(),
                content: "Optimizing Rust code with zero-cost abstractions".to_string(),
                what: None,
                why: None,
                context: None,
                learned: None,
                memory_type: MemoryType::Pattern,
                importance: Importance::High,
                tags: vec!["rust".to_string()],
                topic_key: None,
                capture_prompt: None,
            },
            None,
            None,
        )
        .unwrap();

    let query = SearchQuery {
        text: "abstractions".to_string(),
        project: Some("test".to_string()),
        scope: None,
        memory_type: None,
        importance: None,
        tags: vec![],
        limit: 10,
        include_snippet: true,
        all_projects: false,
    };

    let weights = mneme::store::search::SearchWeights::default();
    let results = store.search(&query, &weights, None).unwrap();
    assert!(!results.is_empty());
    assert!(results[0].snippet.is_some());
}

#[test]
fn test_session_lifecycle() {
    let db = setup_db();
    let sessions = db.sessions();

    let session = sessions.start("test-project", Some("/tmp/test")).unwrap();
    assert_eq!(session.status, "active");

    sessions.add_memory(session.id, Uuid::new_v4()).unwrap();

    let ended = sessions.end(session.id, Some("Summary")).unwrap();
    assert_eq!(ended.status, "ended");
    assert_eq!(ended.summary, Some("Summary".to_string()));
}

#[test]
fn test_stats() {
    let db = setup_db();
    let store = db.memories();

    store
        .save(
            CreateMemoryInput {
                encrypt: false,
                project: "stats-test".to_string(),
                scope: Some(Scope::Project),
                title: "One".to_string(),
                content: "Content".to_string(),
                what: None,
                why: None,
                context: None,
                learned: None,
                memory_type: MemoryType::Bugfix,
                importance: Importance::Critical,
                tags: vec![],
                topic_key: None,
                capture_prompt: None,
            },
            None,
            None,
        )
        .unwrap();

    let stats = store.stats("stats-test").unwrap();
    assert_eq!(stats.total_memories, 1);
    assert!(stats.by_type.contains_key("bugfix"));
    assert!(stats.by_importance.contains_key("critical"));
}
