//! Rendering of the two-column layout.

use chrono::{DateTime, Local, Utc};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};

use crate::app::{App, FeedState, Focus, scroll_offset};
use crate::feed::Article;
use crate::theme::{self, Theme};

/// Draw the whole interface.
pub fn draw(frame: &mut Frame, app: &App) {
    let theme = theme::theme(app.config.theme, app.config.mode);

    // Paint the window background so light and dark themes actually differ.
    frame.render_widget(
        Block::default().style(Style::default().bg(theme.bg).fg(theme.fg)),
        frame.area(),
    );

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(frame.area());
    let content = rows[0];
    let footer = rows[1];

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(app.config.layout.feeds()),
            Constraint::Percentage(app.config.layout.articles()),
        ])
        .split(content);

    draw_feeds(frame, chunks[0], app, &theme);
    draw_articles(frame, chunks[1], app, &theme);
    draw_footer(frame, footer, app, &theme);
}

/// The bottom line: the key help on the left, the status on the right.
fn draw_footer(frame: &mut Frame, area: Rect, app: &App, theme: &Theme) {
    const HELP: &str = " q beenden · ↑↓ wählen · ←→/Tab Spalte · Enter öffnen · r neu laden";

    let cells = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(area);

    frame.render_widget(
        Paragraph::new(HELP).style(Style::default().fg(theme.loading).bg(theme.bg)),
        cells[0],
    );
    frame.render_widget(
        Paragraph::new(format!("{} ", app.status()))
            .alignment(Alignment::Right)
            .style(Style::default().fg(theme.count).bg(theme.bg)),
        cells[1],
    );
}

/// The left column: one row per feed with its load state and cursor.
fn draw_feeds(frame: &mut Frame, area: Rect, app: &App, theme: &Theme) {
    let focused = app.focus == Focus::Feeds;
    let rows: Vec<ListItem> = app
        .feeds
        .iter()
        .enumerate()
        .map(|(index, slot)| {
            let line = match &slot.state {
                FeedState::Loading => Line::from(vec![
                    Span::raw(slot.label()),
                    Span::raw("  lädt …").style(Style::default().fg(theme.loading)),
                ]),
                FeedState::Loaded(feed) => Line::from(vec![
                    Span::raw(slot.label()),
                    Span::raw(format!("  ({})", feed.articles.len()))
                        .style(Style::default().fg(theme.count)),
                ]),
                FeedState::Failed(_) => Line::from(vec![
                    Span::raw(slot.label()),
                    Span::raw("  ⚠ Fehler").style(Style::default().fg(theme.error)),
                ]),
            };
            ListItem::new(line).style(row_style(theme, index == app.selected_feed, focused))
        })
        .collect();

    let block = column_block(theme, "Feeds", focused);
    let list_style = Style::default().fg(theme.fg).bg(theme.bg);

    if rows.is_empty() {
        frame.render_widget(
            Paragraph::new("Keine Feeds konfiguriert.")
                .block(block)
                .style(Style::default().fg(theme.loading).bg(theme.bg)),
            area,
        );
        return;
    }

    let offset = scroll_offset(app.selected_feed, rows.len(), inner_height(area));
    let visible = rows.into_iter().skip(offset);
    frame.render_widget(List::new(visible).block(block).style(list_style), area);
}

/// The right column: the articles of the opened feed, each as title, date and teaser.
fn draw_articles(frame: &mut Frame, area: Rect, app: &App, theme: &Theme) {
    let focused = app.focus == Focus::Articles;
    let title = match app.open_feed_label() {
        Some(label) => format!("Artikel — {label}"),
        None => "Artikel".to_string(),
    };
    let block = column_block(theme, &title, focused);

    let articles = app.articles();
    if articles.is_empty() {
        frame.render_widget(
            Paragraph::new("Feed links auswählen und mit Enter öffnen.")
                .block(block)
                .style(Style::default().fg(theme.loading).bg(theme.bg)),
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
        push_article(&mut lines, article, width, selected, focused, theme);
    }

    let offset = scroll_offset(selected_start, lines.len(), inner_height(area));
    let paragraph = Paragraph::new(Text::from(lines))
        .block(block)
        .style(Style::default().fg(theme.fg).bg(theme.bg))
        .scroll((offset as u16, 0));
    frame.render_widget(paragraph, area);
}

/// Append the lines of one article: bold title, date, teaser, blank separator.
fn push_article(
    lines: &mut Vec<Line<'static>>,
    article: &Article,
    width: usize,
    selected: bool,
    focused: bool,
    theme: &Theme,
) {
    let title_style = Style::default()
        .fg(theme.title)
        .add_modifier(Modifier::BOLD);
    for part in wrap(&article.title, width) {
        lines.push(styled_line(part, title_style, selected, focused, theme));
    }
    if let Some(date) = format_date(article.date) {
        for part in wrap(&date, width) {
            lines.push(styled_line(
                part,
                Style::default().fg(theme.date),
                selected,
                focused,
                theme,
            ));
        }
    }
    if let Some(teaser) = &article.teaser {
        for part in wrap(teaser, width) {
            lines.push(styled_line(
                part,
                Style::default().fg(theme.teaser),
                selected,
                focused,
                theme,
            ));
        }
    }
    lines.push(Line::from(""));
}

/// A line with a base style, plus the selection highlight when it is the cursor.
fn styled_line(
    text: String,
    base: Style,
    selected: bool,
    focused: bool,
    theme: &Theme,
) -> Line<'static> {
    Line::from(text).style(base.patch(row_style(theme, selected, focused)))
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
fn row_style(theme: &Theme, selected: bool, focused: bool) -> Style {
    if !selected {
        return Style::default();
    }
    let base = Style::default()
        .bg(theme.selection_bg)
        .fg(theme.selection_fg);
    if focused {
        base.add_modifier(Modifier::BOLD)
    } else {
        base
    }
}

/// The number of content rows inside a bordered area.
fn inner_height(area: Rect) -> usize {
    area.height.saturating_sub(2) as usize
}

/// A bordered column whose border shows whether it has the focus.
fn column_block(theme: &Theme, title: &str, focused: bool) -> Block<'static> {
    let border = if focused {
        theme.border_focused
    } else {
        theme.border_unfocused
    };
    Block::default()
        .title(title.to_string())
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border))
        .style(Style::default().bg(theme.bg))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Mode, ThemeName};

    fn test_theme() -> Theme {
        theme::theme(ThemeName::Gruvbox, Mode::Dark)
    }

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
        let theme = test_theme();

        // Title + teaser + separator.
        let mut lines = Vec::new();
        push_article(
            &mut lines,
            &article("Titel", None, Some("Teaser")),
            40,
            false,
            false,
            &theme,
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
            &theme,
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
            &test_theme(),
        );
        assert!(
            lines.len() > 4,
            "a wrapped teaser must occupy several lines"
        );
    }

    #[test]
    fn the_selected_row_uses_the_theme_selection_colours() {
        let theme = test_theme();
        let focused = row_style(&theme, true, true);
        assert_eq!(focused.bg, Some(theme.selection_bg));
        assert_eq!(focused.fg, Some(theme.selection_fg));

        let plain = row_style(&theme, false, true);
        assert_eq!(plain.bg, None);
    }
}
