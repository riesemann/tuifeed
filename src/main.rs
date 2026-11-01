//! tuifeed — RSS feeds and their articles in a terminal UI.

use std::io::{self, Stdout};
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyEventKind},
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
    if options.version {
        println!("{}", cli::VERSION);
        return Ok(());
    }
    // `--list` only reports the configured feeds; it starts no UI (and needs no fetch).
    if options.list {
        print!("{}", Config::load().describe());
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

/// Enter raw mode, the alternate screen and mouse reporting.
fn setup_terminal() -> Result<Tui> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    Ok(Terminal::new(CrosstermBackend::new(stdout))?)
}

/// Leave the alternate screen, turn mouse reporting off and leave raw mode.
fn restore_terminal() -> Result<()> {
    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture)?;
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

    // Optional trace of every terminal event, enabled by `TUIFEED_EVENT_LOG=<path>`
    // (diagnostics for "a key/click does nothing in my terminal").
    let trace = std::env::var("TUIFEED_EVENT_LOG").ok();

    while !app.should_quit {
        terminal.draw(|frame| ui::draw(frame, &app))?;

        // Pick up whatever the background tasks have finished.
        drain_results(&rx, &mut app);

        // Wait for an event (but wake up regularly to look for results).
        if let Some(event) = poll_event(TICK)? {
            trace_event(
                trace.as_deref(),
                &format!(
                    "event={event:?} before: focus={:?} sel_feed={} sel_art={}",
                    app.focus, app.selected_feed, app.selected_article
                ),
            );
            match event {
                // Windows reports both press and release; only act on press.
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    app.on_key(key.code, key.modifiers);
                }
                Event::Mouse(mouse) => app.on_mouse(mouse.kind, mouse.column, mouse.row),
                _ => {}
            }
            trace_event(
                trace.as_deref(),
                &format!(
                    "  after: focus={:?} sel_feed={} sel_art={} pending={:?} feed_view={:?} article_view={:?}",
                    app.focus,
                    app.selected_feed,
                    app.selected_article,
                    app.pending_open,
                    app.feed_view.get(),
                    app.article_view.get()
                ),
            );
        }

        // Open the selected article in the browser, if the user asked for it.
        if let Some(url) = app.take_pending_open() {
            trace_event(trace.as_deref(), &format!("OPEN {url}"));
            if let Err(error) = open::that(&url) {
                trace_event(trace.as_deref(), &format!("OPEN FAILED {error}"));
                // stderr is hidden under the alternate screen, so the user is told in the footer.
                app.report(format!("Could not open the browser: {error}"));
            }
        }

        // Reload the feeds: on `r`, or on the auto-update timer when it is switched on.
        let now = Instant::now();
        app.tick(now);
        if app.take_reload(now) {
            trace_event(trace.as_deref(), "RELOAD");
            start_load(&runtime, &client, &mut app, &tx);
        }
    }
    Ok(())
}

/// Append one line to the event trace, if `TUIFEED_EVENT_LOG` points at a file.
fn trace_event(path: Option<&str>, message: &str) {
    use std::io::Write;
    let Some(path) = path else {
        return;
    };
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        let _ = writeln!(file, "{message}");
    }
}

/// Wait up to `timeout` for a terminal event.
fn poll_event(timeout: Duration) -> Result<Option<Event>> {
    if !event::poll(timeout)? {
        return Ok(None);
    }
    Ok(Some(event::read()?))
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
    // A failure is not printed here: stderr is hidden under the alternate screen, and the reason
    // travels in `FeedState::Failed`, which the feed column, the article pane and the status line
    // all show.
    while let Ok(update) = rx.try_recv() {
        app.apply_update(update);
    }
}
