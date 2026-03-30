#![allow(dead_code, unused_imports, unused_variables)]
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::Widget,
};

pub struct EnvPanelState {
    pub open: bool,
    pub selected_file: usize,
    pub selected_entry: usize,
    pub show_values: bool, // masking toggle
    pub edit_mode: bool,
    pub edit_key: String,
    pub edit_value: String,
}

impl EnvPanelState {
    pub fn new() -> Self {
        Self {
            open: false,
            selected_file: 0,
            selected_entry: 0,
            show_values: false,
            edit_mode: false,
            edit_key: String::new(),
            edit_value: String::new(),
        }
    }
}

pub struct EnvPanelWidget<'a> {
    pub state: &'a EnvPanelState,
    pub env: &'a runner::EnvManager,
}

impl<'a> Widget for EnvPanelWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let files = self.env.get_all_files();

        // Background
        let bg_style = Style::default().bg(Color::Rgb(14, 14, 22));
        for y in area.y..area.y + area.height {
            for x in area.x..area.x + area.width {
                buf.get_mut(x, y).set_char(' ').set_style(bg_style);
            }
        }

        if area.height < 4 || area.width < 20 {
            return;
        }

        // Border
        let border_style = Style::default().fg(Color::Yellow);
        buf.get_mut(area.x, area.y).set_char('┌').set_style(border_style);
        buf.get_mut(area.x + area.width - 1, area.y).set_char('┐').set_style(border_style);
        buf.get_mut(area.x, area.y + area.height - 1).set_char('└').set_style(border_style);
        buf.get_mut(area.x + area.width - 1, area.y + area.height - 1).set_char('┘').set_style(border_style);
        for x in (area.x + 1)..(area.x + area.width - 1) {
            buf.get_mut(x, area.y).set_char('─').set_style(border_style);
            buf.get_mut(x, area.y + area.height - 1).set_char('─').set_style(border_style);
        }
        for y in (area.y + 1)..(area.y + area.height - 1) {
            buf.get_mut(area.x, y).set_char('│').set_style(border_style);
            buf.get_mut(area.x + area.width - 1, y).set_char('│').set_style(border_style);
        }

        // Title
        let mask_indicator = if self.state.show_values { " [values visible]" } else { " [masked]" };
        let title = format!(" Env Manager{} ", mask_indicator);
        for (i, ch) in title.chars().enumerate() {
            if area.x + 1 + i as u16 >= area.x + area.width - 1 {
                break;
            }
            buf.get_mut(area.x + 1 + i as u16, area.y)
                .set_char(ch)
                .set_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));
        }

        // Two-pane layout: left = file list (25%), right = entries
        let left_w = (area.width / 4).max(12).min(25);
        let divider_x = area.x + left_w;

        // Vertical divider
        for y in (area.y + 1)..(area.y + area.height - 1) {
            buf.get_mut(divider_x, y)
                .set_char('│')
                .set_style(Style::default().fg(Color::DarkGray));
        }

        // Left: file list
        let left_y_start = area.y + 1;
        if files.is_empty() {
            let msg = "No .env files";
            for (i, ch) in msg.chars().enumerate() {
                let x = area.x + 1 + i as u16;
                if x >= divider_x {
                    break;
                }
                buf.get_mut(x, left_y_start)
                    .set_char(ch)
                    .set_style(Style::default().fg(Color::DarkGray));
            }
        } else {
            for (i, file) in files.iter().enumerate() {
                let row_y = left_y_start + i as u16;
                if row_y >= area.y + area.height - 1 {
                    break;
                }
                let is_selected = i == self.state.selected_file;
                let bg = if is_selected { Color::Rgb(30, 28, 10) } else { Color::Reset };
                let style = Style::default().bg(bg).fg(if is_selected { Color::Yellow } else { Color::Gray });

                let name_short: String = file.name.chars().take((left_w - 2) as usize).collect();
                for (j, ch) in name_short.chars().enumerate() {
                    let x = area.x + 1 + j as u16;
                    if x >= divider_x {
                        break;
                    }
                    buf.get_mut(x, row_y).set_char(ch).set_style(style);
                }
            }
        }

        // Right: key-value pairs for selected file
        let right_x = divider_x + 1;
        let right_w = area.width.saturating_sub(left_w + 2);

        if let Some(file) = files.get(self.state.selected_file) {
            // Header
            let header_y = area.y + 1;
            if header_y < area.y + area.height - 1 {
                let header = format!("{:<25} {}", "Key", "Value");
                let h_style = Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD);
                for (i, ch) in header.chars().enumerate() {
                    let x = right_x + i as u16;
                    if x >= right_x + right_w {
                        break;
                    }
                    buf.get_mut(x, header_y).set_char(ch).set_style(h_style);
                }
            }

            // Separator
            let sep_y = area.y + 2;
            if sep_y < area.y + area.height - 1 {
                for x in right_x..(right_x + right_w) {
                    buf.get_mut(x, sep_y)
                        .set_char('─')
                        .set_style(Style::default().fg(Color::DarkGray));
                }
            }

            // Entries
            let entries_y_start = area.y + 3;
            for (i, entry) in file.entries.iter().enumerate() {
                let row_y = entries_y_start + i as u16;
                if row_y >= area.y + area.height - 2 {
                    break;
                }

                let is_selected = i == self.state.selected_entry;
                let bg = if is_selected { Color::Rgb(25, 25, 40) } else { Color::Reset };

                let value_display = if entry.masked && !self.state.show_values {
                    "●●●●●●".to_string()
                } else {
                    entry.value.chars().take(40).collect::<String>()
                };

                let key_style = if entry.missing {
                    Style::default().bg(bg).fg(Color::Red)
                } else {
                    Style::default().bg(bg).fg(Color::Cyan)
                };
                let val_style = if entry.masked && !self.state.show_values {
                    Style::default().bg(bg).fg(Color::DarkGray)
                } else {
                    Style::default().bg(bg).fg(Color::White)
                };

                let key_str: String = entry.key.chars().take(24).collect();
                let padded_key = format!("{:<25}", key_str);
                for (j, ch) in padded_key.chars().enumerate() {
                    let x = right_x + j as u16;
                    if x >= right_x + right_w {
                        break;
                    }
                    buf.get_mut(x, row_y).set_char(ch).set_style(key_style);
                }

                for (j, ch) in value_display.chars().enumerate() {
                    let x = right_x + 25 + j as u16;
                    if x >= right_x + right_w {
                        break;
                    }
                    buf.get_mut(x, row_y).set_char(ch).set_style(val_style);
                }
            }

            if file.entries.is_empty() {
                let msg = " No entries in this file ";
                let msg_y = entries_y_start;
                if msg_y < area.y + area.height - 1 {
                    for (i, ch) in msg.chars().enumerate() {
                        let x = right_x + i as u16;
                        if x >= right_x + right_w {
                            break;
                        }
                        buf.get_mut(x, msg_y)
                            .set_char(ch)
                            .set_style(Style::default().fg(Color::DarkGray));
                    }
                }
            }
        }

        // Edit mode overlay
        if self.state.edit_mode {
            let edit_y = area.y + area.height - 3;
            if edit_y > area.y + 2 {
                let edit_bg = Style::default().bg(Color::Rgb(30, 30, 50));
                for x in (area.x + 1)..(area.x + area.width - 1) {
                    for ey in edit_y..edit_y + 2 {
                        buf.get_mut(x, ey).set_char(' ').set_style(edit_bg);
                    }
                }
                let key_prompt = format!(" Key: {}", self.state.edit_key);
                let val_prompt = format!(" Value: {}", self.state.edit_value);
                for (i, ch) in key_prompt.chars().enumerate() {
                    let x = area.x + 1 + i as u16;
                    if x >= area.x + area.width - 1 {
                        break;
                    }
                    buf.get_mut(x, edit_y)
                        .set_char(ch)
                        .set_style(Style::default().bg(Color::Rgb(30, 30, 50)).fg(Color::Green));
                }
                for (i, ch) in val_prompt.chars().enumerate() {
                    let x = area.x + 1 + i as u16;
                    if x >= area.x + area.width - 1 {
                        break;
                    }
                    buf.get_mut(x, edit_y + 1)
                        .set_char(ch)
                        .set_style(Style::default().bg(Color::Rgb(30, 30, 50)).fg(Color::White));
                }
            }
        }

        // Key hints
        let hints = " [Tab]=Switch pane  [e]=Edit  [d]=Delete  [n]=New  [v]=Toggle values  [q]=Close ";
        let hints_y = area.y + area.height - 1;
        for (i, ch) in hints.chars().enumerate() {
            let x = area.x + 1 + i as u16;
            if x >= area.x + area.width - 1 {
                break;
            }
            buf.get_mut(x, hints_y)
                .set_char(ch)
                .set_style(Style::default().fg(Color::DarkGray));
        }
    }
}
