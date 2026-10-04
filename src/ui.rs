//! Rendering of the two-column layout.

use chrono::{DateTime, Local, Utc};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Text},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};

use crate::app::{App, FeedState, Focus, scroll_offset};
use crate::feed::Article;

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

/// The right column: the articles of the opened feed, each as title, date and teaser.
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

    // Build every line up front: a teaser wraps over several lines, so one article
    // is not one row. Remembering where the selected article starts lets us scroll
    // it into view by line.
    let width = area.width.saturating_sub(2) as usize;
    let mut lines: Vec<Line> = Vec::new();
    let mut selected_start = 0usize;
    for (index, article) in articles.iter().enumerate() {
        let selected = index == app.selected_article;
        if selected {
            selected_start = lines.len();
        }
        push_article(&mut lines, article, width, selected, focused);
    }

    let offset = scroll_offset(selected_start, lines.len(), inner_height(area));
    let paragraph = Paragraph::new(Text::from(lines))
        .block(block)
        .scroll((offset as u16, 0));
    frame.render_widget(paragraph, area);
}

/// Append the lines of one article: bold title, muted date, teaser, blank separator.
fn push_article(
    lines: &mut Vec<Line<'static>>,
    article: &Article,
    width: usize,
    selected: bool,
    focused: bool,
) {
    let title_style = Style::default().add_modifier(Modifier::BOLD);
    for part in wrap(&article.title, width) {
        lines.push(styled_line(part, title_style, selected, focused));
    }
    if let Some(date) = format_date(article.date) {
        for part in wrap(&date, width) {
            lines.push(styled_line(
                part,
                Style::default().fg(Color::DarkGray),
                selected,
                focused,
            ));
        }
    }
    if let Some(teaser) = &article.teaser {
        for part in wrap(teaser, width) {
            lines.push(styled_line(
                part,
                Style::default().fg(Color::Gray),
                selected,
                focused,
            ));
        }
    }
    lines.push(Line::from(""));
}

/// A line with a base style, plus the selection highlight when it is the cursor.
fn styled_line(text: String, base: Style, selected: bool, focused: bool) -> Line<'static> {
    Line::from(text).style(base.patch(row_style(selected, focused)))
}

/// Format an article date as local time `YYYY-MM-DD HH:MM`.
pub fn format_date(date: Option<DateTime<Utc>>) -> Option<String> {
    date.map(|date| {
        date.with_timezone(&Local)
            .format("%Y-%m-%d %H:%M")
            .to_string()
    })
}

/// Wrap `text` to `width` columns at word boundaries (hard-splitting long words).
pub fn wrap(text: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![text.to_string()];
    }
    let mut lines: Vec<String> = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        if current.is_empty() {
            current.push_str(word);
        } else if current.chars().count() + 1 + word.chars().count() <= width {
            current.push(' ');
            current.push_str(word);
        } else {
            lines.push(std::mem::take(&mut current));
            current.push_str(word);
        }
        while current.chars().count() > width {
            let cut = current
                .char_indices()
                .nth(width)
                .map(|(index, _)| index)
                .unwrap_or(current.len());
            let rest = current.split_off(cut);
            lines.push(std::mem::take(&mut current));
            current = rest;
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
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

#[cfg(test)]
mod tests {
    use super::*;

    fn article(title: &str, date: Option<DateTime<Utc>>, teaser: Option<&str>) -> Article {
        Article {
            title: title.to_string(),
            link: Some("https://example.org".to_string()),
            date,
            teaser: teaser.map(str::to_string),
        }
    }

    #[test]
    fn wrap_keeps_short_text_on_one_line() {
        assert_eq!(wrap("hello world", 20), vec!["hello world"]);
    }

    #[test]
    fn wrap_breaks_at_word_boundaries() {
        assert_eq!(wrap("hello world", 5), vec!["hello", "world"]);
    }

    #[test]
    fn wrap_hard_splits_a_word_that_is_too_long() {
        assert_eq!(wrap("abcdefgh", 3), vec!["abc", "def", "gh"]);
    }

    #[test]
    fn wrap_of_empty_text_is_one_empty_line() {
        assert_eq!(wrap("", 10), vec![""]);
        assert_eq!(wrap("   ", 10), vec![""]);
    }

    #[test]
    fn a_date_is_formatted_as_yyyy_mm_dd_hh_mm() {
        assert_eq!(format_date(None), None);
        let date = DateTime::<Utc>::from_timestamp(0, 0).unwrap();
        let text = format_date(Some(date)).unwrap();
        assert_eq!(text.len(), 16, "unexpected format: {text}");
        assert_eq!(text.as_bytes()[4], b'-');
        assert_eq!(text.as_bytes()[7], b'-');
        assert_eq!(text.as_bytes()[10], b' ');
        assert_eq!(text.as_bytes()[13], b':');
    }

    #[test]
    fn an_article_is_drawn_as_its_lines_plus_a_separator() {
        // Title + teaser + separator.
        let mut lines = Vec::new();
        push_article(
            &mut lines,
            &article("Titel", None, Some("Teaser")),
            40,
            false,
            false,
        );
        assert_eq!(lines.len(), 3);

        // Title + date + teaser + separator.
        let mut lines = Vec::new();
        push_article(
            &mut lines,
            &article(
                "Titel",
                DateTime::<Utc>::from_timestamp(0, 0),
                Some("Teaser"),
            ),
            40,
            false,
            false,
        );
        assert_eq!(lines.len(), 4);
    }

    #[test]
    fn a_long_teaser_uses_more_lines() {
        let long = "wort ".repeat(60);
        let mut lines = Vec::new();
        push_article(
            &mut lines,
            &article("Titel", None, Some(long.trim())),
            20,
            false,
            false,
        );
        assert!(
            lines.len() > 4,
            "a wrapped teaser must occupy several lines"
        );
    }
}
