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
            _ => {}
        },
    }
    Ok(())
}
