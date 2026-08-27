use ratatui::style::{Color, Style};

/// Standard content inset for full-width selectable rows.
pub const SELECTABLE_LEFT_PADDING: u16 = 2;

/// Terminal color scheme used to choose a kit palette.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ColorScheme {
    #[default]
    Dark,
    Light,
}

impl ColorScheme {
    /// Detects a scheme from `UNPEEL_TUI_THEME=dark|light`, then the common
    /// `COLORFGBG` terminal hint. Detection deliberately falls back to dark;
    /// Apps with their own appearance setting should pass it explicitly.
    #[must_use]
    pub fn detect() -> Self {
        std::env::var("UNPEEL_TUI_THEME")
            .ok()
            .as_deref()
            .and_then(parse_name)
            .or_else(|| {
                std::env::var("COLORFGBG")
                    .ok()
                    .as_deref()
                    .and_then(parse_colorfgbg)
            })
            .unwrap_or_default()
    }
}

/// Small shared palette for Ratatui Apps that do not own a product theme.
///
/// Backgrounds remain transparent except for selected rows. This keeps Apps
/// native to the surrounding terminal while ensuring selection contrast in
/// both dark and light terminals.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KitTheme {
    pub scheme: ColorScheme,
    pub text: Color,
    pub muted: Color,
    pub subtle: Color,
    pub accent: Color,
    pub surface: Color,
    pub danger: Color,
    pub selected_row: Style,
    pub scrollbar_track: Style,
    pub scrollbar_thumb: Style,
}

impl KitTheme {
    #[must_use]
    pub const fn dark() -> Self {
        Self {
            scheme: ColorScheme::Dark,
            text: Color::White,
            muted: Color::Gray,
            subtle: Color::DarkGray,
            accent: Color::LightBlue,
            surface: Color::Rgb(39, 39, 42),
            danger: Color::LightRed,
            selected_row: Style::new().fg(Color::White).bg(Color::Rgb(63, 63, 70)),
            scrollbar_track: Style::new().fg(Color::Rgb(65, 65, 65)),
            scrollbar_thumb: Style::new().fg(Color::Rgb(145, 145, 145)),
        }
    }

    #[must_use]
    pub const fn light() -> Self {
        Self {
            scheme: ColorScheme::Light,
            text: Color::Black,
            muted: Color::DarkGray,
            subtle: Color::Gray,
            accent: Color::Blue,
            surface: Color::Rgb(238, 238, 240),
            danger: Color::Red,
            selected_row: Style::new().fg(Color::Black).bg(Color::Rgb(216, 216, 220)),
            scrollbar_track: Style::new().fg(Color::Rgb(200, 200, 200)),
            scrollbar_thumb: Style::new().fg(Color::Rgb(100, 100, 100)),
        }
    }

    #[must_use]
    pub const fn for_scheme(scheme: ColorScheme) -> Self {
        match scheme {
            ColorScheme::Dark => Self::dark(),
            ColorScheme::Light => Self::light(),
        }
    }

    #[must_use]
    pub fn detected() -> Self {
        Self::for_scheme(ColorScheme::detect())
    }
}

impl Default for KitTheme {
    fn default() -> Self {
        Self::dark()
    }
}

fn parse_name(value: &str) -> Option<ColorScheme> {
    match value.trim().to_ascii_lowercase().as_str() {
        "dark" => Some(ColorScheme::Dark),
        "light" => Some(ColorScheme::Light),
        _ => None,
    }
}

fn parse_colorfgbg(value: &str) -> Option<ColorScheme> {
    let background = value
        .split([';', ':'])
        .next_back()?
        .trim()
        .parse::<u8>()
        .ok()?;
    Some(if matches!(background, 0..=6 | 8) {
        ColorScheme::Dark
    } else {
        ColorScheme::Light
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn common_colorfgbg_values_map_to_expected_scheme() {
        assert_eq!(parse_colorfgbg("15;0"), Some(ColorScheme::Dark));
        assert_eq!(parse_colorfgbg("0;15"), Some(ColorScheme::Light));
        assert_eq!(parse_colorfgbg("0:7"), Some(ColorScheme::Light));
        assert_eq!(parse_colorfgbg("unknown"), None);
    }

    #[test]
    fn selected_rows_have_contrasting_full_palette_defaults() {
        assert_eq!(KitTheme::dark().selected_row.fg, Some(Color::White));
        assert_eq!(KitTheme::light().selected_row.fg, Some(Color::Black));
        assert_ne!(
            KitTheme::dark().selected_row.bg,
            KitTheme::light().selected_row.bg
        );
    }
}
