#![allow(dead_code)]
//! Full-pane diagnostics list widget.
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::Widget,
};

#[derive(Debug, Clone)]
pub struct DiagnosticLine {
    pub file: String,
    pub line: u32,
    pub col: u32,
    pub severity: u8, // 1=error, 2=warn, 3=info, 4=hint
    pub message: String,
    pub source: Option<String>,
}

impl DiagnosticLine {
    pub fn severity_char(&self) -> char {
        match self.severity {
            1 => '✖',
            2 => '▲',
            3 => '●',
            _ => '·',
        }
    }

    pub fn severity_style(&self) -> Style {
        match self.severity {
            1 => Style::default().fg(Color::Red),
            2 => Style::default().fg(Color::Yellow),
            3 => Style::default().fg(Color::Blue),
            _ => Style::default().fg(Color::DarkGray),
        }
    }
}

/// Full-pane diagnostics list.
pub struct DiagnosticsPanel<'a> {
    pub entries: &'a [DiagnosticLine],
    pub selected: usize,
    pub scroll: usize,
    pub focused: bool,
}

impl<'a> Widget for DiagnosticsPanel<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.height < 2 || area.width < 8 {
            return;
        }

        // Title row
        let title_style = Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD);
        let title = " DIAGNOSTICS ";
        let mut col = area.x + 1;
        for c in title.chars() {
            if col >= area.x + area.width {
                break;
            }
            buf[(col, area.y)].set_style(title_style).set_char(c);
            col += 1;
        }

        let visible = (area.height as usize).saturating_sub(1);
        for (i, entry) in self
            .entries
            .iter()
            .enumerate()
            .skip(self.scroll)
            .take(visible)
        {
            let row = area.y + 1 + (i - self.scroll) as u16;
            if row >= area.y + area.height {
                break;
            }

            let is_sel = i == self.selected;
            let bg = if is_sel && self.focused {
                Color::DarkGray
            } else {
                Color::Reset
            };

            // Clear row
            for c in area.x..area.x + area.width {
                buf[(c, row)].set_bg(bg).set_char(' ');
            }

            // Severity icon
            let sev_style = entry.severity_style().bg(bg);
            if area.x + 1 < area.x + area.width {
                buf[(area.x + 1, row)]
                    .set_style(sev_style)
                    .set_char(entry.severity_char());
            }

            // File:line:col
            let loc = format!(" {}:{}:{} ", entry.file, entry.line + 1, entry.col + 1);
            let loc_style = Style::default().fg(Color::Gray).bg(bg);
            let mut x = area.x + 3;
            for c in loc.chars() {
                if x >= area.x + area.width {
                    break;
                }
                buf[(x, row)].set_style(loc_style).set_char(c);
                x += 1;
            }

            // Message
            let msg_style = Style::default().fg(Color::White).bg(bg);
            for c in entry.message.chars() {
                if x >= area.x + area.width.saturating_sub(1) {
                    break;
                }
                buf[(x, row)].set_style(msg_style).set_char(c);
                x += 1;
            }
        }

        // Empty state
        if self.entries.is_empty() {
            let msg = " No diagnostics ";
            let msg_style = Style::default().fg(Color::DarkGray);
            let mut x = area.x + 1;
            for c in msg.chars() {
                if x >= area.x + area.width {
                    break;
                }
                buf[(x, area.y + 1)].set_style(msg_style).set_char(c);
                x += 1;
            }
        }
    }
}
