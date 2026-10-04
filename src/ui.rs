//! Rendering of the two-column layout.

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style, Stylize},
    widgets::{Block, Borders, Paragraph},
};

use crate::app::{App, Focus};

/// Draw the whole interface.
pub fn draw(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
        .split(frame.area());

    draw_column(frame, chunks[0], "Feeds", app.focus == Focus::Feeds);
    draw_column(frame, chunks[1], "Articles", app.focus == Focus::Articles);
}

/// Draw one column with a title and focus highlight.
fn draw_column(frame: &mut Frame, area: Rect, title: &str, focused: bool) {
    let style = if focused {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(style);
    frame.render_widget(Paragraph::new("(leer)").block(block).dim(), area);
}
