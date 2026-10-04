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
| `q` | Programm beenden |

## Konfiguration

`~/.config/tuifeed/config.toml` (bzw. das system-typische Konfigurationsverzeichnis).

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

## Lizenz

MIT — siehe [LICENSE](LICENSE).
