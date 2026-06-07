use mneme::export::{export_to_markdown, import_from_markdown};
use mneme::store::db::Database;
use mneme::store::memory::{CreateMemoryInput, Importance, MemoryType, Scope};
use std::path::PathBuf;
use uuid::Uuid;

fn setup_db() -> Database {
    let path = PathBuf::from(format!("/tmp/mneme_export_{}.db", Uuid::new_v4()));
    Database::open(&path).unwrap()
}

fn make_memory(title: &str) -> CreateMemoryInput {
    CreateMemoryInput {
        encrypt: false,
        project: "test-proj".to_string(),
        scope: Some(Scope::Project),
        title: title.to_string(),
        content: format!("Content of {}", title),
        what: Some("what field".to_string()),
        why: Some("why field".to_string()),
        context: None,
        learned: Some("learned field".to_string()),
        memory_type: MemoryType::Decision,
        importance: Importance::High,
        tags: vec!["rust".to_string(), "test".to_string()],
        topic_key: None,
        capture_prompt: None,
    }
}

#[test]
fn test_export_markdown_contains_title() {
    let db = setup_db();
    let store = db.memories();
    store.save(make_memory("JWT auth"), None, None).unwrap();
    let memories = store.list("test-proj", None, None, None, 100, 0).unwrap();

    let md = export_to_markdown(&memories, "test-proj");
    assert!(md.contains("JWT auth"));
    assert!(md.contains("# mneme export"));
}

#[test]
fn test_export_markdown_contains_type_and_importance() {
    let db = setup_db();
    let store = db.memories();
    store.save(make_memory("Test Memory"), None, None).unwrap();
    let memories = store.list("test-proj", None, None, None, 100, 0).unwrap();

    let md = export_to_markdown(&memories, "test-proj");
    assert!(md.contains("decision"));
    assert!(md.contains("high"));
}

#[test]
fn test_export_markdown_contains_tags() {
    let db = setup_db();
    let store = db.memories();
    store
        .save(make_memory("Tagged Memory"), None, None)
        .unwrap();
    let memories = store.list("test-proj", None, None, None, 100, 0).unwrap();

    let md = export_to_markdown(&memories, "test-proj");
    assert!(md.contains("`rust`"));
}

#[test]
fn test_export_empty_list_produces_header() {
    let md = export_to_markdown(&[], "empty-proj");
    assert!(md.contains("empty-proj"));
    assert!(md.contains("Total: 0"));
}

#[test]
fn test_import_markdown_roundtrip() {
    let db = setup_db();
    let store = db.memories();
    store
        .save(make_memory("Roundtrip Memory"), None, None)
        .unwrap();
    let memories = store.list("test-proj", None, None, None, 100, 0).unwrap();

    let md = export_to_markdown(&memories, "test-proj");
    let imported = import_from_markdown(&md).unwrap();

    assert_eq!(imported.len(), 1);
    assert_eq!(imported[0].title, "Roundtrip Memory");
}

#[test]
fn test_import_markdown_extracts_type() {
    let db = setup_db();
    let store = db.memories();
    store.save(make_memory("Type Test"), None, None).unwrap();
    let memories = store.list("test-proj", None, None, None, 100, 0).unwrap();

    let md = export_to_markdown(&memories, "test-proj");
    let imported = import_from_markdown(&md).unwrap();

    assert!(matches!(imported[0].memory_type, MemoryType::Decision));
}

#[test]
fn test_import_markdown_extracts_tags() {
    let db = setup_db();
    let store = db.memories();
    store.save(make_memory("Tags Test"), None, None).unwrap();
    let memories = store.list("test-proj", None, None, None, 100, 0).unwrap();

    let md = export_to_markdown(&memories, "test-proj");
    let imported = import_from_markdown(&md).unwrap();

    assert!(imported[0].tags.contains(&"rust".to_string()));
}

#[test]
fn test_import_empty_markdown_returns_empty() {
    let imported = import_from_markdown("# mneme export\nsin memorias\n").unwrap();
    assert!(imported.is_empty());
}
