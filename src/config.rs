//! Configuration: reading and writing `~/.config/tuifeed/config.toml`.

use std::fmt;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// The colour family of the interface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeName {
    Gruvbox,
    Nord,
    Dracula,
    Catppuccin,
}

impl ThemeName {
    /// The default family.
    pub const DEFAULT: ThemeName = ThemeName::Gruvbox;

    /// Every family, in the order they are offered.
    pub const ALL: [ThemeName; 4] = [
        ThemeName::Gruvbox,
        ThemeName::Nord,
        ThemeName::Dracula,
        ThemeName::Catppuccin,
    ];

    /// The name as written in the configuration file.
    pub fn as_str(self) -> &'static str {
        match self {
            ThemeName::Gruvbox => "gruvbox",
            ThemeName::Nord => "nord",
            ThemeName::Dracula => "dracula",
            ThemeName::Catppuccin => "catppuccin",
        }
    }

    /// Parse a name; an unknown name is not an error — it falls back to the default.
    pub fn from_str_lossy(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "nord" => ThemeName::Nord,
            "dracula" => ThemeName::Dracula,
            "catppuccin" => ThemeName::Catppuccin,
            "gruvbox" => ThemeName::Gruvbox,
            _ => ThemeName::DEFAULT,
        }
    }
}

impl fmt::Display for ThemeName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Serialize for ThemeName {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for ThemeName {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(Self::from_str_lossy(&String::deserialize(deserializer)?))
    }
}

/// Light or dark appearance of the chosen family.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Light,
    Dark,
}

impl Mode {
    pub const DEFAULT: Mode = Mode::Dark;

    pub fn as_str(self) -> &'static str {
        match self {
            Mode::Light => "light",
            Mode::Dark => "dark",
        }
    }

    /// Parse a name; an unknown name falls back to the default.
    pub fn from_str_lossy(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "light" => Mode::Light,
            "dark" => Mode::Dark,
            _ => Mode::DEFAULT,
        }
    }
}

impl fmt::Display for Mode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Serialize for Mode {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for Mode {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(Self::from_str_lossy(&String::deserialize(deserializer)?))
    }
}

/// How many colours the terminal is allowed to use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorMode {
    /// Detect from the environment (`COLORTERM`, `TERM`).
    Auto,
    /// 24-bit colour.
    TrueColor,
    /// The 256-colour palette.
    Ansi256,
    /// The 16 basic colours.
    Ansi16,
}

impl ColorMode {
    pub const DEFAULT: ColorMode = ColorMode::Auto;

    pub const ALL: [ColorMode; 4] = [
        ColorMode::Auto,
        ColorMode::TrueColor,
        ColorMode::Ansi256,
        ColorMode::Ansi16,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            ColorMode::Auto => "auto",
            ColorMode::TrueColor => "truecolor",
            ColorMode::Ansi256 => "256",
            ColorMode::Ansi16 => "16",
        }
    }

    /// Parse a name; an unknown name falls back to `auto`.
    pub fn from_str_lossy(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "truecolor" | "24bit" | "24-bit" => ColorMode::TrueColor,
            "256" | "ansi256" | "256color" => ColorMode::Ansi256,
            "16" | "ansi16" | "16color" => ColorMode::Ansi16,
            _ => ColorMode::DEFAULT,
        }
    }
}

impl fmt::Display for ColorMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Serialize for ColorMode {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for ColorMode {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(Self::from_str_lossy(&String::deserialize(deserializer)?))
    }
}

/// The split of the two columns, in percent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Layout {
    /// Width of the left (feeds) column in percent; the right column takes the rest.
    pub feeds_percent: u16,
}

impl Default for Layout {
    fn default() -> Self {
        Self { feeds_percent: 35 }
    }
}

impl Layout {
    /// Smallest allowed width so a column can still show something.
    pub const MIN_PERCENT: u16 = 1;
    /// Largest allowed width; the right column keeps at least `MIN_PERCENT`.
    pub const MAX_PERCENT: u16 = 99;

    /// The left column width, clamped to a valid range.
    pub fn feeds(&self) -> u16 {
        self.feeds_percent
            .clamp(Self::MIN_PERCENT, Self::MAX_PERCENT)
    }

    /// The right column width (the remainder).
    pub fn articles(&self) -> u16 {
        100 - self.feeds()
    }
}

/// One RSS feed to show.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Source {
    /// Optional display name; without one the feed's own title is used.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// The feed URL.
    pub url: String,
    /// Most articles to show for this feed (`0` = all); overrides `max_articles`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max: Option<usize>,
}

/// The whole configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub theme: ThemeName,
    pub mode: Mode,
    /// How many colours to use (see [`ColorMode`]).
    pub colors: ColorMode,
    /// Most articles to show per feed (`0` = all).
    pub max_articles: usize,
    pub layout: Layout,
    pub sources: Vec<Source>,
}

impl Config {
    /// The default article limit per feed: no limit, every feed shows what it delivers.
    /// A source can lower it with its own `max`, or set `max_articles` globally.
    pub const DEFAULT_MAX_ARTICLES: usize = 0;
}

impl Default for Config {
    fn default() -> Self {
        Self {
            theme: ThemeName::DEFAULT,
            mode: Mode::DEFAULT,
            colors: ColorMode::DEFAULT,
            max_articles: Self::DEFAULT_MAX_ARTICLES,
            layout: Layout::default(),
            sources: Vec::new(),
        }
    }
}

impl Config {
    /// The path of the configuration file (`~/.config/tuifeed/config.toml` on Linux).
    pub fn path() -> Option<PathBuf> {
        ProjectDirs::from("", "", "tuifeed").map(|dirs| dirs.config_dir().join("config.toml"))
    }

    /// A plain-text overview of the configured feeds (for `--list`).
    ///
    /// One line per source: its name (or URL) and the URL, plus its own article cap.
    pub fn describe(&self) -> String {
        let mut out = String::new();
        match Self::path() {
            Some(path) => out.push_str(&format!("Konfiguration: {}\n", path.display())),
            None => out.push_str("Konfiguration: (Pfad unbekannt)\n"),
        }
        if self.sources.is_empty() {
            out.push_str(
                "\nKeine Feeds konfiguriert.\n\nTrage sie in der Datei unter [[sources]] ein.\n",
            );
            return out;
        }

        let labels: Vec<String> = self
            .sources
            .iter()
            .map(|source| match &source.name {
                Some(name) if !name.trim().is_empty() => name.clone(),
                _ => "(ohne Name)".to_string(),
            })
            .collect();
        let width = labels
            .iter()
            .map(|label| label.chars().count())
            .max()
            .unwrap_or(0);
        out.push_str(&format!(
            "\n{} {}:\n\n",
            self.sources.len(),
            if self.sources.len() == 1 {
                "Feed"
            } else {
                "Feeds"
            }
        ));
        for (index, source) in self.sources.iter().enumerate() {
            out.push_str(&format!(
                "  {:>2}. {:<width$}  {}",
                index + 1,
                labels[index],
                source.url,
                width = width
            ));
            if let Some(max) = source.max {
                out.push_str(&format!("   (max {max})"));
            }
            out.push('\n');
        }
        out.push_str("\nEinen davon direkt anzeigen:  tuifeed --feed <NAME|URL>\n");
        out
    }

    /// Load the configuration, falling back to defaults on any problem.
    ///
    /// A missing or unreadable file is not fatal: the program starts with the defaults.
    pub fn load() -> Config {
        match Self::path() {
            Some(path) if path.exists() => match Self::load_from(&path) {
                Ok(config) => config,
                Err(error) => {
                    eprintln!("tuifeed: {error:#}; using defaults");
                    Config::default()
                }
            },
            _ => Config::default(),
        }
    }

    /// Read the configuration from a specific file.
    pub fn load_from(path: &Path) -> Result<Config> {
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("could not read {}", path.display()))?;
        Self::from_toml(&text)
    }

    /// Parse the configuration from TOML text.
    pub fn from_toml(text: &str) -> Result<Config> {
        let config: Config = toml::from_str(text).context("config.toml is not valid TOML")?;
        Ok(config.normalised())
    }

    /// Render the configuration as TOML.
    pub fn to_toml(&self) -> Result<String> {
        toml::to_string_pretty(self).context("configuration could not be written as TOML")
    }

    /// Write the configuration to the default path.
    pub fn save(&self) -> Result<()> {
        let path = Self::path().context("no configuration directory available")?;
        self.save_to(&path)
    }

    /// Write the configuration to a specific file, creating parent directories.
    pub fn save_to(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("could not create {}", parent.display()))?;
        }
        std::fs::write(path, self.to_toml()?)
            .with_context(|| format!("could not write {}", path.display()))
    }

    /// Write a commented example file if none exists yet (first start).
    pub fn write_default_if_missing() -> Result<()> {
        let Some(path) = Self::path() else {
            return Ok(());
        };
        if path.exists() {
            return Ok(());
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, TEMPLATE)?;
        eprintln!(
            "tuifeed: wrote a default configuration to {}",
            path.display()
        );
        Ok(())
    }

    /// Clamp values that must stay in range so a saved file is always canonical.
    fn normalised(mut self) -> Self {
        self.layout.feeds_percent = self.layout.feeds();
        self
    }
}

/// The commented file written on the first start.
const TEMPLATE: &str = r#"# tuifeed configuration
#
# theme:  gruvbox | nord | dracula | catppuccin
# mode:   light | dark
# colors: auto | truecolor | 256 | 16
theme = "gruvbox"
mode = "dark"
colors = "auto"

# Most articles to show per feed (0 = all). A single feed can override it with `max`.
max_articles = 0

[layout]
# Width of the left (feeds) column in percent; the right column takes the rest.
feeds_percent = 35

# Add as many feeds as you like; repeat the [[sources]] block.
[[sources]]
name = "Heise"
url = "https://www.heise.de/rss/heise-atom.xml"
# max = 30
"#;

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("tuifeed-test-{}-{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn defaults_are_gruvbox_dark_and_35_percent() {
        let config = Config::default();
        assert_eq!(config.theme, ThemeName::Gruvbox);
        assert_eq!(config.mode, Mode::Dark);
        assert_eq!(config.layout.feeds(), 35);
        assert_eq!(config.layout.articles(), 65);
        assert_eq!(config.colors, ColorMode::Auto);
        assert_eq!(config.max_articles, Config::DEFAULT_MAX_ARTICLES);
        assert!(config.sources.is_empty());
    }

    #[test]
    fn describe_lists_every_source_with_url_and_cap() {
        let config = Config {
            sources: vec![
                Source {
                    name: Some("Heise".to_string()),
                    url: "https://a.example/feed".to_string(),
                    max: Some(50),
                },
                Source {
                    name: None,
                    url: "https://b.example/feed".to_string(),
                    max: None,
                },
            ],
            ..Config::default()
        };
        let text = config.describe();
        assert!(text.contains("2 Feeds"), "got:\n{text}");
        assert!(text.contains("1. Heise"), "got:\n{text}");
        assert!(text.contains("https://a.example/feed"), "got:\n{text}");
        assert!(text.contains("(max 50)"), "got:\n{text}");
        // A source without a name is marked as such and still shows its URL — exactly once.
        assert!(text.contains("(ohne Name)"), "got:\n{text}");
        assert_eq!(
            text.matches("https://b.example/feed").count(),
            1,
            "got:\n{text}"
        );
        assert!(text.contains("--feed"), "got:\n{text}");
    }

    #[test]
    fn describe_uses_the_singular_for_a_single_feed() {
        let config = Config {
            sources: vec![Source {
                name: Some("Heise".to_string()),
                url: "https://a.example/feed".to_string(),
                max: None,
            }],
            ..Config::default()
        };
        assert!(
            config.describe().contains("1 Feed:"),
            "{}",
            config.describe()
        );
    }

    #[test]
    fn describe_says_when_nothing_is_configured() {
        let text = Config::default().describe();
        assert!(text.contains("Keine Feeds konfiguriert"), "got:\n{text}");
        assert!(text.contains("[[sources]]"), "got:\n{text}");
    }

    #[test]
    fn an_unknown_theme_name_is_not_an_error() {
        let config = Config::from_toml("theme = \"neon\"\nmode = \"twilight\"\n")
            .expect("unknown names must still parse");
        assert_eq!(config.theme, ThemeName::Gruvbox);
        assert_eq!(config.mode, Mode::Dark);
    }

    #[test]
    fn sources_are_read_from_the_file() {
        let text = "[[sources]]\nname = \"A\"\nurl = \"https://a.example/feed\"\n\
                    [[sources]]\nurl = \"https://b.example/feed\"\n";
        let config = Config::from_toml(text).unwrap();
        assert_eq!(config.sources.len(), 2);
        assert_eq!(config.sources[0].name.as_deref(), Some("A"));
        assert_eq!(config.sources[0].url, "https://a.example/feed");
        assert_eq!(config.sources[1].name, None);
    }

    #[test]
    fn the_column_split_is_clamped_to_a_sane_range() {
        let config = Config::from_toml("[layout]\nfeeds_percent = 150\n").unwrap();
        assert_eq!(config.layout.feeds(), 99);
        assert_eq!(config.layout.articles(), 1);

        let config = Config::from_toml("[layout]\nfeeds_percent = 0\n").unwrap();
        assert_eq!(config.layout.feeds(), 1);
        assert_eq!(config.layout.articles(), 99);
    }

    #[test]
    fn the_configuration_survives_a_toml_round_trip() {
        let config = Config {
            theme: ThemeName::Catppuccin,
            mode: Mode::Light,
            colors: ColorMode::Ansi256,
            max_articles: 20,
            layout: Layout { feeds_percent: 40 },
            sources: vec![Source {
                name: Some("Beispiel".to_string()),
                url: "https://example.com/feed.xml".to_string(),
                max: Some(30),
            }],
        };
        let text = config.to_toml().unwrap();
        assert_eq!(Config::from_toml(&text).unwrap(), config);
    }

    #[test]
    fn loading_writes_and_reads_a_file() {
        let dir = scratch("roundtrip");
        let path = dir.join("config.toml");
        let config = Config {
            sources: vec![Source {
                name: None,
                url: "https://example.org/rss".to_string(),
                max: None,
            }],
            ..Config::default()
        };
        config.save_to(&path).unwrap();
        assert_eq!(Config::load_from(&path).unwrap(), config);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_source_can_cap_its_own_article_count() {
        let text = r#"
[[sources]]
name = "A"
url = "https://a.example/feed"
max = 30

[[sources]]
name = "B"
url = "https://b.example/feed"
"#;
        let config = Config::from_toml(text).unwrap();
        assert_eq!(config.sources[0].max, Some(30));
        assert_eq!(config.sources[1].max, None);
    }

    #[test]
    fn a_missing_file_is_an_error_for_load_from_but_not_for_load() {
        let path = scratch("missing").join("config.toml");
        assert!(Config::load_from(&path).is_err());
    }

    #[test]
    fn the_template_is_valid_and_offers_one_source() {
        let config = Config::from_toml(TEMPLATE).unwrap();
        assert_eq!(config.theme, ThemeName::Gruvbox);
        assert_eq!(config.layout.feeds_percent, 35);
        assert_eq!(config.sources.len(), 1);
        assert_eq!(config.sources[0].max, None);
        assert_eq!(config.colors, ColorMode::Auto);
        assert_eq!(config.max_articles, 0);
    }

    #[test]
    fn the_article_limit_and_colour_mode_are_read() {
        let config = Config::from_toml("max_articles = 7\ncolors = \"256\"\n").unwrap();
        assert_eq!(config.max_articles, 7);
        assert_eq!(config.colors, ColorMode::Ansi256);
    }

    #[test]
    fn an_unknown_colour_mode_falls_back_to_auto() {
        let config = Config::from_toml("colors = \"rainbow\"\n").unwrap();
        assert_eq!(config.colors, ColorMode::Auto);
    }
}
