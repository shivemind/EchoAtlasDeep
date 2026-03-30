#![allow(dead_code, unused_imports, unused_variables)]
//! Phase 10 — Point 30: Session manager.

use std::collections::HashMap;
use std::path::PathBuf;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::Widget,
};

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Session {
    pub name: String,
    pub created_at: u64,
    pub open_files: Vec<String>,
    pub layout_json: String, // serialized LayoutTree
    pub theme_name: String,
    pub cursor_positions: HashMap<String, (usize, usize)>,
}

pub struct SessionManager {
    sessions_dir: PathBuf,
}

impl SessionManager {
    pub fn new() -> Self {
        let sessions_dir = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("rmtide")
            .join("sessions");
        let _ = std::fs::create_dir_all(&sessions_dir);
        Self { sessions_dir }
    }

    pub fn list(&self) -> Vec<Session> {
        let Ok(entries) = std::fs::read_dir(&self.sessions_dir) else {
            return Vec::new();
        };
        let mut sessions: Vec<Session> = entries
            .flatten()
            .filter(|e| {
                e.path()
                    .extension()
                    .map(|x| x == "json")
                    .unwrap_or(false)
            })
            .filter_map(|e| {
                let data = std::fs::read_to_string(e.path()).ok()?;
                serde_json::from_str(&data).ok()
            })
            .collect();
        sessions.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        sessions
    }

    pub fn save(&self, session: Session) -> anyhow::Result<()> {
        let _ = std::fs::create_dir_all(&self.sessions_dir);
        let file_name = format!("{}.json", sanitize_name(&session.name));
        let path = self.sessions_dir.join(file_name);
        let data = serde_json::to_string_pretty(&session)?;
        std::fs::write(&path, data)?;
        Ok(())
    }

    pub fn load(&self, name: &str) -> anyhow::Result<Session> {
        let file_name = format!("{}.json", sanitize_name(name));
        let path = self.sessions_dir.join(file_name);
        let data = std::fs::read_to_string(&path)?;
        let session: Session = serde_json::from_str(&data)?;
        Ok(session)
    }

    pub fn delete(&self, name: &str) -> anyhow::Result<()> {
        let file_name = format!("{}.json", sanitize_name(name));
        let path = self.sessions_dir.join(file_name);
        std::fs::remove_file(&path)?;
        Ok(())
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}

fn sanitize_name(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect()
}

#[derive(Clone, Copy, PartialEq)]
pub enum SessionMode {
    Browse,
    SaveAs,
}

pub struct SessionPickerState {
    pub open: bool,
    pub sessions: Vec<Session>,
    pub selected: usize,
    pub query: String,
    pub name_input: String,
    pub mode: SessionMode,
}

impl SessionPickerState {
    pub fn new() -> Self {
        Self {
            open: false,
            sessions: Vec::new(),
            selected: 0,
            query: String::new(),
            name_input: String::new(),
            mode: SessionMode::Browse,
        }
    }
}

impl Default for SessionPickerState {
    fn default() -> Self {
        Self::new()
    }
}

pub struct SessionPickerWidget<'a> {
    pub state: &'a SessionPickerState,
}

impl Widget for SessionPickerWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 20 || area.height < 8 {
            return;
        }

        let picker_w = (area.width * 2 / 3).max(55).min(area.width);
        let picker_h = (area.height * 2 / 3).max(12).min(area.height);
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

        draw_box(buf, picker, Style::default().fg(Color::Rgb(100, 100, 200)));

        let title = match self.state.mode {
            SessionMode::Browse => " Sessions ",
            SessionMode::SaveAs => " Save Session ",
        };
        let title_x = picker.x + (picker.width.saturating_sub(title.len() as u16)) / 2;
        for (i, ch) in title.chars().enumerate() {
            buf.get_mut(title_x + i as u16, picker.y)
                .set_char(ch)
                .set_style(Style::default()
                    .fg(Color::Rgb(130, 130, 230))
                    .add_modifier(Modifier::BOLD));
        }

        let inner = Rect {
            x: picker.x + 1,
            y: picker.y + 1,
            width: picker.width.saturating_sub(2),
            height: picker.height.saturating_sub(2),
        };

        match self.state.mode {
            SessionMode::SaveAs => {
                // Name input row
                let label = "Session name: ";
                for (i, ch) in label.chars().enumerate() {
                    if i as u16 >= inner.width { break; }
                    buf.get_mut(inner.x + i as u16, inner.y)
                        .set_char(ch)
                        .set_style(Style::default().fg(Color::Cyan));
                }
                let input_x = inner.x + label.len() as u16;
                let input_w = inner.width.saturating_sub(label.len() as u16);
                let input_style = Style::default().fg(Color::White).bg(Color::Rgb(40, 40, 60));
                for i in 0..input_w {
                    buf.get_mut(input_x + i, inner.y).set_char(' ').set_style(input_style);
                }
                for (i, ch) in self.state.name_input.chars().enumerate() {
                    if i as u16 >= input_w { break; }
                    buf.get_mut(input_x + i as u16, inner.y)
                        .set_char(ch)
                        .set_style(input_style);
                }
                // Cursor
                let cursor_x = input_x + self.state.name_input.len() as u16;
                if cursor_x < input_x + input_w {
                    buf.get_mut(cursor_x, inner.y)
                        .set_char('_')
                        .set_style(input_style);
                }

                let help = "  Enter: save   Esc: cancel";
                for (i, ch) in help.chars().enumerate() {
                    if i as u16 >= inner.width { break; }
                    buf.get_mut(inner.x + i as u16, inner.y + 1)
                        .set_char(ch)
                        .set_style(Style::default().fg(Color::DarkGray));
                }
            }
            SessionMode::Browse => {
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

                // Filtered sessions
                let q = self.state.query.to_lowercase();
                let filtered: Vec<&Session> = self.state.sessions.iter()
                    .filter(|s| q.is_empty() || s.name.to_lowercase().contains(&q))
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
                    let session = filtered[idx];
                    let y = list_y + row as u16;
                    let is_sel = idx == self.state.selected;

                    let row_style = if is_sel {
                        Style::default().fg(Color::White).bg(Color::Rgb(50, 50, 100))
                    } else {
                        Style::default().fg(Color::Gray)
                    };

                    for x in 0..inner.width {
                        buf.get_mut(inner.x + x, y).set_char(' ').set_style(row_style);
                    }

                    let file_count = session.open_files.len();
                    let theme = &session.theme_name;
                    let line = format!(
                        "  {:<30} {} files  theme:{}",
                        session.name, file_count, theme
                    );
                    for (i, ch) in line.chars().enumerate() {
                        if i as u16 >= inner.width { break; }
                        buf.get_mut(inner.x + i as u16, y)
                            .set_char(ch)
                            .set_style(row_style);
                    }
                }

                if filtered.is_empty() {
                    let msg = "  (no sessions saved yet — use :SessionSave)";
                    for (i, ch) in msg.chars().enumerate() {
                        if i as u16 >= inner.width { break; }
                        buf.get_mut(inner.x + i as u16, list_y)
                            .set_char(ch)
                            .set_style(Style::default().fg(Color::DarkGray));
                    }
                }

                // Footer
                let footer_y = picker.y + picker.height.saturating_sub(1);
                let help = " Enter:load  s:save-as  dd:delete  Esc:close ";
                for (i, ch) in help.chars().enumerate() {
                    if picker.x + 1 + i as u16 >= picker.x + picker.width.saturating_sub(1) { break; }
                    buf.get_mut(picker.x + 1 + i as u16, footer_y)
                        .set_char(ch)
                        .set_style(Style::default().fg(Color::DarkGray));
                }
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
