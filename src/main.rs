//! tuifeed — RSS feeds and their articles in a terminal UI.

use std::io::{self, Stdout};

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};

mod app;
mod ui;

use app::App;

type Tui = Terminal<CrosstermBackend<Stdout>>;

fn main() -> Result<()> {
    // Make sure the terminal is always restored, even if we panic.
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = restore_terminal();
        original_hook(info);
    }));

    let mut terminal = setup_terminal()?;
    let result = run(&mut terminal);
    restore_terminal()?;
    result
}

/// Enter raw mode and the alternate screen.
fn setup_terminal() -> Result<Tui> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    Ok(Terminal::new(CrosstermBackend::new(stdout))?)
}

/// Leave the alternate screen and raw mode.
fn restore_terminal() -> Result<()> {
    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen)?;
    Ok(())
}

fn run(terminal: &mut Tui) -> Result<()> {
    let mut app = App::new();
    while !app.should_quit {
        terminal.draw(|frame| ui::draw(frame, &app))?;
        if let Event::Key(key) = event::read()? {
            // Windows reports both press and release; only act on press.
            if key.kind == KeyEventKind::Press {
                app.on_key(key.code, key.modifiers);
            }
        }
    }
    Ok(())
}
