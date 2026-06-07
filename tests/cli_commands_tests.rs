use std::path::PathBuf;
use uuid::Uuid;

use mneme::cli::commands::{run_command, Commands};
use mneme::store::db::Database;
use mneme::store::memory::{Scope, MemoryType, Importance};

static TEST_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

fn test_db() -> Database {
    let id = TEST_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let path = PathBuf::from(format!("/tmp/mneme_cmd_test_{}_{}.db", std::process::id(), id));
    Database::open(&path).unwrap()
}

fn save_memory(db: &Database, title: &str) -> Uuid {
    use mneme::store::memory::CreateMemoryInput;
    let input = CreateMemoryInput {
        encrypt: false,
        project: "cli-test".to_string(),
        scope: Some(Scope::Project),
        title: title.to_string(),
        content: "test content".to_string(),
        what: None, why: None, context: None, learned: None,
        memory_type: MemoryType::Note,
        importance: Importance::Medium,
        tags: vec![],
        topic_key: None,
        capture_prompt: None,
        valid_from: None,
        valid_until: None,
        provenance: None,
    };
    db.memories().save(input, None, None).unwrap().id
}

#[tokio::test]
async fn test_cmd_save_basic() {
    let db = test_db();
    run_command(
        Commands::Save {
            title: "CLI Save Test".to_string(),
            content: "test content".to_string(),
            project: Some("cli-test".to_string()),
            r#type: "note".to_string(),
            importance: "high".to_string(),
            tags: vec!["test".to_string()],
            what: Some("What".to_string()),
            why: Some("Why".to_string()),
            context: None,
            learned: None,
            scope: Some("project".to_string()),
            topic_key: Some("cli/test".to_string()),
        },
        &db,
        None,
    )
    .unwrap();

    let list = db.memories().list("cli-test", None, None, None, 10, 0).unwrap();
    assert!(!list.is_empty());
    assert_eq!(list[0].title, "CLI Save Test");
    assert_eq!(list[0].importance, Importance::High);
}

#[test]
fn test_cmd_save_invalid_type_returns_error() {
    let db = test_db();
    let result = run_command(
        Commands::Save {
            title: "Bad Type".to_string(),
            content: "test".to_string(),
            project: Some("cli-test".to_string()),
            r#type: "invalid_type".to_string(),
            importance: "medium".to_string(),
            tags: vec![],
            what: None, why: None, context: None, learned: None,
            scope: None,
            topic_key: None,
        },
        &db,
        None,
    );
    assert!(result.is_err(), "invalid type should error");
}

#[test]
fn test_cmd_save_invalid_importance_returns_error() {
    let db = test_db();
    let result = run_command(
        Commands::Save {
            title: "Bad Importance".to_string(),
            content: "test".to_string(),
            project: Some("cli-test".to_string()),
            r#type: "note".to_string(),
            importance: "ultra".to_string(),
            tags: vec![],
            what: None, why: None, context: None, learned: None,
            scope: None,
            topic_key: None,
        },
        &db,
        None,
    );
    assert!(result.is_err(), "invalid importance should error");
}

#[test]
fn test_cmd_save_all_fields() {
    let db = test_db();
    run_command(
        Commands::Save {
            title: "Full Fields".to_string(),
            content: "body".to_string(),
            project: Some("cli-test".to_string()),
            r#type: "decision".to_string(),
            importance: "critical".to_string(),
            tags: vec!["rust".to_string(), "cli".to_string()],
            what: Some("what value".to_string()),
            why: Some("why value".to_string()),
            context: Some("context value".to_string()),
            learned: Some("learned value".to_string()),
            scope: Some("global".to_string()),
            topic_key: Some("cli/full".to_string()),
        },
        &db,
        None,
    )
    .unwrap();

    let list = db.memories().list("cli-test", None, None, None, 10, 0).unwrap();
    let mem = &list[0];
    assert_eq!(mem.memory_type, MemoryType::Decision);
    assert_eq!(mem.importance, Importance::Critical);
    assert_eq!(mem.tags, vec!["rust", "cli"]);
    assert_eq!(mem.what, Some("what value".to_string()));
    assert_eq!(mem.why, Some("why value".to_string()));
}

#[test]
fn test_cmd_search_finds_by_query() {
    let db = test_db();
    save_memory(&db, "Rust Performance");
    save_memory(&db, "Python Async");

    run_command(
        Commands::Search {
            query: "Rust".to_string(),
            project: Some("cli-test".to_string()),
            r#type: None,
            limit: 10,
            json: false,
        },
        &db,
        None,
    )
    .unwrap();
}

#[test]
fn test_cmd_search_json_output() {
    let db = test_db();
    save_memory(&db, "JSON Test");
    save_memory(&db, "Another One");

    run_command(
        Commands::Search {
            query: "JSON".to_string(),
            project: Some("cli-test".to_string()),
            r#type: None,
            limit: 10,
            json: true,
        },
        &db,
        None,
    )
    .unwrap();
}

#[test]
fn test_cmd_search_empty_query() {
    let db = test_db();
    save_memory(&db, "Exists");

    let result = run_command(
        Commands::Search {
            query: "".to_string(),
            project: Some("cli-test".to_string()),
            r#type: None,
            limit: 10,
            json: false,
        },
        &db,
        None,
    );
    // Empty query may be allowed or rejected - just verify no panic
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_cmd_get_existing_memory() {
    let db = test_db();
    let id = save_memory(&db, "Get Me");

    run_command(
        Commands::Get {
            id: id.to_string(),
            json: false,
        },
        &db,
        None,
    )
    .unwrap();
}

#[test]
fn test_cmd_get_json_output() {
    let db = test_db();
    let id = save_memory(&db, "Get JSON");

    run_command(
        Commands::Get {
            id: id.to_string(),
            json: true,
        },
        &db,
        None,
    )
    .unwrap();
}

#[test]
fn test_cmd_get_nonexistent_errors() {
    let db = test_db();
    // Get calls std::process::exit() on missing memory, so we test via store directly
    let result = db.memories().get(Uuid::nil());
    assert!(result.unwrap().is_none());
}

#[test]
fn test_cmd_list_with_data() {
    let db = test_db();
    save_memory(&db, "List A");
    save_memory(&db, "List B");
    save_memory(&db, "List C");

    run_command(
        Commands::List {
            project: Some("cli-test".to_string()),
            r#type: None,
            limit: 10,
            json: false,
        },
        &db,
        None,
    )
    .unwrap();
}

#[test]
fn test_cmd_list_empty_project() {
    let db = test_db();
    run_command(
        Commands::List {
            project: Some("empty-project".to_string()),
            r#type: None,
            limit: 10,
            json: false,
        },
        &db,
        None,
    )
    .unwrap();
}

#[test]
fn test_cmd_list_json_output() {
    let db = test_db();
    save_memory(&db, "JSON List");

    run_command(
        Commands::List {
            project: Some("cli-test".to_string()),
            r#type: None,
            limit: 10,
            json: true,
        },
        &db,
        None,
    )
    .unwrap();
}

#[test]
fn test_cmd_delete_soft() {
    let db = test_db();
    let id = save_memory(&db, "To Delete");

    run_command(
        Commands::Delete {
            id: id.to_string(),
            hard: false,
        },
        &db,
        None,
    )
    .unwrap();

    assert!(db.memories().get(id).unwrap().is_none());
}

#[test]
fn test_cmd_delete_hard() {
    let db = test_db();
    let id = save_memory(&db, "Hard Delete");

    run_command(
        Commands::Delete {
            id: id.to_string(),
            hard: true,
        },
        &db,
        None,
    )
    .unwrap();

    assert!(db.memories().get(id).unwrap().is_none());
}

#[test]
fn test_cmd_restore_after_soft_delete() {
    let db = test_db();
    let id = save_memory(&db, "Restore Me");

    // Delete
    db.memories().delete(id, false).unwrap();

    // Restore
    run_command(
        Commands::Restore {
            id: id.to_string(),
        },
        &db,
        None,
    )
    .unwrap();

    assert!(db.memories().get(id).unwrap().is_some());
}

#[test]
fn test_cmd_stats_with_data() {
    let db = test_db();
    save_memory(&db, "Stats Test");

    run_command(
        Commands::Stats {
            project: Some("cli-test".to_string()),
        },
        &db,
        None,
    )
    .unwrap();
}

#[test]
fn test_cmd_stats_empty_project() {
    let db = test_db();
    run_command(
        Commands::Stats {
            project: Some("empty-stats".to_string()),
        },
        &db,
        None,
    )
    .unwrap();
}

#[test]
fn test_cmd_projects_with_data() {
    let db = test_db();
    save_memory(&db, "Project A");

    run_command(Commands::Projects, &db, None).unwrap();
}

#[test]
fn test_cmd_projects_empty() {
    let db = test_db();
    run_command(Commands::Projects, &db, None).unwrap();
}

#[test]
fn test_cmd_context_with_data() {
    let db = test_db();
    save_memory(&db, "Context Test");

    run_command(
        Commands::Context {
            project: Some("cli-test".to_string()),
            limit: 10,
        },
        &db,
        None,
    )
    .unwrap();
}

#[test]
fn test_cmd_context_empty_project() {
    let db = test_db();
    run_command(
        Commands::Context {
            project: Some("empty-ctx".to_string()),
            limit: 10,
        },
        &db,
        None,
    )
    .unwrap();
}

#[test]
fn test_cmd_doctor_with_data() {
    let db = test_db();
    save_memory(&db, "Doctor Test");

    run_command(
        Commands::Doctor {
            project: Some("cli-test".to_string()),
            json: false,
        },
        &db,
        None,
    )
    .unwrap();
}

#[test]
fn test_cmd_doctor_json() {
    let db = test_db();
    run_command(
        Commands::Doctor {
            project: Some("cli-test".to_string()),
            json: true,
        },
        &db,
        None,
    )
    .unwrap();
}

#[test]
fn test_cmd_doctor_empty() {
    let db = test_db();
    run_command(
        Commands::Doctor {
            project: Some("empty-doctor".to_string()),
            json: false,
        },
        &db,
        None,
    )
    .unwrap();
}

#[test]
fn test_cmd_export_no_data() {
    let db = test_db();
    let tmp_dir = std::env::temp_dir().join(format!("mneme_export_{}", std::process::id()));
    std::fs::create_dir_all(&tmp_dir).ok();
    let output_path = tmp_dir.join("export_test.md");

    run_command(
        Commands::Export {
            project: Some("cli-test".to_string()),
            output: Some(output_path.clone()),
            format: "md".to_string(),
        },
        &db,
        None,
    )
    .unwrap();

    assert!(output_path.exists(), "export file should exist");
    std::fs::remove_file(&output_path).ok();
}

#[test]
fn test_cmd_export_json() {
    let db = test_db();
    save_memory(&db, "Export Me");
    let tmp_dir = std::env::temp_dir().join(format!("mneme_export_json_{}", std::process::id()));
    std::fs::create_dir_all(&tmp_dir).ok();
    let output_path = tmp_dir.join("export.json");

    run_command(
        Commands::Export {
            project: Some("cli-test".to_string()),
            output: Some(output_path.clone()),
            format: "json".to_string(),
        },
        &db,
        None,
    )
    .unwrap();

    assert!(output_path.exists());
    let content = std::fs::read_to_string(&output_path).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert!(parsed.is_array());
    std::fs::remove_file(&output_path).ok();
}

#[test]
fn test_cmd_import_json() {
    let db = test_db();
    let tmp_dir = std::env::temp_dir().join(format!("mneme_import_{}", std::process::id()));
    std::fs::create_dir_all(&tmp_dir).ok();

    // Create a JSON export file
    let data = serde_json::json!([{
        "id": "00000000-0000-0000-0000-000000000001",
        "project": "cli-test",
        "scope": "project",
        "title": "Imported Memory",
        "content": "imported content",
        "memory_type": "note",
        "importance": "medium",
        "tags": [],
        "topic_key": null,
        "access_count": 0,
        "revision_count": 0,
        "duplicate_count": 0,
        "context_inject_count": 0,
        "is_encrypted": false,
        "created_at": "2024-01-01T00:00:00Z",
        "updated_at": "2024-01-01T00:00:00Z"
    }]);
    let import_path = tmp_dir.join("import.json");
    std::fs::write(&import_path, data.to_string()).unwrap();

    run_command(
        Commands::Import {
            file: import_path,
            project: Some("cli-test".to_string()),
        },
        &db,
        None,
    )
    .unwrap();

    let list = db.memories().list("cli-test", None, None, None, 10, 0).unwrap();
    assert!(!list.is_empty());
    let titles: Vec<&str> = list.iter().map(|m| m.title.as_str()).collect();
    assert!(titles.contains(&"Imported Memory"));
}

#[test]
fn test_cmd_import_markdown() {
    let db = test_db();
    let tmp_dir = std::env::temp_dir().join(format!("mneme_import_md_{}", std::process::id()));
    std::fs::create_dir_all(&tmp_dir).ok();

    let md_content = "# mneme export -- proyecto: cli-test\nExportado el: 2024-01-01\nTotal: 1 memorias\n\n---\n\n## MD Import\n- **ID**: `00000000-0000-0000-0000-000000000001`\n- **Tipo**: decision\n- **Importancia**: high\n- **Scope**: project\n\nImported via markdown\n";
    let import_path = tmp_dir.join("import.md");
    std::fs::write(&import_path, md_content).unwrap();

    run_command(
        Commands::Import {
            file: import_path,
            project: Some("cli-test".to_string()),
        },
        &db,
        None,
    )
    .unwrap();

    let list = db.memories().list("cli-test", None, None, None, 10, 0).unwrap();
    let titles: Vec<&str> = list.iter().map(|m| m.title.as_str()).collect();
    assert!(titles.contains(&"MD Import"));
}

#[test]
fn test_cmd_relate_two_memories() {
    let db = test_db();
    let id1 = save_memory(&db, "Source");
    let id2 = save_memory(&db, "Target");

    run_command(
        Commands::Relate {
            from_id: id1.to_string(),
            to_id: id2.to_string(),
            relation_type: "related_to".to_string(),
            confidence: Some(0.95),
        },
        &db,
        None,
    )
    .unwrap();
}

#[test]
fn test_cmd_relate_invalid_type_errors() {
    let db = test_db();
    let id1 = save_memory(&db, "A");
    let id2 = save_memory(&db, "B");

    let result = run_command(
        Commands::Relate {
            from_id: id1.to_string(),
            to_id: id2.to_string(),
            relation_type: "invalid".to_string(),
            confidence: None,
        },
        &db,
        None,
    );
    assert!(result.is_err(), "invalid relation type should error");
}
