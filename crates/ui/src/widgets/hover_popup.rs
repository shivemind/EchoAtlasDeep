#![allow(dead_code)]
//! Floating hover documentation popup.
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::Widget,
};

/// Floating hover documentation popup with a rounded border.
pub struct HoverPopup<'a> {
    pub content: &'a str,
    pub max_width: u16,
    pub max_height: u16,
}

impl<'a> Widget for HoverPopup<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 6 || area.height < 3 {
            return;
        }

        let bg = Color::DarkGray;
        let fg = Color::White;
        let border_style = Style::default().fg(Color::Gray).bg(bg);
        let text_style = Style::default().fg(fg).bg(bg);

        // Fill background
        for row in area.y..area.y + area.height {
            for col in area.x..area.x + area.width {
                buf[(col, row)].set_bg(bg).set_char(' ');
            }
        }

        // Top border
        if area.width >= 2 {
            buf[(area.x, area.y)]
                .set_style(border_style)
                .set_char('╭');
            for col in area.x + 1..area.x + area.width - 1 {
                buf[(col, area.y)].set_style(border_style).set_char('─');
            }
            buf[(area.x + area.width - 1, area.y)]
                .set_style(border_style)
                .set_char('╮');
        }

        // Content rows
        let lines: Vec<&str> = self.content.lines().collect();
        let content_rows = (area.height as usize).saturating_sub(2);
        for (i, line) in lines.iter().take(content_rows).enumerate() {
            let row = area.y + 1 + i as u16;
            if row >= area.y + area.height.saturating_sub(1) {
                break;
            }
            // Side borders
            buf[(area.x, row)].set_style(border_style).set_char('│');
            if area.width >= 2 {
                buf[(area.x + area.width - 1, row)]
                    .set_style(border_style)
                    .set_char('│');
            }
            // Text content (with 1-char padding)
            let mut col = area.x + 2;
            for c in line.chars() {
                if col >= area.x + area.width.saturating_sub(2) {
                    break;
                }
                buf[(col, row)].set_style(text_style).set_char(c);
                col += 1;
            }
        }

        // Bottom border
        let bottom = area.y + area.height - 1;
        if area.width >= 2 {
            buf[(area.x, bottom)]
                .set_style(border_style)
                .set_char('╰');
            for col in area.x + 1..area.x + area.width - 1 {
                buf[(col, bottom)]
                    .set_style(border_style)
                    .set_char('─');
            }
            buf[(area.x + area.width - 1, bottom)]
                .set_style(border_style)
                .set_char('╯');
        }
    }
}
