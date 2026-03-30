#![allow(dead_code, unused_imports, unused_variables)]
//! Phase 10 — Point 24: Symbol browser picker widget.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::Widget,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SymbolKind {
    Function,
    Type,
    Const,
    Variable,
    Module,
    Interface,
    Field,
    Other,
}

impl SymbolKind {
    pub fn icon(&self) -> char {
        match self {
            SymbolKind::Function  => 'ƒ',
            SymbolKind::Type      => 'τ',
            SymbolKind::Const     => 'K',
            SymbolKind::Variable  => 'v',
            SymbolKind::Module    => 'M',
            SymbolKind::Interface => 'I',
            SymbolKind::Field     => 'f',
            SymbolKind::Other     => '◆',
        }
    }

    pub fn color(&self) -> Color {
        match self {
            SymbolKind::Function  => Color::Yellow,
            SymbolKind::Type      => Color::Cyan,
            SymbolKind::Const     => Color::Green,
            SymbolKind::Variable  => Color::White,
            SymbolKind::Module    => Color::Blue,
            SymbolKind::Interface => Color::Magenta,
            SymbolKind::Field     => Color::Gray,
            SymbolKind::Other     => Color::DarkGray,
        }
    }
}

#[derive(Clone, Debug)]
pub struct SymbolEntry {
    pub name: String,
    pub kind: SymbolKind,
    pub file: String,
    pub line: usize,
    pub container: Option<String>,
}

pub struct SymbolBrowserState {
    pub open: bool,
    pub query: String,
    pub symbols: Vec<SymbolEntry>,
    pub filtered: Vec<usize>, // indices into symbols
    pub selected: usize,
    pub workspace_mode: bool, // false = current file only
}

impl SymbolBrowserState {
    pub fn new() -> Self {
        Self {
            open: false,
            query: String::new(),
            symbols: Vec::new(),
            filtered: Vec::new(),
            selected: 0,
            workspace_mode: false,
        }
    }

    pub fn filter(&mut self) {
        if self.query.is_empty() {
            self.filtered = (0..self.symbols.len()).collect();
        } else {
            let q = self.query.to_lowercase();
            self.filtered = self.symbols
                .iter()
                .enumerate()
                .filter(|(_, sym)| sym.name.to_lowercase().contains(&q))
                .map(|(i, _)| i)
                .collect();
        }
        if self.selected >= self.filtered.len() {
            self.selected = self.filtered.len().saturating_sub(1);
        }
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    pub fn move_down(&mut self) {
        if self.selected + 1 < self.filtered.len() {
            self.selected += 1;
        }
    }

    pub fn selected_entry(&self) -> Option<&SymbolEntry> {
        self.filtered
            .get(self.selected)
            .and_then(|&idx| self.symbols.get(idx))
    }
}

impl Default for SymbolBrowserState {
    fn default() -> Self {
        Self::new()
    }
}

pub struct SymbolBrowserWidget<'a> {
    pub state: &'a SymbolBrowserState,
}

impl Widget for SymbolBrowserWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 20 || area.height < 8 {
            return;
        }

        // Centered picker
        let picker_w = (area.width * 2 / 3).max(50).min(area.width);
        let picker_h = (area.height * 3 / 4).max(10).min(area.height);
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

        // Border
        draw_box(buf, picker, Style::default().fg(Color::Magenta));

        // Title
        let mode_label = if self.state.workspace_mode { "Workspace" } else { "File" };
        let title = format!(" Symbols [{}] ", mode_label);
        let title_x = picker.x + (picker.width.saturating_sub(title.len() as u16)) / 2;
        for (i, ch) in title.chars().enumerate() {
            buf.get_mut(title_x + i as u16, picker.y)
                .set_char(ch)
                .set_style(Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD));
        }

        let inner = Rect {
            x: picker.x + 1,
            y: picker.y + 1,
            width: picker.width.saturating_sub(2),
            height: picker.height.saturating_sub(2),
        };

        // Query bar (row 0)
        let query_prefix = "› ";
        let query_style = Style::default().fg(Color::White).bg(Color::Rgb(40, 40, 60));
        for i in 0..inner.width {
            buf.get_mut(inner.x + i, inner.y).set_char(' ').set_style(query_style);
        }
        let query_text = format!("{}{}", query_prefix, self.state.query);
        for (i, ch) in query_text.chars().enumerate() {
            if i as u16 >= inner.width {
                break;
            }
            buf.get_mut(inner.x + i as u16, inner.y)
                .set_char(ch)
                .set_style(query_style);
        }
        // Cursor
        let cursor_x = inner.x + query_text.chars().count() as u16;
        if cursor_x < inner.x + inner.width {
            buf.get_mut(cursor_x, inner.y)
                .set_char('_')
                .set_style(query_style);
        }

        // Separator
        let sep_y = inner.y + 1;
        for x in 0..inner.width {
            buf.get_mut(inner.x + x, sep_y)
                .set_char('─')
                .set_style(Style::default().fg(Color::DarkGray));
        }

        // Results list
        let list_y = sep_y + 1;
        let list_height = (inner.y + inner.height).saturating_sub(list_y) as usize;

        let scroll = if self.state.selected >= list_height {
            self.state.selected + 1 - list_height
        } else {
            0
        };

        for row in 0..list_height {
            let sym_display_idx = scroll + row;
            if sym_display_idx >= self.state.filtered.len() {
                break;
            }
            let sym_idx = self.state.filtered[sym_display_idx];
            let sym = &self.state.symbols[sym_idx];
            let is_selected = sym_display_idx == self.state.selected;

            let row_y = list_y + row as u16;
            let row_style = if is_selected {
                Style::default().fg(Color::White).bg(Color::Blue)
            } else {
                Style::default().fg(Color::Gray)
            };

            // Clear row
            for x in 0..inner.width {
                buf.get_mut(inner.x + x, row_y)
                    .set_char(' ')
                    .set_style(row_style);
            }

            // Kind icon
            let icon_style = if is_selected {
                Style::default().fg(sym.kind.color()).bg(Color::Blue)
            } else {
                Style::default().fg(sym.kind.color())
            };
            buf.get_mut(inner.x, row_y)
                .set_char(sym.kind.icon())
                .set_style(icon_style);
            buf.get_mut(inner.x + 1, row_y)
                .set_char(' ')
                .set_style(row_style);

            // Symbol name
            let name_max = (inner.width as usize).saturating_sub(2 + 20); // leave space for file:line
            let name_style = if is_selected {
                Style::default().fg(Color::White).bg(Color::Blue).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            for (i, ch) in sym.name.chars().enumerate() {
                if i >= name_max {
                    break;
                }
                buf.get_mut(inner.x + 2 + i as u16, row_y)
                    .set_char(ch)
                    .set_style(name_style);
            }

            // File:line right-aligned
            let loc_text = format!("{}:{}", sym.file, sym.line);
            let loc_max = 20usize;
            let loc_text: String = if loc_text.len() > loc_max {
                format!("…{}", &loc_text[loc_text.len() - loc_max + 1..])
            } else {
                loc_text
            };
            let loc_x = inner.x + inner.width - loc_max as u16;
            for (i, ch) in loc_text.chars().enumerate() {
                buf.get_mut(loc_x + i as u16, row_y)
                    .set_char(ch)
                    .set_style(Style::default().fg(Color::DarkGray).bg(
                        if is_selected { Color::Blue } else { Color::Reset }
                    ));
            }
        }

        // Footer
        let footer_y = picker.y + picker.height.saturating_sub(1);
        let help = " ↑↓:navigate  Enter:jump  Tab:workspace  Esc:close ";
        for (i, ch) in help.chars().enumerate() {
            if picker.x + 1 + i as u16 >= picker.x + picker.width.saturating_sub(1) {
                break;
            }
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
