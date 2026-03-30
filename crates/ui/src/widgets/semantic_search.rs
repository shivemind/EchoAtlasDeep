#![allow(dead_code, unused_imports, unused_variables)]
//! Semantic search overlay — Phase 12 Point 41.
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::Widget,
};

#[derive(Clone, Debug)]
pub struct SemanticResult {
    pub file: String,
    pub line: usize,
    pub snippet: String,
    pub relevance: f32,
    pub match_kind: String,
}

pub struct SemanticSearchState {
    pub open: bool,
    pub query: String,
    pub results: Vec<SemanticResult>,
    pub selected: usize,
    pub searching: bool,
}

impl SemanticSearchState {
    pub fn new() -> Self {
        Self {
            open: false,
            query: String::new(),
            results: Vec::new(),
            selected: 0,
            searching: false,
        }
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    pub fn move_down(&mut self) {
        if self.selected + 1 < self.results.len() {
            self.selected += 1;
        }
    }

    pub fn selected_result(&self) -> Option<&SemanticResult> {
        self.results.get(self.selected)
    }
}

impl Default for SemanticSearchState {
    fn default() -> Self {
        Self::new()
    }
}

pub struct SemanticSearchWidget<'a> {
    pub state: &'a SemanticSearchState,
}

impl Widget for SemanticSearchWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 10 || area.height < 6 {
            return;
        }
        // Centered overlay: 70% wide, 60% tall
        let w = (area.width * 7 / 10).max(50).min(area.width);
        let h = (area.height * 3 / 5).max(10).min(area.height);
        let x = area.x + (area.width.saturating_sub(w)) / 2;
        let y = area.y + (area.height.saturating_sub(h)) / 2;

        let bg = Style::default().bg(Color::Rgb(18, 18, 28));
        let border_style = Style::default().fg(Color::Cyan);
        let title_style = Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD);
        let query_style = Style::default().fg(Color::White).bg(Color::Rgb(30, 30, 45));
        let selected_style = Style::default().bg(Color::Rgb(40, 40, 60)).fg(Color::White);
        let normal_style = Style::default().fg(Color::Gray);
        let kind_style = Style::default().fg(Color::DarkGray);
        let file_style = Style::default().fg(Color::Green);
        let relevance_style = Style::default().fg(Color::Yellow);

        // Fill background
        for row in y..y + h {
            for col in x..x + w {
                buf.get_mut(col, row).set_char(' ').set_style(bg);
            }
        }

        // Border top
        buf.get_mut(x, y).set_char('╔').set_style(border_style);
        buf.get_mut(x + w - 1, y).set_char('╗').set_style(border_style);
        for col in x + 1..x + w - 1 {
            buf.get_mut(col, y).set_char('═').set_style(border_style);
        }
        // Border bottom
        buf.get_mut(x, y + h - 1).set_char('╚').set_style(border_style);
        buf.get_mut(x + w - 1, y + h - 1).set_char('╝').set_style(border_style);
        for col in x + 1..x + w - 1 {
            buf.get_mut(col, y + h - 1).set_char('═').set_style(border_style);
        }
        // Border sides
        for row in y + 1..y + h - 1 {
            buf.get_mut(x, row).set_char('║').set_style(border_style);
            buf.get_mut(x + w - 1, row).set_char('║').set_style(border_style);
        }

        // Title
        let title = " Semantic Search ";
        let title_x = x + (w.saturating_sub(title.len() as u16)) / 2;
        for (i, ch) in title.chars().enumerate() {
            if title_x + i as u16 >= x + w - 1 { break; }
            buf.get_mut(title_x + i as u16, y).set_char(ch).set_style(title_style);
        }

        // Query bar (row y+1)
        let prompt = "› ";
        let query_y = y + 1;
        for col in x + 1..x + w - 1 {
            buf.get_mut(col, query_y).set_char(' ').set_style(query_style);
        }
        for (i, ch) in prompt.chars().enumerate() {
            if x + 1 + i as u16 >= x + w - 1 { break; }
            buf.get_mut(x + 1 + i as u16, query_y)
                .set_char(ch)
                .set_style(Style::default().fg(Color::Cyan).bg(Color::Rgb(30, 30, 45)));
        }
        let query_display: String = self.state.query.chars().take((w as usize).saturating_sub(6)).collect();
        for (i, ch) in query_display.chars().enumerate() {
            let col = x + 3 + i as u16;
            if col >= x + w - 1 { break; }
            buf.get_mut(col, query_y).set_char(ch).set_style(query_style);
        }

        // Separator
        let sep_y = y + 2;
        for col in x + 1..x + w - 1 {
            buf.get_mut(col, sep_y).set_char('─').set_style(border_style);
        }
        buf.get_mut(x, sep_y).set_char('╟').set_style(border_style);
        buf.get_mut(x + w - 1, sep_y).set_char('╢').set_style(border_style);

        // Searching spinner
        if self.state.searching {
            let spinner = " ⠋ Searching...";
            for (i, ch) in spinner.chars().enumerate() {
                let col = x + 1 + i as u16;
                if col >= x + w - 1 { break; }
                buf.get_mut(col, y + 3)
                    .set_char(ch)
                    .set_style(Style::default().fg(Color::Yellow));
            }
            return;
        }

        if self.state.results.is_empty() {
            let empty = if self.state.query.is_empty() {
                " Type to search across files, symbols, and patterns..."
            } else {
                " No results found."
            };
            for (i, ch) in empty.chars().enumerate() {
                let col = x + 1 + i as u16;
                if col >= x + w - 1 { break; }
                buf.get_mut(col, y + 3).set_char(ch).set_style(kind_style);
            }
            return;
        }

        // Results list
        let list_start_y = sep_y + 1;
        let list_height = (h as usize).saturating_sub(4);
        let scroll_offset = if self.state.selected >= list_height {
            self.state.selected - list_height + 1
        } else {
            0
        };

        for (idx, result) in self.state.results.iter().enumerate().skip(scroll_offset).take(list_height) {
            let row_y = list_start_y + (idx - scroll_offset) as u16;
            if row_y >= y + h - 1 { break; }

            let is_selected = idx == self.state.selected;
            let row_style = if is_selected { selected_style } else { normal_style };

            // Fill row background
            for col in x + 1..x + w - 1 {
                buf.get_mut(col, row_y).set_char(' ').set_style(row_style);
            }

            // Relevance bar (10 chars)
            let bar_x = x + 1;
            let filled = (self.state.results[idx].relevance * 10.0) as usize;
            for i in 0..10usize {
                if bar_x + i as u16 >= x + w - 1 { break; }
                let bar_ch = if i < filled { '█' } else { '░' };
                let bar_color = if self.state.results[idx].relevance > 0.7 {
                    Color::Green
                } else if self.state.results[idx].relevance > 0.4 {
                    Color::Yellow
                } else {
                    Color::DarkGray
                };
                buf.get_mut(bar_x + i as u16, row_y)
                    .set_char(bar_ch)
                    .set_style(Style::default().fg(bar_color).bg(if is_selected { Color::Rgb(40,40,60) } else { Color::Reset }));
            }

            // File:line
            let file_col = bar_x + 11;
            let file_text: String = format!("{}:{}", result.file, result.line);
            let file_display: String = file_text.chars().take(20).collect();
            for (i, ch) in file_display.chars().enumerate() {
                let col = file_col + i as u16;
                if col >= x + w - 1 { break; }
                buf.get_mut(col, row_y)
                    .set_char(ch)
                    .set_style(if is_selected { Style::default().fg(Color::Cyan).bg(Color::Rgb(40,40,60)) } else { file_style });
            }

            // Kind badge
            let kind_col = file_col + 22;
            let kind_text: String = format!("[{}]", result.match_kind);
            for (i, ch) in kind_text.chars().enumerate() {
                let col = kind_col + i as u16;
                if col >= x + w - 1 { break; }
                buf.get_mut(col, row_y)
                    .set_char(ch)
                    .set_style(if is_selected { Style::default().fg(Color::Magenta).bg(Color::Rgb(40,40,60)) } else { kind_style });
            }

            // Snippet preview on next line if selected
            if is_selected && row_y + 1 < y + h - 1 {
                let snip_row = row_y + 1;
                for col in x + 1..x + w - 1 {
                    buf.get_mut(col, snip_row).set_char(' ').set_style(selected_style);
                }
                let snip: String = format!("  {}", result.snippet);
                let snip_display: String = snip.chars().take((w as usize).saturating_sub(4)).collect();
                for (i, ch) in snip_display.chars().enumerate() {
                    let col = x + 1 + i as u16;
                    if col >= x + w - 1 { break; }
                    buf.get_mut(col, snip_row)
                        .set_char(ch)
                        .set_style(Style::default().fg(Color::White).bg(Color::Rgb(40,40,60)));
                }
            }
        }

        // Status line at bottom
        let status_y = y + h - 2;
        let status = format!(
            " {} results  [↑↓] navigate  [Enter] jump  [Esc] close",
            self.state.results.len()
        );
        for (i, ch) in status.chars().enumerate() {
            let col = x + 1 + i as u16;
            if col >= x + w - 1 { break; }
            buf.get_mut(col, status_y)
                .set_char(ch)
                .set_style(Style::default().fg(Color::DarkGray));
        }
    }
}
