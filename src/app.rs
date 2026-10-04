//! Application state and input handling.

use crossterm::event::{KeyCode, KeyModifiers};

use crate::config::Config;

/// Which column currently holds keyboard focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Feeds,
    Articles,
}

impl Focus {
    /// The other column (there are exactly two).
    fn other(self) -> Self {
        match self {
            Focus::Feeds => Focus::Articles,
            Focus::Articles => Focus::Feeds,
        }
    }
}

/// The whole application state.
pub struct App {
    pub should_quit: bool,
    pub focus: Focus,
    pub config: Config,
}

impl Default for App {
    fn default() -> Self {
        Self::new(Config::default())
    }
}

impl App {
    pub fn new(config: Config) -> Self {
        Self {
            should_quit: false,
            focus: Focus::Feeds,
            config,
        }
    }

    /// Switch the focused column (Tab / Shift-Tab).
    pub fn toggle_column(&mut self) {
        self.focus = self.focus.other();
    }

    /// Handle a key press.
    pub fn on_key(&mut self, code: KeyCode, modifiers: KeyModifiers) {
        match code {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => {
                self.should_quit = true;
            }
            KeyCode::Tab | KeyCode::BackTab => self.toggle_column(),
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn q_asks_to_quit() {
        let mut app = App::default();
        app.on_key(KeyCode::Char('q'), KeyModifiers::NONE);
        assert!(app.should_quit);
    }

    #[test]
    fn tab_moves_the_focus_between_the_two_columns() {
        let mut app = App::default();
        assert_eq!(app.focus, Focus::Feeds);
        app.on_key(KeyCode::Tab, KeyModifiers::NONE);
        assert_eq!(app.focus, Focus::Articles);
        app.on_key(KeyCode::BackTab, KeyModifiers::SHIFT);
        assert_eq!(app.focus, Focus::Feeds);
    }

    #[test]
    fn ordinary_keys_do_not_quit() {
        let mut app = App::default();
        app.on_key(KeyCode::Char('x'), KeyModifiers::NONE);
        assert!(!app.should_quit);
    }
}
