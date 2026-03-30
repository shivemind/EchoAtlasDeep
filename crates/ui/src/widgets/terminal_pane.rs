/// Ratatui widget that renders a ScreenBuffer into a Rect.
use ratatui::{
    buffer::Buffer as TuiBuffer,
    layout::Rect,
    style::{Color as TuiColor, Modifier, Style},
    text::{Span, Line},
    widgets::Widget,
};

use terminal::screen::{Cell, ScreenBuffer};
use terminal::vt::color::Color as VtColor;
use terminal::vt::attrs::Attrs;

pub struct TerminalPaneWidget<'a> {
    pub screen: &'a ScreenBuffer,
    pub focused: bool,
}

impl<'a> Widget for TerminalPaneWidget<'a> {
    fn render(self, area: Rect, buf: &mut TuiBuffer) {
        let rows = (area.height as usize).min(self.screen.rows());
        let cols = (area.width  as usize).min(self.screen.cols());

        for row in 0..rows {
            for col in 0..cols {
                let cell = self.screen.get(row, col);
                let x = area.x + col as u16;
                let y = area.y + row as u16;

                if x >= buf.area.x + buf.area.width
                || y >= buf.area.y + buf.area.height {
                    continue;
                }

                let style = cell_style(cell);
                let ch = if cell.ch == '\0' { ' ' } else { cell.ch };
                buf[(x, y)].set_char(ch).set_style(style);
            }
        }
    }
}

fn cell_style(cell: &Cell) -> Style {
    let mut style = Style::default()
        .fg(vt_color_to_tui(cell.fg))
        .bg(vt_color_to_tui(cell.bg));

    if cell.attrs.contains(Attrs::BOLD)          { style = style.add_modifier(Modifier::BOLD); }
    if cell.attrs.contains(Attrs::DIM)           { style = style.add_modifier(Modifier::DIM); }
    if cell.attrs.contains(Attrs::ITALIC)        { style = style.add_modifier(Modifier::ITALIC); }
    if cell.attrs.contains(Attrs::UNDERLINE)     { style = style.add_modifier(Modifier::UNDERLINED); }
    if cell.attrs.contains(Attrs::BLINK)         { style = style.add_modifier(Modifier::SLOW_BLINK); }
    if cell.attrs.contains(Attrs::BLINK_RAPID)   { style = style.add_modifier(Modifier::RAPID_BLINK); }
    if cell.attrs.contains(Attrs::REVERSE)       { style = style.add_modifier(Modifier::REVERSED); }
    if cell.attrs.contains(Attrs::HIDDEN)        { style = style.add_modifier(Modifier::HIDDEN); }
    if cell.attrs.contains(Attrs::STRIKETHROUGH) { style = style.add_modifier(Modifier::CROSSED_OUT); }

    style
}

fn vt_color_to_tui(color: terminal::vt::color::Color) -> TuiColor {
    match color {
        VtColor::Default       => TuiColor::Reset,
        VtColor::Indexed(idx)  => TuiColor::Indexed(idx),
        VtColor::Rgb(r, g, b)  => TuiColor::Rgb(r, g, b),
    }
}
