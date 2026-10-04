# tuifeed

RSS-Feeds und ihre Artikel in einem Terminal-Interface (TUI), geschrieben in Rust.
Läuft unter Linux, macOS und Windows.

## Steuerung

| Taste | Wirkung |
|---|---|
| `↑` / `↓` | Eintrag in der fokussierten Spalte auswählen |
| `←` / `→` | Spalte wechseln (Feeds ↔ Artikel) |
| `Tab` / `Shift-Tab` | Spalte wechseln (Feeds ↔ Artikel) |
| `Enter` | Feed öffnen / Artikel im Browser öffnen |
| `r` | Feeds neu laden |
| `/` | Artikel suchen: tippen filtert live, `Enter` übernimmt, `Esc` verwirft |
| `Esc` | Filter zurücksetzen (ganze Artikelliste wieder zeigen) |
| `?` | Tastenhilfe ein-/ausblenden |
| `q` | Programm beenden |

## Konfiguration

`~/.config/tuifeed/config.toml` (bzw. das system-typische Konfigurationsverzeichnis);
beim ersten Start wird eine Vorlage angelegt.

```toml
theme = "gruvbox"   # gruvbox | nord | dracula | catppuccin
mode = "dark"       # light | dark
colors = "auto"     # auto | truecolor | 256 | 16
max_articles = 50   # Artikel je Feed (0 = alle)

[layout]
feeds_percent = 35  # Breite der Feed-Spalte in Prozent

[[sources]]
name = "Heise"
url = "https://www.heise.de/rss/heise-atom.xml"
```

- `colors`: `auto` erkennt die Terminal-Fähigkeit (`COLORTERM`, `TERM`); `truecolor`,
  `256` und `16` erzwingen die jeweilige Farbtiefe. Ohne 24-Bit-Terminal werden die
  Palettenfarben auf den nächstliegenden 256er- bzw. 16er-Eintrag abgebildet.
- `max_articles`: begrenzt nur die Anzeige; geparst wird der ganze Feed (der Zähler
  in der Feed-Spalte zeigt weiterhin die Gesamtzahl).

## Scrollen in der Artikelspalte

Solange unter dem hervorgehobenen Artikel noch weitere folgen, bleibt mindestens die erste
Zeile des nächsten Artikels sichtbar — der Cursor sitzt also im vorletzten sichtbaren Platz
und man sieht, was als Nächstes kommt. Erst beim letzten Artikel scrollt die Liste bis ans Ende.

## Bauen und starten

```
cargo run
```

## Entwicklung

```
cargo fmt
cargo build
cargo test
cargo clippy --all-targets
```

Nicht-visueller Prüflauf (zeichnet in einem pty fester Größe und gibt den
Terminal-Stream aus, damit sich ein Lauf ohne Bildschirm prüfen lässt):

```
python3 scripts/pty_run.py <XDG_CONFIG_HOME> ./target/debug/tuifeed 12 $'\x1b[B\r'
```

Hinweis: `Enter` im pty ist ein CR (`\r`), nicht ein LF (`\n`) — mit `\n` bleibt z. B.
eine offene Suche offen und das abschließende `q` tippt in die Suche statt zu beenden.

## Lizenz

MIT — siehe [LICENSE](LICENSE).
