# tuifeed

RSS feeds and their articles in a terminal interface (TUI), written in Rust.
Runs on Linux, macOS and Windows.

Version 1.1.0 — see [CHANGELOG.md](CHANGELOG.md) for what the release contains.

## Controls

| Key | Effect |
|---|---|
| `↑` / `↓` | Select an entry in the focused column |
| `PgUp` / `PgDn` | Page through the focused column |
| `←` / `→` | Switch column (feeds ↔ articles) |
| `Tab` / `Shift-Tab` | Switch column (feeds ↔ articles) |
| `Enter` | Open a feed / open the article in the browser |
| Mouse wheel | Scroll the focused column |
| Mouse click | Open the feed / article under the pointer |
| `r` | Reload the feeds |
| `/` | Search articles: typing filters live, `Enter` commits, `Esc` discards |
| `Esc` | Reset the filter (the whole article list comes back) |
| `?` | Show/hide the key help |
| `q` | Quit |

The first configured feed opens on start, so its articles are there without an extra keypress.
Mouse reporting is on; hold `Shift` while dragging to select text with the terminal.

## Configuration

`~/.config/tuifeed/config.toml` (or the system's usual configuration directory); a commented
template is written on the first start.

```toml
theme = "gruvbox"   # gruvbox | nord | dracula | catppuccin
mode = "dark"       # light | dark
colors = "auto"     # auto | truecolor | 256 | 16
max_articles = 0    # fallback for feeds without their own max (0 = all)

[layout]
feeds_percent = 35  # width of the feed column, in percent

[[sources]]
name = "Heise"
url = "https://www.heise.de/rss/heise-atom.xml"
max = 30            # optional: articles for this feed (overrides max_articles, 0 = all)
```

- `colors`: `auto` detects the terminal's capability (`COLORTERM`, `TERM`); `truecolor`, `256` and
  `16` force that colour depth. Without a 24-bit terminal the palette colours are mapped to the
  nearest 256-colour or basic colour.
- `max_articles`: fallback for feeds **without** their own `max` (`0` = all). Without it, every feed
  shows everything it delivers.
- `max` (per `[[sources]]` entry): this feed's own cap, and it takes precedence over `max_articles`.
  The feed is cut from the **top**, so the newest articles stay (feed order). `0` = all.
- The number in brackets in the feed column is the count that is **used**: the feed's own `max`,
  otherwise the delivered count (never more than the feed offers).

## Scrolling in the article column

The highlighted article is shown as **completely** as possible. As long as more articles follow
below it, at least the first line of the next one stays visible — the cursor therefore sits in the
second-to-last visible spot and you can see what comes next. For the **last** article (no successor)
the remaining height is used for its own text, so the title and the teaser are visible. Only an
article taller than the column is cut off at the top.

If the content does not fit into the column, the column additionally shows a **scrollbar** on the
right (a continuous track without arrow heads, thumb `█`); short lists leave the column free. The
text is then wrapped one column narrower so the scrollbar covers nothing. The thumb runs to the
bottom end as soon as the list is scrolled all the way down.

## Highlight of the selected article

The cursor colours **only the background** (the theme's `selection_bg`), not the text: title, date
and teaser keep their own colours (accent / muted / secondary) and stay distinguishable that way.
The band stretches across the **full width** of the column (up to the scrollbar) instead of ending
behind the text. When the column has the focus, bold is added.

## Usage

```
tuifeed                     # two columns: feed list on the left, articles on the right
tuifeed -f Heise            # this feed only: no list, articles across the whole window
tuifeed --feed 2            # the same via the number from --list
tuifeed -l                  # list the configured feeds and exit
tuifeed -V                  # print the version
tuifeed --help              # usage and options
```

With `--feed <NR|NAME>` (`-f`) tuifeed starts with **one** feed: the feed list is gone, the article
column uses the whole window and always holds the focus. `Tab`/`←`/`→` do nothing then, while
`↑`/`↓`, `/`, `r` and `Enter` work as usual. The feed is given either as the number from `--list` or
as the `name` from the configuration (case-insensitive); a feed without a `name` can only be
addressed by its number. If nothing matches, tuifeed names the available feeds (exit code 2) without
taking over the terminal.

`--list` (`-l`) shows which feeds are configured — number, name and the respective `max`:

```
Configuration: /home/…/.config/tuifeed/config.toml

2 feeds:

   1. Heise   (max 50)
   2. 9to5Linux

Show one of them directly:  tuifeed --feed <NR|NAME>
```

It starts no interface and fetches nothing from the network.

## Build and run

```
cargo run
```

## Development

```
cargo fmt
cargo build
cargo test
cargo clippy --all-targets
```

Non-visual check run (draws in a pty of a fixed size and prints the terminal stream, so a run can be
checked without a screen):

```
python3 scripts/pty_run.py <XDG_CONFIG_HOME> ./target/debug/tuifeed 12 $'\x1b[B\r'
```

Note: `Enter` in a pty is a CR (`\r`), not an LF (`\n`) — with `\n`, for example, an open search
stays open and the final `q` types into the search instead of quitting.

To see what a terminal actually sends for a key or a click (useful when input behaves differently in
your terminal), set `TUIFEED_EVENT_LOG` and check the file afterwards:

```
TUIFEED_EVENT_LOG=/tmp/tuifeed-events.log cargo run
```

Every `Event::Key` / `Event::Mouse` is appended with the state before and after it is handled.

## License

MIT — see [LICENSE](LICENSE).
