//! Rendering of the two-column layout.

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::Line,
    widgets::{Block, Borders, List, ListItem, Paragraph},
};

use crate::app::{App, FeedState, Focus, scroll_offset};

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

/// The left column: one row per feed with its load state and cursor.
fn draw_feeds(frame: &mut Frame, area: Rect, app: &App) {
    let focused = app.focus == Focus::Feeds;
    let rows: Vec<ListItem> = app
        .feeds
        .iter()
        .enumerate()
        .map(|(index, slot)| {
            let line = match &slot.state {
                FeedState::Loading => Line::from(format!("{}  lädt …", slot.label())).dim(),
                FeedState::Loaded(feed) => {
                    Line::from(format!("{}  ({})", slot.label(), feed.articles.len()))
                }
                FeedState::Failed(_) => Line::from(format!("{}  ⚠ Fehler", slot.label())).red(),
            };
            ListItem::new(line).style(row_style(index == app.selected_feed, focused))
        })
        .collect();

    let block = column_block("Feeds", focused);
    if rows.is_empty() {
        frame.render_widget(
            Paragraph::new("Keine Feeds konfiguriert.")
                .block(block)
                .dim(),
            area,
        );
        return;
    }

    let offset = scroll_offset(app.selected_feed, rows.len(), inner_height(area));
    let visible = rows.into_iter().skip(offset);
    frame.render_widget(List::new(visible).block(block), area);
}

/// The right column: the articles of the opened feed, with cursor.
fn draw_articles(frame: &mut Frame, area: Rect, app: &App) {
    let focused = app.focus == Focus::Articles;
    let title = match app.open_feed_label() {
        Some(label) => format!("Artikel — {label}"),
        None => "Artikel".to_string(),
    };
    let block = column_block(&title, focused);

    let articles = app.articles();
    if articles.is_empty() {
        frame.render_widget(
            Paragraph::new("Feed links auswählen und mit Enter öffnen.")
                .block(block)
                .dim(),
            area,
        );
        return;
    }

    let rows: Vec<ListItem> = articles
        .iter()
        .enumerate()
        .map(|(index, article)| {
            ListItem::new(Line::from(article.title.clone()))
                .style(row_style(index == app.selected_article, focused))
        })
        .collect();

    let offset = scroll_offset(app.selected_article, rows.len(), inner_height(area));
    let visible = rows.into_iter().skip(offset);
    frame.render_widget(List::new(visible).block(block), area);
}

/// The style of a list row: highlighted when it is the cursor, given the focus.
fn row_style(selected: bool, focused: bool) -> Style {
    if !selected {
        return Style::default();
    }
    if focused {
        Style::default()
            .bg(Color::Blue)
            .fg(Color::White)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().bg(Color::DarkGray).fg(Color::White)
    }
}

/// The number of content rows inside a bordered area.
fn inner_height(area: Rect) -> usize {
    area.height.saturating_sub(2) as usize
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
