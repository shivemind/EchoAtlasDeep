#![allow(dead_code, unused_imports, unused_variables)]
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    text::Span,
    widgets::Widget,
};
use git::{GutterLine, GutterMark};

pub struct DiffGutterWidget<'a> {
    pub marks: &'a [GutterLine],
    pub scroll_row: usize,
    pub height: u16,
}

impl<'a> Widget for DiffGutterWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        for screen_row in 0..area.height {
            let buf_line = self.scroll_row + screen_row as usize;
            let y = area.y + screen_row;
            let x = area.x;

            // Find mark for this line
            let mark = self.marks.iter().find(|m| m.line == buf_line);
            let (ch, color) = match mark.map(|m| &m.mark) {
                Some(GutterMark::Added)        => ('+', Color::Green),
                Some(GutterMark::Modified)     => ('~', Color::Yellow),
                Some(GutterMark::DeletedBelow) => ('_', Color::Red),
                None                           => (' ', Color::Reset),
            };

            if x < buf.area.x + buf.area.width && y < buf.area.y + buf.area.height {
                buf.get_mut(x, y)
                    .set_char(ch)
                    .set_style(Style::default().fg(color));
            }
        }
    }
}
