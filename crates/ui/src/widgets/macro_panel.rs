#![allow(dead_code, unused_imports, unused_variables)]
//! Phase 10 — Point 28: Macro recorder and manager.

use std::collections::HashMap;
use std::path::PathBuf;
use parking_lot::RwLock;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::Widget,
};

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct MacroDef {
    pub register: char,
    pub name: Option<String>,
    pub keystrokes: Vec<String>,
    pub description: Option<String>,
}

pub struct MacroManager {
    macros: RwLock<HashMap<char, MacroDef>>,
    recording: RwLock<Option<char>>,
    recording_buf: RwLock<Vec<String>>,
    last_register: RwLock<Option<char>>,
    db_path: PathBuf,
}

impl MacroManager {
    pub fn new() -> Self {
        let db_path = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("rmtide")
            .join("macros.json");
        let manager = Self {
            macros: RwLock::new(HashMap::new()),
            recording: RwLock::new(None),
            recording_buf: RwLock::new(Vec::new()),
            last_register: RwLock::new(None),
            db_path,
        };
        let _ = manager.load_from_disk();
        manager
    }

    fn load_from_disk(&self) -> anyhow::Result<()> {
        if self.db_path.exists() {
            let data = std::fs::read_to_string(&self.db_path)?;
            let macros: HashMap<char, MacroDef> = serde_json::from_str(&data)?;
            *self.macros.write() = macros;
        }
        Ok(())
    }

    /// Start recording keystrokes into the given register.
    pub fn start_recording(&self, register: char) {
        *self.recording.write() = Some(register);
        self.recording_buf.write().clear();
    }

    /// Stop recording and save to the register.
    pub fn stop_recording(&self) {
        let register = match *self.recording.read() {
            Some(r) => r,
            None => return,
        };
        let keystrokes = self.recording_buf.read().clone();
        *self.recording.write() = None;
        *self.last_register.write() = Some(register);
        self.macros.write().insert(
            register,
            MacroDef {
                register,
                name: None,
                keystrokes,
                description: None,
            },
        );
    }

    pub fn is_recording(&self) -> bool {
        self.recording.read().is_some()
    }

    pub fn recording_register(&self) -> Option<char> {
        *self.recording.read()
    }

    /// Push a keystroke to the recording buffer if currently recording.
    pub fn push_key(&self, key: &str) {
        if self.recording.read().is_some() {
            self.recording_buf.write().push(key.to_string());
        }
    }

    pub fn get(&self, register: char) -> Option<MacroDef> {
        self.macros.read().get(&register).cloned()
    }

    pub fn list(&self) -> Vec<MacroDef> {
        let mut v: Vec<MacroDef> = self.macros.read().values().cloned().collect();
        v.sort_by_key(|m| m.register);
        v
    }

    /// Return the keystroke list for replaying the given register.
    pub fn replay(&self, register: char) -> Vec<String> {
        *self.last_register.write() = Some(register);
        self.macros
            .read()
            .get(&register)
            .map(|m| m.keystrokes.clone())
            .unwrap_or_default()
    }

    /// Replay the last-used register.
    pub fn replay_last(&self) -> Vec<String> {
        let reg = match *self.last_register.read() {
            Some(r) => r,
            None => return Vec::new(),
        };
        self.replay(reg)
    }

    pub fn flush(&self) -> anyhow::Result<()> {
        if let Some(parent) = self.db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let data = serde_json::to_string_pretty(&*self.macros.read())?;
        std::fs::write(&self.db_path, data)?;
        Ok(())
    }
}

impl Default for MacroManager {
    fn default() -> Self {
        Self::new()
    }
}

pub struct MacroPanelState {
    pub open: bool,
    pub selected: usize,
    pub macros: Vec<MacroDef>,
    pub is_recording: bool,
    pub recording_register: Option<char>,
}

impl MacroPanelState {
    pub fn new() -> Self {
        Self {
            open: false,
            selected: 0,
            macros: Vec::new(),
            is_recording: false,
            recording_register: None,
        }
    }
}

impl Default for MacroPanelState {
    fn default() -> Self {
        Self::new()
    }
}

pub struct MacroPanelWidget<'a> {
    pub state: &'a MacroPanelState,
    pub manager: Option<&'a MacroManager>,
}

impl Widget for MacroPanelWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 20 || area.height < 6 {
            return;
        }

        let panel_w = (area.width * 2 / 3).max(50).min(area.width);
        let panel_h = (area.height * 2 / 3).max(10).min(area.height);
        let panel_x = area.x + (area.width.saturating_sub(panel_w)) / 2;
        let panel_y = area.y + (area.height.saturating_sub(panel_h)) / 2;

        let panel = Rect {
            x: panel_x,
            y: panel_y,
            width: panel_w,
            height: panel_h,
        };

        let bg_style = Style::default().bg(Color::Rgb(25, 25, 35));
        for y in panel.y..panel.y + panel.height {
            for x in panel.x..panel.x + panel.width {
                buf.get_mut(x, y).set_char(' ').set_style(bg_style);
            }
        }

        draw_box(buf, panel, Style::default().fg(Color::Green));

        // Recording indicator
        let is_recording = self.manager
            .map(|m| m.is_recording())
            .unwrap_or(self.state.is_recording);
        let rec_reg = self.manager
            .and_then(|m| m.recording_register())
            .or(self.state.recording_register);

        let recording_indicator = if is_recording {
            format!(" ● REC @{} ", rec_reg.unwrap_or('?'))
        } else {
            " Macros ".to_string()
        };
        let title_style = if is_recording {
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
        };
        let title_x = panel.x + 1;
        for (i, ch) in recording_indicator.chars().enumerate() {
            if title_x + i as u16 >= panel.x + panel.width.saturating_sub(1) {
                break;
            }
            buf.get_mut(title_x + i as u16, panel.y)
                .set_char(ch)
                .set_style(title_style);
        }

        let inner = Rect {
            x: panel.x + 1,
            y: panel.y + 1,
            width: panel.width.saturating_sub(2),
            height: panel.height.saturating_sub(2),
        };

        // Column headers
        let header = format!("{:<3} {:<20} {:<8} {}", "Reg", "Name", "Keys", "Description");
        for (i, ch) in header.chars().enumerate() {
            if i as u16 >= inner.width { break; }
            buf.get_mut(inner.x + i as u16, inner.y)
                .set_char(ch)
                .set_style(Style::default().fg(Color::DarkGray).add_modifier(Modifier::UNDERLINED));
        }

        let macros: Vec<MacroDef> = if let Some(mgr) = self.manager {
            mgr.list()
        } else {
            self.state.macros.clone()
        };

        let list_y = inner.y + 1;
        let list_h = (inner.y + inner.height).saturating_sub(list_y) as usize;
        let scroll = if self.state.selected >= list_h {
            self.state.selected + 1 - list_h
        } else {
            0
        };

        for row in 0..list_h {
            let idx = scroll + row;
            if idx >= macros.len() { break; }
            let m = &macros[idx];
            let y = list_y + row as u16;
            let is_sel = idx == self.state.selected;
            let row_style = if is_sel {
                Style::default().fg(Color::White).bg(Color::Rgb(40, 60, 40))
            } else {
                Style::default().fg(Color::Gray)
            };

            for x in 0..inner.width {
                buf.get_mut(inner.x + x, y).set_char(' ').set_style(row_style);
            }

            let name_str = m.name.as_deref().unwrap_or("");
            let desc_str = m.description.as_deref().unwrap_or("");
            let key_count = m.keystrokes.len();
            let line = format!(
                "@{:<2} {:<20} {:<8} {}",
                m.register, name_str, key_count, desc_str
            );
            for (i, ch) in line.chars().enumerate() {
                if i as u16 >= inner.width { break; }
                buf.get_mut(inner.x + i as u16, y)
                    .set_char(ch)
                    .set_style(row_style);
            }
        }

        if macros.is_empty() {
            let msg = "  (no macros recorded — press q in normal mode to record)";
            for (i, ch) in msg.chars().enumerate() {
                if i as u16 >= inner.width { break; }
                buf.get_mut(inner.x + i as u16, list_y)
                    .set_char(ch)
                    .set_style(Style::default().fg(Color::DarkGray));
            }
        }

        // Footer
        let footer_y = panel.y + panel.height.saturating_sub(1);
        let help = " q:record/stop  @reg:replay  .:replay last  Esc:close ";
        for (i, ch) in help.chars().enumerate() {
            if panel.x + 1 + i as u16 >= panel.x + panel.width.saturating_sub(1) { break; }
            buf.get_mut(panel.x + 1 + i as u16, footer_y)
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
