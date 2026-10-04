//! Rendering regression tests against the real payloads in `fixtures/`.
//!
//! 1. The *last* article must be shown in full, not just its heading
//!    ("der letzte artikel zeigt nur noch die überschrift -> soll ganz angezeigt werden").
//! 2. A source's own `max` caps the list that is actually rendered
//!    ("ich habe max = 30 als config zu sources hinzugefügt").
//!
//! Both go through the public library API and a `TestBackend`, so a wrong scroll rule or a
//! cap that is only applied in the model but not on screen is caught without a terminal.

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

/// A config with one source; `max` is the caller's, the global cap is switched off.
fn config_with_source_max(max: Option<usize>) -> Config {
    Config {
        colors: ColorMode::TrueColor,
        max_articles: 0, // so a source's own `max` is what decides
        sources: vec![Source {
            name: Some("Test".to_string()),
            url: "https://example.org/feed".to_string(),
            max,
        }],
        ..Config::default()
    }
}

/// Render the app and return the screen row by row.
fn screen(
    feed: Feed,
    config: Config,
    height: u16,
    select: impl FnOnce(&mut App, usize),
) -> Vec<String> {
    let count = feed.articles.len();
    let mut app = App::new(config);
    app.apply_update(FeedUpdate {
        index: 0,
        result: Ok(feed),
    });
    app.open_selected_feed();
    select(&mut app, count);

    let mut terminal = Terminal::new(TestBackend::new(100, height)).unwrap();
    terminal.draw(|frame| ui::draw(frame, &app)).unwrap();
    let buffer = terminal.backend().buffer().clone();
    (0..height)
        .map(|y| (0..100).map(|x| buffer[(x, y)].symbol()).collect())
        .collect()
}

fn head(text: &str, chars: usize) -> String {
    text.chars().take(chars).collect()
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
        let title_probe = head(&last.title, 12);
        let teaser_probe = head(last.teaser.as_deref().unwrap_or_default(), 24);

        // 10 rows is the smallest window the pty harness uses; taller terminals must match.
        for height in [10u16, 16, 24, 30, 50] {
            let rows = screen(
                feed.clone(),
                config_with_source_max(None),
                height,
                |app, count| {
                    app.selected_article = count - 1;
                },
            );

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

#[test]
fn a_source_max_caps_the_rendered_article_list() {
    let feed = feed::parse_feed(HEISE).unwrap();
    assert!(feed.articles.len() > 5, "the fixture needs a few articles");

    // The newest article stays in the list, a later one is cut off (feed order = newest first).
    let kept = head(&feed.articles[0].title, 12);
    let dropped = head(&feed.articles[5].title, 12);
    assert_ne!(kept, dropped, "pick two distinguishable probes");

    let rows = screen(feed, config_with_source_max(Some(3)), 30, |_, _| {});

    assert!(
        rows.iter().any(|row| row.contains(&kept)),
        "the newest article must be listed:\n{}",
        rows.join("\n")
    );
    assert!(
        !rows.iter().any(|row| row.contains(&dropped)),
        "`max = 3` must keep the list short:\n{}",
        rows.join("\n")
    );
    // The feed column names the count that is actually used.
    assert!(
        rows.iter().any(|row| row.contains("(3)")),
        "the feed column must show `max`:\n{}",
        rows.join("\n")
    );
}

#[test]
fn the_article_column_gets_a_scrollbar_only_when_it_is_needed() {
    let feed = feed::parse_feed(HEISE).unwrap();
    assert!(
        feed.articles.len() > 10,
        "the fixture needs enough articles to overflow"
    );

    // x = 98 is the last column inside the article block at 100 columns / 35 % feeds column.
    let bar = |rows: &[String]| -> Vec<Option<char>> {
        rows.iter().map(|row| row.chars().nth(98)).collect()
    };

    // A whole feed in a 30-row window does not fit: track and thumb are drawn.
    let rows = screen(feed.clone(), config_with_source_max(None), 30, |_, _| {});
    let drawn = bar(&rows);
    assert!(
        drawn.contains(&Some('█')),
        "expected a scrollbar thumb:\n{}",
        rows.join("\n")
    );
    assert!(drawn.contains(&Some('│')), "expected a scrollbar track");

    // With the last article selected the thumb must reach the last inner row (27 at 30 rows).
    let rows = screen(
        feed.clone(),
        config_with_source_max(None),
        30,
        |app, count| app.selected_article = count - 1,
    );
    let thumb_rows: Vec<usize> = rows
        .iter()
        .enumerate()
        .filter(|(_, row)| row.chars().nth(98) == Some('█'))
        .map(|(y, _)| y)
        .collect();
    assert_eq!(
        thumb_rows.last().copied(),
        Some(27),
        "the thumb must reach the bottom:\n{}",
        rows.join("\n")
    );

    // Two articles fit: no scrollbar at all.
    let rows = screen(feed, config_with_source_max(Some(2)), 30, |_, _| {});
    let drawn = bar(&rows);
    assert!(
        !drawn.contains(&Some('█')) && !drawn.contains(&Some('│')),
        "no scrollbar expected:\n{}",
        rows.join("\n")
    );
}

#[test]
fn without_a_max_the_feed_column_shows_what_was_delivered() {
    let feed = feed::parse_feed(HEISE).unwrap();
    let delivered = feed.articles.len();

    let rows = screen(feed, config_with_source_max(None), 30, |_, _| {});

    assert!(
        rows.iter()
            .any(|row| row.contains(&format!("({delivered})"))),
        "the feed column must show the delivered count ({delivered}):\n{}",
        rows.join("\n")
    );
}
