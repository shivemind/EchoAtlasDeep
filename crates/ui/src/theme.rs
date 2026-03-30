#![allow(dead_code, unused_imports, unused_variables)]
use ratatui::style::Color;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::highlight::TokenKind;

/// One complete color theme.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Theme {
    pub name: String,

    // ── Syntax token colors ─────────────────────────────────────────────────
    pub keyword: ThemeColor,
    pub keyword_control: ThemeColor,   // if/else/return/match
    pub string: ThemeColor,
    pub string_escape: ThemeColor,     // \n \t etc
    pub number: ThemeColor,
    pub float: ThemeColor,
    pub comment: ThemeColor,
    pub comment_doc: ThemeColor,       // //! ///
    pub function: ThemeColor,
    pub function_builtin: ThemeColor,
    pub method: ThemeColor,
    pub type_name: ThemeColor,
    pub type_builtin: ThemeColor,      // i32, str, bool
    pub variable: ThemeColor,
    pub variable_builtin: ThemeColor,  // self, super
    pub constant: ThemeColor,
    pub constant_builtin: ThemeColor,  // true, false, null
    pub operator: ThemeColor,
    pub punctuation: ThemeColor,
    pub punctuation_bracket: ThemeColor,
    pub punctuation_delimiter: ThemeColor,
    pub attribute: ThemeColor,
    pub label: ThemeColor,             // lifetime labels 'a
    pub namespace: ThemeColor,
    pub property: ThemeColor,
    pub tag: ThemeColor,               // HTML/JSX tags
    pub tag_attribute: ThemeColor,

    // ── Diff colors ─────────────────────────────────────────────────────────
    pub diff_add: ThemeColor,
    pub diff_delete: ThemeColor,
    pub diff_change: ThemeColor,
    pub diff_add_bg: ThemeColor,
    pub diff_delete_bg: ThemeColor,

    // ── UI chrome ────────────────────────────────────────────────────────────
    pub bg: ThemeColor,
    pub bg_dark: ThemeColor,
    pub bg_highlight: ThemeColor,      // selection, current line
    pub fg: ThemeColor,
    pub fg_dark: ThemeColor,
    pub fg_gutter: ThemeColor,         // line numbers, signs
    pub border: ThemeColor,
    pub border_highlight: ThemeColor,  // focused pane border

    // ── Status bar ───────────────────────────────────────────────────────────
    pub statusbar_bg: ThemeColor,
    pub statusbar_fg: ThemeColor,
    pub statusbar_mode_normal: ThemeColor,
    pub statusbar_mode_insert: ThemeColor,
    pub statusbar_mode_visual: ThemeColor,
    pub statusbar_mode_command: ThemeColor,

    // ── Cursor ───────────────────────────────────────────────────────────────
    pub cursor: ThemeColor,
    pub cursor_line: ThemeColor,       // current line background

    // ── Selection ────────────────────────────────────────────────────────────
    pub selection: ThemeColor,
    pub selection_inactive: ThemeColor,

    // ── Diagnostic colors ────────────────────────────────────────────────────
    pub error: ThemeColor,
    pub warning: ThemeColor,
    pub info: ThemeColor,
    pub hint: ThemeColor,
}

/// A color that can be RGB, an indexed terminal color, or the default terminal color.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ThemeColor {
    Rgb(u8, u8, u8),
    Indexed(u8),
    Reset,
    Named(String),  // "red", "blue", etc.
}

impl ThemeColor {
    pub fn to_ratatui(&self) -> Color {
        match self {
            ThemeColor::Rgb(r, g, b) => Color::Rgb(*r, *g, *b),
            ThemeColor::Indexed(n)   => Color::Indexed(*n),
            ThemeColor::Reset        => Color::Reset,
            ThemeColor::Named(name)  => match name.as_str() {
                "black"         => Color::Black,
                "red"           => Color::Red,
                "green"         => Color::Green,
                "yellow"        => Color::Yellow,
                "blue"          => Color::Blue,
                "magenta"       => Color::Magenta,
                "cyan"          => Color::Cyan,
                "white"         => Color::White,
                "gray" | "grey" => Color::Gray,
                "dark_gray"     => Color::DarkGray,
                "light_red"     => Color::LightRed,
                "light_green"   => Color::LightGreen,
                "light_yellow"  => Color::LightYellow,
                "light_blue"    => Color::LightBlue,
                "light_magenta" => Color::LightMagenta,
                "light_cyan"    => Color::LightCyan,
                _               => Color::Reset,
            },
        }
    }
}

/// Map a token kind to the theme color for it.
impl Theme {
    pub fn token_color(&self, kind: TokenKind) -> Color {
        match kind {
            TokenKind::Keyword     => self.keyword.to_ratatui(),
            TokenKind::String      => self.string.to_ratatui(),
            TokenKind::Comment     => self.comment.to_ratatui(),
            TokenKind::Number      => self.number.to_ratatui(),
            TokenKind::Function    => self.function.to_ratatui(),
            TokenKind::Type        => self.type_name.to_ratatui(),
            TokenKind::Variable    => self.variable.to_ratatui(),
            TokenKind::Operator    => self.operator.to_ratatui(),
            TokenKind::Punctuation => self.punctuation.to_ratatui(),
            TokenKind::Constant    => self.constant.to_ratatui(),
            TokenKind::Attribute   => self.attribute.to_ratatui(),
            TokenKind::Default     => self.fg.to_ratatui(),
        }
    }
}

// ─── Built-in themes ─────────────────────────────────────────────────────────

fn rgb(r: u8, g: u8, b: u8) -> ThemeColor { ThemeColor::Rgb(r, g, b) }
fn named(s: &str) -> ThemeColor { ThemeColor::Named(s.to_string()) }

pub fn catppuccin_mocha() -> Theme {
    Theme {
        name: "catppuccin-mocha".into(),
        keyword:                rgb(203, 166, 247), // mauve
        keyword_control:        rgb(243, 139, 168), // red
        string:                 rgb(166, 227, 161), // green
        string_escape:          rgb(250, 179, 135), // peach
        number:                 rgb(250, 179, 135), // peach
        float:                  rgb(250, 179, 135),
        comment:                rgb(88,  91,  112), // overlay0
        comment_doc:            rgb(108, 112, 134), // overlay1
        function:               rgb(137, 180, 250), // blue
        function_builtin:       rgb(137, 180, 250),
        method:                 rgb(137, 180, 250),
        type_name:              rgb(249, 226, 175), // yellow
        type_builtin:           rgb(250, 179, 135), // peach
        variable:               rgb(205, 214, 244), // text
        variable_builtin:       rgb(243, 139, 168), // red
        constant:               rgb(250, 179, 135), // peach
        constant_builtin:       rgb(250, 179, 135),
        operator:               rgb(137, 220, 235), // sky
        punctuation:            rgb(148, 226, 213), // teal
        punctuation_bracket:    rgb(148, 226, 213),
        punctuation_delimiter:  rgb(148, 226, 213),
        attribute:              rgb(245, 194, 231), // pink
        label:                  rgb(249, 226, 175), // yellow
        namespace:              rgb(249, 226, 175),
        property:               rgb(137, 220, 235),
        tag:                    rgb(137, 180, 250),
        tag_attribute:          rgb(249, 226, 175),
        diff_add:               rgb(166, 227, 161), // green
        diff_delete:            rgb(243, 139, 168), // red
        diff_change:            rgb(249, 226, 175), // yellow
        diff_add_bg:            rgb(40,  55,  40),
        diff_delete_bg:         rgb(60,  30,  30),
        bg:                     rgb(30,  30,  46),  // base
        bg_dark:                rgb(24,  24,  37),  // mantle
        bg_highlight:           rgb(49,  50,  68),  // surface0
        fg:                     rgb(205, 214, 244), // text
        fg_dark:                rgb(166, 173, 200), // subtext1
        fg_gutter:              rgb(88,  91,  112), // overlay0
        border:                 rgb(69,  71,  90),  // surface1
        border_highlight:       rgb(137, 180, 250), // blue
        statusbar_bg:           rgb(24,  24,  37),
        statusbar_fg:           rgb(205, 214, 244),
        statusbar_mode_normal:  rgb(137, 180, 250), // blue
        statusbar_mode_insert:  rgb(166, 227, 161), // green
        statusbar_mode_visual:  rgb(203, 166, 247), // mauve
        statusbar_mode_command: rgb(249, 226, 175), // yellow
        cursor:                 rgb(205, 214, 244),
        cursor_line:            rgb(49,  50,  68),
        selection:              rgb(69,  71,  90),
        selection_inactive:     rgb(49,  50,  68),
        error:                  rgb(243, 139, 168), // red
        warning:                rgb(249, 226, 175), // yellow
        info:                   rgb(137, 220, 235), // sky
        hint:                   rgb(148, 226, 213), // teal
    }
}

pub fn tokyonight_storm() -> Theme {
    Theme {
        name: "tokyonight".into(),
        keyword:                rgb(187, 154, 247),
        keyword_control:        rgb(247, 118, 142),
        string:                 rgb(158, 206, 106),
        string_escape:          rgb(255, 158, 100),
        number:                 rgb(255, 158, 100),
        float:                  rgb(255, 158, 100),
        comment:                rgb(86,  95,  137),
        comment_doc:            rgb(100, 110, 150),
        function:               rgb(122, 162, 247),
        function_builtin:       rgb(122, 162, 247),
        method:                 rgb(122, 162, 247),
        type_name:              rgb(224, 175, 104),
        type_builtin:           rgb(255, 158, 100),
        variable:               rgb(192, 202, 245),
        variable_builtin:       rgb(247, 118, 142),
        constant:               rgb(255, 158, 100),
        constant_builtin:       rgb(255, 158, 100),
        operator:               rgb(137, 221, 255),
        punctuation:            rgb(192, 202, 245),
        punctuation_bracket:    rgb(192, 202, 245),
        punctuation_delimiter:  rgb(192, 202, 245),
        attribute:              rgb(224, 175, 104),
        label:                  rgb(224, 175, 104),
        namespace:              rgb(224, 175, 104),
        property:               rgb(115, 218, 202),
        tag:                    rgb(247, 118, 142),
        tag_attribute:          rgb(224, 175, 104),
        diff_add:               rgb(158, 206, 106),
        diff_delete:            rgb(247, 118, 142),
        diff_change:            rgb(224, 175, 104),
        diff_add_bg:            rgb(32,  50,  20),
        diff_delete_bg:         rgb(60,  20,  30),
        bg:                     rgb(36,  40,  59),
        bg_dark:                rgb(26,  27,  38),
        bg_highlight:           rgb(43,  48,  71),
        fg:                     rgb(192, 202, 245),
        fg_dark:                rgb(138, 150, 200),
        fg_gutter:              rgb(86,  95,  137),
        border:                 rgb(65,  72, 104),
        border_highlight:       rgb(122, 162, 247),
        statusbar_bg:           rgb(26,  27,  38),
        statusbar_fg:           rgb(192, 202, 245),
        statusbar_mode_normal:  rgb(122, 162, 247),
        statusbar_mode_insert:  rgb(158, 206, 106),
        statusbar_mode_visual:  rgb(187, 154, 247),
        statusbar_mode_command: rgb(224, 175, 104),
        cursor:                 rgb(192, 202, 245),
        cursor_line:            rgb(43,  48,  71),
        selection:              rgb(65,  72, 104),
        selection_inactive:     rgb(43,  48,  71),
        error:                  rgb(219,  75,  75),
        warning:                rgb(224, 175, 104),
        info:                   rgb(137, 221, 255),
        hint:                   rgb(115, 218, 202),
    }
}

pub fn gruvbox_dark() -> Theme {
    Theme {
        name: "gruvbox-dark".into(),
        keyword:                rgb(251, 73,  52),  // red
        keyword_control:        rgb(251, 73,  52),
        string:                 rgb(184, 187, 38),  // green
        string_escape:          rgb(254, 128, 25),  // orange
        number:                 rgb(211, 134, 155), // purple
        float:                  rgb(211, 134, 155),
        comment:                rgb(146, 131, 116), // gray
        comment_doc:            rgb(146, 131, 116),
        function:               rgb(250, 189, 47),  // yellow
        function_builtin:       rgb(254, 128, 25),  // orange
        method:                 rgb(250, 189, 47),
        type_name:              rgb(142, 192, 124), // aqua
        type_builtin:           rgb(254, 128, 25),
        variable:               rgb(235, 219, 178), // fg
        variable_builtin:       rgb(254, 128, 25),
        constant:               rgb(211, 134, 155),
        constant_builtin:       rgb(211, 134, 155),
        operator:               rgb(251, 241, 199),
        punctuation:            rgb(235, 219, 178),
        punctuation_bracket:    rgb(235, 219, 178),
        punctuation_delimiter:  rgb(235, 219, 178),
        attribute:              rgb(142, 192, 124),
        label:                  rgb(250, 189, 47),
        namespace:              rgb(250, 189, 47),
        property:               rgb(131, 165, 152), // blue
        tag:                    rgb(251, 73,  52),
        tag_attribute:          rgb(250, 189, 47),
        diff_add:               rgb(184, 187, 38),
        diff_delete:            rgb(251, 73,  52),
        diff_change:            rgb(250, 189, 47),
        diff_add_bg:            rgb(50,  57,  26),
        diff_delete_bg:         rgb(64,  29,  22),
        bg:                     rgb(40,  40,  40),
        bg_dark:                rgb(29,  32,  33),
        bg_highlight:           rgb(60,  56,  54),
        fg:                     rgb(235, 219, 178),
        fg_dark:                rgb(213, 196, 161),
        fg_gutter:              rgb(124, 111, 100),
        border:                 rgb(80,  73,  69),
        border_highlight:       rgb(131, 165, 152),
        statusbar_bg:           rgb(50,  48,  47),
        statusbar_fg:           rgb(235, 219, 178),
        statusbar_mode_normal:  rgb(131, 165, 152),
        statusbar_mode_insert:  rgb(184, 187, 38),
        statusbar_mode_visual:  rgb(211, 134, 155),
        statusbar_mode_command: rgb(250, 189, 47),
        cursor:                 rgb(235, 219, 178),
        cursor_line:            rgb(60,  56,  54),
        selection:              rgb(80,  73,  69),
        selection_inactive:     rgb(60,  56,  54),
        error:                  rgb(251, 73,  52),
        warning:                rgb(250, 189, 47),
        info:                   rgb(131, 165, 152),
        hint:                   rgb(142, 192, 124),
    }
}

pub fn one_dark() -> Theme {
    Theme {
        name: "one-dark".into(),
        keyword:                rgb(198, 120, 221),
        keyword_control:        rgb(198, 120, 221),
        string:                 rgb(152, 195, 121),
        string_escape:          rgb(209, 154, 102),
        number:                 rgb(209, 154, 102),
        float:                  rgb(209, 154, 102),
        comment:                rgb(92,  99,  112),
        comment_doc:            rgb(92,  99,  112),
        function:               rgb(97,  175, 239),
        function_builtin:       rgb(229, 192, 123),
        method:                 rgb(97,  175, 239),
        type_name:              rgb(229, 192, 123),
        type_builtin:           rgb(209, 154, 102),
        variable:               rgb(224, 108, 117),
        variable_builtin:       rgb(224, 108, 117),
        constant:               rgb(209, 154, 102),
        constant_builtin:       rgb(209, 154, 102),
        operator:               rgb(86,  182, 194),
        punctuation:            rgb(171, 178, 191),
        punctuation_bracket:    rgb(171, 178, 191),
        punctuation_delimiter:  rgb(171, 178, 191),
        attribute:              rgb(224, 108, 117),
        label:                  rgb(229, 192, 123),
        namespace:              rgb(229, 192, 123),
        property:               rgb(86,  182, 194),
        tag:                    rgb(224, 108, 117),
        tag_attribute:          rgb(229, 192, 123),
        diff_add:               rgb(152, 195, 121),
        diff_delete:            rgb(224, 108, 117),
        diff_change:            rgb(229, 192, 123),
        diff_add_bg:            rgb(32,  50,  20),
        diff_delete_bg:         rgb(60,  20,  30),
        bg:                     rgb(40,  44,  52),
        bg_dark:                rgb(33,  37,  43),
        bg_highlight:           rgb(49,  53,  65),
        fg:                     rgb(171, 178, 191),
        fg_dark:                rgb(130, 137, 151),
        fg_gutter:              rgb(75,  82,  99),
        border:                 rgb(62,  68,  81),
        border_highlight:       rgb(97,  175, 239),
        statusbar_bg:           rgb(33,  37,  43),
        statusbar_fg:           rgb(171, 178, 191),
        statusbar_mode_normal:  rgb(97,  175, 239),
        statusbar_mode_insert:  rgb(152, 195, 121),
        statusbar_mode_visual:  rgb(198, 120, 221),
        statusbar_mode_command: rgb(229, 192, 123),
        cursor:                 rgb(171, 178, 191),
        cursor_line:            rgb(49,  53,  65),
        selection:              rgb(62,  68,  81),
        selection_inactive:     rgb(49,  53,  65),
        error:                  rgb(224, 108, 117),
        warning:                rgb(229, 192, 123),
        info:                   rgb(86,  182, 194),
        hint:                   rgb(152, 195, 121),
    }
}

/// Registry of built-in themes.
pub fn builtin_themes() -> Vec<Theme> {
    vec![
        catppuccin_mocha(),
        tokyonight_storm(),
        gruvbox_dark(),
        one_dark(),
    ]
}

/// Find a theme by name (case-insensitive).
pub fn find_theme(name: &str) -> Option<Theme> {
    builtin_themes().into_iter().find(|t| t.name.eq_ignore_ascii_case(name))
}

/// Load a theme from a TOML file.
pub fn load_from_file(path: &std::path::Path) -> anyhow::Result<Theme> {
    let text = std::fs::read_to_string(path)?;
    let theme: Theme = toml::from_str(&text)?;
    Ok(theme)
}

/// Load the active theme: check user config dir, fall back to built-in.
pub fn load_theme(name: &str) -> Theme {
    // Check ~/.config/rmtide/themes/<name>.toml
    if let Some(config_dir) = dirs::config_dir() {
        let theme_path = config_dir.join("rmtide").join("themes").join(format!("{name}.toml"));
        if theme_path.exists() {
            if let Ok(t) = load_from_file(&theme_path) {
                return t;
            }
        }
    }
    // Fall back to built-in
    find_theme(name).unwrap_or_else(catppuccin_mocha)
}
