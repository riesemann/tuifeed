//! tuifeed — RSS feeds and their articles in a terminal UI.

use std::io::{self, Stdout};
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::Duration;

use anyhow::{Context, Result};
use crossterm::{
    event::{self, Event, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use tokio::runtime::Runtime;
use tuifeed::{
    app::{App, FeedUpdate},
    cli,
    config::Config,
    feed, ui,
};

type Tui = Terminal<CrosstermBackend<Stdout>>;

/// How long the loop waits for a key before it looks for fetch results again.
const TICK: Duration = Duration::from_millis(100);

/// Exit code for a wrong command line.
const EXIT_USAGE: i32 = 2;

fn main() -> Result<()> {
    let options = match cli::parse(std::env::args().skip(1)) {
        Ok(options) => options,
        Err(error) => {
            eprintln!("tuifeed: {error}");
            eprint!("\n{}", cli::USAGE);
            std::process::exit(EXIT_USAGE);
        }
    };
    if options.help {
        print!("{}", cli::USAGE);
        return Ok(());
    }

    if let Err(error) = Config::write_default_if_missing() {
        eprintln!("tuifeed: {error:#}");
    }
    // Resolve `--feed` before the terminal is switched over, so the message stays readable.
    let mut app = App::new(Config::load());
    if let Some(name) = &options.feed
        && let Err(error) = app.restrict_to_feed(name)
    {
        eprintln!("tuifeed: {error}");
        std::process::exit(EXIT_USAGE);
    }

    // Make sure the terminal is always restored, even if we panic.
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = restore_terminal();
        original_hook(info);
    }));

    let mut terminal = setup_terminal()?;
    let result = run(&mut terminal, app);
    restore_terminal()?;
    result
}

/// Enter raw mode and the alternate screen.
fn setup_terminal() -> Result<Tui> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    Ok(Terminal::new(CrosstermBackend::new(stdout))?)
}

/// Leave the alternate screen and raw mode.
fn restore_terminal() -> Result<()> {
    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen)?;
    Ok(())
}

/// The main loop: draw, collect fetch results, react to keys.
fn run(terminal: &mut Tui, mut app: App) -> Result<()> {
    let runtime = Runtime::new().context("could not start the async runtime")?;
    let client = reqwest::Client::builder()
        .user_agent(concat!("tuifeed/", env!("CARGO_PKG_VERSION")))
        .build()
        .context("could not build the HTTP client")?;
    let (tx, rx) = mpsc::channel();

    start_load(&runtime, &client, &mut app, &tx);

    while !app.should_quit {
        terminal.draw(|frame| ui::draw(frame, &app))?;

        // Pick up whatever the background tasks have finished.
        drain_results(&rx, &mut app);

        // Wait for a key (but wake up regularly to look for results).
        if let Some(key) = poll_key(TICK)? {
            // Windows reports both press and release; only act on press.
            if key.kind == KeyEventKind::Press {
                app.on_key(key.code, key.modifiers);
            }
        }

        // Open the selected article in the browser, if the user asked for it.
        if let Some(url) = app.take_pending_open() {
            open_in_browser(&url);
        }

        if app.reload {
            app.reload = false;
            start_load(&runtime, &client, &mut app, &tx);
        }
    }
    Ok(())
}

/// Hand an address to the system's default browser.
fn open_in_browser(url: &str) {
    if let Err(error) = open::that(url) {
        eprintln!("tuifeed: could not open {url}: {error}");
    }
}

/// Wait up to `timeout` for a key event.
fn poll_key(timeout: Duration) -> Result<Option<event::KeyEvent>> {
    if !event::poll(timeout)? {
        return Ok(None);
    }
    match event::read()? {
        Event::Key(key) => Ok(Some(key)),
        _ => Ok(None),
    }
}

/// Mark all feeds as loading and fetch each of them in the background.
fn start_load(runtime: &Runtime, client: &reqwest::Client, app: &mut App, tx: &Sender<FeedUpdate>) {
    app.begin_load();
    for (index, slot) in app.feeds.iter().enumerate() {
        let client = client.clone();
        let url = slot.url.clone();
        let tx = tx.clone();
        runtime.spawn(async move {
            let result = feed::fetch(&client, &url)
                .await
                .map_err(|error| format!("{error:#}"));
            let _ = tx.send(FeedUpdate { index, result });
        });
    }
}

/// Take every finished fetch out of the channel and store it.
fn drain_results(rx: &Receiver<FeedUpdate>, app: &mut App) {
    while let Ok(update) = rx.try_recv() {
        if let Err(error) = &update.result {
            eprintln!("tuifeed: feed {} failed: {error}", update.index);
        }
        app.apply_update(update);
    }
}
