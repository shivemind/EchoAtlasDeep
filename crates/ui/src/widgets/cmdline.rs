#![allow(dead_code)]
//! Command-line / search input widget.
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::Widget,
};

pub struct CmdLineState {
    pub input: String,
    pub cursor: usize,
    pub history: Vec<String>,
    pub history_idx: Option<usize>,
}

impl CmdLineState {
    pub fn new() -> Self {
        Self {
            input: String::new(),
            cursor: 0,
            history: Vec::new(),
            history_idx: None,
        }
    }

    pub fn push(&mut self, c: char) {
        self.input.insert(self.cursor, c);
        self.cursor += c.len_utf8();
    }

    pub fn backspace(&mut self) {
        if self.cursor > 0 {
            // Find the char boundary before cursor
            let mut pos = self.cursor;
            while pos > 0 && !self.input.is_char_boundary(pos - 1) {
                pos -= 1;
            }
            if pos > 0 {
                let removed_start = pos - 1;
                // Actually remove the char properly
                let ch = self.input[removed_start..self.cursor].chars().next_back();
                if let Some(ch) = ch {
                    let char_len = ch.len_utf8();
                    let remove_start = self.cursor - char_len;
                    self.input.drain(remove_start..self.cursor);
                    self.cursor = remove_start;
                }
            }
        }
    }

    pub fn history_up(&mut self) {
        if self.history.is_empty() {
            return;
        }
        let new_idx = match self.history_idx {
            None => self.history.len() - 1,
            Some(0) => 0,
            Some(i) => i - 1,
        };
        self.history_idx = Some(new_idx);
        self.input = self.history[new_idx].clone();
        self.cursor = self.input.len();
    }

    pub fn history_down(&mut self) {
        if self.history.is_empty() {
            return;
        }
        match self.history_idx {
            None => {}
            Some(i) if i + 1 >= self.history.len() => {
                self.history_idx = None;
                self.input.clear();
                self.cursor = 0;
            }
            Some(i) => {
                self.history_idx = Some(i + 1);
                self.input = self.history[i + 1].clone();
                self.cursor = self.input.len();
            }
        }
    }

    /// Confirm the command; saves to history and returns the input string.
    pub fn confirm(&mut self) -> String {
        let cmd = self.input.clone();
        if !cmd.is_empty() {
            self.history.push(cmd.clone());
        }
        self.input.clear();
        self.cursor = 0;
        self.history_idx = None;
        cmd
    }

    pub fn cancel(&mut self) {
        self.input.clear();
        self.cursor = 0;
        self.history_idx = None;
    }
}

impl Default for CmdLineState {
    fn default() -> Self {
        Self::new()
    }
}

/// Renders the command line at the bottom of its area.
pub struct CmdLineWidget<'a> {
    pub state: &'a CmdLineState,
    pub search_mode: bool,
    pub search_prefix: char,
}

impl<'a> Widget for CmdLineWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.height == 0 || area.width == 0 {
            return;
        }

        let prefix = if self.search_mode {
            format!("{}", self.search_prefix)
        } else {
            ":".to_string()
        };

        let display = format!("{}{}", prefix, self.state.input);
        let cursor_screen_col = prefix.len() + self.state.cursor;

        let style = Style::default().fg(Color::White).bg(Color::DarkGray);

        // Clear area
        for x in area.x..area.x + area.width {
            buf.get_mut(x, area.y).set_char(' ').set_style(style);
        }

        // Draw text
        for (i, ch) in display.chars().enumerate() {
            let x = area.x + i as u16;
            if x >= area.x + area.width {
                break;
            }
            let cell_style = if i == cursor_screen_col {
                style.add_modifier(Modifier::REVERSED)
            } else {
                style
            };
            buf.get_mut(x, area.y).set_char(ch).set_style(cell_style);
        }

        // Show cursor block if at end
        let end_x = area.x + cursor_screen_col as u16;
        if cursor_screen_col >= display.len() && end_x < area.x + area.width {
            buf.get_mut(end_x, area.y)
                .set_char(' ')
                .set_style(style.add_modifier(Modifier::REVERSED));
        }
    }
}
