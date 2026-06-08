use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use std::time::Duration;

use crate::tui::app::{App, Screen, Action};

pub fn next_event(timeout: Duration) -> crate::error::Result<Option<Event>> {
    if event::poll(timeout).map_err(crate::error::MnemeError::Io)? {
        let ev = event::read().map_err(crate::error::MnemeError::Io)?;
        Ok(Some(ev))
    } else {
        Ok(None)
    }
}

pub fn handle_key(app: &mut App, key: KeyEvent) -> crate::error::Result<()> {
    match app.screen {
        // ── DASHBOARD ──
        Screen::Dashboard => {
            handle_key_dashboard(app, key)?;
        }

        // ── SEARCH ──
        Screen::Search => match key.code {
            KeyCode::Esc => app.cancel_search(),
            KeyCode::Enter => app.confirm_search()?,
            KeyCode::Backspace => app.pop_search_char(),
            KeyCode::Char(c) => app.push_search_char(c),
            _ => {}
        },

        // ── MEMORIES / PROMPTS ──
        Screen::Memories | Screen::Prompts => match key.code {
            KeyCode::Esc | KeyCode::Char('q') => app.screen = Screen::Dashboard,
            KeyCode::Up | KeyCode::Char('k') => app.select_prev(),
            KeyCode::Down | KeyCode::Char('j') => app.select_next(),
            KeyCode::Char('g') => app.select_first(),
            KeyCode::Char('G') => app.select_last(),
            KeyCode::PageUp => app.page_up(),
            KeyCode::PageDown => app.page_down(),
            KeyCode::Char('/') => app.start_search(),
            KeyCode::Char('r') => { app.load_memories()?; }
            KeyCode::Char('d') => app.delete_selected()?,
            KeyCode::Char('K') => app.detail_scroll_up(),
            KeyCode::Char('J') => app.detail_scroll_down(),
            KeyCode::Char('[') => app.detail_prev_tab(),
            KeyCode::Char(']') => app.detail_next_tab(),
            _ => {}
        },

        // ── SESSIONS ──
        Screen::Sessions => match key.code {
            KeyCode::Esc | KeyCode::Char('q') => app.screen = Screen::Dashboard,
            KeyCode::Up | KeyCode::Char('k') => app.select_prev(),
            KeyCode::Down | KeyCode::Char('j') => app.select_next(),
            KeyCode::Enter => { if let Some(s) = app.sessions.get(app.selected) { app.view_session(s.clone()); } }
            KeyCode::Char('r') => app.load_sessions(),
            _ => {}
        },

        // ── SESSION DETAIL ──
        Screen::SessionDetail => match key.code {
            KeyCode::Esc | KeyCode::Char('q') => app.screen = Screen::Sessions,
            _ => {}
        },

        // ── PROJECTS ──
        Screen::Projects => match key.code {
            KeyCode::Esc | KeyCode::Char('q') => app.screen = Screen::Dashboard,
            KeyCode::Up | KeyCode::Char('k') => app.select_prev(),
            KeyCode::Down | KeyCode::Char('j') => app.select_next(),
            KeyCode::Enter => {
                if let Some(p) = app.projects.get(app.selected) {
                    app.project = p.name.clone();
                    app.load_memories()?;
                    app.screen = Screen::Dashboard;
                }
            }
            KeyCode::Char('r') => { app.projects = app.db.memories().list_projects().unwrap_or_default(); }
            _ => {}
        },

        // ── AGENT SETUP ──
        Screen::AgentSetup => match key.code {
            KeyCode::Esc | KeyCode::Char('q') => app.screen = Screen::Dashboard,
            _ => {}
        },

        // ── GRAPH / ENTITY / TEMPORAL ──
        Screen::Graph => match key.code {
            KeyCode::Tab | KeyCode::Esc => app.screen = Screen::Dashboard,
            KeyCode::Char('j') | KeyCode::Down => app.graph_next(),
            KeyCode::Char('k') | KeyCode::Up => app.graph_prev(),
            KeyCode::Char('r') => { app.load_graph()?; }
            _ => {}
        },
        Screen::EntityGraph => match key.code {
            KeyCode::Tab | KeyCode::Esc => app.screen = Screen::Dashboard,
            KeyCode::Char('r') => { app.load_entity_graph()?; }
            _ => {}
        },
        Screen::Temporal => match key.code {
            KeyCode::Tab | KeyCode::Esc => app.screen = Screen::Dashboard,
            KeyCode::Char('m') => app.temporal_cycle_mode(),
            KeyCode::Char('r') => { app.load_temporal()?; }
            _ => {}
        },
    }
    Ok(())
}

fn handle_key_dashboard(app: &mut App, key: KeyEvent) -> crate::error::Result<()> {
    match key.code {
        KeyCode::Char('1') | KeyCode::Char('s') => app.execute_action(Action::Search),
        KeyCode::Char('2') | KeyCode::Char('o') => app.execute_action(Action::RecentObservations),
        KeyCode::Char('3') | KeyCode::Char('b') => app.execute_action(Action::BrowseSessions),
        KeyCode::Char('4') | KeyCode::Char('p') => app.execute_action(Action::ViewPrompts),
        KeyCode::Char('5') | KeyCode::Char('r') => app.execute_action(Action::Projects),
        KeyCode::Char('6') | KeyCode::Char('a') => app.execute_action(Action::AgentPlugin),
        KeyCode::Char('7') | KeyCode::Char('q') => { app.should_quit = true; Ok(()) },
        KeyCode::Esc => { app.should_quit = true; Ok(()) },
        KeyCode::Down | KeyCode::Char('j') => { app.selected = (app.selected + 1).min(6); Ok(()) }
        KeyCode::Up | KeyCode::Char('k') => { app.selected = app.selected.saturating_sub(1); Ok(()) }
        KeyCode::Enter => {
            let action = match app.selected { 0=>Action::Search, 1=>Action::RecentObservations, 2=>Action::BrowseSessions, 3=>Action::ViewPrompts, 4=>Action::Projects, 5=>Action::AgentPlugin, _=>Action::Quit };
            app.execute_action(action)
        }
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::settings::Settings;
    use crate::store::db::Database;
    use std::path::PathBuf;
    use std::sync::Arc;
    fn setup_db() -> Arc<Database> { Arc::new(Database::open(&PathBuf::from(format!("/tmp/mneme_tui_test_{}.db", uuid::Uuid::new_v4()))).unwrap()) }
    fn make_app() -> App { App::new(setup_db(), Arc::new(Settings::default())).unwrap() }
    fn key(k: KeyCode) -> KeyEvent { KeyEvent::new(k, KeyModifiers::empty()) }
    #[test] fn test_q_quits() { let mut app = make_app(); handle_key(&mut app, key(KeyCode::Char('q'))).unwrap(); assert!(app.should_quit); }
    #[test] fn test_esc_quits() { let mut app = make_app(); handle_key(&mut app, key(KeyCode::Esc)).unwrap(); assert!(app.should_quit); }
    #[test] fn test_s_opens_search() { let mut app = make_app(); handle_key(&mut app, key(KeyCode::Char('s'))).unwrap(); assert!(matches!(app.screen, Screen::Search)); }
    #[test] fn test_2_recent() { let mut app = make_app(); handle_key(&mut app, key(KeyCode::Char('2'))).unwrap(); assert!(matches!(app.screen, Screen::Memories)); }
    #[test] fn test_3_sessions() { let mut app = make_app(); handle_key(&mut app, key(KeyCode::Char('3'))).unwrap(); assert!(matches!(app.screen, Screen::Sessions)); }
    #[test] fn test_4_prompts() { let mut app = make_app(); handle_key(&mut app, key(KeyCode::Char('4'))).unwrap(); assert!(matches!(app.screen, Screen::Prompts)); }
    #[test] fn test_5_projects() { let mut app = make_app(); handle_key(&mut app, key(KeyCode::Char('5'))).unwrap(); assert!(matches!(app.screen, Screen::Projects)); }
    #[test] fn test_agent_setup() { let mut app = make_app(); handle_key(&mut app, key(KeyCode::Char('6'))).unwrap(); assert!(matches!(app.screen, Screen::AgentSetup)); }
    #[test] fn test_enter_on_sessions() { let mut app = make_app(); app.screen = Screen::Sessions; /* no crash */ handle_key(&mut app, key(KeyCode::Enter)).unwrap(); }
    #[test] fn test_search_input() { let mut app = make_app(); app.screen = Screen::Search; handle_key(&mut app, key(KeyCode::Char('r'))).unwrap(); assert_eq!(app.search_query, "r"); }
}
