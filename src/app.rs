//! Application state and input handling.

use crossterm::event::{KeyCode, KeyModifiers};

use crate::config::Config;
use crate::feed::{Article, Feed};

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

/// The load state of one feed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FeedState {
    /// The feed is being fetched (or was just re-requested).
    Loading,
    /// The feed was fetched and parsed.
    Loaded(Feed),
    /// The feed could not be fetched or parsed, with the reason.
    Failed(String),
}

/// One configured feed plus its current content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeedSlot {
    /// The name from the configuration (may be empty; then the feed's title is used).
    pub name: String,
    /// The feed URL.
    pub url: String,
    /// What we currently know about the feed.
    pub state: FeedState,
}

impl FeedSlot {
    /// What to show in the feed list.
    pub fn label(&self) -> String {
        if !self.name.is_empty() {
            return self.name.clone();
        }
        match &self.state {
            FeedState::Loaded(feed) => feed.title.clone(),
            _ => self.url.clone(),
        }
    }

    /// The articles, if the feed has been loaded.
    pub fn articles(&self) -> &[Article] {
        match &self.state {
            FeedState::Loaded(feed) => &feed.articles,
            _ => &[],
        }
    }
}

/// A result of a background fetch, sent back to the UI loop.
#[derive(Debug)]
pub struct FeedUpdate {
    /// Which feed this is about (index into [`App::feeds`]).
    pub index: usize,
    /// The feed or the reason it failed.
    pub result: Result<Feed, String>,
}

/// The whole application state.
#[derive(Debug)]
pub struct App {
    pub should_quit: bool,
    /// Set when the user asked to reload the feeds.
    pub reload: bool,
    pub focus: Focus,
    pub config: Config,
    /// One slot per configured source.
    pub feeds: Vec<FeedSlot>,
    /// The cursor in the feed list.
    pub selected_feed: usize,
    /// The cursor in the article list.
    pub selected_article: usize,
    /// The feed whose articles the right column shows (set by Enter).
    pub open_feed: Option<usize>,
    /// An address the user asked to open in the browser (taken by the main loop).
    pub pending_open: Option<String>,
}

impl Default for App {
    fn default() -> Self {
        Self::new(Config::default())
    }
}

impl App {
    pub fn new(config: Config) -> Self {
        let feeds = config
            .sources
            .iter()
            .map(|source| FeedSlot {
                name: source.name.clone().unwrap_or_default(),
                url: source.url.clone(),
                state: FeedState::Loading,
            })
            .collect();
        Self {
            should_quit: false,
            reload: false,
            focus: Focus::Feeds,
            config,
            feeds,
            selected_feed: 0,
            selected_article: 0,
            open_feed: None,
            pending_open: None,
        }
    }

    /// Mark every feed as loading again (used on start and on reload).
    pub fn begin_load(&mut self) {
        for slot in &mut self.feeds {
            slot.state = FeedState::Loading;
        }
    }

    /// Store the result of a background fetch.
    pub fn apply_update(&mut self, update: FeedUpdate) {
        if let Some(slot) = self.feeds.get_mut(update.index) {
            slot.state = match update.result {
                Ok(feed) => FeedState::Loaded(feed),
                Err(error) => FeedState::Failed(error),
            };
        }
    }

    /// Switch the focused column (Tab / Shift-Tab).
    pub fn toggle_column(&mut self) {
        self.focus = self.focus.other();
    }

    /// Focus the left column.
    pub fn focus_feeds(&mut self) {
        self.focus = Focus::Feeds;
    }

    /// Focus the right column.
    pub fn focus_articles(&mut self) {
        self.focus = Focus::Articles;
    }

    /// The articles shown in the right column (of the opened feed).
    pub fn articles(&self) -> &[Article] {
        self.open_feed
            .and_then(|index| self.feeds.get(index))
            .map(FeedSlot::articles)
            .unwrap_or(&[])
    }

    /// The name of the opened feed, if any.
    pub fn open_feed_label(&self) -> Option<String> {
        self.open_feed
            .and_then(|index| self.feeds.get(index))
            .map(FeedSlot::label)
    }

    /// A short status for the footer.
    pub fn status(&self) -> String {
        if self.feeds.is_empty() {
            return "Keine Feeds konfiguriert.".to_string();
        }
        let loading = self
            .feeds
            .iter()
            .filter(|slot| matches!(slot.state, FeedState::Loading))
            .count();
        let failed = self
            .feeds
            .iter()
            .filter(|slot| matches!(slot.state, FeedState::Failed(_)))
            .count();
        let articles: usize = self.feeds.iter().map(|slot| slot.articles().len()).sum();

        if loading > 0 {
            format!("Lade Feeds … ({loading} von {})", self.feeds.len())
        } else if failed > 0 {
            format!(
                "{} Feeds · {failed} fehlgeschlagen · {articles} Artikel",
                self.feeds.len()
            )
        } else {
            format!("{} Feeds · {articles} Artikel", self.feeds.len())
        }
    }

    /// Move the cursor in the focused column by `delta` rows.
    pub fn move_selection(&mut self, delta: isize) {
        match self.focus {
            Focus::Feeds => {
                self.selected_feed = step(self.selected_feed, delta, self.feeds.len());
            }
            Focus::Articles => {
                self.selected_article = step(self.selected_article, delta, self.articles().len());
            }
        }
    }

    /// Show the selected feed in the right column and move the focus there.
    pub fn open_selected_feed(&mut self) {
        if self.focus == Focus::Feeds && !self.feeds.is_empty() {
            self.open_feed = Some(self.selected_feed);
            self.selected_article = 0;
            self.focus = Focus::Articles;
        }
    }

    /// Ask for the selected article's address to be opened in the browser.
    ///
    /// Does nothing if there is no open feed, no article, or the article has no link.
    pub fn queue_open_selected_article(&mut self) {
        if let Some(link) = self
            .articles()
            .get(self.selected_article)
            .and_then(|article| article.link.clone())
        {
            self.pending_open = Some(link);
        }
    }

    /// Take a pending browser request, if any.
    pub fn take_pending_open(&mut self) -> Option<String> {
        self.pending_open.take()
    }

    /// Handle a key press.
    pub fn on_key(&mut self, code: KeyCode, modifiers: KeyModifiers) {
        match code {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => {
                self.should_quit = true;
            }
            KeyCode::Char('r') => self.reload = true,
            KeyCode::Up => self.move_selection(-1),
            KeyCode::Down => self.move_selection(1),
            KeyCode::Left => self.focus_feeds(),
            KeyCode::Right => self.focus_articles(),
            KeyCode::Enter => {
                if self.focus == Focus::Feeds {
                    self.open_selected_feed();
                } else {
                    self.queue_open_selected_article();
                }
            }
            KeyCode::Tab | KeyCode::BackTab => self.toggle_column(),
            _ => {}
        }
    }
}

/// Move an index by `delta`, clamped into `0..len` (an empty list keeps index 0).
fn step(index: usize, delta: isize, len: usize) -> usize {
    if len == 0 {
        return 0;
    }
    (index as isize + delta).clamp(0, len as isize - 1) as usize
}

/// The first row to draw so that `selected` is visible in a window of `height` rows.
pub fn scroll_offset(selected: usize, len: usize, height: usize) -> usize {
    if height == 0 {
        return 0;
    }
    let max_offset = len.saturating_sub(height);
    selected
        .saturating_sub(height.saturating_sub(1))
        .min(max_offset)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Source, ThemeName};

    fn config_with_sources() -> Config {
        Config {
            sources: vec![
                Source {
                    name: Some("Heise".to_string()),
                    url: "https://a.example/feed".to_string(),
                },
                Source {
                    name: None,
                    url: "https://b.example/feed".to_string(),
                },
            ],
            ..Config::default()
        }
    }

    fn feed(title: &str, articles: usize) -> Feed {
        Feed {
            title: title.to_string(),
            articles: (0..articles)
                .map(|i| Article {
                    title: format!("Artikel {i}"),
                    link: Some(format!("https://example.org/{i}")),
                    date: None,
                    teaser: None,
                })
                .collect(),
        }
    }

    fn loaded_app() -> App {
        let mut app = App::new(config_with_sources());
        app.apply_update(FeedUpdate {
            index: 0,
            result: Ok(feed("Erster", 3)),
        });
        app.apply_update(FeedUpdate {
            index: 1,
            result: Ok(feed("Zweiter", 5)),
        });
        app
    }

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
    fn left_and_right_arrows_pick_a_column() {
        let mut app = App::default();
        app.on_key(KeyCode::Right, KeyModifiers::NONE);
        assert_eq!(app.focus, Focus::Articles);
        app.on_key(KeyCode::Left, KeyModifiers::NONE);
        assert_eq!(app.focus, Focus::Feeds);
    }

    #[test]
    fn ordinary_keys_do_not_quit() {
        let mut app = App::default();
        app.on_key(KeyCode::Char('x'), KeyModifiers::NONE);
        assert!(!app.should_quit);
    }

    #[test]
    fn r_asks_for_a_reload() {
        let mut app = App::default();
        app.on_key(KeyCode::Char('r'), KeyModifiers::NONE);
        assert!(app.reload);
    }

    #[test]
    fn every_source_becomes_a_feed_slot_in_loading_state() {
        let app = App::new(config_with_sources());
        assert_eq!(app.feeds.len(), 2);
        assert!(app.feeds.iter().all(|s| s.state == FeedState::Loading));
    }

    #[test]
    fn a_loaded_feed_shows_its_title_when_no_name_is_configured() {
        let mut app = App::new(config_with_sources());
        app.apply_update(FeedUpdate {
            index: 1,
            result: Ok(feed("Beispiel-Feed", 3)),
        });
        assert_eq!(app.feeds[0].label(), "Heise"); // configured name wins
        assert_eq!(app.feeds[1].label(), "Beispiel-Feed"); // feed title
        assert_eq!(app.feeds[1].articles().len(), 3);
    }

    #[test]
    fn a_failed_feed_keeps_a_reason_and_offers_no_articles() {
        let mut app = App::new(config_with_sources());
        app.apply_update(FeedUpdate {
            index: 0,
            result: Err("HTTP 500".to_string()),
        });
        assert_eq!(
            app.feeds[0].state,
            FeedState::Failed("HTTP 500".to_string())
        );
        assert!(app.feeds[0].articles().is_empty());
        assert_eq!(app.feeds[0].label(), "Heise");
    }

    #[test]
    fn begin_load_puts_every_slot_back_to_loading() {
        let mut app = App::new(config_with_sources());
        app.apply_update(FeedUpdate {
            index: 0,
            result: Ok(feed("X", 1)),
        });
        app.begin_load();
        assert!(app.feeds.iter().all(|s| s.state == FeedState::Loading));
    }

    #[test]
    fn the_config_defaults_to_gruvbox() {
        let app = App::new(config_with_sources());
        assert_eq!(app.config.theme, ThemeName::Gruvbox);
    }

    #[test]
    fn the_status_counts_feeds_and_articles() {
        let app = loaded_app();
        assert_eq!(app.status(), "2 Feeds · 8 Artikel");
    }

    #[test]
    fn the_status_says_while_feeds_are_still_loading() {
        let app = App::new(config_with_sources());
        assert!(app.status().contains("Lade Feeds"), "got: {}", app.status());
    }

    #[test]
    fn the_status_names_the_failures() {
        let mut app = App::new(config_with_sources());
        app.apply_update(FeedUpdate {
            index: 0,
            result: Err("HTTP 500".to_string()),
        });
        app.apply_update(FeedUpdate {
            index: 1,
            result: Ok(feed("Zweiter", 5)),
        });
        assert!(
            app.status().contains("fehlgeschlagen"),
            "got: {}",
            app.status()
        );
    }

    #[test]
    fn the_status_of_an_empty_configuration_says_so() {
        let app = App::new(Config::default());
        assert_eq!(app.status(), "Keine Feeds konfiguriert.");
    }

    #[test]
    fn the_feed_cursor_stays_within_the_list() {
        let mut app = loaded_app();
        app.move_selection(-1); // already at the top
        assert_eq!(app.selected_feed, 0);
        app.move_selection(1);
        assert_eq!(app.selected_feed, 1);
        app.move_selection(1); // past the end
        assert_eq!(app.selected_feed, 1);
    }

    #[test]
    fn enter_opens_the_selected_feed_and_focuses_the_articles() {
        let mut app = loaded_app();
        app.move_selection(1);
        app.on_key(KeyCode::Enter, KeyModifiers::NONE);
        assert_eq!(app.open_feed, Some(1));
        assert_eq!(app.focus, Focus::Articles);
        assert_eq!(app.articles().len(), 5);
        assert_eq!(app.open_feed_label().as_deref(), Some("Zweiter"));
    }

    #[test]
    fn the_article_cursor_stays_within_the_open_feed() {
        let mut app = loaded_app();
        app.open_selected_feed(); // opens feed 0 (3 articles)
        assert_eq!(app.articles().len(), 3);
        app.move_selection(1);
        app.move_selection(1);
        app.move_selection(1); // past the end
        assert_eq!(app.selected_article, 2);
        app.move_selection(-5);
        assert_eq!(app.selected_article, 0);
    }

    #[test]
    fn enter_in_the_article_column_asks_for_the_browser_and_keeps_the_feed() {
        let mut app = loaded_app();
        app.open_selected_feed();
        app.on_key(KeyCode::Enter, KeyModifiers::NONE);
        assert_eq!(app.open_feed, Some(0));
        assert_eq!(app.focus, Focus::Articles);
        assert_eq!(app.pending_open.as_deref(), Some("https://example.org/0"));
    }

    #[test]
    fn the_pending_open_is_taken_once() {
        let mut app = loaded_app();
        app.open_selected_feed();
        app.queue_open_selected_article();
        assert_eq!(
            app.take_pending_open().as_deref(),
            Some("https://example.org/0")
        );
        assert_eq!(app.take_pending_open(), None);
    }

    #[test]
    fn an_article_without_a_link_does_not_ask_for_the_browser() {
        let mut app = App::new(config_with_sources());
        app.apply_update(FeedUpdate {
            index: 0,
            result: Ok(Feed {
                title: "Ohne Link".to_string(),
                articles: vec![Article {
                    title: "Kein Link".to_string(),
                    link: None,
                    date: None,
                    teaser: None,
                }],
            }),
        });
        app.open_selected_feed();
        app.queue_open_selected_article();
        assert_eq!(app.pending_open, None);
    }

    #[test]
    fn an_empty_feed_list_cannot_be_opened() {
        let mut app = App::new(Config::default());
        app.on_key(KeyCode::Enter, KeyModifiers::NONE);
        assert_eq!(app.open_feed, None);
    }

    #[test]
    fn without_an_open_feed_there_are_no_articles() {
        let app = loaded_app();
        assert!(app.articles().is_empty());
        assert_eq!(app.open_feed_label(), None);
    }

    #[test]
    fn the_scroll_offset_keeps_the_selection_visible() {
        // Short list: never scroll.
        assert_eq!(scroll_offset(0, 3, 10), 0);
        assert_eq!(scroll_offset(2, 3, 10), 0);
        // Long list: scroll just enough to show the last row.
        assert_eq!(scroll_offset(0, 100, 10), 0);
        assert_eq!(scroll_offset(9, 100, 10), 0);
        assert_eq!(scroll_offset(10, 100, 10), 1);
        assert_eq!(scroll_offset(99, 100, 10), 90);
        // Degenerate cases.
        assert_eq!(scroll_offset(5, 0, 10), 0);
        assert_eq!(scroll_offset(5, 10, 0), 0);
    }
}
