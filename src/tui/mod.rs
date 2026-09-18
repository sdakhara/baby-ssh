pub mod screens;
pub mod theme;

use std::io::{self, Stdout};

use anyhow::Result;
use crossterm::event::{DisableBracketedPaste, EnableBracketedPaste};
use crossterm::execute;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::backend::CrosstermBackend;
use ratatui::{Frame, Terminal};

use crate::app::state::Screen;
use crate::app::App;

pub type Tui = Terminal<CrosstermBackend<Stdout>>;

pub fn init() -> Result<Tui> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableBracketedPaste)?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

/// Leaves the alternate screen and disables raw mode. Errors are swallowed here because this
/// runs during shutdown, including after a prior error — failing to restore the terminal is
/// worse than logging nothing.
pub fn restore() {
    let mut stdout = io::stdout();
    let _ = execute!(stdout, DisableBracketedPaste, LeaveAlternateScreen);
    let _ = disable_raw_mode();
}

pub fn draw(f: &mut Frame, app: &App) {
    match &app.screen {
        Screen::Connections => screens::connections::render(f, app),
        Screen::Form(form) => screens::connection_form::render(f, form),
        Screen::DeleteConfirm { name, host, .. } => {
            screens::connections::render(f, app);
            screens::delete_confirm::render(f, name, host);
        }
        Screen::Connecting(state) => screens::connecting::render(f, state),
        Screen::Error(state) => screens::error::render(f, state),
    }
}
