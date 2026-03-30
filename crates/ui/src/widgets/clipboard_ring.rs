#![allow(dead_code, unused_imports, unused_variables)]
//! Phase 10 — Point 29: Clipboard history ring.

use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use parking_lot::RwLock;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::Widget,
};

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct ClipEntry {
    pub content: String,
    pub timestamp: u64,
    pub pinned: bool,
    pub size: usize,
}

pub struct ClipboardRing {
    entries: RwLock<Vec<ClipEntry>>,
    max: usize,
    db_path: PathBuf,
}

impl ClipboardRing {
    pub fn new() -> Self {
        let db_path = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("rmtide")
            .join("clipboard.json");
        Self {
            entries: RwLock::new(Vec::new()),
            max: 100,
            db_path,
        }
    }

    fn now_secs() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }

    /// Push new content to the ring. Deduplicate: if identical content already
    /// exists, remove the old entry first.
    pub fn push(&self, content: String) {
        if content.is_empty() {
            return;
        }
        let size = content.len();
        let entry = ClipEntry {
            content: content.clone(),
            timestamp: Self::now_secs(),
            pinned: false,
            size,
        };
        let mut entries = self.entries.write();
        // Remove duplicate (non-pinned) if it exists
        entries.retain(|e| e.content != content || e.pinned);
        entries.insert(0, entry);
        // Trim non-pinned entries to max
        let pinned: Vec<ClipEntry> = entries.iter().filter(|e| e.pinned).cloned().collect();
        let unpinned: Vec<ClipEntry> = entries.iter().filter(|e| !e.pinned).cloned().collect();
        let max_unpinned = self.max.saturating_sub(pinned.len());
        let trimmed_unpinned: Vec<ClipEntry> = unpinned.into_iter().take(max_unpinned).collect();
        // Keep pinned at the start, unpinned after
        *entries = pinned.into_iter().chain(trimmed_unpinned).collect();
    }

    pub fn list(&self) -> Vec<ClipEntry> {
        self.entries.read().clone()
    }

    /// Toggle pin on the entry at the given index.
    pub fn pin(&self, idx: usize) {
        let mut entries = self.entries.write();
        if let Some(e) = entries.get_mut(idx) {
            e.pinned = !e.pinned;
        }
    }

    pub fn flush(&self) -> anyhow::Result<()> {
        if let Some(parent) = self.db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let data = serde_json::to_string_pretty(&*self.entries.read())?;
        std::fs::write(&self.db_path, data)?;
        Ok(())
    }

    pub fn load(&self) -> anyhow::Result<()> {
        if self.db_path.exists() {
            let data = std::fs::read_to_string(&self.db_path)?;
            let entries: Vec<ClipEntry> = serde_json::from_str(&data)?;
            *self.entries.write() = entries;
        }
        Ok(())
    }
}

impl Default for ClipboardRing {
    fn default() -> Self {
        Self::new()
    }
}

pub struct ClipboardPickerState {
    pub open: bool,
    pub entries: Vec<ClipEntry>,
    pub selected: usize,
    pub query: String,
}

impl ClipboardPickerState {
    pub fn new() -> Self {
        Self {
            open: false,
            entries: Vec::new(),
            selected: 0,
            query: String::new(),
        }
    }
}

impl Default for ClipboardPickerState {
    fn default() -> Self {
        Self::new()
    }
}

pub struct ClipboardPickerWidget<'a> {
    pub state: &'a ClipboardPickerState,
}

impl Widget for ClipboardPickerWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 20 || area.height < 6 {
            return;
        }

        let picker_w = (area.width * 2 / 3).max(55).min(area.width);
        let picker_h = (area.height * 2 / 3).max(10).min(area.height);
        let picker_x = area.x + (area.width.saturating_sub(picker_w)) / 2;
        let picker_y = area.y + (area.height.saturating_sub(picker_h)) / 2;

        let picker = Rect {
            x: picker_x,
            y: picker_y,
            width: picker_w,
            height: picker_h,
        };

        let bg_style = Style::default().bg(Color::Rgb(25, 25, 35));
        for y in picker.y..picker.y + picker.height {
            for x in picker.x..picker.x + picker.width {
                buf.get_mut(x, y).set_char(' ').set_style(bg_style);
            }
        }

        draw_box(buf, picker, Style::default().fg(Color::Cyan));

        let title = " Clipboard History ";
        let title_x = picker.x + (picker.width.saturating_sub(title.len() as u16)) / 2;
        for (i, ch) in title.chars().enumerate() {
            buf.get_mut(title_x + i as u16, picker.y)
                .set_char(ch)
                .set_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));
        }

        let inner = Rect {
            x: picker.x + 1,
            y: picker.y + 1,
            width: picker.width.saturating_sub(2),
            height: picker.height.saturating_sub(2),
        };

        // Query bar
        let query_style = Style::default().fg(Color::White).bg(Color::Rgb(40, 40, 55));
        for i in 0..inner.width {
            buf.get_mut(inner.x + i, inner.y).set_char(' ').set_style(query_style);
        }
        let query_text = format!("› {}", self.state.query);
        for (i, ch) in query_text.chars().enumerate() {
            if i as u16 >= inner.width { break; }
            buf.get_mut(inner.x + i as u16, inner.y)
                .set_char(ch)
                .set_style(query_style);
        }

        // Filtered entries
        let q = self.state.query.to_lowercase();
        let filtered: Vec<&ClipEntry> = self.state.entries.iter()
            .filter(|e| q.is_empty() || e.content.to_lowercase().contains(&q))
            .collect();

        let list_y = inner.y + 1;
        let list_h = (inner.y + inner.height).saturating_sub(list_y) as usize;
        let scroll = if self.state.selected >= list_h {
            self.state.selected + 1 - list_h
        } else {
            0
        };

        for row in 0..list_h {
            let idx = scroll + row;
            if idx >= filtered.len() { break; }
            let entry = filtered[idx];
            let y = list_y + row as u16;
            let is_sel = idx == self.state.selected;

            let row_style = if is_sel {
                Style::default().fg(Color::White).bg(Color::Blue)
            } else {
                Style::default().fg(Color::Gray)
            };

            for x in 0..inner.width {
                buf.get_mut(inner.x + x, y).set_char(' ').set_style(row_style);
            }

            let pin_icon = if entry.pinned { "📌 " } else { "   " };
            // Flatten to a single line preview
            let preview: String = entry.content
                .lines()
                .next()
                .unwrap_or("")
                .chars()
                .take(inner.width as usize - 20)
                .collect();
            let size_label = if entry.size < 1024 {
                format!("{}b", entry.size)
            } else {
                format!("{}K", entry.size / 1024)
            };
            let line = format!("{pin_icon}{:<width$} {size_label:>5}", preview, width = (inner.width as usize).saturating_sub(22));

            for (i, ch) in line.chars().enumerate() {
                if i as u16 >= inner.width { break; }
                buf.get_mut(inner.x + i as u16, y)
                    .set_char(ch)
                    .set_style(row_style);
            }
        }

        if filtered.is_empty() {
            let msg = "  (clipboard history is empty)";
            for (i, ch) in msg.chars().enumerate() {
                if i as u16 >= inner.width { break; }
                buf.get_mut(inner.x + i as u16, list_y)
                    .set_char(ch)
                    .set_style(Style::default().fg(Color::DarkGray));
            }
        }

        // Footer
        let footer_y = picker.y + picker.height.saturating_sub(1);
        let help = " Enter:paste  p:pin  dd:delete  Esc:close ";
        for (i, ch) in help.chars().enumerate() {
            if picker.x + 1 + i as u16 >= picker.x + picker.width.saturating_sub(1) { break; }
            buf.get_mut(picker.x + 1 + i as u16, footer_y)
                .set_char(ch)
                .set_style(Style::default().fg(Color::DarkGray));
        }
    }
}

fn draw_box(buf: &mut Buffer, area: Rect, style: Style) {
    buf.get_mut(area.x, area.y).set_char('┌').set_style(style);
    buf.get_mut(area.x + area.width.saturating_sub(1), area.y)
        .set_char('┐')
        .set_style(style);
    let by = area.y + area.height.saturating_sub(1);
    buf.get_mut(area.x, by).set_char('└').set_style(style);
    buf.get_mut(area.x + area.width.saturating_sub(1), by)
        .set_char('┘')
        .set_style(style);
    for x in (area.x + 1)..(area.x + area.width.saturating_sub(1)) {
        buf.get_mut(x, area.y).set_char('─').set_style(style);
        buf.get_mut(x, by).set_char('─').set_style(style);
    }
    for y in (area.y + 1)..by {
        buf.get_mut(area.x, y).set_char('│').set_style(style);
        buf.get_mut(area.x + area.width.saturating_sub(1), y)
            .set_char('│')
            .set_style(style);
    }
}
