#![allow(dead_code)]
//! Quickfix list widget.
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::Widget,
};

#[derive(Debug, Clone)]
pub struct QuickfixEntry {
    pub file: String,
    pub line: usize,
    pub col: usize,
    pub message: String,
}

pub struct QuickfixWidget<'a> {
    pub entries: &'a [QuickfixEntry],
    pub selected: usize,
    pub focused: bool,
}

impl<'a> Widget for QuickfixWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        let title = " Quickfix ";
        let border_style = if self.focused {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        // Draw border
        for x in area.x..area.x + area.width {
            buf.get_mut(x, area.y)
                .set_char('─')
                .set_style(border_style);
            buf.get_mut(x, area.y + area.height - 1)
                .set_char('─')
                .set_style(border_style);
        }
        for y in area.y..area.y + area.height {
            buf.get_mut(area.x, y)
                .set_char('│')
                .set_style(border_style);
            buf.get_mut(area.x + area.width - 1, y)
                .set_char('│')
                .set_style(border_style);
        }
        buf.get_mut(area.x, area.y)
            .set_char('┌')
            .set_style(border_style);
        buf.get_mut(area.x + area.width - 1, area.y)
            .set_char('┐')
            .set_style(border_style);
        buf.get_mut(area.x, area.y + area.height - 1)
            .set_char('└')
            .set_style(border_style);
        buf.get_mut(area.x + area.width - 1, area.y + area.height - 1)
            .set_char('┘')
            .set_style(border_style);

        // Title
        for (i, ch) in title.chars().enumerate() {
            let x = area.x + 1 + i as u16;
            if x < area.x + area.width - 1 {
                buf.get_mut(x, area.y).set_char(ch).set_style(border_style);
            }
        }

        let inner_x = area.x + 1;
        let inner_y = area.y + 1;
        let inner_w = area.width.saturating_sub(2) as usize;
        let inner_h = area.height.saturating_sub(2) as usize;

        for (row, entry) in self.entries.iter().enumerate().take(inner_h) {
            let screen_y = inner_y + row as u16;
            let is_selected = row == self.selected;

            let style = if is_selected && self.focused {
                Style::default()
                    .bg(Color::Blue)
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else if is_selected {
                Style::default().bg(Color::DarkGray).fg(Color::White)
            } else {
                Style::default().fg(Color::Gray)
            };

            // Format: "file.rs:10:5 message"
            let line = format!(
                "{}:{}:{} {}",
                entry.file, entry.line, entry.col, entry.message
            );

            // Clear row
            for x in inner_x..inner_x + inner_w as u16 {
                buf.get_mut(x, screen_y).set_char(' ').set_style(style);
            }

            for (i, ch) in line.chars().enumerate() {
                if i >= inner_w {
                    break;
                }
                buf.get_mut(inner_x + i as u16, screen_y)
                    .set_char(ch)
                    .set_style(style);
            }
        }
    }
}
