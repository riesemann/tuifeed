# tuifeed

RSS-Feeds und ihre Artikel in einem Terminal-Interface (TUI), geschrieben in Rust.
Läuft unter Linux, macOS und Windows.

## Steuerung

| Taste | Wirkung |
|---|---|
| `↑` / `↓` | Eintrag in der fokussierten Spalte auswählen |
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

## Lizenz

MIT — siehe [LICENSE](LICENSE).
