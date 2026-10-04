//! Command-line options.
//!
//! tuifeed takes exactly two things on the command line: a request for the usage text and the
//! name of a single feed to pin (`--feed`). Everything else lives in the configuration file.

/// What the user asked for on the command line.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Options {
    /// Show only this feed (by configured name or URL) instead of the two-column view.
    pub feed: Option<String>,
    /// Print the configured feeds and stop.
    pub list: bool,
    /// Print the version and stop.
    pub version: bool,
    /// The user asked for the usage text.
    pub help: bool,
}

/// The usage text.
pub const USAGE: &str = "\
tuifeed — RSS-Feeds und ihre Artikel im Terminal

Aufruf:
  tuifeed [OPTIONEN]

Optionen:
  -f, --feed <NR|NAME>   nur diesen Feed anzeigen (Nummer aus --list oder Name),
                         ohne Feed-Liste
  -l, --list             die konfigurierten Feeds auflisten und beenden
  -V, --version          Version anzeigen
  -h, --help             diese Hilfe
";

/// The version line, e.g. `tuifeed 1.0.0`.
pub const VERSION: &str = concat!("tuifeed ", env!("CARGO_PKG_VERSION"));

/// Parse the arguments after the program name.
///
/// Errors carry a message for the user; the caller prints it together with [`USAGE`].
pub fn parse<I: IntoIterator<Item = String>>(args: I) -> Result<Options, String> {
    let mut options = Options::default();
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => options.help = true,
            "-V" | "--version" => options.version = true,
            "-l" | "--list" => options.list = true,
            "-f" | "--feed" => {
                let value = args
                    .next()
                    .ok_or_else(|| "--feed braucht einen Namen".to_string())?;
                if value.trim().is_empty() {
                    return Err("--feed braucht einen Namen".to_string());
                }
                options.feed = Some(value);
            }
            other if other.starts_with("--feed=") => {
                let value = &other["--feed=".len()..];
                if value.trim().is_empty() {
                    return Err("--feed braucht einen Namen".to_string());
                }
                options.feed = Some(value.to_string());
            }
            other => return Err(format!("unbekannte Option: {other}")),
        }
    }
    Ok(options)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_args(args: &[&str]) -> Result<Options, String> {
        parse(args.iter().map(|arg| (*arg).to_string()))
    }

    #[test]
    fn no_arguments_means_the_normal_view() {
        assert_eq!(parse_args(&[]).unwrap(), Options::default());
    }

    #[test]
    fn feed_accepts_both_spellings() {
        let expected = Options {
            feed: Some("Heise".to_string()),
            list: false,
            version: false,
            help: false,
        };
        assert_eq!(parse_args(&["--feed", "Heise"]).unwrap(), expected);
        assert_eq!(parse_args(&["-f", "Heise"]).unwrap(), expected);
        assert_eq!(parse_args(&["--feed=Heise"]).unwrap(), expected);
    }

    #[test]
    fn version_is_recognised_and_names_the_package() {
        assert!(parse_args(&["-V"]).unwrap().version);
        assert!(parse_args(&["--version"]).unwrap().version);
        assert!(!parse_args(&[]).unwrap().version);
        assert_eq!(VERSION, format!("tuifeed {}", env!("CARGO_PKG_VERSION")));
    }

    #[test]
    fn list_is_recognised() {
        assert!(parse_args(&["-l"]).unwrap().list);
        assert!(parse_args(&["--list"]).unwrap().list);
        assert!(!parse_args(&[]).unwrap().list);
    }

    #[test]
    fn list_and_feed_can_be_combined() {
        let options = parse_args(&["-l", "-f", "Heise"]).unwrap();
        assert!(options.list);
        assert_eq!(options.feed.as_deref(), Some("Heise"));
    }

    #[test]
    fn help_is_recognised() {
        assert!(parse_args(&["-h"]).unwrap().help);
        assert!(parse_args(&["--help"]).unwrap().help);
    }

    #[test]
    fn a_feed_without_a_name_is_an_error() {
        assert!(parse_args(&["--feed"]).is_err());
        assert!(parse_args(&["-f"]).is_err());
        assert!(parse_args(&["--feed="]).is_err());
        assert!(parse_args(&["--feed", " "]).is_err());
    }

    #[test]
    fn unknown_options_are_rejected() {
        let error = parse_args(&["--nope"]).unwrap_err();
        assert!(error.contains("--nope"), "got: {error}");
        assert!(parse_args(&["extra"]).is_err());
    }
}
