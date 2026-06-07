use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use std::time::Duration;

use crate::tui::app::{App, AppMode};

/// Espera el siguiente evento de crossterm respetando el timeout.
pub fn next_event(timeout: Duration) -> crate::error::Result<Option<Event>> {
    if event::poll(timeout).map_err(crate::error::MnemeError::Io)? {
        let ev = event::read().map_err(crate::error::MnemeError::Io)?;
        Ok(Some(ev))
    } else {
        Ok(None)
    }
}

/// Despacha un evento de teclado según el modo actual de la app.
pub fn handle_key(app: &mut App, key: KeyEvent) -> crate::error::Result<()> {
    match &app.mode {
        AppMode::Searching => match key.code {
            KeyCode::Esc => app.cancel_search(),
            KeyCode::Enter => app.confirm_search()?,
            KeyCode::Backspace => app.pop_search_char(),
            KeyCode::Char(c) => app.push_search_char(c),
            _ => {}
        },
        AppMode::Confirming { .. } => match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => app.confirm_action()?,
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => app.cancel_confirm(),
            _ => {}
        },
        AppMode::Help => {
            app.toggle_help();
        }
        AppMode::Graph => match key.code {
            KeyCode::Tab | KeyCode::Esc => app.toggle_graph()?,
            KeyCode::Char('j') | KeyCode::Down => app.graph_next(),
            KeyCode::Char('k') | KeyCode::Up => app.graph_prev(),
            KeyCode::Char('r') => app.load_graph()?,
            KeyCode::Char('q') | KeyCode::Char('Q') => app.quit(),
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => app.quit(),
            _ => {}
        },
        AppMode::EntityGraph => match key.code {
            KeyCode::Tab | KeyCode::Esc => app.toggle_entity_graph()?,
            KeyCode::Char('q') | KeyCode::Char('Q') => app.quit(),
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => app.quit(),
            KeyCode::Char('r') => app.load_entity_graph()?,
            _ => {}
        },
        AppMode::Temporal => match key.code {
            KeyCode::Tab | KeyCode::Esc => app.toggle_temporal()?,
            KeyCode::Char('m') | KeyCode::Char('M') => app.temporal_cycle_mode(),
            KeyCode::Char('q') | KeyCode::Char('Q') => app.quit(),
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => app.quit(),
            KeyCode::Char('r') => app.load_temporal()?,
            _ => {}
        },
        AppMode::Normal => match key.code {
            KeyCode::Char('q') | KeyCode::Char('Q') => app.quit(),
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => app.quit(),
            KeyCode::Up | KeyCode::Char('k') => app.select_prev(),
            KeyCode::Down | KeyCode::Char('j') => app.select_next(),
            KeyCode::Char('g') => app.select_first(),
            KeyCode::Char('G') => app.select_last(),
            KeyCode::PageUp => app.page_up(),
            KeyCode::PageDown => app.page_down(),
            KeyCode::Char('/') => app.start_search(),
            KeyCode::Char('r') => app.load_memories()?,
            KeyCode::Char('d') => app.delete_selected()?,
            KeyCode::Char('?') => app.toggle_help(),
            KeyCode::Tab => app.toggle_graph()?,
            KeyCode::Char('e') => app.toggle_entity_graph()?,
            KeyCode::Char('t') => app.toggle_temporal()?,
            _ => {}
        },
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::settings::Settings;
    use crate::store::db::Database;
    use std::path::PathBuf;
    use std::sync::Arc;
    use uuid::Uuid;

    fn setup_db() -> Arc<Database> {
        let p = PathBuf::from(format!("/tmp/mneme_events_test_{}.db", Uuid::new_v4()));
        Arc::new(Database::open(&p).unwrap())
    }

    fn make_app(db: Arc<Database>) -> App {
        let s = Arc::new(Settings::default());
        App::new(db, s).unwrap()
    }

    fn key(kc: KeyCode) -> KeyEvent {
        KeyEvent::new(kc, KeyModifiers::empty())
    }

    fn key_with_mod(kc: KeyCode, mods: KeyModifiers) -> KeyEvent {
        KeyEvent::new(kc, mods)
    }

    #[test]
    fn test_normal_q_quits() {
        let db = setup_db();
        let mut app = make_app(db);
        handle_key(&mut app, key(KeyCode::Char('q'))).unwrap();
        assert!(app.should_quit);
    }

    #[test]
    fn test_normal_capital_q_quits() {
        let db = setup_db();
        let mut app = make_app(db);
        handle_key(&mut app, key(KeyCode::Char('Q'))).unwrap();
        assert!(app.should_quit);
    }

    #[test]
    fn test_normal_ctrl_c_quits() {
        let db = setup_db();
        let mut app = make_app(db);
        handle_key(
            &mut app,
            key_with_mod(KeyCode::Char('c'), KeyModifiers::CONTROL),
        )
        .unwrap();
        assert!(app.should_quit);
    }

    #[test]
    fn test_normal_down_moves_selection() {
        let db = setup_db();
        let mut app = make_app(db);
        handle_key(&mut app, key(KeyCode::Down)).unwrap();
        // No memories loaded, should not panic
        assert!(matches!(app.mode, AppMode::Normal));
    }

    #[test]
    fn test_normal_slash_starts_search() {
        let db = setup_db();
        let mut app = make_app(db);
        handle_key(&mut app, key(KeyCode::Char('/'))).unwrap();
        assert!(matches!(app.mode, AppMode::Searching));
    }

    #[test]
    fn test_normal_question_opens_help() {
        let db = setup_db();
        let mut app = make_app(db);
        handle_key(&mut app, key(KeyCode::Char('?'))).unwrap();
        assert!(matches!(app.mode, AppMode::Help));
    }

    #[test]
    fn test_normal_tab_opens_graph() {
        let db = setup_db();
        let mut app = make_app(db);
        handle_key(&mut app, key(KeyCode::Tab)).unwrap();
        assert!(matches!(app.mode, AppMode::Graph));
    }

    #[test]
    fn test_help_key_toggles_back_to_normal() {
        let db = setup_db();
        let mut app = make_app(db);
        // Enter help
        app.toggle_help();
        assert!(matches!(app.mode, AppMode::Help));
        // Any key closes help
        handle_key(&mut app, key(KeyCode::Enter)).unwrap();
        assert!(matches!(app.mode, AppMode::Normal));
    }

    #[test]
    fn test_search_char_appends_to_query() {
        let db = setup_db();
        let mut app = make_app(db);
        app.mode = AppMode::Searching;
        handle_key(&mut app, key(KeyCode::Char('r'))).unwrap();
        handle_key(&mut app, key(KeyCode::Char('u'))).unwrap();
        handle_key(&mut app, key(KeyCode::Char('s'))).unwrap();
        handle_key(&mut app, key(KeyCode::Char('t'))).unwrap();
        assert_eq!(app.search_query, "rust");
    }

    #[test]
    fn test_search_backspace_removes_char() {
        let db = setup_db();
        let mut app = make_app(db);
        app.mode = AppMode::Searching;
        app.search_query = "hello".to_string();
        handle_key(&mut app, key(KeyCode::Backspace)).unwrap();
        assert_eq!(app.search_query, "hell");
    }

    #[test]
    fn test_search_esc_cancels_and_clears() {
        let db = setup_db();
        let mut app = make_app(db);
        app.mode = AppMode::Searching;
        app.search_query = "test".to_string();
        handle_key(&mut app, key(KeyCode::Esc)).unwrap();
        assert!(matches!(app.mode, AppMode::Normal));
        assert!(app.search_query.is_empty());
    }

    #[test]
    fn test_confirming_y_confirms() {
        let db = setup_db();
        let mut app = make_app(db.clone());
        let store = db.memories();
        store
            .save(
                crate::store::memory::CreateMemoryInput {
                    encrypt: false,
                    project: app.project.clone(),
                    scope: Some(crate::store::memory::Scope::Project),
                    title: "Confirm Me".to_string(),
                    content: "test".to_string(),
                    what: None,
                    why: None,
                    context: None,
                    learned: None,
                    memory_type: crate::store::memory::MemoryType::Note,
                    importance: crate::store::memory::Importance::Medium,
                    tags: vec![],
                    topic_key: None,
                    capture_prompt: None,
                    valid_from: None,
                    valid_until: None,
                    provenance: None,
                },
                None,
                None,
            )
            .unwrap();
        drop(store);
        app.load_memories().unwrap();
        app.delete_selected().unwrap();
        assert!(matches!(app.mode, AppMode::Confirming { .. }));

        handle_key(&mut app, key(KeyCode::Char('y'))).unwrap();
        assert!(matches!(app.mode, AppMode::Normal));
    }

    #[test]
    fn test_confirming_n_cancels() {
        let db = setup_db();
        let mut app = make_app(db);
        app.mode = AppMode::Confirming {
            action: "delete".into(),
            memory_id: Uuid::nil(),
        };
        handle_key(&mut app, key(KeyCode::Char('n'))).unwrap();
        assert!(matches!(app.mode, AppMode::Normal));
    }

    #[test]
    fn test_confirming_esc_cancels() {
        let db = setup_db();
        let mut app = make_app(db);
        app.mode = AppMode::Confirming {
            action: "delete".into(),
            memory_id: Uuid::nil(),
        };
        handle_key(&mut app, key(KeyCode::Esc)).unwrap();
        assert!(matches!(app.mode, AppMode::Normal));
    }

    #[test]
    fn test_graph_tab_returns_to_normal() {
        let db = setup_db();
        let mut app = make_app(db);
        app.mode = AppMode::Graph;
        handle_key(&mut app, key(KeyCode::Tab)).unwrap();
        assert!(matches!(app.mode, AppMode::Normal));
    }

    #[test]
    fn test_graph_esc_returns_to_normal() {
        let db = setup_db();
        let mut app = make_app(db);
        app.mode = AppMode::Graph;
        handle_key(&mut app, key(KeyCode::Esc)).unwrap();
        assert!(matches!(app.mode, AppMode::Normal));
    }

    #[test]
    fn test_graph_j_moves_next() {
        let db = setup_db();
        let mut app = make_app(db);
        app.mode = AppMode::Graph;
        let old = app.graph_selected;
        handle_key(&mut app, key(KeyCode::Char('j'))).unwrap();
        // With no graph data loaded, graph_selected stays 0
        assert_eq!(app.graph_selected, old);
    }

    #[test]
    fn test_graph_q_quits() {
        let db = setup_db();
        let mut app = make_app(db);
        app.mode = AppMode::Graph;
        handle_key(&mut app, key(KeyCode::Char('q'))).unwrap();
        assert!(app.should_quit);
    }

    #[test]
    fn test_graph_ctrl_c_quits() {
        let db = setup_db();
        let mut app = make_app(db);
        app.mode = AppMode::Graph;
        handle_key(
            &mut app,
            key_with_mod(KeyCode::Char('c'), KeyModifiers::CONTROL),
        )
        .unwrap();
        assert!(app.should_quit);
    }

    #[test]
    fn test_normal_refresh_loads_memories() {
        let db = setup_db();
        let store = db.memories();
        store
            .save(
                crate::store::memory::CreateMemoryInput {
                    encrypt: false,
                    project: "r-test".to_string(),
                    scope: Some(crate::store::memory::Scope::Project),
                    title: "Refresh".to_string(),
                    content: "test".to_string(),
                    what: None,
                    why: None,
                    context: None,
                    learned: None,
                    memory_type: crate::store::memory::MemoryType::Note,
                    importance: crate::store::memory::Importance::Medium,
                    tags: vec![],
                    topic_key: None,
                    capture_prompt: None,
                    valid_from: None,
                    valid_until: None,
                    provenance: None,
                },
                None,
                None,
            )
            .unwrap();
        drop(store); // release borrow on db

        let mut app = make_app(db.clone());
        app.project = "r-test".to_string();
        handle_key(&mut app, key(KeyCode::Char('r'))).unwrap();
        assert!(!app.memories.is_empty());
    }

    #[test]
    fn test_normal_g_and_g_move_selection() {
        let db = setup_db();
        let mut app = make_app(db);
        // With no memories, g and G should not panic
        handle_key(&mut app, key(KeyCode::Char('g'))).unwrap();
        handle_key(&mut app, key(KeyCode::Char('G'))).unwrap();
        assert_eq!(app.selected, 0);
    }

    #[test]
    fn test_normal_page_keys_no_panic() {
        let db = setup_db();
        let mut app = make_app(db);
        handle_key(&mut app, key(KeyCode::PageUp)).unwrap();
        handle_key(&mut app, key(KeyCode::PageDown)).unwrap();
        assert_eq!(app.selected, 0);
    }
}
