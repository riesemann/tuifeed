//! Rendering of the two-column layout.

use chrono::{DateTime, Local, Utc};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::{
        Block, Borders, Clear, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation,
        ScrollbarState,
    },
};

use crate::app::{App, FeedState, Focus, scroll_offset, scroll_offset_with_next};
use crate::feed::Article;
use crate::theme::{self, Theme};

/// Draw the whole interface.
pub fn draw(frame: &mut Frame, app: &App) {
    let depth = theme::depth_for(app.config.colors);
    let theme = theme::theme(app.config.theme, app.config.mode, depth);

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
    if app.show_help {
        draw_help(frame, content, &theme);
    }
}

/// A centred overlay with the key bindings.
fn draw_help(frame: &mut Frame, area: Rect, theme: &Theme) {
    let lines = vec![
        Line::from("Tastenbelegung").style(
            Style::default()
                .fg(theme.title)
                .add_modifier(Modifier::BOLD),
        ),
        Line::from(""),
        Line::from("  ↑ / ↓            Eintrag wählen"),
        Line::from("  ← / → , Tab      Spalte wechseln"),
        Line::from("  Enter            Feed öffnen / Artikel im Browser"),
        Line::from("  r                Feeds neu laden"),
        Line::from("  /                Artikel suchen (Enter übernehmen, Esc abbrechen)"),
        Line::from("  Esc              Filter zurücksetzen / Hilfe schließen"),
        Line::from("  ?                Diese Hilfe schließen"),
        Line::from("  q                Beenden"),
    ];

    let width = 64.min(area.width);
    let height = (lines.len() as u16 + 2).min(area.height);
    let popup = Rect::new(
        area.x + area.width.saturating_sub(width) / 2,
        area.y + area.height.saturating_sub(height) / 2,
        width,
        height,
    );

    frame.render_widget(Clear, popup);
    frame.render_widget(
        Paragraph::new(lines)
            .block(
                Block::default()
                    .title("Hilfe")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(theme.border_focused)),
            )
            .style(Style::default().bg(theme.bg).fg(theme.fg)),
        popup,
    );
}

/// The bottom line: the search prompt, or the key help and the status.
fn draw_footer(frame: &mut Frame, area: Rect, app: &App, theme: &Theme) {
    if let Some(text) = &app.search {
        let prompt = format!(" Suche: {text}_      (Enter übernehmen · Esc abbrechen)");
        frame.render_widget(
            Paragraph::new(prompt).style(Style::default().fg(theme.title).bg(theme.bg)),
            area,
        );
        return;
    }

    const HELP: &str = " q beenden · ↑↓ · Tab · Enter · r · / suchen · Esc Filter · ? Hilfe";

    let cells = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
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
                FeedState::Loaded(_) => Line::from(vec![
                    Span::raw(slot.label()),
                    Span::raw(format!("  ({})", app.shown_article_count(index)))
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
        Some(label) if app.filter().is_empty() => format!("Artikel — {label}"),
        Some(label) => format!("Artikel — {label}  ·  Filter: {}", app.filter()),
        None => "Artikel".to_string(),
    };
    let block = column_block(theme, &title, focused);

    let articles = app.visible_articles();
    if articles.is_empty() {
        let hint = if app.open_feed_label().is_some() {
            "Keine Artikel (oder kein Treffer). — / löscht den Filter."
        } else {
            "Feed links auswählen und mit Enter öffnen."
        };
        frame.render_widget(
            Paragraph::new(hint)
                .block(block)
                .style(Style::default().fg(theme.loading).bg(theme.bg)),
            area,
        );
        return;
    }

    // Build every line up front: a teaser wraps over several lines, so one article
    // is not one row. Remembering where each article starts lets us scroll by line and
    // keep the highlighted article in the second-to-last spot while others follow.
    let inner = block.inner(area);
    let height = inner.height as usize;

    // First pass at the full width; if the content does not fit, the scrollbar takes the last
    // column and the text is wrapped one column narrower.
    let (mut lines, mut starts) =
        article_lines(&articles, inner.width as usize, app, focused, theme);
    let needs_scrollbar = lines.len() > height;
    if needs_scrollbar {
        (lines, starts) = article_lines(
            &articles,
            (inner.width as usize).saturating_sub(1),
            app,
            focused,
            theme,
        );
    }

    let selected_start = starts.get(app.selected_article).copied().unwrap_or(0);
    let next_start = starts.get(app.selected_article + 1).copied();
    // The blank separator ends every article; the content stops one line before the next one.
    let selected_end = match next_start {
        Some(next) => next.saturating_sub(2),
        None => lines.len().saturating_sub(2),
    };
    let total = lines.len();
    let offset = scroll_offset_with_next(selected_start, selected_end, next_start, total, height);
    let paragraph = Paragraph::new(Text::from(lines))
        .block(block)
        .style(Style::default().fg(theme.fg).bg(theme.bg))
        .scroll((offset as u16, 0));
    frame.render_widget(paragraph, area);

    if needs_scrollbar {
        // ratatui places the thumb at `position / (content_length - 1)`, while we scroll lines
        // `0..=max_offset`. Mapping the two ends onto each other makes the thumb reach the bottom
        // row when the list is scrolled to its end (passing `offset` directly left it one viewport
        // short of the end).
        let max_offset = total.saturating_sub(height);
        let position = (offset * total.saturating_sub(1))
            .checked_div(max_offset)
            .unwrap_or(0);
        let mut state = ScrollbarState::new(total)
            .position(position)
            .viewport_content_length(height);
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(None)
            .end_symbol(None)
            .track_symbol(Some("│"))
            .thumb_symbol("█")
            .track_style(Style::default().fg(theme.count))
            .thumb_style(Style::default().fg(theme.title));
        frame.render_stateful_widget(scrollbar, inner, &mut state);
    }
}

/// The lines of every visible article, plus the line each article starts on.
fn article_lines(
    articles: &[&Article],
    width: usize,
    app: &App,
    focused: bool,
    theme: &Theme,
) -> (Vec<Line<'static>>, Vec<usize>) {
    let mut lines: Vec<Line> = Vec::new();
    let mut starts: Vec<usize> = Vec::with_capacity(articles.len());
    for (index, article) in articles.iter().copied().enumerate() {
        starts.push(lines.len());
        push_article(
            &mut lines,
            article,
            width,
            index == app.selected_article,
            focused,
            theme,
        );
    }
    (lines, starts)
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
        theme::theme(ThemeName::Gruvbox, Mode::Dark, theme::ColorDepth::TrueColor)
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
    fn the_selected_article_keeps_room_for_the_next_one() {
        use crate::app::FeedUpdate;
        use crate::config::{ColorMode, Config, Source};
        use crate::feed::Feed;
        use ratatui::Terminal;
        use ratatui::backend::TestBackend;

        let config = Config {
            colors: ColorMode::TrueColor,
            sources: vec![Source {
                name: Some("F".to_string()),
                url: "https://example.org/feed".to_string(),
                max: None,
            }],
            ..Config::default()
        };
        let mut app = App::new(config);
        app.apply_update(FeedUpdate {
            index: 0,
            result: Ok(Feed {
                title: "F".to_string(),
                articles: [
                    "Titel-A", "Titel-B", "Titel-C", "Titel-D", "Titel-E", "Titel-F",
                ]
                .iter()
                .map(|title| Article {
                    title: (*title).to_string(),
                    link: None,
                    date: None,
                    teaser: None,
                })
                .collect(),
            }),
        });
        app.open_selected_feed();
        app.selected_article = 3; // "Titel-D": two articles follow below

        // 100x10: one footer row, then the article column is 9 rows tall (7 inside borders).
        // Each article is a title line plus a blank, so "Titel-D" starts on line 6.
        let mut terminal = Terminal::new(TestBackend::new(100, 10)).unwrap();
        terminal.draw(|frame| draw(frame, &app)).unwrap();
        let buffer = terminal.backend().buffer().clone();

        let rows: Vec<String> = (0..10)
            .map(|y| (0..100).map(|x| buffer[(x, y)].symbol()).collect())
            .collect();

        // The highlighted article is visible …
        assert!(
            rows.iter().any(|row| row.contains("Titel-D")),
            "the selection must be visible: {rows:#?}"
        );
        // … and so is the *next* one: without the margin the last inner row would show the
        // blank separator instead.
        assert!(
            rows.iter().any(|row| row.contains("Titel-E")),
            "the next article must stay in view: {rows:#?}"
        );

        // The highlighted row really carries the selection background.
        let selected_row = rows.iter().position(|row| row.contains("Titel-D")).unwrap() as u16;
        let theme = test_theme();
        assert_eq!(buffer[(36, selected_row)].bg, theme.selection_bg);
    }

    #[test]
    fn the_last_article_is_rendered_completely() {
        use crate::app::FeedUpdate;
        use crate::config::{ColorMode, Config, Source};
        use crate::feed::Feed;
        use ratatui::Terminal;
        use ratatui::backend::TestBackend;

        let config = Config {
            colors: ColorMode::TrueColor,
            sources: vec![Source {
                name: Some("F".to_string()),
                url: "https://example.org/feed".to_string(),
                max: None,
            }],
            ..Config::default()
        };
        let mut app = App::new(config);
        app.apply_update(FeedUpdate {
            index: 0,
            result: Ok(Feed {
                title: "F".to_string(),
                articles: [
                    "Titel-A", "Titel-B", "Titel-C", "Titel-D", "Titel-E", "Titel-F",
                ]
                .iter()
                .map(|title| Article {
                    title: (*title).to_string(),
                    link: None,
                    date: None,
                    teaser: Some(format!("Teaser-{}", title.trim_start_matches("Titel-"))),
                })
                .collect(),
            }),
        });
        app.open_selected_feed();
        app.selected_article = 5; // the last article

        let mut terminal = Terminal::new(TestBackend::new(100, 10)).unwrap();
        terminal.draw(|frame| draw(frame, &app)).unwrap();
        let buffer = terminal.backend().buffer().clone();
        let rows: Vec<String> = (0..10)
            .map(|y| (0..100).map(|x| buffer[(x, y)].symbol()).collect())
            .collect();

        // Both the heading *and* the teaser of the last article must be on screen.
        assert!(
            rows.iter().any(|row| row.contains("Titel-F")),
            "the last heading must be visible: {rows:#?}"
        );
        assert!(
            rows.iter().any(|row| row.contains("Teaser-F")),
            "the last article must be shown in full, not just its heading: {rows:#?}"
        );
    }

    /// Render a feed of `count` articles (title-only) in a 100x10 terminal, `selected` highlighted.
    fn article_screen(count: usize, selected: usize) -> ratatui::buffer::Buffer {
        use crate::app::FeedUpdate;
        use crate::config::{ColorMode, Config, Source};
        use crate::feed::Feed;
        use ratatui::Terminal;
        use ratatui::backend::TestBackend;

        let config = Config {
            colors: ColorMode::TrueColor,
            sources: vec![Source {
                name: Some("F".to_string()),
                url: "https://example.org/feed".to_string(),
                max: None,
            }],
            ..Config::default()
        };
        let mut app = App::new(config);
        app.apply_update(FeedUpdate {
            index: 0,
            result: Ok(Feed {
                title: "F".to_string(),
                articles: (0..count)
                    .map(|i| Article {
                        title: format!("Titel-{i}"),
                        link: None,
                        date: None,
                        teaser: None,
                    })
                    .collect(),
            }),
        });
        app.open_selected_feed();
        app.selected_article = selected;
        let mut terminal = Terminal::new(TestBackend::new(100, 10)).unwrap();
        terminal.draw(|frame| draw(frame, &app)).unwrap();
        terminal.backend().buffer().clone()
    }

    #[test]
    fn a_scrollbar_appears_only_when_the_articles_do_not_fit() {
        let render = |count: usize| article_screen(count, 0);

        // The scrollbar sits in the last column inside the article block (x = 98 here).
        let bar = |buffer: &ratatui::buffer::Buffer| -> Vec<String> {
            (0..10)
                .map(|y| buffer[(98, y)].symbol().to_string())
                .collect()
        };

        // Two articles (four lines) fit into the seven inner rows: nothing to scroll.
        let fitting = bar(&render(2));
        assert!(
            !fitting.iter().any(|symbol| symbol == "█" || symbol == "│"),
            "no scrollbar expected, got {fitting:?}"
        );

        // Twenty articles do not fit: a track with a thumb is drawn.
        let scrolling = bar(&render(20));
        assert!(
            scrolling.iter().any(|symbol| symbol == "█"),
            "expected a scrollbar thumb, got {scrolling:?}"
        );
        assert!(
            scrolling.iter().any(|symbol| symbol == "│"),
            "expected a scrollbar track, got {scrolling:?}"
        );
    }

    #[test]
    fn the_scrollbar_thumb_reaches_the_end_of_the_track() {
        // The article block spans rows 0..8, its text rows 1..7; the bar is the x = 98 column.
        let thumb_rows = |buffer: &ratatui::buffer::Buffer| -> Vec<u16> {
            (0..10)
                .filter(|y| buffer[(98, *y)].symbol() == "█")
                .collect()
        };
        let bar = |buffer: &ratatui::buffer::Buffer| -> Vec<String> {
            (0..10)
                .map(|y| buffer[(98, y)].symbol().to_string())
                .collect()
        };

        // At the top the thumb sits on the first inner row …
        assert_eq!(
            thumb_rows(&article_screen(20, 0)).first().copied(),
            Some(1),
            "top: {:?}",
            bar(&article_screen(20, 0))
        );

        // … and with the last article selected it must reach the last inner row.
        assert_eq!(
            thumb_rows(&article_screen(20, 19)).last().copied(),
            Some(7),
            "end: {:?}",
            bar(&article_screen(20, 19))
        );
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
