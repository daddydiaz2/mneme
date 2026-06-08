use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use std::time::Duration;

use crate::tui::app::{App, AppMode};

/// Espera el próximo evento del teclado.
pub fn next_event(timeout: Duration) -> crate::error::Result<Option<Event>> {
    if event::poll(timeout).map_err(crate::error::MnemeError::Io)? {
        let ev = event::read().map_err(crate::error::MnemeError::Io)?;
        Ok(Some(ev))
    } else {
        Ok(None)
    }
}

/// Despacha teclas según el modo actual.
pub fn handle_key(app: &mut App, key: KeyEvent) -> crate::error::Result<()> {
    match &app.mode {
        // ── MODO BÚSQUEDA ──
        AppMode::Searching => match key.code {
            KeyCode::Esc => app.cancel_search(),
            KeyCode::Enter => app.confirm_search()?,
            KeyCode::Backspace => app.pop_search_char(),
            KeyCode::Char(c) => app.push_search_char(c),
            _ => {}
        },

        // ── CONFIRMACIÓN ──
        AppMode::Confirming { .. } => match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => app.confirm_action()?,
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => app.cancel_confirm(),
            _ => {}
        },

        // ── HELP ──
        AppMode::Help => { app.toggle_help(); }

        // ── GRAFO DE RELACIONES ──
        AppMode::Graph => match key.code {
            KeyCode::Tab | KeyCode::Esc => app.toggle_graph()?,
            KeyCode::Char('j') | KeyCode::Down => app.graph_next(),
            KeyCode::Char('k') | KeyCode::Up => app.graph_prev(),
            KeyCode::Char('r') => app.load_graph()?,
            KeyCode::Char('q') | KeyCode::Char('Q') => app.quit(),
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => app.quit(),
            _ => {}
        },

        // ── GRAFO DE ENTIDADES ──
        AppMode::EntityGraph => match key.code {
            KeyCode::Tab | KeyCode::Esc => app.toggle_entity_graph()?,
            KeyCode::Char('q') | KeyCode::Char('Q') => app.quit(),
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => app.quit(),
            KeyCode::Char('r') => app.load_entity_graph()?,
            _ => {}
        },

        // ── VISTA TEMPORAL ──
        AppMode::Temporal => match key.code {
            KeyCode::Tab | KeyCode::Esc => app.toggle_temporal()?,
            KeyCode::Char('m') | KeyCode::Char('M') => app.temporal_cycle_mode(),
            KeyCode::Char('q') | KeyCode::Char('Q') => app.quit(),
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => app.quit(),
            KeyCode::Char('r') => app.load_temporal()?,
            _ => {}
        },

        // ── MODO NORMAL (principal) ──
        AppMode::Normal => match key.code {
            // Salir
            KeyCode::Char('q') | KeyCode::Char('Q') => app.quit(),
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => app.quit(),

            // Navegación de lista
            KeyCode::Up | KeyCode::Char('k') => app.select_prev(),
            KeyCode::Down | KeyCode::Char('j') => app.select_next(),
            KeyCode::Char('g') => app.select_first(),
            KeyCode::Char('G') => app.select_last(),
            KeyCode::PageUp => app.page_up(),
            KeyCode::PageDown => app.page_down(),

            // Scroll de detalle
            KeyCode::Char('K') => app.detail_scroll_up(),
            KeyCode::Char('J') => app.detail_scroll_down(),

            // Tabs de detalle
            KeyCode::Char('[') => app.detail_prev_tab(),
            KeyCode::Char(']') => app.detail_next_tab(),

            // Acciones
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
        let p = PathBuf::from(format!("/tmp/mneme_tui_test_{}.db", Uuid::new_v4()));
        Arc::new(Database::open(&p).unwrap())
    }
    fn make_app(db: Arc<Database>) -> App {
        let s = Arc::new(Settings::default());
        App::new(db, s).unwrap()
    }
    fn key(kc: KeyCode) -> KeyEvent { KeyEvent::new(kc, KeyModifiers::empty()) }

    #[test] fn test_q_quits() {
        let mut app = make_app(setup_db());
        handle_key(&mut app, key(KeyCode::Char('q'))).unwrap();
        assert!(app.should_quit);
    }
    #[test] fn test_slash_starts_search() {
        let mut app = make_app(setup_db());
        handle_key(&mut app, key(KeyCode::Char('/'))).unwrap();
        assert!(matches!(app.mode, AppMode::Searching));
    }
    #[test] fn test_question_starts_help() {
        let mut app = make_app(setup_db());
        handle_key(&mut app, key(KeyCode::Char('?'))).unwrap();
        assert!(matches!(app.mode, AppMode::Help));
    }
    #[test] fn test_tab_switches_to_graph() {
        let mut app = make_app(setup_db());
        handle_key(&mut app, key(KeyCode::Tab)).unwrap();
        assert!(matches!(app.mode, AppMode::Graph));
    }
    #[test] fn test_e_switches_to_entity() {
        let mut app = make_app(setup_db());
        handle_key(&mut app, key(KeyCode::Char('e'))).unwrap();
        assert!(matches!(app.mode, AppMode::EntityGraph));
    }
    #[test] fn test_t_switches_to_temporal() {
        let mut app = make_app(setup_db());
        handle_key(&mut app, key(KeyCode::Char('t'))).unwrap();
        assert!(matches!(app.mode, AppMode::Temporal));
    }
    #[test] fn test_down_moves_selection() {
        let mut app = make_app(setup_db());
        handle_key(&mut app, key(KeyCode::Down)).unwrap();
        // No crash with empty list
    }
    #[test] fn test_graph_tab_returns() {
        let mut app = make_app(setup_db());
        handle_key(&mut app, key(KeyCode::Tab)).unwrap();
        assert!(matches!(app.mode, AppMode::Graph));
        handle_key(&mut app, key(KeyCode::Tab)).unwrap();
        assert!(matches!(app.mode, AppMode::Normal));
    }
    #[test] fn test_confirm_delete_and_cancel() {
        let mut app = make_app(setup_db());
        app.mode = AppMode::Confirming { action: "delete".to_string(), memory_id: Uuid::new_v4() };
        handle_key(&mut app, key(KeyCode::Char('n'))).unwrap();
        assert!(matches!(app.mode, AppMode::Normal));
    }
    #[test] fn test_search_input_and_cancel() {
        let mut app = make_app(setup_db());
        handle_key(&mut app, key(KeyCode::Char('/'))).unwrap();
        assert!(matches!(app.mode, AppMode::Searching));
        handle_key(&mut app, key(KeyCode::Char('r'))).unwrap();
        handle_key(&mut app, key(KeyCode::Char('u'))).unwrap();
        handle_key(&mut app, key(KeyCode::Char('s'))).unwrap();
        handle_key(&mut app, key(KeyCode::Char('t'))).unwrap();
        assert_eq!(app.search_query, "rust");
        handle_key(&mut app, key(KeyCode::Esc)).unwrap();
        assert!(app.search_query.is_empty());
        assert!(matches!(app.mode, AppMode::Normal));
    }
}
