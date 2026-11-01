# Changelog

All notable changes to tuifeed are documented in this file. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project uses
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Security

- An article's link is only handed to the browser when it is an `http`/`https` address. Feed content
  is untrusted: another scheme (`file:`, `smb:`, …) would reach whatever handler is registered for
  it, and a value like `-x` would arrive at the opener as an option instead of a URL.
- A feed body larger than 10 MiB is refused instead of being read into memory.

### Fixed

- The feed column's error marker still read `⚠ Fehler`; it is `⚠ Error` now.
- A browser that could not be started is reported in the footer instead of on stderr, where the
  alternate screen hid it. The same line now carries the refusal of a link that is not `http`/`https`.
- The per-feed `eprintln!` on a failed fetch is gone: it was invisible under the alternate screen,
  and the reason is already shown in the feed column, the article pane and the status line.

## [1.2.0] - 2026-11-01

### Added

- `[auto_update]`: with `enabled = true` every configured feed is fetched again on a timer.
  `interval_minutes` sets the interval in whole minutes (at least 1, off by default).
- `r` reloads by hand at any time and restarts the interval, so a manual reload is not followed by a
  second fetch a moment later. While auto-update is switched on, the `?` overlay names the interval
  (`auto-refresh  every 30 min`).

## [1.1.0] - 2026-10-05

### Added

- Mouse support: the wheel scrolls the focused column; a left click opens the feed or article under
  the pointer (the same action as `Enter`). Mouse reporting is on; hold `Shift` to select text with
  the terminal.
- `PgUp` / `PgDn` page through the focused column.
- The first configured feed opens on start, so its articles are visible without pressing `Enter`.
- An optional event trace for diagnosing input problems: set `TUIFEED_EVENT_LOG=<path>` and every
  terminal event is appended to that file.

### Fixed

- The article count in the status line respects `max` / `max_articles` now and agrees with the
  count in the feed column.

## [1.0.0] - 2026-10-04

First release. Every milestone of the project's specification (M0–M6) is implemented, together with
the follow-up features, and the behaviour is covered by tests.

### Added

**Feeds and articles**

- Two-column layout: the configured feeds on the left, the articles of the opened feed on the right.
- Every feed is fetched at start and on demand (`r`); no polling.
- RSS 2.0 and Atom are parsed with `feed-rs`; an article shows a bold title, its date in local time
  and a plain-text teaser, wrapped to the column width.
- `Enter` in the article column opens the selected article in the default browser.
- A feed that fails keeps its reason and is named in the status line; the article column shows it.

**Configuration** — `~/.config/tuifeed/config.toml`; a commented template is written on first start

- `theme` × `mode`: Gruvbox, Nord, Dracula (Alucard) and Catppuccin (Mocha/Latte), each in a light
  and a dark variant, defined as RGB palettes and used through semantic tokens.
- `colors`: `auto` (detected from `COLORTERM`/`TERM`), `truecolor`, `256` or `16`. Without a 24-bit
  terminal the palette colours are mapped to the nearest 256-colour or basic colour.
- `max_articles` as a global fallback and `max` per `[[sources]]` entry cap how many articles are
  shown. `0` means no cap; feeds are cut from the top, so the newest entries stay.
- `[layout] feeds_percent` sets the width of the feed column.
- The number in brackets in the feed column is the count actually used: the feed's own `max`,
  otherwise what was delivered — never more than the feed offers.

**Keys**

- `↑`/`↓` select an entry, `←`/`→` and `Tab`/`Shift-Tab` switch columns, `Enter` opens a feed or an
  article, `r` reloads, `/` searches (live filter, `Enter` commits, `Esc` discards), `Esc` also
  drops a committed filter, `?` toggles the key help overlay, `q` quits.
- The selected row is highlighted with a background band across the full column width while the
  title, date and teaser keep their own colours; the focused column prints in bold.
- The article column scrolls so that the selected article is visible in full; while more articles
  follow, the first line of the next one stays in view. A scrollbar appears when the list does not
  fit into the column, and its thumb reaches the bottom.

**Command line**

- `-f, --feed <NR|NAME>` starts with a single feed: no feed list, the articles use the whole window.
  The feed is addressed by its number (as shown by `--list`) or by its configured name.
- `-l, --list` prints the configured feeds (number, name and `max`) and exits; it starts no UI and
  fetches nothing.
- `-V, --version` prints the version, `-h, --help` the usage.
- A wrong command line reports the problem on stderr and exits with code 2 — before the terminal is
  taken over, so the message stays readable.

[Unreleased]: https://github.com/riesemann/tuifeed/compare/v1.2.0...HEAD
[1.2.0]: https://github.com/riesemann/tuifeed/releases/tag/v1.2.0
[1.1.0]: https://github.com/riesemann/tuifeed/releases/tag/v1.1.0
[1.0.0]: https://github.com/riesemann/tuifeed/releases/tag/v1.0.0
