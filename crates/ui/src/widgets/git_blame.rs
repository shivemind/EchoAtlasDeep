#![allow(dead_code, unused_imports, unused_variables)]
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Widget,
};
use git::BlameLine;

pub struct GitBlameWidget<'a> {
    pub blame_lines: &'a [BlameLine],
    pub scroll_row: usize,
    pub cursor_line: usize,
}

impl<'a> Widget for GitBlameWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        for screen_row in 0..area.height {
            let buf_line = self.scroll_row + screen_row as usize;
            let y = area.y + screen_row;

            if let Some(blame) = self.blame_lines.get(buf_line) {
                let is_cursor = buf_line == self.cursor_line;

                // Format: "a1b2c3d shive     2 days ago"
                let text = format!("{:<7} {:<12} {:<12}",
                    &blame.sha,
                    truncate(&blame.author, 12),
                    truncate(&blame.time_ago, 12),
                );

                let style = if is_cursor {
                    Style::default().fg(Color::White).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::DarkGray)
                };

                let line = Line::from(Span::styled(text, style));

                // Render into the buffer row
                let x = area.x;
                let width = area.width as usize;
                let content = line.spans.iter().map(|s| s.content.as_ref()).collect::<String>();
                let truncated: String = content.chars().take(width).collect();
                for (ci, ch) in truncated.chars().enumerate() {
                    let cx = x + ci as u16;
                    if cx < area.right() {
                        buf.get_mut(cx, y).set_char(ch).set_style(style);
                    }
                }
            }
        }
    }
}

fn truncate(s: &str, max: usize) -> &str {
    let end = s.char_indices().nth(max).map(|(i, _)| i).unwrap_or(s.len());
    &s[..end]
}
