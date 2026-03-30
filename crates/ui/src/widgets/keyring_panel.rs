#![allow(dead_code, unused_imports, unused_variables)]
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Widget},
};

#[derive(Debug, Clone)]
pub struct KeyEntry {
    pub provider: String,
    pub label: String,
    pub masked: String,     // "********...ab12"
    pub age_days: u64,
    pub last_used: Option<u64>,
    pub needs_rotation: bool,
}

pub struct KeyringPanelState {
    pub entries: Vec<KeyEntry>,
    pub selected: usize,
    pub input_mode: KeyringInputMode,
    pub input_buf: String,
    pub input_provider: String,
    pub input_label: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum KeyringInputMode {
    Browse,
    EnterKey,    // user is typing a new API key
    EnterLabel,
}

impl KeyringPanelState {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            selected: 0,
            input_mode: KeyringInputMode::Browse,
            input_buf: String::new(),
            input_provider: String::new(),
            input_label: String::new(),
        }
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 { self.selected -= 1; }
    }

    pub fn move_down(&mut self) {
        if self.selected + 1 < self.entries.len() { self.selected += 1; }
    }
}

pub struct KeyringPanelWidget<'a> {
    pub state: &'a KeyringPanelState,
}

impl<'a> Widget for KeyringPanelWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block = Block::default()
            .title(" API Key Vault (BYOK) ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan));
        let inner = block.inner(area);
        block.render(area, buf);

        let mut y = inner.y;

        // Header
        if y < inner.bottom() {
            let header = format!("{:<12} {:<12} {:<20} {:>8} {}",
                "Provider", "Label", "Key", "Age", "Status");
            for (i, ch) in header.chars().enumerate() {
                let x = inner.x + i as u16;
                if x < inner.right() {
                    buf[(x, y)].set_char(ch).set_style(
                        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));
                }
            }
            y += 1;
        }

        for (idx, entry) in self.state.entries.iter().enumerate() {
            if y >= inner.bottom() { break; }
            let is_sel = idx == self.state.selected;

            let status = if entry.needs_rotation {
                format!("⚠ {} days", entry.age_days)
            } else {
                format!("✓ {} days", entry.age_days)
            };

            let row = format!("{:<12} {:<12} {:<20} {:>8} {}",
                entry.provider, entry.label, entry.masked, entry.age_days, status);

            let base_style = if is_sel {
                Style::default().bg(Color::DarkGray).fg(Color::White)
            } else {
                Style::default().fg(Color::White)
            };

            for (i, ch) in row.chars().enumerate() {
                let x = inner.x + i as u16;
                if x < inner.right() {
                    buf[(x, y)].set_char(ch).set_style(base_style);
                }
            }
            y += 1;
        }

        if self.state.entries.is_empty() && y < inner.bottom() {
            let msg = "  No keys stored. Press 'a' to add a key.";
            for (i, ch) in msg.chars().enumerate() {
                let x = inner.x + i as u16;
                if x < inner.right() {
                    buf[(x, y)].set_char(ch).set_style(Style::default().fg(Color::DarkGray));
                }
            }
            y += 1;
        }

        // Input mode
        if self.state.input_mode == KeyringInputMode::EnterKey && y < inner.bottom() {
            y += 1;
            let prompt = format!("Enter API key for {} [{}]: {}█",
                self.state.input_provider,
                self.state.input_label,
                "*".repeat(self.state.input_buf.len().min(20)));
            for (i, ch) in prompt.chars().enumerate() {
                let x = inner.x + i as u16;
                if x < inner.right() {
                    buf[(x, y)].set_char(ch).set_style(Style::default().fg(Color::Cyan));
                }
            }
        } else if y + 1 < inner.bottom() {
            // Hint bar
            y = inner.bottom() - 1;
            let hint = " a add  d delete  r rotate  Enter select  q close";
            for (i, ch) in hint.chars().enumerate() {
                let x = inner.x + i as u16;
                if x < inner.right() {
                    buf[(x, y)].set_char(ch).set_style(Style::default().fg(Color::DarkGray));
                }
            }
        }
    }
}
