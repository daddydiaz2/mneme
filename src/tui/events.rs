use crate::tui::app::{App, Screen};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use std::time::Duration;

pub fn next_event(timeout: Duration) -> crate::error::Result<Option<Event>> {
    if event::poll(timeout).map_err(crate::error::MnemeError::Io)? {
        Ok(Some(event::read().map_err(crate::error::MnemeError::Io)?))
    } else {
        Ok(None)
    }
}

pub fn handle_key(app: &mut App, key: KeyEvent) -> crate::error::Result<()> {
    if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
        app.quit = true;
        return Ok(());
    }
    match app.screen {
        Screen::Help => {
            if key.code == KeyCode::Esc || key.code == KeyCode::Char('q') {
                app.quit = true;
            }
        }
        Screen::Projects => handle_projects(app, key),
        Screen::Memories => handle_memories(app, key),
        Screen::Detail => handle_detail(app, key),
        Screen::Sessions => handle_sessions(app, key),
    }
    Ok(())
}

fn handle_projects(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Up | KeyCode::Char('k') => app.proj_sel = app.proj_sel.saturating_sub(1),
        KeyCode::Down | KeyCode::Char('j') => {
            app.proj_sel = app
                .proj_sel
                .saturating_add(1)
                .min(app.projects.len().saturating_sub(1))
        }
        KeyCode::Enter => {
            if let Some(p) = app.projects.get(app.proj_sel) {
                app.project = p.name.clone();
                app.load_memories();
                app.screen = Screen::Memories;
            }
        }
        KeyCode::Char('s') => {
            app.screen = Screen::Sessions;
            if !app.project.is_empty() {
                app.load_sessions();
            }
        }
        KeyCode::Char('/') => {
            app.search.clear();
            app.searching = true;
        }
        KeyCode::Char('?') => app.screen = Screen::Help,
        KeyCode::Char('q') | KeyCode::Esc => app.quit = true,
        _ => {}
    }
}

fn handle_memories(app: &mut App, key: KeyEvent) {
    if app.searching {
        match key.code {
            KeyCode::Enter => {
                app.searching = false;
                app.search_all();
            }
            KeyCode::Backspace => {
                app.search.pop();
            }
            KeyCode::Esc => app.searching = false,
            KeyCode::Char(c) => app.search.push(c),
            _ => {}
        }
        return;
    }
    match key.code {
        KeyCode::Up | KeyCode::Char('k') => app.mem_sel = app.mem_sel.saturating_sub(1),
        KeyCode::Down | KeyCode::Char('j') => {
            app.mem_sel = app
                .mem_sel
                .saturating_add(1)
                .min(app.memories.len().saturating_sub(1))
        }
        KeyCode::Enter => {
            app.detail = app.memories.get(app.mem_sel).cloned();
            app.detail_scroll = 0;
            app.screen = Screen::Detail;
        }
        KeyCode::Esc => {
            app.screen = Screen::Projects;
            app.load_projects();
        }
        KeyCode::Char('/') => {
            app.search.clear();
            app.searching = true;
        }
        KeyCode::Char('d') => {
            if let Some(m) = app.memories.get(app.mem_sel) {
                let id = m.id;
                let title = m.title.clone();
                let _ = app.db.memories().delete(id, false);
                app.msg = format!("Deleted: {}", title);
                app.load_memories();
            }
        }
        KeyCode::Char('s') => {
            app.load_sessions();
            app.screen = Screen::Sessions;
        }
        KeyCode::Char('?') => app.screen = Screen::Help,
        KeyCode::Char('q') | KeyCode::Esc => app.quit = true,
        _ => {}
    }
}

fn handle_detail(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Up | KeyCode::Char('k') => app.detail_scroll = app.detail_scroll.saturating_sub(1),
        KeyCode::Down | KeyCode::Char('j') => {
            app.detail_scroll = app.detail_scroll.saturating_add(1)
        }
        KeyCode::Esc => {
            app.screen = Screen::Memories;
        }
        KeyCode::Char('d') => app.delete_sel(),
        KeyCode::Char('?') => app.screen = Screen::Help,
        _ => {}
    }
}

fn handle_sessions(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => app.screen = Screen::Memories,
        _ => {}
    }
}
