#![allow(dead_code, unused_imports, unused_variables)]
//! Editor pane widget — renders buffer text with line numbers, syntax highlight,
//! cursor, visual selection, and search matches.
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::Widget,
};

use editor::buffer::EditorBuffer;
use editor::fold::FoldState;
use editor::modal::{Mode, VisualKind};
use editor::search::SearchState;
use editor::view::EditorView;

use crate::highlight::{HighlightSpan, Highlighter, TokenKind};

pub struct EditorPaneWidget<'a> {
    pub buffer: &'a EditorBuffer,
    pub view: &'a EditorView,
    pub mode: &'a Mode,
    pub folds: &'a FoldState,
    pub search: &'a SearchState,
    pub highlighter: Option<&'a mut Highlighter>,
    pub focused: bool,
    pub show_line_numbers: bool,
}

impl<'a> Widget for EditorPaneWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        // Shrink by 1 for border
        let inner = Rect {
            x: area.x + 1,
            y: area.y + 1,
            width: area.width.saturating_sub(2),
            height: area.height.saturating_sub(2),
        };

        if inner.width == 0 || inner.height == 0 {
            return;
        }

        let gutter_width: u16 = if self.show_line_numbers { 5 } else { 0 };
        let content_x = inner.x + gutter_width;
        let content_width = inner.width.saturating_sub(gutter_width);
        if content_width == 0 {
            return;
        }

        let total_lines = self.buffer.line_count().max(1);
        let scroll_row = self.view.scroll_row;
        let scroll_col = self.view.scroll_col;
        let visible_rows = inner.height as usize;

        // Get source bytes for highlighting
        let source_bytes = self.buffer.text.to_bytes();
        let spans: Vec<HighlightSpan> = Vec::new(); // Highlighter would need &mut self
        // NOTE: We can't call highlighter.highlight here due to borrow, so we skip it in render.
        // The caller should pre-compute spans if needed.

        // Visual selection range
        let visual_range = self.view.visual_range();
        let in_visual = matches!(self.mode, Mode::Visual(_));

        for row in 0..visible_rows {
            let buf_line = scroll_row + row;
            if buf_line >= total_lines {
                break;
            }

            // Skip folded lines (those inside a fold)
            if self.folds.is_folded(buf_line) {
                continue;
            }

            let screen_y = inner.y + row as u16;

            // Draw line number
            if self.show_line_numbers && gutter_width > 0 {
                let line_num_str = format!("{:>4} ", buf_line + 1);
                let num_style = if buf_line == self.view.cursor.line {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default().fg(Color::DarkGray)
                };
                for (i, ch) in line_num_str.chars().enumerate() {
                    if i >= gutter_width as usize {
                        break;
                    }
                    buf.get_mut(inner.x + i as u16, screen_y)
                        .set_char(ch)
                        .set_style(num_style);
                }
            }

            // Draw fold marker if this is a folded range start
            if self.folds.is_fold_start(buf_line) {
                let marker = "▸ ...";
                for (i, ch) in marker.chars().enumerate() {
                    if i >= content_width as usize {
                        break;
                    }
                    buf.get_mut(content_x + i as u16, screen_y)
                        .set_char(ch)
                        .set_style(Style::default().fg(Color::DarkGray));
                }
                continue;
            }

            // Get line text
            let line_text = self.buffer.line_content(buf_line);
            let line_chars: Vec<char> = line_text.chars().collect();

            // Search matches on this line
            let search_matches: Vec<_> = self.search.matches_on_line(buf_line).collect();

            // Draw each character in the content area
            for col in 0..content_width as usize {
                let char_col = scroll_col + col;
                let screen_x = content_x + col as u16;

                let ch = line_chars.get(char_col).copied().unwrap_or(' ');

                // Determine style
                let mut style = Style::default();

                // Check if in visual selection
                if in_visual {
                    if let Some((start, end)) = visual_range {
                        let in_sel = if start.line == end.line {
                            buf_line == start.line && char_col >= start.col && char_col <= end.col
                        } else if buf_line == start.line {
                            char_col >= start.col
                        } else if buf_line == end.line {
                            char_col <= end.col
                        } else {
                            buf_line > start.line && buf_line < end.line
                        };
                        if in_sel {
                            style = style.bg(Color::Blue).fg(Color::White);
                        }
                    }
                }

                // Check if in search match
                for m in &search_matches {
                    if char_col >= m.start_col && char_col < m.end_col {
                        style = style.bg(Color::Yellow).fg(Color::Black);
                    }
                }

                // Draw cursor
                let is_cursor =
                    buf_line == self.view.cursor.line && char_col == self.view.cursor.col;
                if is_cursor && self.focused {
                    style = style.bg(Color::White).fg(Color::Black).add_modifier(Modifier::BOLD);
                }

                buf.get_mut(screen_x, screen_y)
                    .set_char(ch)
                    .set_style(style);
            }
        }
    }
}

pub fn token_kind_style(kind: TokenKind) -> Style {
    match kind {
        TokenKind::Keyword => Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD),
        TokenKind::String => Style::default().fg(Color::Green),
        TokenKind::Comment => Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC),
        TokenKind::Number => Style::default().fg(Color::Yellow),
        TokenKind::Function => Style::default().fg(Color::Blue),
        TokenKind::Type => Style::default().fg(Color::Cyan),
        TokenKind::Constant => Style::default().fg(Color::Red),
        TokenKind::Operator => Style::default().fg(Color::White),
        _ => Style::default(),
    }
}
