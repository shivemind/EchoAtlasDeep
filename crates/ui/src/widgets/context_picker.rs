#![allow(dead_code, unused_imports, unused_variables)]
//! Context injection toggle overlay — lets the user choose what context to send to the AI.
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Widget},
};
use serde::{Deserialize, Serialize};

/// Which context sources to include when sending to the AI.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContextSources {
    pub current_file: bool,
    pub open_buffers: bool,
    pub lsp_diagnostics: bool,
    pub git_diff_staged: bool,
    pub git_diff_unstaged: bool,
    pub git_log: bool,
    pub git_log_count: usize,
    pub clipboard: bool,
    pub terminal_scrollback: bool,
}

impl Default for ContextSources {
    fn default() -> Self {
        Self {
            current_file: true,
            open_buffers: true,
            lsp_diagnostics: true,
            git_diff_staged: true,
            git_diff_unstaged: true,
            git_log: false,
            git_log_count: 10,
            clipboard: false,
            terminal_scrollback: false,
        }
    }
}

impl ContextSources {
    /// Estimate total token cost (very rough heuristic).
    pub fn estimated_tokens(&self, file_lines: usize, diag_count: usize) -> usize {
        let mut tokens = 0usize;
        if self.current_file { tokens += file_lines * 5; }
        if self.open_buffers { tokens += 2000; }
        if self.lsp_diagnostics { tokens += diag_count * 30; }
        if self.git_diff_staged { tokens += 500; }
        if self.git_diff_unstaged { tokens += 500; }
        if self.git_log { tokens += self.git_log_count * 80; }
        if self.clipboard { tokens += 200; }
        if self.terminal_scrollback { tokens += 1000; }
        tokens
    }
}

pub struct ContextPickerState {
    pub open: bool,
    pub sources: ContextSources,
    pub selected: usize,
    pub estimated_tokens: usize,
}

impl ContextPickerState {
    pub fn new() -> Self {
        Self {
            open: false,
            sources: ContextSources::default(),
            selected: 0,
            estimated_tokens: 0,
        }
    }

    const ITEM_COUNT: usize = 9;

    /// Toggle the currently selected source.
    pub fn toggle_selected(&mut self) {
        match self.selected {
            0 => self.sources.current_file = !self.sources.current_file,
            1 => self.sources.open_buffers = !self.sources.open_buffers,
            2 => self.sources.lsp_diagnostics = !self.sources.lsp_diagnostics,
            3 => self.sources.git_diff_staged = !self.sources.git_diff_staged,
            4 => self.sources.git_diff_unstaged = !self.sources.git_diff_unstaged,
            5 => self.sources.git_log = !self.sources.git_log,
            6 => {
                // Increment git_log_count (cycle: 5, 10, 20, 50)
                self.sources.git_log_count = match self.sources.git_log_count {
                    5 => 10, 10 => 20, 20 => 50, _ => 5,
                };
            }
            7 => self.sources.clipboard = !self.sources.clipboard,
            8 => self.sources.terminal_scrollback = !self.sources.terminal_scrollback,
            _ => {}
        }
    }

    pub fn select_up(&mut self) {
        if self.selected > 0 { self.selected -= 1; }
    }

    pub fn select_down(&mut self) {
        if self.selected < Self::ITEM_COUNT - 1 { self.selected += 1; }
    }
}

impl Default for ContextPickerState {
    fn default() -> Self {
        Self::new()
    }
}

pub struct ContextPickerWidget<'a> {
    pub state: &'a ContextPickerState,
}

impl<'a> Widget for ContextPickerWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Center the overlay
        let w = 50u16.min(area.width);
        let h = 16u16.min(area.height);
        let x = area.x + area.width.saturating_sub(w) / 2;
        let y = area.y + area.height.saturating_sub(h) / 2;
        let overlay = Rect { x, y, width: w, height: h };

        // Clear background
        for row in y..y + h {
            for col in x..x + w {
                buf[(col, row)].set_char(' ')
                    .set_style(Style::default().bg(Color::Black));
            }
        }

        let block = Block::default()
            .title(" AI Context Sources ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Magenta));
        let inner = block.inner(overlay);
        block.render(overlay, buf);

        if inner.height == 0 { return; }

        let items: &[(&str, bool)] = &[
            ("Current file",          self.state.sources.current_file),
            ("Open buffers",          self.state.sources.open_buffers),
            ("LSP diagnostics",       self.state.sources.lsp_diagnostics),
            ("Git diff (staged)",     self.state.sources.git_diff_staged),
            ("Git diff (unstaged)",   self.state.sources.git_diff_unstaged),
            ("Git log",               self.state.sources.git_log),
        ];

        let mut iy = inner.y;
        let max_y = inner.y + inner.height;

        for (idx, (label, enabled)) in items.iter().enumerate() {
            if iy >= max_y.saturating_sub(3) { break; }
            let is_selected = idx == self.state.selected;
            let checkbox = if *enabled { "[x]" } else { "[ ]" };
            let line = format!("  {} {}", checkbox, label);
            let style = if is_selected {
                Style::default().fg(Color::Black).bg(Color::Magenta)
            } else if *enabled {
                Style::default().fg(Color::Green)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            render_text(buf, inner.x, iy, inner.width, &line, style);
            iy += 1;
        }

        // Git log count row
        if iy < max_y.saturating_sub(3) {
            let is_selected = 6 == self.state.selected;
            let line = format!("  [{}] Git log count: {}",
                if self.state.sources.git_log { "x" } else { " " },
                self.state.sources.git_log_count);
            let style = if is_selected {
                Style::default().fg(Color::Black).bg(Color::Magenta)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            render_text(buf, inner.x, iy, inner.width, &line, style);
            iy += 1;
        }

        // Clipboard
        if iy < max_y.saturating_sub(3) {
            let is_selected = 7 == self.state.selected;
            let line = format!("  {} Clipboard",
                if self.state.sources.clipboard { "[x]" } else { "[ ]" });
            let style = if is_selected {
                Style::default().fg(Color::Black).bg(Color::Magenta)
            } else if self.state.sources.clipboard {
                Style::default().fg(Color::Green)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            render_text(buf, inner.x, iy, inner.width, &line, style);
            iy += 1;
        }

        // Terminal scrollback
        if iy < max_y.saturating_sub(3) {
            let is_selected = 8 == self.state.selected;
            let line = format!("  {} Terminal scrollback",
                if self.state.sources.terminal_scrollback { "[x]" } else { "[ ]" });
            let style = if is_selected {
                Style::default().fg(Color::Black).bg(Color::Magenta)
            } else if self.state.sources.terminal_scrollback {
                Style::default().fg(Color::Green)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            render_text(buf, inner.x, iy, inner.width, &line, style);
            iy += 1;
        }

        // Token budget bar
        if max_y > inner.y + 2 {
            let bar_y = max_y - 2;
            let tokens = self.state.estimated_tokens;
            let max_tokens = 128_000usize;
            let fraction = (tokens as f32 / max_tokens as f32).min(1.0);
            let bar_w = inner.width.saturating_sub(20) as usize;
            let filled = (fraction * bar_w as f32) as usize;
            let bar_color = if fraction > 0.9 { Color::Red }
                else if fraction > 0.7 { Color::Yellow }
                else { Color::Green };

            let prefix = format!("~{:>6} tok ", tokens);
            render_text(buf, inner.x, bar_y, inner.width, &prefix,
                Style::default().fg(Color::White));
            let bar_x = inner.x + prefix.chars().count() as u16;
            for i in 0..bar_w {
                let ch = if i < filled { '█' } else { '░' };
                let style = if i < filled {
                    Style::default().fg(bar_color)
                } else {
                    Style::default().fg(Color::DarkGray)
                };
                let bx = bar_x + i as u16;
                if bx < inner.right() {
                    buf[(bx, bar_y)].set_char(ch).set_style(style);
                }
            }

            // Help footer
            render_text(buf, inner.x, max_y - 1, inner.width,
                "  Space:toggle  Enter:confirm  Esc:close",
                Style::default().fg(Color::DarkGray));
        }
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn render_text(buf: &mut Buffer, x: u16, y: u16, width: u16, text: &str, style: Style) {
    let chars: Vec<char> = text.chars().collect();
    for (i, &ch) in chars.iter().enumerate() {
        let cx = x + i as u16;
        if cx >= x + width { break; }
        buf[(cx, y)].set_char(ch).set_style(style);
    }
}
