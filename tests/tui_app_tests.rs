use mneme::config::settings::Settings;
use mneme::store::db::Database;
use mneme::tui::app::{App, Screen};
use std::path::PathBuf;
use std::sync::Arc;
use uuid::Uuid;

fn setup_db() -> Arc<Database> {
    let path = PathBuf::from(format!("/tmp/mneme_tui_integration_{}.db", Uuid::new_v4()));
    Arc::new(Database::open(&path).unwrap())
}

fn make_app() -> App {
    App::new(setup_db(), Arc::new(Settings::default())).unwrap()
}

#[test]
fn test_app_starts_at_dashboard() {
    let app = make_app();
    assert!(matches!(app.screen, Screen::Dashboard));
}

#[test]
fn test_app_has_project_name() {
    let app = make_app();
    assert!(!app.project.is_empty());
}

#[test]
fn test_navigate_to_memories() {
    let mut app = make_app();
    app.navigate(Screen::Memories);
    assert!(matches!(app.screen, Screen::Memories));
}

#[test]
fn test_navigate_to_sessions() {
    let mut app = make_app();
    app.navigate(Screen::Sessions);
    assert!(matches!(app.screen, Screen::Sessions));
}

#[test]
fn test_navigate_to_prompts() {
    let mut app = make_app();
    app.navigate(Screen::Prompts);
    assert!(matches!(app.screen, Screen::Prompts));
}

#[test]
fn test_navigate_to_projects() {
    let mut app = make_app();
    app.navigate(Screen::Projects);
    assert!(matches!(app.screen, Screen::Projects));
}

#[test]
fn test_quit_sets_flag() {
    let mut app = make_app();
    app.quit();
    assert!(app.should_quit);
}

#[test]
fn test_load_empty_memories() {
    let mut app = make_app();
    app.load_memories().unwrap();
    assert!(app.memories.is_empty());
}
