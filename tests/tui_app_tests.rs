use std::path::PathBuf;
use std::sync::Arc;
use uuid::Uuid;

use mneme::config::settings::Settings;
use mneme::store::db::Database;
use mneme::store::memory::CreateMemoryInput;
use mneme::tui::app::{App, AppMode};

fn setup() -> (App, Arc<Database>) {
    let path = PathBuf::from(format!("/tmp/mneme_tui_test_{}.db", Uuid::new_v4()));
    let db = Arc::new(Database::open(&path).unwrap());
    let settings = Arc::new(Settings::default());
    let app = App::new(db.clone(), settings).unwrap();
    (app, db)
}

#[test]
fn test_app_creation_sets_initial_state() {
    let (app, _db) = setup();
    assert!(matches!(app.mode, AppMode::Normal));
    assert!(!app.should_quit);
    assert!(app.memories.is_empty());
    assert!(app.graph_data.is_none());
    assert_eq!(app.graph_selected, 0);
}

#[test]
fn test_load_memories_empty_project() {
    let (mut app, _db) = setup();
    app.load_memories().unwrap();
    assert!(app.memories.is_empty());
}

#[test]
fn test_load_memories_with_saved_memory() {
    let (mut app, db) = setup();
    let store = db.memories();

    store
        .save(
            CreateMemoryInput {
                encrypt: false,
                project: app.project.clone(),
                scope: Some(mneme::store::memory::Scope::Project),
                title: "TUI Test".to_string(),
                content: "Test content".to_string(),
                what: None,
                why: None,
                context: None,
                learned: None,
                memory_type: mneme::store::memory::MemoryType::Decision,
                importance: mneme::store::memory::Importance::High,
                tags: vec!["tui".to_string()],
                topic_key: None,
                capture_prompt: None,
            },
            None,
            None,
        )
        .unwrap();

    app.load_memories().unwrap();
    assert_eq!(app.memories.len(), 1);
    assert_eq!(app.memories[0].title, "TUI Test");
}

#[test]
fn test_select_next_cycles_correctly() {
    let (mut app, db) = setup();
    let store = db.memories();

    // Create 3 memories
    for i in 0..3 {
        store
            .save(
                CreateMemoryInput {
                    encrypt: false,
                    project: app.project.clone(),
                    scope: Some(mneme::store::memory::Scope::Project),
                    title: format!("Memory {}", i),
                    content: "test".to_string(),
                    what: None,
                    why: None,
                    context: None,
                    learned: None,
                    memory_type: mneme::store::memory::MemoryType::Note,
                    importance: mneme::store::memory::Importance::Low,
                    tags: vec![],
                    topic_key: None,
                    capture_prompt: None,
                },
                None,
                None,
            )
            .unwrap();
    }
    app.load_memories().unwrap();

    assert_eq!(app.selected, 0);
    app.select_next();
    assert_eq!(app.selected, 1);
    app.select_next();
    assert_eq!(app.selected, 2);
    app.select_next(); // should stop at last
    assert_eq!(app.selected, 2);
}

#[test]
fn test_select_prev_stops_at_zero() {
    let (mut app, db) = setup();
    let store = db.memories();

    store
        .save(
            CreateMemoryInput {
                encrypt: false,
                project: app.project.clone(),
                scope: Some(mneme::store::memory::Scope::Project),
                title: "Only One".to_string(),
                content: "test".to_string(),
                what: None,
                why: None,
                context: None,
                learned: None,
                memory_type: mneme::store::memory::MemoryType::Note,
                importance: mneme::store::memory::Importance::Low,
                tags: vec![],
                topic_key: None,
                capture_prompt: None,
            },
            None,
            None,
        )
        .unwrap();
    app.load_memories().unwrap();

    assert_eq!(app.selected, 0);
    app.select_prev();
    assert_eq!(app.selected, 0);
}

#[test]
fn test_selected_memory_returns_saved_memory() {
    let (mut app, db) = setup();
    let store = db.memories();

    store
        .save(
            CreateMemoryInput {
                encrypt: false,
                project: app.project.clone(),
                scope: Some(mneme::store::memory::Scope::Project),
                title: "Selected".to_string(),
                content: "test".to_string(),
                what: None,
                why: None,
                context: None,
                learned: None,
                memory_type: mneme::store::memory::MemoryType::Note,
                importance: mneme::store::memory::Importance::Medium,
                tags: vec![],
                topic_key: None,
                capture_prompt: None,
            },
            None,
            None,
        )
        .unwrap();
    app.load_memories().unwrap();

    let mem = app.selected_memory().unwrap();
    assert_eq!(mem.title, "Selected");
}

#[test]
fn test_selected_memory_returns_none_when_empty() {
    let (app, _db) = setup();
    assert!(app.selected_memory().is_none());
}

#[test]
fn test_search_mode_management() {
    let (mut app, _db) = setup();

    app.start_search();
    assert!(matches!(app.mode, AppMode::Searching));
    assert!(app.search_query.is_empty());

    app.push_search_char('r');
    app.push_search_char('u');
    app.push_search_char('s');
    app.push_search_char('t');
    assert_eq!(app.search_query, "rust");

    app.pop_search_char();
    assert_eq!(app.search_query, "rus");

    app.cancel_search();
    assert!(matches!(app.mode, AppMode::Normal));
    assert!(app.search_query.is_empty());
}

#[test]
fn test_graph_toggle_enters_and_exits() {
    let (mut app, lb) = setup();
    let store = lb.memories();

    // Create a memory so graph has at least something
    store
        .save(
            CreateMemoryInput {
                encrypt: false,
                project: app.project.clone(),
                scope: Some(mneme::store::memory::Scope::Project),
                title: "Graph Node".to_string(),
                content: "test".to_string(),
                what: None,
                why: None,
                context: None,
                learned: None,
                memory_type: mneme::store::memory::MemoryType::Note,
                importance: mneme::store::memory::Importance::Medium,
                tags: vec![],
                topic_key: None,
                capture_prompt: None,
            },
            None,
            None,
        )
        .unwrap();

    app.toggle_graph().unwrap();
    assert!(matches!(app.mode, AppMode::Graph));
    assert!(app.graph_data.is_some());

    app.toggle_graph().unwrap();
    assert!(matches!(app.mode, AppMode::Normal));
}

#[test]
fn test_graph_navigation() {
    let (mut app, db) = setup();
    let store = db.memories();

    // Create 2 memories for graph
    for i in 0..2 {
        store
            .save(
                CreateMemoryInput {
                    encrypt: false,
                    project: app.project.clone(),
                    scope: Some(mneme::store::memory::Scope::Project),
                    title: format!("Node{}", i),
                    content: "test".to_string(),
                    what: None,
                    why: None,
                    context: None,
                    learned: None,
                    memory_type: mneme::store::memory::MemoryType::Note,
                    importance: mneme::store::memory::Importance::Low,
                    tags: vec![],
                    topic_key: None,
                    capture_prompt: None,
                },
                None,
                None,
            )
            .unwrap();
    }

    app.toggle_graph().unwrap();
    let node_count = app.graph_data.as_ref().unwrap().nodes.len();

    assert_eq!(app.graph_selected, 0);
    app.graph_next();
    assert_eq!(app.graph_selected, if node_count > 1 { 1 } else { 0 });
    app.graph_prev();
    assert_eq!(app.graph_selected, 0);
}

#[test]
fn test_help_toggle() {
    let (mut app, _db) = setup();

    app.toggle_help();
    assert!(matches!(app.mode, AppMode::Help));

    app.toggle_help();
    assert!(matches!(app.mode, AppMode::Normal));
}

#[test]
fn test_quit_sets_flag() {
    let (mut app, _db) = setup();
    app.quit();
    assert!(app.should_quit);
}

#[test]
fn test_delete_none_when_empty() {
    let (mut app, _db) = setup();
    // Should not panic when no memories
    app.delete_selected().unwrap();
    assert!(matches!(app.mode, AppMode::Normal));
}

#[test]
fn test_confirm_delete_flow() {
    let (mut app, db) = setup();
    let store = db.memories();

    store
        .save(
            CreateMemoryInput {
                encrypt: false,
                project: app.project.clone(),
                scope: Some(mneme::store::memory::Scope::Project),
                title: "Delete Me".to_string(),
                content: "bye".to_string(),
                what: None,
                why: None,
                context: None,
                learned: None,
                memory_type: mneme::store::memory::MemoryType::Note,
                importance: mneme::store::memory::Importance::Low,
                tags: vec![],
                topic_key: None,
                capture_prompt: None,
            },
            None,
            None,
        )
        .unwrap();
    app.load_memories().unwrap();

    app.delete_selected().unwrap();
    assert!(matches!(app.mode, AppMode::Confirming { .. }));

    app.cancel_confirm();
    assert!(matches!(app.mode, AppMode::Normal));

    // Now actually confirm
    app.delete_selected().unwrap();
    app.confirm_action().unwrap();
    assert!(matches!(app.mode, AppMode::Normal));
    assert!(app.status_message.is_some());
}

#[test]
fn test_select_first_and_last() {
    let (mut app, db) = setup();
    let store = db.memories();

    for i in 0..5 {
        store
            .save(
                CreateMemoryInput {
                    encrypt: false,
                    project: app.project.clone(),
                    scope: Some(mneme::store::memory::Scope::Project),
                    title: format!("M{}", i),
                    content: "test".to_string(),
                    what: None,
                    why: None,
                    context: None,
                    learned: None,
                    memory_type: mneme::store::memory::MemoryType::Note,
                    importance: mneme::store::memory::Importance::Low,
                    tags: vec![],
                    topic_key: None,
                    capture_prompt: None,
                },
                None,
                None,
            )
            .unwrap();
    }
    app.load_memories().unwrap();

    app.select_last();
    assert_eq!(app.selected, 4);

    app.select_first();
    assert_eq!(app.selected, 0);
}

#[test]
fn test_page_up_down() {
    let (mut app, db) = setup();
    let store = db.memories();

    for i in 0..25 {
        store
            .save(
                CreateMemoryInput {
                    encrypt: false,
                    project: app.project.clone(),
                    scope: Some(mneme::store::memory::Scope::Project),
                    title: format!("M{}", i),
                    content: "test".to_string(),
                    what: None,
                    why: None,
                    context: None,
                    learned: None,
                    memory_type: mneme::store::memory::MemoryType::Note,
                    importance: mneme::store::memory::Importance::Low,
                    tags: vec![],
                    topic_key: None,
                    capture_prompt: None,
                },
                None,
                None,
            )
            .unwrap();
    }
    app.load_memories().unwrap();
    assert_eq!(app.memories.len(), 25);

    app.page_down();
    assert!(app.selected >= 20);

    app.page_up();
    assert!(app.selected < 5);
}

#[test]
fn test_mode_transitions_are_exclusive() {
    let (mut app, _db) = setup();

    // Start: Normal
    assert!(matches!(app.mode, AppMode::Normal));

    // Mode should only be one at a time
    app.toggle_graph().unwrap();
    assert!(matches!(app.mode, AppMode::Graph));

    app.toggle_help();
    assert!(matches!(app.mode, AppMode::Help));
}

#[test]
fn test_graph_load_no_memories_still_loads() {
    let (mut app, _db) = setup();

    // Even with no memories, graph should load (empty nodes/edges)
    app.toggle_graph().unwrap();
    assert!(matches!(app.mode, AppMode::Graph));

    let data = app.graph_data.as_ref().unwrap();
    assert!(data.nodes.is_empty());
    assert!(data.edges.is_empty());
}
