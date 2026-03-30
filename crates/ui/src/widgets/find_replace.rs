#![allow(dead_code, unused_imports, unused_variables)]
//! Phase 10 — Point 23: Project-wide find and replace overlay.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::Widget,
};

#[derive(Clone)]
pub struct FindResult {
    pub file: String,
    pub line: usize,
    pub col: usize,
    pub preview: String,
    pub selected: bool,
}

#[derive(Clone, Copy, PartialEq)]
pub enum FindField {
    Search,
    Replace,
}

pub struct FindReplaceState {
    pub open: bool,
    pub search_query: String,
    pub replace_query: String,
    pub case_sensitive: bool,
    pub whole_word: bool,
    pub use_regex: bool,
    pub results: Vec<FindResult>,
    pub selected: usize,
    pub active_field: FindField,
    pub running: bool,
    pub total_matches: usize,
}

impl FindReplaceState {
    pub fn new() -> Self {
        Self {
            open: false,
            search_query: String::new(),
            replace_query: String::new(),
            case_sensitive: false,
            whole_word: false,
            use_regex: false,
            results: Vec::new(),
            selected: 0,
            active_field: FindField::Search,
            running: false,
            total_matches: 0,
        }
    }

    /// Space: toggle the currently selected match.
    pub fn toggle_match(&mut self) {
        if let Some(r) = self.results.get_mut(self.selected) {
            r.selected = !r.selected;
        }
    }

    /// 'a': toggle all results in the same file as the currently selected result.
    pub fn toggle_all_in_file(&mut self) {
        let file = match self.results.get(self.selected) {
            Some(r) => r.file.clone(),
            None => return,
        };
        let all_in_file_selected = self.results
            .iter()
            .filter(|r| r.file == file)
            .all(|r| r.selected);
        let new_state = !all_in_file_selected;
        for r in &mut self.results {
            if r.file == file {
                r.selected = new_state;
            }
        }
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    pub fn move_down(&mut self) {
        if self.selected + 1 < self.results.len() {
            self.selected += 1;
        }
    }
}

impl Default for FindReplaceState {
    fn default() -> Self {
        Self::new()
    }
}

pub struct FindReplaceWidget<'a> {
    pub state: &'a FindReplaceState,
}

impl Widget for FindReplaceWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 20 || area.height < 8 {
            return;
        }

        // Centered overlay dimensions
        let overlay_w = (area.width * 3 / 4).max(50).min(area.width);
        let overlay_h = (area.height * 3 / 4).max(12).min(area.height);
        let overlay_x = area.x + (area.width.saturating_sub(overlay_w)) / 2;
        let overlay_y = area.y + (area.height.saturating_sub(overlay_h)) / 2;

        let overlay = Rect {
            x: overlay_x,
            y: overlay_y,
            width: overlay_w,
            height: overlay_h,
        };

        // Background fill
        let bg_style = Style::default().bg(Color::Rgb(30, 30, 40));
        for y in overlay.y..overlay.y + overlay.height {
            for x in overlay.x..overlay.x + overlay.width {
                buf.get_mut(x, y).set_char(' ').set_style(bg_style);
            }
        }

        // Border
        let border_style = Style::default().fg(Color::Cyan);
        draw_box(buf, overlay, border_style);

        // Title
        let title = " Find & Replace ";
        let title_x = overlay.x + (overlay.width.saturating_sub(title.len() as u16)) / 2;
        for (i, ch) in title.chars().enumerate() {
            buf.get_mut(title_x + i as u16, overlay.y)
                .set_char(ch)
                .set_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));
        }

        let inner = Rect {
            x: overlay.x + 1,
            y: overlay.y + 1,
            width: overlay.width.saturating_sub(2),
            height: overlay.height.saturating_sub(2),
        };

        // Row 0: Search bar
        let search_label = "Search:  ";
        let search_active = self.state.active_field == FindField::Search;
        let search_style = if search_active {
            Style::default().fg(Color::White).bg(Color::Rgb(50, 50, 80))
        } else {
            Style::default().fg(Color::Gray).bg(bg_style.bg.unwrap_or(Color::Reset))
        };

        render_label(buf, inner.x, inner.y, search_label, Style::default().fg(Color::Cyan));
        let search_field_x = inner.x + search_label.len() as u16;
        let search_field_w = inner.width.saturating_sub(search_label.len() as u16 + 12);
        render_field(buf, search_field_x, inner.y, search_field_w, &self.state.search_query, search_style);

        // Toggles: Aa /\b /.*
        let toggles_x = search_field_x + search_field_w + 1;
        let case_style = if self.state.case_sensitive {
            Style::default().fg(Color::Black).bg(Color::Yellow)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        let word_style = if self.state.whole_word {
            Style::default().fg(Color::Black).bg(Color::Yellow)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        let regex_style = if self.state.use_regex {
            Style::default().fg(Color::Black).bg(Color::Yellow)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        render_label(buf, toggles_x, inner.y, "Aa", case_style);
        render_label(buf, toggles_x + 3, inner.y, "\\b", word_style);
        render_label(buf, toggles_x + 6, inner.y, ".*", regex_style);

        // Row 1: Replace bar
        let replace_label = "Replace: ";
        let replace_active = self.state.active_field == FindField::Replace;
        let replace_style = if replace_active {
            Style::default().fg(Color::White).bg(Color::Rgb(50, 50, 80))
        } else {
            Style::default().fg(Color::Gray).bg(bg_style.bg.unwrap_or(Color::Reset))
        };
        render_label(buf, inner.x, inner.y + 1, replace_label, Style::default().fg(Color::Cyan));
        let replace_field_x = inner.x + replace_label.len() as u16;
        render_field(buf, replace_field_x, inner.y + 1, search_field_w, &self.state.replace_query, replace_style);

        // Status line
        let status = if self.state.running {
            "Searching...".to_string()
        } else if self.state.results.is_empty() {
            "No results".to_string()
        } else {
            let sel_count = self.state.results.iter().filter(|r| r.selected).count();
            format!("{} matches ({} selected)", self.state.total_matches, sel_count)
        };
        render_label(buf, inner.x, inner.y + 2, &status, Style::default().fg(Color::Gray));

        // Separator
        let sep_y = inner.y + 3;
        for x in 0..inner.width {
            buf.get_mut(inner.x + x, sep_y)
                .set_char('─')
                .set_style(Style::default().fg(Color::DarkGray));
        }

        // Results list grouped by file
        let list_y = sep_y + 1;
        let list_height = (inner.y + inner.height).saturating_sub(list_y) as usize;

        // Determine scroll to keep selected visible
        let scroll = if self.state.selected >= list_height {
            self.state.selected + 1 - list_height
        } else {
            0
        };

        let mut current_file: Option<&str> = None;
        let mut display_row = 0usize;
        let mut result_idx = 0usize;

        for result in &self.state.results {
            // File header
            let show_file_header = current_file.map(|f| f != result.file.as_str()).unwrap_or(true);
            if show_file_header {
                current_file = Some(result.file.as_str());
                if display_row >= scroll && (display_row - scroll) < list_height {
                    let row_y = list_y + (display_row - scroll) as u16;
                    let file_text = format!("  {}", result.file);
                    for (i, ch) in file_text.chars().enumerate() {
                        if i as u16 >= inner.width {
                            break;
                        }
                        buf.get_mut(inner.x + i as u16, row_y)
                            .set_char(ch)
                            .set_style(Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD));
                    }
                }
                display_row += 1;
            }

            // Result row
            if display_row >= scroll && (display_row - scroll) < list_height {
                let row_y = list_y + (display_row - scroll) as u16;
                let is_sel = result_idx == self.state.selected;
                let row_style = if is_sel {
                    Style::default().fg(Color::White).bg(Color::Blue)
                } else {
                    Style::default().fg(Color::White)
                };

                // Clear row
                for x in 0..inner.width {
                    buf.get_mut(inner.x + x, row_y)
                        .set_char(' ')
                        .set_style(row_style);
                }

                let check = if result.selected { "[x]" } else { "[ ]" };
                let line_text = format!("    {} {:>4}:{:<3} {}", check, result.line, result.col, result.preview);
                for (i, ch) in line_text.chars().enumerate() {
                    if i as u16 >= inner.width {
                        break;
                    }
                    buf.get_mut(inner.x + i as u16, row_y)
                        .set_char(ch)
                        .set_style(row_style);
                }
            }

            display_row += 1;
            result_idx += 1;
        }

        // Help footer
        let footer_y = overlay.y + overlay.height.saturating_sub(1);
        let help = " Tab:switch field  Space:toggle  a:all  Enter:replace  Esc:close ";
        for (i, ch) in help.chars().enumerate() {
            if overlay.x + 1 + i as u16 >= overlay.x + overlay.width.saturating_sub(1) {
                break;
            }
            buf.get_mut(overlay.x + 1 + i as u16, footer_y)
                .set_char(ch)
                .set_style(Style::default().fg(Color::DarkGray));
        }
    }
}

fn render_label(buf: &mut Buffer, x: u16, y: u16, text: &str, style: Style) {
    for (i, ch) in text.chars().enumerate() {
        buf.get_mut(x + i as u16, y).set_char(ch).set_style(style);
    }
}

fn render_field(buf: &mut Buffer, x: u16, y: u16, width: u16, text: &str, style: Style) {
    for i in 0..width {
        buf.get_mut(x + i, y).set_char(' ').set_style(style);
    }
    for (i, ch) in text.chars().enumerate() {
        if i as u16 >= width {
            break;
        }
        buf.get_mut(x + i as u16, y).set_char(ch).set_style(style);
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
