#![allow(dead_code)]
//! Completion popup widget.
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::Widget,
};

#[derive(Debug, Clone)]
pub struct CompletionEntry {
    pub label: String,
    pub kind_label: String, // e.g. "fn", "var", "type"
    pub detail: Option<String>,
    pub insert_text: Option<String>,
    pub is_snippet: bool,
}

impl CompletionEntry {
    pub fn kind_style(&self) -> Style {
        match self.kind_label.as_str() {
            "fn" | "method" => Style::default().fg(Color::Blue),
            "var" | "field" => Style::default().fg(Color::Green),
            "type" | "struct" => Style::default().fg(Color::Cyan),
            "mod" | "use" => Style::default().fg(Color::Magenta),
            "kw" => Style::default().fg(Color::Yellow),
            _ => Style::default().fg(Color::Gray),
        }
    }
}

/// Completion popup — renders over the editor at the cursor position.
pub struct CompletionPopup<'a> {
    pub entries: &'a [CompletionEntry],
    pub selected: usize,
    pub max_visible: usize,
}

impl<'a> Widget for CompletionPopup<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if self.entries.is_empty() || area.width < 10 || area.height < 2 {
            return;
        }

        let visible_count = self
            .max_visible
            .min(self.entries.len())
            .min(area.height as usize);
        let scroll_offset = if self.selected >= visible_count {
            self.selected - visible_count + 1
        } else {
            0
        };

        for (i, entry) in self
            .entries
            .iter()
            .enumerate()
            .skip(scroll_offset)
            .take(visible_count)
        {
            let row = area.y + (i - scroll_offset) as u16;
            if row >= area.y + area.height {
                break;
            }

            let is_selected = i == self.selected;
            let bg = if is_selected {
                Color::DarkGray
            } else {
                Color::Reset
            };
            let fg = if is_selected {
                Color::White
            } else {
                Color::Gray
            };

            // Fill row background
            for col in area.x..area.x + area.width {
                buf[(col, row)].set_bg(bg);
            }

            // Kind badge (up to 4 chars)
            let kind_str: String = entry.kind_label.chars().take(4).collect();
            let kind_padded = format!("{:<4}", kind_str);
            let kind_style = entry.kind_style().bg(bg);
            let mut x = area.x;
            for c in kind_padded.chars() {
                if x >= area.x + area.width {
                    break;
                }
                buf[(x, row)].set_style(kind_style).set_char(c);
                x += 1;
            }
            // Separator space
            if x < area.x + area.width {
                buf[(x, row)].set_bg(bg).set_char(' ');
                x += 1;
            }

            // Label
            let label_style = Style::default()
                .fg(Color::White)
                .bg(bg)
                .add_modifier(if is_selected {
                    Modifier::BOLD
                } else {
                    Modifier::empty()
                });
            let label_start = x;
            for c in entry.label.chars() {
                if x >= area.x + area.width.saturating_sub(1) {
                    break;
                }
                buf[(x, row)].set_style(label_style).set_char(c);
                x += 1;
            }

            // Detail (right-aligned, dimmed)
            if let Some(detail) = &entry.detail {
                let detail_style = Style::default().fg(Color::DarkGray).bg(bg);
                let available = (area.x + area.width).saturating_sub(x + 1) as usize;
                if available > 2 {
                    let d: String = detail.chars().take(available).collect();
                    let dx = area.x + area.width - d.len() as u16;
                    for (ci, c) in d.chars().enumerate() {
                        let cx = dx + ci as u16;
                        if cx >= area.x + area.width {
                            break;
                        }
                        buf[(cx, row)].set_style(detail_style).set_char(c);
                    }
                }
            }
        }
    }
}

/// Convert LSP CompletionItemKind (u8) to a short human label.
pub fn kind_label(kind: Option<u8>) -> &'static str {
    match kind {
        Some(1) => "txt",
        Some(2) => "fn",
        Some(3) => "fn",
        Some(4) => "fn",
        Some(5) => "field",
        Some(6) => "var",
        Some(7) => "class",
        Some(8) => "iface",
        Some(9) => "mod",
        Some(10) => "prop",
        Some(12) => "var",
        Some(13) => "kw",
        Some(14) => "snip",
        Some(21) => "struct",
        Some(22) => "event",
        Some(23) => "op",
        _ => "·",
    }
}
