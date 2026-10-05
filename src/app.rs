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
    /// Whether the key help overlay is shown.
    pub show_help: bool,
    /// The text being typed while the search is open (`None` = not searching).
    pub search: Option<String>,
    /// The committed search filter applied to the article list.
    pub filter: String,
    /// When set, only this feed is shown: the feed column is hidden, the articles fill the
    /// window and the focus stays there (set by `--feed`).
    pub only_feed: Option<usize>,
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
            show_help: false,
            search: None,
            filter: String::new(),
            only_feed: None,
        }
    }

    /// Restrict the interface to a single configured feed.
    ///
    /// The feed is found by its number (as `--list` shows it) or by its name (case-insensitive).
    /// The feed column is hidden and its articles fill the window.
    ///
    /// Fails with a hint listing the configured feeds when nothing matches.
    pub fn restrict_to_feed(&mut self, feed: &str) -> Result<(), String> {
        let wanted = feed.trim();
        if wanted.is_empty() {
            return Err(format!("empty feed. Available: {}", self.feed_names()));
        }
        let needle = wanted.to_lowercase();
        // A name wins over a number, so a feed literally called "2" stays reachable.
        let by_name = self
            .feeds
            .iter()
            .position(|slot| !slot.name.is_empty() && slot.name.to_lowercase() == needle);
        let by_number = wanted
            .parse::<usize>()
            .ok()
            .and_then(|number| number.checked_sub(1))
            .filter(|index| *index < self.feeds.len());
        match by_name.or(by_number) {
            Some(index) => {
                self.only_feed = Some(index);
                self.selected_feed = index;
                self.open_feed = Some(index);
                self.selected_article = 0;
                self.focus = Focus::Articles;
                Ok(())
            }
            None => Err(format!(
                "no feed \"{feed}\". Available: {}",
                self.feed_names()
            )),
        }
    }

    /// Whether the feed column is drawn (it is not when a single feed is pinned).
    pub fn shows_feed_column(&self) -> bool {
        self.only_feed.is_none()
    }

    /// The configured feeds as `1 = Heise, 2 = (no name)`, for messages.
    pub fn feed_names(&self) -> String {
        if self.feeds.is_empty() {
            return "(none)".to_string();
        }
        self.feeds
            .iter()
            .enumerate()
            .map(|(index, slot)| {
                let label = if slot.name.trim().is_empty() {
                    "(no name)"
                } else {
                    slot.name.as_str()
                };
                format!("{} = {label}", index + 1)
            })
            .collect::<Vec<_>>()
            .join(", ")
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

    /// Switch the focused column (Tab / Shift-Tab). Without effect while a single feed is pinned.
    pub fn toggle_column(&mut self) {
        if self.only_feed.is_some() {
            return;
        }
        self.focus = self.focus.other();
    }

    /// Focus the left column. Without effect while a single feed is pinned (there is no list).
    pub fn focus_feeds(&mut self) {
        if self.only_feed.is_some() {
            return;
        }
        self.focus = Focus::Feeds;
    }

    /// Focus the right column.
    pub fn focus_articles(&mut self) {
        self.focus = Focus::Articles;
    }

    /// The articles actually shown in the right column.
    ///
    /// The opened feed's articles, capped at `max_articles` (`0` = all) and filtered by
    /// the current search needle (matches the title or the teaser, case-insensitive).
    pub fn visible_articles(&self) -> Vec<&Article> {
        let Some(index) = self.open_feed else {
            return Vec::new();
        };
        let Some(slot) = self.feeds.get(index) else {
            return Vec::new();
        };
        let all = slot.articles();
        let cap = self.article_limit(index);
        let limit = if cap == 0 { all.len() } else { cap };
        let needle = self.needle();
        all.iter()
            .take(limit)
            .filter(|article| article_matches(article, &needle))
            .collect()
    }

    /// The article cap for one feed: the source's own `max`, else the global `max_articles`
    /// (`0` meaning "no cap" in both places).
    pub fn article_limit(&self, index: usize) -> usize {
        self.config
            .sources
            .get(index)
            .and_then(|source| source.max)
            .unwrap_or(self.config.max_articles)
    }

    /// How many articles this feed contributes to the list: the delivered count, capped by
    /// [`Self::article_limit`]. This is the number shown in the feed column.
    pub fn shown_article_count(&self, index: usize) -> usize {
        let Some(slot) = self.feeds.get(index) else {
            return 0;
        };
        let delivered = slot.articles().len();
        let cap = self.article_limit(index);
        if cap == 0 {
            delivered
        } else {
            delivered.min(cap)
        }
    }

    /// The needle currently filtering the article list.
    fn needle(&self) -> String {
        match &self.search {
            Some(text) => text.to_lowercase(),
            None => self.filter.to_lowercase(),
        }
    }

    /// The committed search filter (for the UI).
    pub fn filter(&self) -> &str {
        &self.filter
    }

    /// Begin typing a search.
    fn start_search(&mut self) {
        self.search = Some(String::new());
        self.selected_article = 0;
    }

    /// Keep the typed filter and return to the list.
    fn commit_search(&mut self) {
        if let Some(text) = self.search.take() {
            self.filter = text;
        }
        self.selected_article = 0;
        self.focus = Focus::Articles;
    }

    /// Drop the search: nothing typed, nothing filtered.
    fn cancel_search(&mut self) {
        self.search = None;
        self.filter.clear();
        self.selected_article = 0;
    }

    /// Drop the committed search filter, bringing the whole article list back.
    pub fn clear_filter(&mut self) {
        self.filter.clear();
        self.selected_article = 0;
    }

    /// Handle a key while the search is open.
    fn on_search_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Esc => self.cancel_search(),
            KeyCode::Enter => self.commit_search(),
            KeyCode::Backspace => {
                if let Some(text) = &mut self.search {
                    text.pop();
                }
                self.selected_article = 0;
            }
            KeyCode::Char(ch) => {
                if let Some(text) = &mut self.search {
                    text.push(ch);
                }
                self.selected_article = 0;
            }
            _ => {}
        }
    }

    /// The name of the opened feed, if any.
    pub fn open_feed_label(&self) -> Option<String> {
        self.open_feed
            .and_then(|index| self.feeds.get(index))
            .map(FeedSlot::label)
    }

    /// The load state of the feed shown in the article column, if any.
    pub fn open_feed_state(&self) -> Option<&FeedState> {
        self.open_feed
            .and_then(|index| self.feeds.get(index))
            .map(|slot| &slot.state)
    }

    /// A short status for the footer.
    pub fn status(&self) -> String {
        if let Some(index) = self.only_feed {
            return self.single_feed_status(index);
        }
        if self.feeds.is_empty() {
            return "No feeds configured.".to_string();
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
        // The used count, capped per feed — the same number the feed column shows, so the
        // footer never disagrees with the list beside it.
        let articles: usize = (0..self.feeds.len())
            .map(|index| self.shown_article_count(index))
            .sum();

        if loading > 0 {
            format!("Loading feeds … ({loading} of {})", self.feeds.len())
        } else if failed > 0 {
            format!(
                "{} · {failed} failed · {}",
                count_of(self.feeds.len(), "feed", "feeds"),
                count_of(articles, "article", "articles")
            )
        } else {
            format!(
                "{} · {}",
                count_of(self.feeds.len(), "feed", "feeds"),
                count_of(articles, "article", "articles")
            )
        }
    }

    /// The footer text while a single feed is pinned: only that feed, with the count in use.
    fn single_feed_status(&self, index: usize) -> String {
        let Some(slot) = self.feeds.get(index) else {
            return "No feeds configured.".to_string();
        };
        match &slot.state {
            FeedState::Loading => "Loading feed …".to_string(),
            FeedState::Failed(_) => format!("{} · failed", slot.label()),
            FeedState::Loaded(_) => format!(
                "{} · {}",
                slot.label(),
                count_of(self.shown_article_count(index), "article", "articles")
            ),
        }
    }

    /// Move the cursor in the focused column by `delta` rows.
    pub fn move_selection(&mut self, delta: isize) {
        match self.focus {
            Focus::Feeds => {
                self.selected_feed = step(self.selected_feed, delta, self.feeds.len());
            }
            Focus::Articles => {
                self.selected_article =
                    step(self.selected_article, delta, self.visible_articles().len());
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
            .visible_articles()
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
        // While the search is open, keys type into it.
        if self.search.is_some() {
            self.on_search_key(code);
            return;
        }
        match code {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => {
                self.should_quit = true;
            }
            KeyCode::Char('r') => self.reload = true,
            KeyCode::Char('?') => self.show_help = !self.show_help,
            KeyCode::Char('/') => self.start_search(),
            KeyCode::Esc => {
                self.show_help = false;
                self.clear_filter();
            }
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

/// Whether an article matches the (lowercase) search needle.
fn article_matches(article: &Article, needle: &str) -> bool {
    if needle.is_empty() {
        return true;
    }
    article.title.to_lowercase().contains(needle)
        || article
            .teaser
            .as_deref()
            .is_some_and(|teaser| teaser.to_lowercase().contains(needle))
}

/// `1 feed` / `3 feeds`.
fn count_of(count: usize, singular: &str, plural: &str) -> String {
    format!("{count} {}", if count == 1 { singular } else { plural })
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

/// Like [`scroll_offset`], but shows the whole selected article where it fits.
///
/// `selected_start`/`selected_end` are the first and last *content* line of the highlighted
/// article (the blank separator is not counted), `next_start` the first line of the article
/// after it. As long as another article follows, at least its first line stays on screen —
/// the highlighted entry sits in the second-to-last spot. The last article (no `next_start`)
/// is shown in full whenever it fits, instead of only its heading at the bottom row.
pub fn scroll_offset_with_next(
    selected_start: usize,
    selected_end: usize,
    next_start: Option<usize>,
    len: usize,
    height: usize,
) -> usize {
    if height == 0 {
        return 0;
    }
    let max_offset = len.saturating_sub(height);
    let block = selected_end.saturating_sub(selected_start) + 1;

    let offset = if block >= height {
        // Taller than the window: show it from its top, there is no way to show it all.
        selected_start
    } else {
        // The block's end on the bottom row is the lowest offset that still shows it all;
        // its start on the top row is the highest one.
        let lowest = (selected_end + 1).saturating_sub(height);
        let highest = selected_start;
        let prefer = match next_start {
            Some(next) => next.saturating_sub(height - 1),
            None => lowest,
        };
        prefer.max(lowest).min(highest)
    };
    offset.min(max_offset)
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
                    max: None,
                },
                Source {
                    name: None,
                    url: "https://b.example/feed".to_string(),
                    max: None,
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
        assert_eq!(app.status(), "2 feeds · 8 articles");
    }

    #[test]
    fn the_status_article_count_respects_the_caps() {
        let mut app = loaded_app();
        // Feed 0: its own `max`; feed 1: the global fallback. The footer must show the
        // numbers the feed column shows, not the raw delivered ones.
        app.config.sources[0].max = Some(2);
        app.config.max_articles = 4;
        assert_eq!(app.status(), "2 feeds · 6 articles");

        // A cap never claims more than the feed delivers (5 delivered, `max` 500).
        app.config.sources[0].max = Some(500);
        app.config.max_articles = 0;
        assert_eq!(app.status(), "2 feeds · 8 articles");
    }

    #[test]
    fn the_status_says_while_feeds_are_still_loading() {
        let app = App::new(config_with_sources());
        assert!(
            app.status().contains("Loading feeds"),
            "got: {}",
            app.status()
        );
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
        assert!(app.status().contains("failed"), "got: {}", app.status());
    }

    #[test]
    fn the_status_of_an_empty_configuration_says_so() {
        let app = App::new(Config::default());
        assert_eq!(app.status(), "No feeds configured.");
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
        assert_eq!(app.visible_articles().len(), 5);
        assert_eq!(app.open_feed_label().as_deref(), Some("Zweiter"));
    }

    #[test]
    fn the_article_cursor_stays_within_the_open_feed() {
        let mut app = loaded_app();
        app.open_selected_feed(); // opens feed 0 (3 articles)
        assert_eq!(app.visible_articles().len(), 3);
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
        assert!(app.visible_articles().is_empty());
        assert_eq!(app.open_feed_label(), None);
    }

    #[test]
    fn question_mark_toggles_the_help() {
        let mut app = App::default();
        app.on_key(KeyCode::Char('?'), KeyModifiers::NONE);
        assert!(app.show_help);
        app.on_key(KeyCode::Char('?'), KeyModifiers::NONE);
        assert!(!app.show_help);
    }

    #[test]
    fn the_feed_column_number_is_the_count_actually_used() {
        let mut app = App::new(config_with_sources());
        app.apply_update(FeedUpdate {
            index: 0,
            result: Ok(feed("Erster", 10)),
        });
        app.apply_update(FeedUpdate {
            index: 1,
            result: Ok(feed("Zweiter", 120)),
        });

        // Without any limit every delivered article is used.
        assert_eq!(app.shown_article_count(0), 10);
        assert_eq!(app.shown_article_count(1), 120);

        // A source's `max` lowers it …
        app.config.sources[1].max = Some(5);
        assert_eq!(app.shown_article_count(1), 5);

        // … but never claims more than the feed delivers.
        app.config.sources[1].max = Some(500);
        assert_eq!(app.shown_article_count(1), 120);

        // The global limit is the fallback for feeds without their own `max`.
        app.config.max_articles = 7;
        assert_eq!(app.shown_article_count(0), 7);
        assert_eq!(app.shown_article_count(1), 120); // still the source's own max

        // An unknown feed has no articles.
        assert_eq!(app.shown_article_count(9), 0);
    }

    #[test]
    fn a_source_max_overrides_the_global_article_limit() {
        let mut app = App::new(config_with_sources());
        app.apply_update(FeedUpdate {
            index: 0,
            result: Ok(feed("Erster", 10)),
        });
        app.apply_update(FeedUpdate {
            index: 1,
            result: Ok(feed("Zweiter", 10)),
        });
        app.config.max_articles = 8;
        app.config.sources[0].max = Some(3);

        app.open_selected_feed(); // feed 0: its own `max` wins
        assert_eq!(app.visible_articles().len(), 3);

        app.focus_feeds();
        app.selected_feed = 1;
        app.open_selected_feed(); // feed 1: falls back to the global limit
        assert_eq!(app.visible_articles().len(), 8);
        // The cap keeps the first entries, i.e. the newest ones (feed order).
        assert_eq!(app.visible_articles()[0].title, "Artikel 0");
        assert_eq!(app.visible_articles()[7].title, "Artikel 7");

        // `max = 0` on a source means "no cap", just like the global field.
        app.config.sources[1].max = Some(0);
        assert_eq!(app.visible_articles().len(), 10);
    }

    #[test]
    fn the_article_limit_caps_the_visible_list() {
        let mut app = App::new(config_with_sources());
        app.apply_update(FeedUpdate {
            index: 0,
            result: Ok(feed("Viele", 10)),
        });
        app.config.max_articles = 4;
        app.open_selected_feed();
        assert_eq!(app.visible_articles().len(), 4);

        app.config.max_articles = 0; // 0 means "all"
        assert_eq!(app.visible_articles().len(), 10);
    }

    #[test]
    fn the_search_filters_the_visible_articles_while_typing() {
        let mut app = loaded_app();
        app.open_selected_feed(); // 3 articles, titled "Artikel 0".."Artikel 2"
        app.on_key(KeyCode::Char('/'), KeyModifiers::NONE);
        assert_eq!(app.search.as_deref(), Some(""));
        app.on_key(KeyCode::Char('1'), KeyModifiers::NONE);
        assert_eq!(app.visible_articles().len(), 1);
        assert_eq!(app.visible_articles()[0].title, "Artikel 1");
    }

    #[test]
    fn enter_keeps_the_filter_and_esc_drops_it() {
        let mut app = loaded_app();
        app.open_selected_feed();
        app.on_key(KeyCode::Char('/'), KeyModifiers::NONE);
        app.on_key(KeyCode::Char('2'), KeyModifiers::NONE);
        app.on_key(KeyCode::Enter, KeyModifiers::NONE);
        assert!(app.search.is_none());
        assert_eq!(app.filter(), "2");
        assert_eq!(app.visible_articles().len(), 1);

        app.on_key(KeyCode::Char('/'), KeyModifiers::NONE);
        app.on_key(KeyCode::Esc, KeyModifiers::NONE);
        assert!(app.filter().is_empty());
        assert_eq!(app.visible_articles().len(), 3);
    }

    #[test]
    fn the_search_does_not_quit_the_program() {
        let mut app = loaded_app();
        app.on_key(KeyCode::Char('/'), KeyModifiers::NONE);
        app.on_key(KeyCode::Char('q'), KeyModifiers::NONE);
        assert!(!app.should_quit);
        assert_eq!(app.search.as_deref(), Some("q"));
    }

    #[test]
    fn esc_drops_a_committed_filter() {
        let mut app = loaded_app();
        app.open_selected_feed();
        app.on_key(KeyCode::Char('/'), KeyModifiers::NONE);
        app.on_key(KeyCode::Char('2'), KeyModifiers::NONE);
        app.on_key(KeyCode::Enter, KeyModifiers::NONE);
        assert_eq!(app.visible_articles().len(), 1);

        app.on_key(KeyCode::Esc, KeyModifiers::NONE);
        assert!(app.filter().is_empty());
        assert_eq!(app.visible_articles().len(), 3);
        assert_eq!(app.selected_article, 0);
    }

    #[test]
    fn esc_also_closes_the_help() {
        let mut app = App::default();
        app.on_key(KeyCode::Char('?'), KeyModifiers::NONE);
        app.on_key(KeyCode::Esc, KeyModifiers::NONE);
        assert!(!app.show_help);
    }

    #[test]
    fn pinning_a_feed_by_name_hides_the_list_and_opens_it() {
        let mut app = loaded_app();
        // The name is matched case-insensitively.
        app.restrict_to_feed("heise").unwrap();
        assert_eq!(app.only_feed, Some(0));
        assert!(!app.shows_feed_column());
        assert_eq!(app.open_feed, Some(0));
        assert_eq!(app.focus, Focus::Articles);
        assert_eq!(app.visible_articles().len(), 3);
        assert_eq!(app.open_feed_label().as_deref(), Some("Heise"));
    }

    #[test]
    fn a_feed_can_be_pinned_by_its_number() {
        let mut app = loaded_app();
        // The numbers are the ones `--list` prints, i.e. 1-based.
        app.restrict_to_feed("2").unwrap();
        assert_eq!(app.only_feed, Some(1));
        assert_eq!(app.visible_articles().len(), 5);

        // Out of range (and zero) is an error, not a silent fallback.
        let mut app = loaded_app();
        assert!(app.restrict_to_feed("0").is_err());
        assert!(app.restrict_to_feed("3").is_err());
        assert!(app.restrict_to_feed("-1").is_err());
        assert_eq!(app.only_feed, None);
    }

    #[test]
    fn a_name_beats_a_number_that_looks_like_one() {
        let config = Config {
            sources: vec![
                Source {
                    name: Some("2".to_string()),
                    url: "https://a.example/feed".to_string(),
                    max: None,
                },
                Source {
                    name: Some("Zweiter".to_string()),
                    url: "https://b.example/feed".to_string(),
                    max: None,
                },
            ],
            ..Config::default()
        };
        let mut app = App::new(config);
        app.restrict_to_feed("2").unwrap();
        // The feed *named* "2" is the first one, not the second.
        assert_eq!(app.only_feed, Some(0));
    }

    #[test]
    fn a_nameless_feed_is_reachable_by_its_number() {
        let mut app = loaded_app(); // feed 1 has no configured name
        app.restrict_to_feed("2").unwrap();
        assert_eq!(app.only_feed, Some(1));
        assert_eq!(app.visible_articles().len(), 5);
        // Its name is not usable, only the number.
        let mut app = loaded_app();
        assert!(app.restrict_to_feed("https://b.example/feed").is_err());
    }

    #[test]
    fn an_unknown_feed_is_reported_with_the_configured_names() {
        let mut app = loaded_app();
        let error = app.restrict_to_feed("gibtsnicht").unwrap_err();
        assert!(error.contains("gibtsnicht"), "got: {error}");
        assert!(error.contains("1 = Heise"), "got: {error}");
        assert!(error.contains("2 = (no name)"), "got: {error}");
        // Nothing changed: the normal two-column view stays.
        assert_eq!(app.only_feed, None);
        assert!(app.shows_feed_column());
    }

    #[test]
    fn an_empty_feed_name_is_rejected() {
        let mut app = loaded_app();
        assert!(app.restrict_to_feed("   ").is_err());
        assert_eq!(app.only_feed, None);
    }

    #[test]
    fn the_column_keys_do_nothing_while_a_single_feed_is_pinned() {
        let mut app = loaded_app();
        app.restrict_to_feed("Heise").unwrap();
        app.on_key(KeyCode::Tab, KeyModifiers::NONE);
        app.on_key(KeyCode::Left, KeyModifiers::NONE);
        assert_eq!(app.focus, Focus::Articles);
        // Moving inside the articles keeps working.
        app.on_key(KeyCode::Down, KeyModifiers::NONE);
        assert_eq!(app.selected_article, 1);
    }

    #[test]
    fn the_status_of_a_pinned_feed_names_it() {
        let mut app = loaded_app();
        app.restrict_to_feed("Heise").unwrap();
        assert_eq!(app.status(), "Heise · 3 articles");

        // While it is still loading, and when it failed.
        let mut app = App::new(config_with_sources());
        app.restrict_to_feed("Heise").unwrap();
        assert_eq!(app.status(), "Loading feed …");
        app.apply_update(FeedUpdate {
            index: 0,
            result: Err("HTTP 500".to_string()),
        });
        assert_eq!(app.status(), "Heise · failed");
    }

    #[test]
    fn the_scroll_offset_leaves_room_for_the_next_article() {
        // Six articles, title plus blank (two lines each: start 0, 2, 4, 6, 8, 10), window of
        // seven rows; the content of every article is a single line.
        // Selecting the first: the next article is already visible.
        assert_eq!(scroll_offset_with_next(0, 0, Some(2), 12, 7), 0);
        // Selecting article 4: its first line of the *next* one shows at the bottom row.
        assert_eq!(scroll_offset_with_next(6, 6, Some(8), 12, 7), 2);
        // The last article has no successor: its content stays fully in view.
        assert_eq!(scroll_offset_with_next(10, 10, None, 12, 7), 4);
    }

    #[test]
    fn the_last_article_is_shown_in_full_not_just_its_heading() {
        // Four articles of title + teaser + blank (three lines each: start 0, 3, 6, 9), so the
        // last one spans lines 9 and 10. A window of six rows must show both lines.
        let offset = scroll_offset_with_next(9, 10, None, 12, 6);
        assert_eq!(offset, 5);
        assert!(
            10 - offset < 6,
            "the last content line must be inside the window (offset {offset})"
        );
    }

    #[test]
    fn an_article_taller_than_the_window_is_shown_from_its_top() {
        // Seven content lines in a five-row window: only its start can be shown.
        assert_eq!(scroll_offset_with_next(6, 12, Some(13), 20, 5), 6);
        // Degenerate window.
        assert_eq!(scroll_offset_with_next(5, 6, Some(7), 10, 0), 0);
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
