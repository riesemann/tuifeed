//! Colour themes: four families, each in a light and a dark variant.
//!
//! Colours are the families' own RGB values (truecolor terminals); each palette is
//! mapped onto a small set of semantic tokens the UI draws with.

use ratatui::style::Color;

use crate::config::{Mode, ThemeName};

/// The semantic colours the interface uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Theme {
    /// The window background.
    pub bg: Color,
    /// The default text colour.
    pub fg: Color,
    /// The border of the focused column.
    pub border_focused: Color,
    /// The border of the unfocused column.
    pub border_unfocused: Color,
    /// The background of the selected row.
    pub selection_bg: Color,
    /// The text colour on the selected row.
    pub selection_fg: Color,
    /// The article title.
    pub title: Color,
    /// The article date.
    pub date: Color,
    /// The article teaser.
    pub teaser: Color,
    /// The article count / secondary emphasis.
    pub count: Color,
    /// The "loading" hint.
    pub loading: Color,
    /// Errors.
    pub error: Color,
}

/// The raw colours of one family variant.
struct Palette {
    bg: u32,
    fg: u32,
    accent: u32,
    secondary: u32,
    muted: u32,
    selection_bg: u32,
    selection_fg: u32,
    danger: u32,
}

/// The theme for a family and mode.
pub fn theme(family: ThemeName, mode: Mode) -> Theme {
    match (family, mode) {
        (ThemeName::Gruvbox, Mode::Dark) => gruvbox_dark(),
        (ThemeName::Gruvbox, Mode::Light) => gruvbox_light(),
        (ThemeName::Nord, Mode::Dark) => nord_dark(),
        (ThemeName::Nord, Mode::Light) => nord_light(),
        (ThemeName::Dracula, Mode::Dark) => dracula_dark(),
        (ThemeName::Dracula, Mode::Light) => dracula_light(),
        (ThemeName::Catppuccin, Mode::Dark) => catppuccin_dark(),
        (ThemeName::Catppuccin, Mode::Light) => catppuccin_light(),
    }
}

fn resolve(palette: Palette) -> Theme {
    Theme {
        bg: rgb(palette.bg),
        fg: rgb(palette.fg),
        border_focused: rgb(palette.accent),
        border_unfocused: rgb(palette.muted),
        selection_bg: rgb(palette.selection_bg),
        selection_fg: rgb(palette.selection_fg),
        title: rgb(palette.accent),
        date: rgb(palette.muted),
        teaser: rgb(palette.secondary),
        count: rgb(palette.muted),
        loading: rgb(palette.muted),
        error: rgb(palette.danger),
    }
}

fn rgb(value: u32) -> Color {
    Color::Rgb(
        ((value >> 16) & 0xff) as u8,
        ((value >> 8) & 0xff) as u8,
        (value & 0xff) as u8,
    )
}

fn gruvbox_dark() -> Theme {
    resolve(Palette {
        bg: 0x282828,
        fg: 0xebdbb2,
        accent: 0xfabd2f,    // bright yellow
        secondary: 0x83a598, // bright blue
        muted: 0x928374,     // gray
        selection_bg: 0x3c3836,
        selection_fg: 0xebdbb2,
        danger: 0xfb4934,
    })
}

fn gruvbox_light() -> Theme {
    resolve(Palette {
        bg: 0xfbf1c7,
        fg: 0x3c3836,
        accent: 0xb57614,    // neutral yellow
        secondary: 0x076678, // neutral blue
        muted: 0x928374,
        selection_bg: 0xebdbb2,
        selection_fg: 0x3c3836,
        danger: 0x9d0006,
    })
}

fn nord_dark() -> Theme {
    resolve(Palette {
        bg: 0x2e3440,        // nord0
        fg: 0xd8dee9,        // nord4
        accent: 0x88c0d0,    // nord8
        secondary: 0x81a1c1, // nord9
        muted: 0x616e88,
        selection_bg: 0x434c5e, // nord2
        selection_fg: 0xeceff4, // nord6
        danger: 0xbf616a,       // nord11
    })
}

fn nord_light() -> Theme {
    resolve(Palette {
        bg: 0xeceff4,        // nord6
        fg: 0x2e3440,        // nord0
        accent: 0x5e81ac,    // nord10
        secondary: 0x81a1c1, // nord9
        muted: 0x7b88a1,
        selection_bg: 0xd8dee9, // nord4
        selection_fg: 0x2e3440, // nord0
        danger: 0xbf616a,       // nord11
    })
}

fn dracula_dark() -> Theme {
    resolve(Palette {
        bg: 0x282a36,
        fg: 0xf8f8f2,
        accent: 0xbd93f9,       // purple
        secondary: 0x8be9fd,    // cyan
        muted: 0x6272a4,        // comment
        selection_bg: 0x44475a, // current line
        selection_fg: 0xf8f8f2,
        danger: 0xff5555,
    })
}

fn dracula_light() -> Theme {
    // Dracula's light companion is "Alucard".
    resolve(Palette {
        bg: 0xf5f5f5,
        fg: 0x1f1f1f,
        accent: 0x644ac9,
        secondary: 0x036a96,
        muted: 0x6c664b,
        selection_bg: 0xdedede,
        selection_fg: 0x1f1f1f,
        danger: 0xcb3a2a,
    })
}

fn catppuccin_dark() -> Theme {
    // Mocha.
    resolve(Palette {
        bg: 0x1e1e2e,           // base
        fg: 0xcdd6f4,           // text
        accent: 0x89b4fa,       // blue
        secondary: 0x94e2d5,    // teal
        muted: 0x7f849c,        // overlay1
        selection_bg: 0x313244, // surface0
        selection_fg: 0xcdd6f4,
        danger: 0xf38ba8, // red
    })
}

fn catppuccin_light() -> Theme {
    // Latte.
    resolve(Palette {
        bg: 0xeff1f5,           // base
        fg: 0x4c4f69,           // text
        accent: 0x1e66f5,       // blue
        secondary: 0x179299,    // teal
        muted: 0x8c8fa1,        // overlay1
        selection_bg: 0xccd0da, // surface0
        selection_fg: 0x4c4f69,
        danger: 0xd20f39, // red
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all() -> Vec<(ThemeName, Mode)> {
        let mut combos = Vec::new();
        for family in ThemeName::ALL {
            for mode in [Mode::Light, Mode::Dark] {
                combos.push((family, mode));
            }
        }
        combos
    }

    #[test]
    fn every_family_has_both_modes() {
        assert_eq!(all().len(), 8);
    }

    #[test]
    fn no_token_is_left_at_the_terminal_default() {
        for (family, mode) in all() {
            let t = theme(family, mode);
            let tokens = [
                t.bg,
                t.fg,
                t.border_focused,
                t.border_unfocused,
                t.selection_bg,
                t.selection_fg,
                t.title,
                t.date,
                t.teaser,
                t.count,
                t.loading,
                t.error,
            ];
            for token in tokens {
                assert!(
                    matches!(token, Color::Rgb(..)),
                    "{family:?}/{mode:?} has a non-RGB token: {token:?}"
                );
            }
        }
    }

    #[test]
    fn light_and_dark_differ_per_family() {
        for family in ThemeName::ALL {
            let light = theme(family, Mode::Light);
            let dark = theme(family, Mode::Dark);
            assert_ne!(
                light.bg, dark.bg,
                "{family:?}: light and dark share a background"
            );
            assert_ne!(
                light.fg, dark.fg,
                "{family:?}: light and dark share a foreground"
            );
        }
    }

    #[test]
    fn gruvbox_keeps_its_official_colours() {
        // Regression: the exact tones must not drift.
        let t = theme(ThemeName::Gruvbox, Mode::Dark);
        assert_eq!(t.bg, Color::Rgb(0x28, 0x28, 0x28));
        assert_eq!(t.fg, Color::Rgb(0xeb, 0xdb, 0xb2));
        assert_eq!(t.title, Color::Rgb(0xfa, 0xbd, 0x2f));
    }

    #[test]
    fn title_date_and_teaser_are_three_different_colours() {
        for (family, mode) in all() {
            let t = theme(family, mode);
            assert_ne!(t.title, t.date, "{family:?}/{mode:?}");
            assert_ne!(t.date, t.teaser, "{family:?}/{mode:?}");
            assert_ne!(t.title, t.teaser, "{family:?}/{mode:?}");
        }
    }
}
