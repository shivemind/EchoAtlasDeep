#![allow(dead_code, unused_imports, unused_variables)]
//! AI commit message composer — Phase 12 Point 42.
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::Widget,
};

pub struct CommitComposerState {
    pub open: bool,
    pub draft: String,
    pub cursor_pos: usize,
    pub diff_preview: String,
    pub generating: bool,
    pub conventional_type: String,
}

impl CommitComposerState {
    pub fn new() -> Self {
        Self {
            open: false,
            draft: String::new(),
            cursor_pos: 0,
            diff_preview: String::new(),
            generating: false,
            conventional_type: "feat".to_string(),
        }
    }

    pub fn set_draft(&mut self, draft: &str) {
        self.draft = draft.to_string();
        self.cursor_pos = self.draft.len();
    }

    pub fn insert_char(&mut self, c: char) {
        if self.cursor_pos <= self.draft.len() {
            self.draft.insert(self.cursor_pos, c);
            self.cursor_pos += 1;
        }
    }

    pub fn delete_char(&mut self) {
        if self.cursor_pos > 0 && !self.draft.is_empty() {
            self.cursor_pos -= 1;
            self.draft.remove(self.cursor_pos);
        }
    }

    pub fn move_cursor_left(&mut self) {
        if self.cursor_pos > 0 {
            self.cursor_pos -= 1;
        }
    }

    pub fn move_cursor_right(&mut self) {
        if self.cursor_pos < self.draft.len() {
            self.cursor_pos += 1;
        }
    }

    pub fn cycle_type_forward(&mut self) {
        let types = ["feat", "fix", "chore", "docs", "style", "refactor", "test", "perf", "ci", "build"];
        let current = types.iter().position(|&t| t == self.conventional_type.as_str()).unwrap_or(0);
        self.conventional_type = types[(current + 1) % types.len()].to_string();
    }

    pub fn cycle_type_backward(&mut self) {
        let types = ["feat", "fix", "chore", "docs", "style", "refactor", "test", "perf", "ci", "build"];
        let current = types.iter().position(|&t| t == self.conventional_type.as_str()).unwrap_or(0);
        self.conventional_type = types[(current + types.len() - 1) % types.len()].to_string();
    }
}

impl Default for CommitComposerState {
    fn default() -> Self {
        Self::new()
    }
}

pub struct CommitComposerWidget<'a> {
    pub state: &'a CommitComposerState,
}

impl Widget for CommitComposerWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 10 || area.height < 8 {
            return;
        }

        let w = (area.width * 3 / 4).max(60).min(area.width);
        let h = (area.height * 2 / 3).max(18).min(area.height);
        let x = area.x + (area.width.saturating_sub(w)) / 2;
        let y = area.y + (area.height.saturating_sub(h)) / 2;

        let bg = Style::default().bg(Color::Rgb(14, 18, 24));
        let border_style = Style::default().fg(Color::Green);
        let title_style = Style::default().fg(Color::Green).add_modifier(Modifier::BOLD);
        let label_style = Style::default().fg(Color::DarkGray);
        let value_style = Style::default().fg(Color::White);
        let selected_type_style = Style::default().fg(Color::Black).bg(Color::Green);
        let unselected_type_style = Style::default().fg(Color::DarkGray);
        let input_bg = Style::default().bg(Color::Rgb(22, 26, 34));
        let diff_style = Style::default().fg(Color::DarkGray).bg(Color::Rgb(14, 18, 24));

        // Fill background
        for row in y..y + h {
            for col in x..x + w {
                buf.get_mut(col, row).set_char(' ').set_style(bg);
            }
        }

        // Border
        buf.get_mut(x, y).set_char('╔').set_style(border_style);
        buf.get_mut(x + w - 1, y).set_char('╗').set_style(border_style);
        for col in x + 1..x + w - 1 {
            buf.get_mut(col, y).set_char('═').set_style(border_style);
        }
        buf.get_mut(x, y + h - 1).set_char('╚').set_style(border_style);
        buf.get_mut(x + w - 1, y + h - 1).set_char('╝').set_style(border_style);
        for col in x + 1..x + w - 1 {
            buf.get_mut(col, y + h - 1).set_char('═').set_style(border_style);
        }
        for row in y + 1..y + h - 1 {
            buf.get_mut(x, row).set_char('║').set_style(border_style);
            buf.get_mut(x + w - 1, row).set_char('║').set_style(border_style);
        }

        // Title
        let title = " AI Commit Composer ";
        let title_x = x + (w.saturating_sub(title.len() as u16)) / 2;
        for (i, ch) in title.chars().enumerate() {
            if title_x + i as u16 >= x + w - 1 { break; }
            buf.get_mut(title_x + i as u16, y).set_char(ch).set_style(title_style);
        }

        // Type selector row (y+1)
        let type_label = " Type: ";
        for (i, ch) in type_label.chars().enumerate() {
            if x + 1 + i as u16 >= x + w - 1 { break; }
            buf.get_mut(x + 1 + i as u16, y + 1).set_char(ch).set_style(label_style);
        }

        let types = ["feat", "fix", "chore", "docs", "style", "refactor", "test", "perf", "ci", "build"];
        let mut tx = x + 1 + type_label.len() as u16;
        for t in &types {
            if tx >= x + w - 1 { break; }
            let is_active = *t == self.state.conventional_type.as_str();
            let ts = if is_active { selected_type_style } else { unselected_type_style };
            let label = format!(" {} ", t);
            for ch in label.chars() {
                if tx >= x + w - 1 { break; }
                buf.get_mut(tx, y + 1).set_char(ch).set_style(ts);
                tx += 1;
            }
            tx += 1; // gap
        }

        // Separator
        for col in x + 1..x + w - 1 {
            buf.get_mut(col, y + 2).set_char('─').set_style(border_style);
        }
        buf.get_mut(x, y + 2).set_char('╟').set_style(border_style);
        buf.get_mut(x + w - 1, y + 2).set_char('╢').set_style(border_style);

        // Message label
        let msg_label = " Commit Message:";
        for (i, ch) in msg_label.chars().enumerate() {
            if x + 1 + i as u16 >= x + w - 1 { break; }
            buf.get_mut(x + 1 + i as u16, y + 3).set_char(ch).set_style(label_style);
        }

        // Generating indicator
        if self.state.generating {
            let gen = " ⠙ Generating with AI...";
            for (i, ch) in gen.chars().enumerate() {
                let col = x + 1 + i as u16;
                if col >= x + w - 1 { break; }
                buf.get_mut(col, y + 4)
                    .set_char(ch)
                    .set_style(Style::default().fg(Color::Yellow));
            }
        } else {
            // Input box (rows y+4 to y+6, multi-line message)
            let input_height = 3u16;
            for row in y + 4..y + 4 + input_height {
                for col in x + 1..x + w - 1 {
                    buf.get_mut(col, row).set_char(' ').set_style(input_bg);
                }
            }
            // Show full formatted draft: "type: message"
            let full_draft = format!("{}: {}", self.state.conventional_type, self.state.draft);
            let chars: Vec<char> = full_draft.chars().collect();
            let cols_avail = (w as usize).saturating_sub(3);
            let wrap_lines: Vec<String> = chars
                .chunks(cols_avail)
                .map(|chunk| chunk.iter().collect::<String>())
                .collect();
            for (li, line) in wrap_lines.iter().enumerate().take(input_height as usize) {
                let ry = y + 4 + li as u16;
                for (ci, ch) in line.chars().enumerate() {
                    let col = x + 2 + ci as u16;
                    if col >= x + w - 1 { break; }
                    buf.get_mut(col, ry).set_char(ch).set_style(
                        Style::default().fg(Color::White).bg(Color::Rgb(22, 26, 34))
                    );
                }
            }
            // Draw cursor
            let draft_type_prefix = format!("{}: ", self.state.conventional_type);
            let cursor_abs = draft_type_prefix.len() + self.state.cursor_pos;
            let cursor_row_offset = cursor_abs / cols_avail;
            let cursor_col_offset = cursor_abs % cols_avail;
            if cursor_row_offset < input_height as usize {
                let cy = y + 4 + cursor_row_offset as u16;
                let cx = x + 2 + cursor_col_offset as u16;
                if cx < x + w - 1 {
                    let current_ch = buf.get_mut(cx, cy).symbol().chars().next().unwrap_or(' ');
                    buf.get_mut(cx, cy)
                        .set_char(current_ch)
                        .set_style(Style::default().bg(Color::White).fg(Color::Black));
                }
            }
        }

        // Separator before diff
        let sep2_y = y + 7;
        if sep2_y < y + h - 1 {
            for col in x + 1..x + w - 1 {
                buf.get_mut(col, sep2_y).set_char('─').set_style(border_style);
            }
            buf.get_mut(x, sep2_y).set_char('╟').set_style(border_style);
            buf.get_mut(x + w - 1, sep2_y).set_char('╢').set_style(border_style);

            // Diff preview
            let diff_label = " Staged Diff Preview:";
            for (i, ch) in diff_label.chars().enumerate() {
                let col = x + 1 + i as u16;
                if col >= x + w - 1 { break; }
                buf.get_mut(col, sep2_y + 1).set_char(ch).set_style(label_style);
            }

            let diff_area_start = sep2_y + 2;
            let diff_area_height = (y + h - 1).saturating_sub(diff_area_start + 1);
            for (li, line) in self.state.diff_preview.lines().enumerate().take(diff_area_height as usize) {
                let ry = diff_area_start + li as u16;
                if ry >= y + h - 1 { break; }
                let line_style = if line.starts_with('+') {
                    Style::default().fg(Color::Green)
                } else if line.starts_with('-') {
                    Style::default().fg(Color::Red)
                } else if line.starts_with('@') {
                    Style::default().fg(Color::Cyan)
                } else {
                    diff_style
                };
                let display: String = line.chars().take((w as usize).saturating_sub(4)).collect();
                for (ci, ch) in display.chars().enumerate() {
                    let col = x + 2 + ci as u16;
                    if col >= x + w - 1 { break; }
                    buf.get_mut(col, ry).set_char(ch).set_style(line_style);
                }
            }
        }

        // Hint bar
        let hint_y = y + h - 2;
        if hint_y > y + 7 {
            let hint = " [←/→] move cursor  [Tab] cycle type  [Ctrl+G] generate  [Enter] commit  [Esc] cancel";
            for (i, ch) in hint.chars().enumerate() {
                let col = x + 1 + i as u16;
                if col >= x + w - 1 { break; }
                buf.get_mut(col, hint_y)
                    .set_char(ch)
                    .set_style(Style::default().fg(Color::DarkGray));
            }
        }
    }
}
