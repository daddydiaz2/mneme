use std::collections::HashMap;
use std::path::PathBuf;
use uuid::Uuid;

use mneme::store::db::Database;
use mneme::store::memory::{
    CreateMemoryInput, Importance, MemoryType, Scope, SearchQuery,
};
use mneme::store::search::SearchWeights;

fn setup_db() -> Database {
    let path = PathBuf::from(format!("/tmp/mneme_search_{}.db", Uuid::new_v4()));
    Database::open(&path).unwrap()
}

fn make_input(project: &str, title: &str, content: &str) -> CreateMemoryInput {
    CreateMemoryInput {
        encrypt: false,
        project: project.to_string(),
        scope: Some(Scope::Project),
        title: title.to_string(),
        content: content.to_string(),
        what: None,
        why: None,
        context: None,
        learned: None,
        memory_type: MemoryType::Note,
        importance: Importance::Medium,
        tags: vec![],
        topic_key: None,
        capture_prompt: None,
    }
}

#[test]
fn test_search_finds_by_title() {
    let db = setup_db();
    let store = db.memories();
    store.save(make_input("p", "Rust ownership", "borrow checker"), None, None).unwrap();
    store.save(make_input("p", "Python asyncio", "event loop"), None, None).unwrap();

    let q = SearchQuery {
        text: "Rust ownership".to_string(),
        project: Some("p".to_string()),
        limit: 10,
        scope: None,
        memory_type: None,
        importance: None,
        tags: vec![],
        include_snippet: false,
        all_projects: false,
    };
    let weights = SearchWeights::default();
    let results = store.search(&q, &weights, None).unwrap();
    assert!(!results.is_empty());
    assert!(results[0].memory.title.contains("Rust"));
}

#[test]
fn test_search_finds_by_content() {
    let db = setup_db();
    let store = db.memories();
    store.save(make_input("p", "Auth middleware", "JWT Bearer token"), None, None).unwrap();

    let q = SearchQuery {
        text: "Bearer".to_string(),
        project: Some("p".to_string()),
        limit: 10,
        scope: None,
        memory_type: None,
        importance: None,
        tags: vec![],
        include_snippet: false,
        all_projects: false,
    };
    let weights = SearchWeights::default();
    let results = store.search(&q, &weights, None).unwrap();
    assert!(!results.is_empty());
}

#[test]
fn test_search_filters_by_type() {
    let db = setup_db();
    let store = db.memories();
    store.save(make_input("p", "Architecture doc", "system design"), None, None).unwrap();
    
    let mut input = make_input("p", "Bug doc", "crash on startup");
    input.memory_type = MemoryType::Bugfix;
    store.save(input, None, None).unwrap();

    let q = SearchQuery {
        text: "doc".to_string(),
        project: Some("p".to_string()),
        limit: 10,
        scope: None,
        memory_type: Some(MemoryType::Bugfix),
        importance: None,
        tags: vec![],
        include_snippet: false,
        all_projects: false,
    };
    let weights = SearchWeights::default();
    let results = store.search(&q, &weights, None).unwrap();
    assert!(!results.is_empty());
    assert!(results.iter().all(|r| r.memory.memory_type == MemoryType::Bugfix));
}

#[test]
fn test_search_respects_project_isolation() {
    let db = setup_db();
    let store = db.memories();
    store.save(make_input("proj-a", "uniquetermxyz", "content"), None, None).unwrap();

    let q = SearchQuery {
        text: "uniquetermxyz".to_string(),
        project: Some("proj-b".to_string()),
        limit: 10,
        scope: None,
        memory_type: None,
        importance: None,
        tags: vec![],
        include_snippet: false,
        all_projects: false,
    };
    let weights = SearchWeights::default();
    let results = store.search(&q, &weights, None).unwrap();
    assert!(results.is_empty());
}

#[test]
fn test_search_score_ordering() {
    let db = setup_db();
    let store = db.memories();
    store.save(make_input("p", "JWT authentication", "JWT Bearer token auth"), None, None).unwrap();
    store.save(make_input("p", "Unrelated topic", "something completely different"), None, None).unwrap();

    let q = SearchQuery {
        text: "JWT".to_string(),
        project: Some("p".to_string()),
        limit: 10,
        scope: None,
        memory_type: None,
        importance: None,
        tags: vec![],
        include_snippet: false,
        all_projects: false,
    };
    let weights = SearchWeights::default();
    let results = store.search(&q, &weights, None).unwrap();
    assert!(!results.is_empty());
    assert!(results[0].score >= results.last().unwrap().score);
}
