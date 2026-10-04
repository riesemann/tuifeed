//! Rendering of the two-column layout.

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Text},
    widgets::{Block, Borders, Paragraph},
};

use crate::app::{App, FeedState, Focus};

/// Draw the whole interface.
pub fn draw(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(app.config.layout.feeds()),
            Constraint::Percentage(app.config.layout.articles()),
        ])
        .split(frame.area());

    draw_feeds(frame, chunks[0], app);
    draw_articles(frame, chunks[1], app);
}

/// The left column: one line per feed with its load state.
fn draw_feeds(frame: &mut Frame, area: Rect, app: &App) {
    let lines: Vec<Line> = app
        .feeds
        .iter()
        .map(|slot| match &slot.state {
            FeedState::Loading => Line::from(format!("{}  lädt …", slot.label())).dim(),
            FeedState::Loaded(feed) => {
                Line::from(format!("{}  ({})", slot.label(), feed.articles.len()))
            }
            FeedState::Failed(_) => Line::from(format!("{}  ⚠ Fehler", slot.label())).red(),
        })
        .collect();

    let text = if lines.is_empty() {
        Text::from("Keine Feeds konfiguriert.")
    } else {
        Text::from(lines)
    };

    let block = column_block("Feeds", app.focus == Focus::Feeds);
    frame.render_widget(Paragraph::new(text).block(block), area);
}

/// The right column: for now a placeholder (filled in a later step).
fn draw_articles(frame: &mut Frame, area: Rect, app: &App) {
    let block = column_block("Artikel", app.focus == Focus::Articles);
    let hint = match app.feeds.iter().find(|slot| !slot.articles().is_empty()) {
        Some(slot) => format!("{} Artikel geladen.", slot.articles().len()),
        None => "Noch keine Artikel.".to_string(),
    };
    frame.render_widget(Paragraph::new(hint).block(block).dim(), area);
}

/// A bordered column whose border shows whether it has the focus.
fn column_block(title: &str, focused: bool) -> Block<'static> {
    let style = if focused {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    Block::default()
        .title(title.to_string())
        .borders(Borders::ALL)
        .border_style(style)
}
