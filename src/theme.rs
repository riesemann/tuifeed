//! Colour themes: four families, each in a light and a dark variant.
//!
//! Each palette holds the families' own RGB values; they are mapped onto a small set
//! of semantic tokens and down-converted to whatever colour depth the terminal offers.

use ratatui::style::Color;

use crate::config::{ColorMode, Mode, ThemeName};

/// How many colours the terminal can show.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorDepth {
    /// 24-bit colour.
    TrueColor,
    /// The 256-colour palette.
    Ansi256,
    /// The 16 basic colours.
    Ansi16,
}

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
    /// The background of the selected row. It stays a background only: the selected row keeps
    /// its own text colours (title/date/teaser), so the highlight does not flatten them.
    pub selection_bg: Color,
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
    danger: u32,
}

/// The depth to use for a configured colour mode.
pub fn depth_for(mode: ColorMode) -> ColorDepth {
    match mode {
        ColorMode::TrueColor => ColorDepth::TrueColor,
        ColorMode::Ansi256 => ColorDepth::Ansi256,
        ColorMode::Ansi16 => ColorDepth::Ansi16,
        ColorMode::Auto => detected_depth(),
    }
}

/// Guess the colour depth from the environment.
pub fn detected_depth() -> ColorDepth {
    let colorterm = std::env::var("COLORTERM")
        .unwrap_or_default()
        .to_lowercase();
    if colorterm.contains("truecolor") || colorterm.contains("24bit") {
        return ColorDepth::TrueColor;
    }
    let term = std::env::var("TERM").unwrap_or_default().to_lowercase();
    if term.contains("256color") {
        return ColorDepth::Ansi256;
    }
    ColorDepth::Ansi16
}

/// The theme for a family, mode and colour depth.
pub fn theme(family: ThemeName, mode: Mode, depth: ColorDepth) -> Theme {
    let palette = match (family, mode) {
        (ThemeName::Gruvbox, Mode::Dark) => gruvbox_dark(),
        (ThemeName::Gruvbox, Mode::Light) => gruvbox_light(),
        (ThemeName::Nord, Mode::Dark) => nord_dark(),
        (ThemeName::Nord, Mode::Light) => nord_light(),
        (ThemeName::Dracula, Mode::Dark) => dracula_dark(),
        (ThemeName::Dracula, Mode::Light) => dracula_light(),
        (ThemeName::Catppuccin, Mode::Dark) => catppuccin_dark(),
        (ThemeName::Catppuccin, Mode::Light) => catppuccin_light(),
    };
    resolve(palette, depth)
}

fn resolve(palette: Palette, depth: ColorDepth) -> Theme {
    let c = |value: u32| color(value, depth);
    Theme {
        bg: c(palette.bg),
        fg: c(palette.fg),
        border_focused: c(palette.accent),
        border_unfocused: c(palette.muted),
        selection_bg: c(palette.selection_bg),
        title: c(palette.accent),
        date: c(palette.muted),
        teaser: c(palette.secondary),
        count: c(palette.muted),
        loading: c(palette.muted),
        error: c(palette.danger),
    }
}

/// Convert a `0xRRGGBB` value to a colour at the given depth.
fn color(value: u32, depth: ColorDepth) -> Color {
    let r = ((value >> 16) & 0xff) as u8;
    let g = ((value >> 8) & 0xff) as u8;
    let b = (value & 0xff) as u8;
    match depth {
        ColorDepth::TrueColor => Color::Rgb(r, g, b),
        ColorDepth::Ansi256 => Color::Indexed(nearest_256(r, g, b)),
        ColorDepth::Ansi16 => Color::Indexed(nearest_16(r, g, b)),
    }
}

/// The nearest entry of the xterm 256-colour palette (cube + grayscale ramp).
fn nearest_256(r: u8, g: u8, b: u8) -> u8 {
    // The 6x6x6 colour cube (indices 16..231).
    let to_cube = |v: u8| -> u8 {
        if v < 48 {
            0
        } else if v < 115 {
            1
        } else {
            ((v as u16 - 35) / 40) as u8
        }
    };
    let level = |i: u8| -> u8 { if i == 0 { 0 } else { 55 + 40 * i } };
    let (cr, cg, cb) = (to_cube(r), to_cube(g), to_cube(b));
    let cube = (level(cr), level(cg), level(cb));
    let cube_index = 16 + 36 * cr as u16 + 6 * cg as u16 + cb as u16;

    // The 24-step grayscale ramp (indices 232..255).
    let average = ((r as u16 + g as u16 + b as u16) / 3) as u8;
    let gray_index = if average > 238 {
        23
    } else {
        (average.saturating_sub(3) / 10) as u16
    };
    let gray = 8 + 10 * gray_index as u8;
    let gray_rgb = (gray, gray, gray);

    let distance = |a: (u8, u8, u8), b: (u8, u8, u8)| -> u32 {
        let dr = a.0 as i32 - b.0 as i32;
        let dg = a.1 as i32 - b.1 as i32;
        let db = a.2 as i32 - b.2 as i32;
        (dr * dr + dg * dg + db * db) as u32
    };

    if distance((r, g, b), gray_rgb) < distance((r, g, b), cube) {
        (232 + gray_index) as u8
    } else {
        cube_index as u8
    }
}

/// The standard xterm colours of the 16-colour palette.
const ANSI16: [(u8, u8, u8); 16] = [
    (0, 0, 0),
    (205, 0, 0),
    (0, 205, 0),
    (205, 205, 0),
    (0, 0, 238),
    (205, 0, 205),
    (0, 205, 205),
    (229, 229, 229),
    (127, 127, 127),
    (255, 0, 0),
    (0, 255, 0),
    (255, 255, 0),
    (92, 92, 255),
    (255, 0, 255),
    (0, 255, 255),
    (255, 255, 255),
];

/// The nearest index of the 16 basic colours.
fn nearest_16(r: u8, g: u8, b: u8) -> u8 {
    let mut best = 0u8;
    let mut best_distance = u32::MAX;
    for (index, (cr, cg, cb)) in ANSI16.iter().enumerate() {
        let dr = r as i32 - *cr as i32;
        let dg = g as i32 - *cg as i32;
        let db = b as i32 - *cb as i32;
        let distance = (dr * dr + dg * dg + db * db) as u32;
        if distance < best_distance {
            best_distance = distance;
            best = index as u8;
        }
    }
    best
}

fn gruvbox_dark() -> Palette {
    Palette {
        bg: 0x282828,
        fg: 0xebdbb2,
        accent: 0xfabd2f,    // bright yellow
        secondary: 0x83a598, // bright blue
        muted: 0x928374,     // gray
        selection_bg: 0x3c3836,
        danger: 0xfb4934,
    }
}

fn gruvbox_light() -> Palette {
    Palette {
        bg: 0xfbf1c7,
        fg: 0x3c3836,
        accent: 0xb57614,    // neutral yellow
        secondary: 0x076678, // neutral blue
        muted: 0x928374,
        selection_bg: 0xebdbb2,
        danger: 0x9d0006,
    }
}

fn nord_dark() -> Palette {
    Palette {
        bg: 0x2e3440,        // nord0
        fg: 0xd8dee9,        // nord4
        accent: 0x88c0d0,    // nord8
        secondary: 0x81a1c1, // nord9
        muted: 0x616e88,
        selection_bg: 0x434c5e, // nord2
        danger: 0xbf616a,       // nord11
    }
}

fn nord_light() -> Palette {
    Palette {
        bg: 0xeceff4,        // nord6
        fg: 0x2e3440,        // nord0
        accent: 0x5e81ac,    // nord10
        secondary: 0x81a1c1, // nord9
        muted: 0x7b88a1,
        selection_bg: 0xd8dee9, // nord4
        danger: 0xbf616a,       // nord11
    }
}

fn dracula_dark() -> Palette {
    Palette {
        bg: 0x282a36,
        fg: 0xf8f8f2,
        accent: 0xbd93f9,       // purple
        secondary: 0x8be9fd,    // cyan
        muted: 0x6272a4,        // comment
        selection_bg: 0x44475a, // current line
        danger: 0xff5555,
    }
}

fn dracula_light() -> Palette {
    // Dracula's light companion is "Alucard".
    Palette {
        bg: 0xf5f5f5,
        fg: 0x1f1f1f,
        accent: 0x644ac9,
        secondary: 0x036a96,
        muted: 0x6c664b,
        selection_bg: 0xdedede,
        danger: 0xcb3a2a,
    }
}

fn catppuccin_dark() -> Palette {
    // Mocha.
    Palette {
        bg: 0x1e1e2e,           // base
        fg: 0xcdd6f4,           // text
        accent: 0x89b4fa,       // blue
        secondary: 0x94e2d5,    // teal
        muted: 0x7f849c,        // overlay1
        selection_bg: 0x313244, // surface0
        danger: 0xf38ba8,       // red
    }
}

fn catppuccin_light() -> Palette {
    // Latte.
    Palette {
        bg: 0xeff1f5,           // base
        fg: 0x4c4f69,           // text
        accent: 0x1e66f5,       // blue
        secondary: 0x179299,    // teal
        muted: 0x8c8fa1,        // overlay1
        selection_bg: 0xccd0da, // surface0
        danger: 0xd20f39,       // red
    }
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

    fn tokens(t: &Theme) -> [Color; 11] {
        [
            t.bg,
            t.fg,
            t.border_focused,
            t.border_unfocused,
            t.selection_bg,
            t.title,
            t.date,
            t.teaser,
            t.count,
            t.loading,
            t.error,
        ]
    }

    #[test]
    fn every_family_has_both_modes() {
        assert_eq!(all().len(), 8);
    }

    #[test]
    fn every_token_is_set_at_every_depth() {
        for depth in [
            ColorDepth::TrueColor,
            ColorDepth::Ansi256,
            ColorDepth::Ansi16,
        ] {
            for (family, mode) in all() {
                for token in tokens(&theme(family, mode, depth)) {
                    assert_ne!(token, Color::Reset, "{family:?}/{mode:?}/{depth:?}");
                }
            }
        }
    }

    #[test]
    fn the_configured_mode_picks_the_depth() {
        assert_eq!(depth_for(ColorMode::TrueColor), ColorDepth::TrueColor);
        assert_eq!(depth_for(ColorMode::Ansi256), ColorDepth::Ansi256);
        assert_eq!(depth_for(ColorMode::Ansi16), ColorDepth::Ansi16);
    }

    #[test]
    fn truecolor_uses_rgb_and_the_lower_depths_use_indices() {
        let rgb = theme(ThemeName::Gruvbox, Mode::Dark, ColorDepth::TrueColor);
        assert!(matches!(rgb.bg, Color::Rgb(..)));

        let c256 = theme(ThemeName::Gruvbox, Mode::Dark, ColorDepth::Ansi256);
        assert!(matches!(c256.bg, Color::Indexed(_)));

        let c16 = theme(ThemeName::Gruvbox, Mode::Dark, ColorDepth::Ansi16);
        match c16.bg {
            Color::Indexed(i) => assert!(i < 16, "expected a basic colour, got {i}"),
            other => panic!("expected an index, got {other:?}"),
        }
    }

    #[test]
    fn the_256_conversion_hits_the_expected_slots() {
        assert_eq!(nearest_256(255, 0, 0), 196); // bright red in the cube
        assert_eq!(nearest_256(0, 0, 0), 16); // black
        assert_eq!(nearest_256(255, 255, 255), 231); // white
        // A pure grey lands on the grayscale ramp, not in the cube.
        assert!(nearest_256(128, 128, 128) >= 232);
    }

    #[test]
    fn the_16_conversion_hits_the_expected_slots() {
        assert_eq!(nearest_16(0, 0, 0), 0);
        assert_eq!(nearest_16(255, 0, 0), 9);
        assert_eq!(nearest_16(255, 255, 255), 15);
    }

    #[test]
    fn light_and_dark_differ_per_family() {
        for family in ThemeName::ALL {
            let light = theme(family, Mode::Light, ColorDepth::TrueColor);
            let dark = theme(family, Mode::Dark, ColorDepth::TrueColor);
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
        let t = theme(ThemeName::Gruvbox, Mode::Dark, ColorDepth::TrueColor);
        assert_eq!(t.bg, Color::Rgb(0x28, 0x28, 0x28));
        assert_eq!(t.fg, Color::Rgb(0xeb, 0xdb, 0xb2));
        assert_eq!(t.title, Color::Rgb(0xfa, 0xbd, 0x2f));
    }

    #[test]
    fn title_date_and_teaser_are_three_different_colours() {
        for (family, mode) in all() {
            let t = theme(family, mode, ColorDepth::TrueColor);
            assert_ne!(t.title, t.date, "{family:?}/{mode:?}");
            assert_ne!(t.date, t.teaser, "{family:?}/{mode:?}");
            assert_ne!(t.title, t.teaser, "{family:?}/{mode:?}");
        }
    }
}
