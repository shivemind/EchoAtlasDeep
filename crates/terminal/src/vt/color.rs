use serde::{Deserialize, Serialize};

/// Terminal color value — mirrors xterm color model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Color {
    /// Default terminal color (inherits from theme).
    Default,
    /// One of the 256 indexed colors.
    Indexed(u8),
    /// True-color RGB.
    Rgb(u8, u8, u8),
}

impl Default for Color {
    fn default() -> Self {
        Color::Default
    }
}

/// Map a standard ANSI color index (0-7 / 8-15 for bright) to a Color.
pub fn ansi_color(index: u8) -> Color {
    Color::Indexed(index)
}
