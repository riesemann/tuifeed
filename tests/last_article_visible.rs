//! Regression: the *last* article of a real feed must be shown in full, not just its heading.
//!
//! Requests: "der letzte artikel zeigt nur noch die überschrift -> soll ganz angezeigt werden".
//! Uses the real payloads in `fixtures/` and several terminal heights, so a wrong scroll rule
//! that pins the selected article's *first* line to the bottom row is caught again.

use ratatui::{Terminal, backend::TestBackend};
use tuifeed::{
    app::{App, FeedUpdate},
    config::{ColorMode, Config, Source},
    feed::{self, Feed},
    ui,
};

const HEISE: &[u8] = include_bytes!("../fixtures/heise_atom.xml");
const BBC: &[u8] = include_bytes!("../fixtures/bbc_rss.xml");
const TAGESSCHAU: &[u8] = include_bytes!("../fixtures/tagesschau_rss.xml");

/// Render the app with `feed` open and its last article selected.
fn screen_with_last_article_selected(feed: Feed, height: u16) -> Vec<String> {
    let count = feed.articles.len();
    let config = Config {
        colors: ColorMode::TrueColor,
        max_articles: 0, // show the whole feed, so the very last entry is reachable
        sources: vec![Source {
            name: Some("Test".to_string()),
            url: "https://example.org/feed".to_string(),
        }],
        ..Config::default()
    };
    let mut app = App::new(config);
    app.apply_update(FeedUpdate {
        index: 0,
        result: Ok(feed),
    });
    app.open_selected_feed();
    app.selected_article = count - 1;

    let mut terminal = Terminal::new(TestBackend::new(100, height)).unwrap();
    terminal.draw(|frame| ui::draw(frame, &app)).unwrap();
    let buffer = terminal.backend().buffer().clone();
    (0..height)
        .map(|y| (0..100).map(|x| buffer[(x, y)].symbol()).collect())
        .collect()
}

#[test]
fn the_last_article_of_every_fixture_is_shown_in_full_at_every_height() {
    let feeds = [
        ("heise", feed::parse_feed(HEISE).unwrap()),
        ("bbc", feed::parse_feed(BBC).unwrap()),
        ("tagesschau", feed::parse_feed(TAGESSCHAU).unwrap()),
    ];

    for (name, feed) in feeds {
        let last = feed.articles.last().unwrap().clone();
        let title_probe: String = last.title.chars().take(12).collect();
        let teaser_probe: String = last
            .teaser
            .as_deref()
            .unwrap_or_default()
            .chars()
            .take(24)
            .collect();

        // 10 rows is the smallest window the harness uses; taller terminals must behave the same.
        for height in [10u16, 16, 24, 30, 50] {
            let rows = screen_with_last_article_selected(feed.clone(), height);

            assert!(
                rows.iter().any(|row| row.contains(&title_probe)),
                "{name} @ {height}: the last heading must be visible:\n{}",
                rows.join("\n")
            );
            if !teaser_probe.is_empty() {
                assert!(
                    rows.iter().any(|row| row.contains(&teaser_probe)),
                    "{name} @ {height}: the last article must be shown in full, not just its heading:\n{}",
                    rows.join("\n")
                );
            }
        }
    }
}
