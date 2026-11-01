# tuifeed

RSS feeds and their articles in a terminal interface (TUI), written in Rust.
Runs on Linux, macOS and Windows.

**Version 1.2.0** — see [CHANGELOG.md](CHANGELOG.md) for what each release contains.
Built with AI assistance — see [How this was built](#how-this-was-built).

## Quick start

Install from a clone of this repository:

```
cargo install --path . --locked
```

Put your feeds into `~/.config/tuifeed/config.toml` — a commented template is written on the first
start — and run:

```
tuifeed
```

`tuifeed --help` lists the options, `tuifeed --list` the feeds you have configured.

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

`Enter` hands an article's address to the system's browser only when it is an `http` or `https`
address. Feed content is untrusted: a link with another scheme (`file:`, `smb:`, …) would reach
whatever program is registered for that scheme, and a value like `-x` would arrive at the opener as
an option instead of a URL. Such a link is refused, and the footer says so until the next keypress —
as does a browser that cannot be started. The same line carries a failed feed.

## Configuration

`~/.config/tuifeed/config.toml` (or the system's usual configuration directory). A commented
template is written on the first start; the example below shows every setting.

```toml
theme = "gruvbox"
mode = "dark"
colors = "auto"
max_articles = 0

[auto_update]
enabled = false
interval_minutes = 15

[layout]
feeds_percent = 35

[[sources]]
name = "Heise"
url = "https://www.heise.de/rss/heise-atom.xml"
max = 30
```

| Setting | Default | Meaning |
|---|---|---|
| `theme` | `gruvbox` | Colour family: `gruvbox`, `nord`, `dracula` or `catppuccin` |
| `mode` | `dark` | `light` or `dark` |
| `colors` | `auto` | How many colours to use: `auto`, `truecolor`, `256` or `16` |
| `max_articles` | `0` | Fallback cap for feeds without their own `max` (`0` = all) |
| `[auto_update] enabled` | `false` | Refresh all feeds on a timer |
| `[auto_update] interval_minutes` | `15` | How often, in whole minutes (at least `1`) |
| `[layout] feeds_percent` | `35` | Width of the feed column, in percent |
| `[[sources]] name` | — | Label for the feed; without one the feed's own title is used |
| `[[sources]] url` | — | The feed URL |
| `[[sources]] max` | — | Cap for this feed, overrides `max_articles` (`0` = all) |

Repeat the `[[sources]]` block for every feed you want. Three settings need a word of explanation:

- **`colors`** — `auto` detects what the terminal can do (`COLORTERM`, `TERM`); the other values
  force that colour depth. Without a 24-bit terminal the palette colours are mapped to the nearest
  256-colour or basic colour.
- **`max_articles` and `max`** — they cap how many articles are shown, `0` means no cap, and a
  feed's own `max` wins over the global `max_articles`. The list is cut from the **top**, so the
  newest entries stay.
- **The number in brackets** in the feed column is the count that is *used*: the feed's own `max`,
  otherwise the delivered count — never more than the feed offers.

### Automatic refresh

`[auto_update] enabled = true` fetches **every** configured feed again once `interval_minutes`
minutes have passed. The timer starts when the program starts, so the first automatic fetch happens
one interval after the start, not immediately.

```
[auto_update]
enabled = true
interval_minutes = 30
```

- Off by default: without it the feeds are loaded at start and only when you press `r`.
- `interval_minutes` counts whole minutes and is at least `1`; a smaller value is raised to one
  minute.
- `r` reloads by hand at any time and **restarts** the interval, so a manual reload is not followed
  by a second fetch a moment later.
- The refresh replaces the feeds' contents; the cursor, the open feed, the filter and the scroll
  position stay where they are. While a refresh runs, the feed column and the status line show
  `loading …` again — a feed that then fails keeps its reason and is named in the status line.
- All feeds are refreshed, also while `--feed` pins a single one.
- With auto-update switched on, `?` lists the interval (`auto-refresh  every 30 min`).

## Command line

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

## The article column

### Scrolling

The highlighted article is shown as **completely** as possible. As long as more articles follow
below it, at least the first line of the next one stays visible — the cursor therefore sits in the
second-to-last visible spot and you can see what comes next. For the **last** article (no successor)
the remaining height is used for its own text, so the title and the teaser are visible. Only an
article taller than the column is cut off at the top.

If the content does not fit into the column, the column additionally shows a **scrollbar** on the
right (a continuous track without arrow heads, thumb `█`); short lists leave the column free. The
text is then wrapped one column narrower so the scrollbar covers nothing. The thumb runs to the
bottom end as soon as the list is scrolled all the way down.

### Highlight

The cursor colours **only the background** (the theme's `selection_bg`), not the text: title, date
and teaser keep their own colours (accent / muted / secondary) and stay distinguishable that way.
The band stretches across the **full width** of the column (up to the scrollbar) instead of ending
behind the text. When the column has the focus, bold is added.

## Development

Build and run from the source tree:

```
cargo run
```

Before a commit:

```
cargo fmt
cargo build
cargo test
cargo clippy --all-targets
```

### Checking a run without a screen

`scripts/pty_run.py` starts the binary in a pty of a fixed size (100×30), sends a key sequence after
a delay, quits with `q` and prints everything the program drew — so a run can be checked without a
screen:

```
python3 scripts/pty_run.py <XDG_CONFIG_HOME> ./target/debug/tuifeed 12 $'\x1b[B\r'
```

Note: `Enter` in a pty is a CR (`\r`), not an LF (`\n`) — with `\n`, for example, an open search
stays open and the final `q` types into the search instead of quitting.

### Diagnosing input

To see what your terminal actually sends for a key or a click, set `TUIFEED_EVENT_LOG` and read the
file afterwards:

```
TUIFEED_EVENT_LOG=/tmp/tuifeed-events.log cargo run
```

Every `Event::Key` / `Event::Mouse` is appended with the state before and after it is handled.

## How this was built

tuifeed is developed with AI assistance: the code, the tests and this documentation are written in
pairing sessions with an AI coding agent (Hermes Agent by Nous Research) and reviewed by the
maintainer. Nothing ships on the agent's word alone — every change passes `cargo fmt`,
`cargo build`, `cargo test` and `cargo clippy --all-targets`, the feed parsers are tested against
recorded real payloads, and the running interface is looked at by a human before a release.

## License

MIT — see [LICENSE](LICENSE).
