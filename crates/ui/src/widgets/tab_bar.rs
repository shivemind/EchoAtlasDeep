#![allow(dead_code, unused_imports, unused_variables)]
//! Phase 10 — Point 22: Multi-tab buffer bar widget.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::Widget,
};

#[derive(Clone, Debug)]
pub struct TabEntry {
    pub buffer_id: usize,
    pub name: String,
    pub is_dirty: bool,
    pub is_active: bool,
}

pub struct TabBarState {
    pub tabs: Vec<TabEntry>,
    pub scroll_offset: usize,
}

impl TabBarState {
    pub fn new() -> Self {
        Self {
            tabs: Vec::new(),
            scroll_offset: 0,
        }
    }

    pub fn add_tab(&mut self, id: usize, name: &str) {
        // Deactivate all others
        for t in &mut self.tabs {
            t.is_active = false;
        }
        self.tabs.push(TabEntry {
            buffer_id: id,
            name: name.to_string(),
            is_dirty: false,
            is_active: true,
        });
    }

    pub fn close_tab(&mut self, id: usize) {
        let was_active = self.tabs.iter().find(|t| t.buffer_id == id).map(|t| t.is_active).unwrap_or(false);
        self.tabs.retain(|t| t.buffer_id != id);
        // If we closed the active tab, activate the last one
        if was_active && !self.tabs.is_empty() {
            let last = self.tabs.len() - 1;
            self.tabs[last].is_active = true;
        }
    }

    pub fn set_active(&mut self, id: usize) {
        for t in &mut self.tabs {
            t.is_active = t.buffer_id == id;
        }
    }

    pub fn set_dirty(&mut self, id: usize, dirty: bool) {
        if let Some(t) = self.tabs.iter_mut().find(|t| t.buffer_id == id) {
            t.is_dirty = dirty;
        }
    }

    pub fn active_idx(&self) -> Option<usize> {
        self.tabs.iter().position(|t| t.is_active)
    }

    pub fn cycle_next(&mut self) {
        if self.tabs.is_empty() {
            return;
        }
        let cur = self.active_idx().unwrap_or(0);
        let next = (cur + 1) % self.tabs.len();
        for (i, t) in self.tabs.iter_mut().enumerate() {
            t.is_active = i == next;
        }
    }

    pub fn cycle_prev(&mut self) {
        if self.tabs.is_empty() {
            return;
        }
        let cur = self.active_idx().unwrap_or(0);
        let prev = if cur == 0 { self.tabs.len() - 1 } else { cur - 1 };
        for (i, t) in self.tabs.iter_mut().enumerate() {
            t.is_active = i == prev;
        }
    }
}

impl Default for TabBarState {
    fn default() -> Self {
        Self::new()
    }
}

pub struct TabBarWidget<'a> {
    pub state: &'a TabBarState,
}

impl Widget for TabBarWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.height == 0 || area.width == 0 {
            return;
        }

        // Fill background
        for x in 0..area.width {
            buf.get_mut(area.x + x, area.y)
                .set_char(' ')
                .set_style(Style::default().bg(Color::DarkGray));
        }

        if self.state.tabs.is_empty() {
            return;
        }

        let available_width = area.width as usize;
        let has_overflow_left = self.state.scroll_offset > 0;
        let left_arrow_width = if has_overflow_left { 2 } else { 0 };

        // Build tab strings to measure total width
        let tab_strings: Vec<String> = self.state.tabs.iter().map(|t| {
            if t.is_dirty {
                format!(" {} [+] ", t.name)
            } else {
                format!(" {} ", t.name)
            }
        }).collect();

        // Calculate visible tabs from scroll_offset
        let mut x_pos = area.x as usize + left_arrow_width;
        let has_overflow_right: bool;

        // Render left overflow arrow
        if has_overflow_left {
            buf.get_mut(area.x, area.y)
                .set_char('«')
                .set_style(Style::default().fg(Color::Yellow).bg(Color::DarkGray));
            buf.get_mut(area.x + 1, area.y)
                .set_char(' ')
                .set_style(Style::default().bg(Color::DarkGray));
        }

        let right_reserve = 2usize; // for » arrow if needed
        let mut last_rendered = self.state.scroll_offset;

        for (idx, tab) in self.state.tabs.iter().enumerate().skip(self.state.scroll_offset) {
            let tab_str = &tab_strings[idx];
            let tab_width = tab_str.chars().count();

            let remaining = available_width.saturating_sub(x_pos - area.x as usize).saturating_sub(right_reserve);
            if tab_width > remaining {
                break;
            }

            let style = if tab.is_active {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Gray).bg(Color::DarkGray)
            };

            for (i, ch) in tab_str.chars().enumerate() {
                let tx = x_pos + i;
                if tx >= area.x as usize + available_width {
                    break;
                }
                buf.get_mut(tx as u16, area.y)
                    .set_char(ch)
                    .set_style(style);
            }

            // Separator between tabs
            let sep_x = x_pos + tab_width;
            if sep_x < area.x as usize + available_width.saturating_sub(right_reserve) {
                buf.get_mut(sep_x as u16, area.y)
                    .set_char('│')
                    .set_style(Style::default().fg(Color::Gray).bg(Color::DarkGray));
            }

            x_pos += tab_width + 1; // +1 for separator
            last_rendered = idx;
        }

        // Right overflow arrow
        let overflow_right = last_rendered + 1 < self.state.tabs.len();
        if overflow_right {
            let arrow_x = area.x + area.width.saturating_sub(2);
            buf.get_mut(arrow_x, area.y)
                .set_char(' ')
                .set_style(Style::default().bg(Color::DarkGray));
            buf.get_mut(arrow_x + 1, area.y)
                .set_char('»')
                .set_style(Style::default().fg(Color::Yellow).bg(Color::DarkGray));
        }
    }
}
