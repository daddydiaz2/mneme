use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use std::time::Duration;

use crate::tui::app::{App, DetailTab};

pub fn next_event(timeout: Duration) -> crate::error::Result<Option<Event>> {
    if event::poll(timeout).map_err(crate::error::MnemeError::Io)? {
        Ok(Some(event::read().map_err(crate::error::MnemeError::Io)?))
    } else { Ok(None) }
}

pub fn handle_key(app: &mut App, key: KeyEvent) -> crate::error::Result<()> {
    handle_key_inner(app, key);
    Ok(())
}

fn handle_key_inner(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('q') => app.quit = true,
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => app.quit = true,
        KeyCode::Esc => { app.active_panel = if app.search.is_empty() { app.active_panel } else { app.search.clear(); 0 }; },

        // ── NAVIGATION ──
        KeyCode::Down | KeyCode::Char('j') => app.down(),
        KeyCode::Up | KeyCode::Char('k') => app.up(),
        KeyCode::Char('g') => app.first(),
        KeyCode::Char('G') => app.last(),
        KeyCode::PageDown | KeyCode::Char('J') => app.pgdn(),
        KeyCode::PageUp | KeyCode::Char('K') => app.pgup(),

        // ── SEARCH ──
        KeyCode::Char('/') => { app.search.clear(); app.active_panel = 2; }
        KeyCode::Enter if app.active_panel == 2 => { app.active_panel = 0; app.load(); }
        KeyCode::Backspace if app.active_panel == 2 => { app.search.pop(); }
        KeyCode::Char(c) if app.active_panel == 2 => { app.search.push(c); },

        // ── DETAIL TABS ──
        KeyCode::Char('[') => app.tab_prev(),
        KeyCode::Char(']') => app.tab_next(),
        KeyCode::Right if app.active_panel == 0 => app.active_panel = 1,
        KeyCode::Left if app.active_panel == 1 => app.active_panel = 0,

        // ── SCROLL ──
        KeyCode::Char('z') if app.active_panel == 1 => app.dscroll_down(),
        KeyCode::Char('Z') if app.active_panel == 1 => app.dscroll_up(),

        // ── ACTIONS ──
        KeyCode::Tab => { app.load_graph(); app.active_panel = 3; }
        KeyCode::Char('e') => { app.load_entity(); app.active_panel = 4; }
        KeyCode::Char('t') => { app.load_temporal(); app.active_panel = 5; }
        KeyCode::Char('r') => app.load(),
        KeyCode::Char('d') => app.delete_sel(),

        // ── GRAPH (panel 3) ──
        KeyCode::Char('j') | KeyCode::Down if app.active_panel == 3 => app.graph_next(),
        KeyCode::Char('k') | KeyCode::Up if app.active_panel == 3 => app.graph_prev(),
        KeyCode::Tab if app.active_panel == 3 => app.active_panel = 0,

        // ── ENTITY GRAPH (panel 4) ──
        KeyCode::Tab if app.active_panel == 4 => app.active_panel = 0,

        // ── TEMPORAL (panel 5) ──
        KeyCode::Char('m') if app.active_panel == 5 => app.temporal_cycle(),
        KeyCode::Tab if app.active_panel == 5 => app.active_panel = 0,

        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::settings::Settings;
    use crate::store::db::Database;
    use std::path::PathBuf;
    use std::sync::Arc;

    fn make() -> App {
        let p = PathBuf::from(format!("/tmp/mneme_tui_test_{}.db", uuid::Uuid::new_v4()));
        let db = Arc::new(Database::open(&p).unwrap());
        App::new(db, Arc::new(Settings::default()))
    }
    fn k(c: KeyCode) -> KeyEvent { KeyEvent::new(c, KeyModifiers::empty()) }

    #[test] fn q_quits() { let mut a = make(); handle_key(&mut a, k(KeyCode::Char('q'))).ok(); assert!(a.quit); }
    #[test] fn j_down() { let mut a = make(); handle_key(&mut a, k(KeyCode::Char('j'))).ok(); assert_eq!(a.selected, 0); }
    #[test] fn slash_search() { let mut a = make(); handle_key(&mut a, k(KeyCode::Char('/'))).ok(); assert_eq!(a.active_panel, 2); }
    #[test] fn bracket_tabs() { let mut a = make(); handle_key(&mut a, k(KeyCode::Char(']'))).ok(); assert_eq!(a.detail_tab, DetailTab::Structured); }
    #[test] fn r_reload() { let mut a = make(); handle_key(&mut a, k(KeyCode::Char('r'))).ok(); }
    #[test] fn scrollers() { let mut a = make(); handle_key(&mut a, k(KeyCode::Char('g'))).ok(); handle_key(&mut a, k(KeyCode::Char('G'))).ok(); }
}
