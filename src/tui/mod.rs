pub mod app;
pub mod events;
pub mod graph;
pub mod ui;

use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::sync::Arc;
use std::time::Duration;

/// Inicializa el terminal, corre el loop principal y limpia al salir.
pub fn run_tui(
    db: Arc<crate::store::db::Database>,
    settings: Arc<crate::config::settings::Settings>,
) -> crate::error::Result<()> {
    enable_raw_mode().map_err(crate::error::MnemeError::Io)?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen).map_err(crate::error::MnemeError::Io)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal =
        Terminal::new(backend).map_err(|e| crate::error::MnemeError::Config(e.to_string()))?;

    let mut app = app::App::new(db, settings);
    app.load();

    let result = run_loop(&mut terminal, &mut app);

    // Siempre limpiar aunque haya error
    disable_raw_mode().ok();
    let _ = execute!(terminal.backend_mut(), LeaveAlternateScreen);
    let _ = terminal.show_cursor();

    result
}

fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    app: &mut app::App,
) -> crate::error::Result<()> {
    loop {
        terminal
            .draw(|frame| ui::render(frame, app))
            .map_err(|e| crate::error::MnemeError::Config(e.to_string()))?;

        if let Some(crossterm::event::Event::Key(key)) =
            events::next_event(Duration::from_millis(250))?
        {
            events::handle_key(app, key)?;
        }

        if app.quit {
            break;
        }
    }
    Ok(())
}
