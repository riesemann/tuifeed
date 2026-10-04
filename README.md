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
max_articles = 0    # Fallback für Feeds ohne eigenes max (0 = alle)

[layout]
feeds_percent = 35  # Breite der Feed-Spalte in Prozent

[[sources]]
name = "Heise"
url = "https://www.heise.de/rss/heise-atom.xml"
max = 30            # optional: Artikel für diesen Feed (überschreibt max_articles, 0 = alle)
```

- `colors`: `auto` erkennt die Terminal-Fähigkeit (`COLORTERM`, `TERM`); `truecolor`,
  `256` und `16` erzwingen die jeweilige Farbtiefe. Ohne 24-Bit-Terminal werden die
  Palettenfarben auf den nächstliegenden 256er- bzw. 16er-Eintrag abgebildet.
- `max_articles`: Fallback für Feeds **ohne** eigenes `max` (`0` = alle). Ohne Angabe zeigt jeder
  Feed alles, was er liefert.
- `max` (je `[[sources]]`-Block): eigener Deckel für diesen Feed und hat Vorrang vor
  `max_articles`. Gekürzt wird von **oben**, also bleiben die neuesten Artikel stehen
  (Feed-Reihenfolge). `0` = alle.
- Die Zahl in Klammern in der Feed-Spalte ist die Anzahl, die **verwendet** wird: der eigene
  `max`, sonst die gelieferte Anzahl (nie mehr, als der Feed hergibt).

## Scrollen in der Artikelspalte

Der hervorgehobene Artikel wird möglichst **vollständig** gezeigt. Solange unter ihm noch weitere
Artikel folgen, bleibt zusätzlich mindestens die erste Zeile des nächsten sichtbar — der Cursor
sitzt also im vorletzten sichtbaren Platz und man sieht, was als Nächstes kommt. Beim **letzten**
Artikel (kein Nachfolger) wird die restliche Höhe für seinen eigenen Text genutzt, damit Titel und
Teaser sichtbar sind. Nur ein Artikel, der höher ist als die Spalte, wird von oben angeschnitten.

Passt der Inhalt nicht in die Spalte, zeigt sie zusätzlich rechts eine **Scrollbar** (Track + Griff);
bei kurzen Listen bleibt die Spalte frei. Der Text wird dann um eine Spalte schmaler umbrochen,
damit die Scrollbar nichts verdeckt.

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
