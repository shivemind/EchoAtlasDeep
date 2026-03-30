#![allow(dead_code, unused_imports, unused_variables)]
//! Phase 10 — Point 26: Bookmarks and jump list.

use std::path::{Path, PathBuf};
use parking_lot::RwLock;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::Widget,
};

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Bookmark {
    pub file: String,
    pub line: usize,
    pub note: Option<String>,
}

pub struct BookmarkManager {
    bookmarks: RwLock<Vec<Bookmark>>,
    db_path: PathBuf,
}

impl BookmarkManager {
    pub fn new(workspace_root: &Path) -> Self {
        let db_path = workspace_root.join(".rmtide").join("bookmarks.json");
        let mut manager = Self {
            bookmarks: RwLock::new(Vec::new()),
            db_path,
        };
        // Try to load existing bookmarks
        let _ = manager.load_from_disk();
        manager
    }

    fn load_from_disk(&self) -> anyhow::Result<()> {
        if self.db_path.exists() {
            let data = std::fs::read_to_string(&self.db_path)?;
            let bms: Vec<Bookmark> = serde_json::from_str(&data)?;
            *self.bookmarks.write() = bms;
        }
        Ok(())
    }

    /// Toggle bookmark at file:line. If it exists, remove it; otherwise add it.
    pub fn toggle(&self, file: &str, line: usize) {
        let mut bms = self.bookmarks.write();
        if let Some(pos) = bms.iter().position(|b| b.file == file && b.line == line) {
            bms.remove(pos);
        } else {
            bms.push(Bookmark {
                file: file.to_string(),
                line,
                note: None,
            });
        }
    }

    pub fn is_bookmarked(&self, file: &str, line: usize) -> bool {
        self.bookmarks
            .read()
            .iter()
            .any(|b| b.file == file && b.line == line)
    }

    pub fn list(&self) -> Vec<Bookmark> {
        self.bookmarks.read().clone()
    }

    /// Next bookmark after the given position (wraps around).
    pub fn next_after(&self, file: &str, line: usize) -> Option<Bookmark> {
        let bms = self.bookmarks.read();
        // First, look for the next bookmark in the same file after the current line
        let next_in_file = bms.iter()
            .filter(|b| b.file == file && b.line > line)
            .min_by_key(|b| b.line);
        if let Some(b) = next_in_file {
            return Some(b.clone());
        }
        // Wrap: find any bookmark in any file
        bms.first().cloned()
    }

    /// Previous bookmark before the given position (wraps around).
    pub fn prev_before(&self, file: &str, line: usize) -> Option<Bookmark> {
        let bms = self.bookmarks.read();
        let prev_in_file = bms.iter()
            .filter(|b| b.file == file && b.line < line)
            .max_by_key(|b| b.line);
        if let Some(b) = prev_in_file {
            return Some(b.clone());
        }
        bms.last().cloned()
    }

    /// Persist bookmarks to disk.
    pub fn flush(&self) -> anyhow::Result<()> {
        if let Some(parent) = self.db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let data = serde_json::to_string_pretty(&*self.bookmarks.read())?;
        std::fs::write(&self.db_path, data)?;
        Ok(())
    }
}

pub struct BookmarkPickerState {
    pub open: bool,
    pub bookmarks: Vec<Bookmark>,
    pub selected: usize,
    pub query: String,
}

impl BookmarkPickerState {
    pub fn new() -> Self {
        Self {
            open: false,
            bookmarks: Vec::new(),
            selected: 0,
            query: String::new(),
        }
    }
}

impl Default for BookmarkPickerState {
    fn default() -> Self {
        Self::new()
    }
}

pub struct BookmarkPickerWidget<'a> {
    pub state: &'a BookmarkPickerState,
}

impl Widget for BookmarkPickerWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 20 || area.height < 6 {
            return;
        }

        let picker_w = (area.width * 2 / 3).max(50).min(area.width);
        let picker_h = (area.height / 2).max(8).min(area.height);
        let picker_x = area.x + (area.width.saturating_sub(picker_w)) / 2;
        let picker_y = area.y + (area.height.saturating_sub(picker_h)) / 2;

        let picker = Rect {
            x: picker_x,
            y: picker_y,
            width: picker_w,
            height: picker_h,
        };

        // Background
        let bg_style = Style::default().bg(Color::Rgb(25, 25, 35));
        for y in picker.y..picker.y + picker.height {
            for x in picker.x..picker.x + picker.width {
                buf.get_mut(x, y).set_char(' ').set_style(bg_style);
            }
        }

        draw_box(buf, picker, Style::default().fg(Color::Yellow));

        let title = " Bookmarks ";
        let title_x = picker.x + (picker.width.saturating_sub(title.len() as u16)) / 2;
        for (i, ch) in title.chars().enumerate() {
            buf.get_mut(title_x + i as u16, picker.y)
                .set_char(ch)
                .set_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));
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

        // Filtered bookmarks
        let q = self.state.query.to_lowercase();
        let filtered: Vec<&Bookmark> = self.state.bookmarks.iter()
            .filter(|b| q.is_empty() || b.file.to_lowercase().contains(&q))
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
            let bm = filtered[idx];
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

            let note_part = bm.note.as_deref().map(|n| format!(" — {n}")).unwrap_or_default();
            let line = format!("  ● {}:{}{}", bm.file, bm.line, note_part);
            for (i, ch) in line.chars().enumerate() {
                if i as u16 >= inner.width { break; }
                buf.get_mut(inner.x + i as u16, y).set_char(ch).set_style(row_style);
            }
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
