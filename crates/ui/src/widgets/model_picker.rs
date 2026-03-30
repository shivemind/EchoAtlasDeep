#![allow(dead_code, unused_imports, unused_variables)]
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::Widget,
};

#[derive(Debug, Clone)]
pub struct ModelEntry {
    pub backend: String,
    pub model_id: String,
    pub model_name: String,
    pub context_window: Option<u32>,
}

pub struct ModelPickerWidget<'a> {
    pub entries: &'a [ModelEntry],
    pub selected: usize,
}

impl<'a> Widget for ModelPickerWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 20 || area.height < 3 {
            return;
        }

        let title_style = Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD);
        let title = " Model Switcher (<Enter> select, <Esc> cancel) ";
        let mut col = area.x + 1;
        for c in title.chars() {
            if col >= area.x + area.width - 1 {
                break;
            }
            buf[(col, area.y)].set_style(title_style).set_char(c);
            col += 1;
        }

        for (i, entry) in self.entries.iter().enumerate().take(area.height as usize - 1) {
            let row = area.y + 1 + i as u16;
            if row >= area.y + area.height {
                break;
            }

            let is_sel = i == self.selected;
            let bg = if is_sel { Color::DarkGray } else { Color::Reset };

            let bold_mod = if is_sel {
                Modifier::BOLD
            } else {
                Modifier::empty()
            };
            let backend_style = Style::default().fg(Color::Magenta).bg(bg).add_modifier(bold_mod);
            let name_style = Style::default().fg(Color::White).bg(bg).add_modifier(bold_mod);
            let ctx_style = Style::default().fg(Color::DarkGray).bg(bg);

            // Fill row background
            for cx in area.x..area.x + area.width {
                buf[(cx, row)].set_bg(bg);
            }

            let mut x = area.x + 1;
            let backend_str = format!("[{:8}] ", entry.backend);
            for c in backend_str.chars() {
                if x >= area.x + area.width {
                    break;
                }
                buf[(x, row)].set_style(backend_style).set_char(c);
                x += 1;
            }
            for c in entry.model_name.chars() {
                if x >= area.x + area.width.saturating_sub(10) {
                    break;
                }
                buf[(x, row)].set_style(name_style).set_char(c);
                x += 1;
            }
            if let Some(ctx) = entry.context_window {
                let ctx_str = format!(" {:>6}K", ctx / 1000);
                let ctx_x = area
                    .x
                    .saturating_add(area.width.saturating_sub(ctx_str.len() as u16 + 1));
                for (ci, c) in ctx_str.chars().enumerate() {
                    let cx = ctx_x + ci as u16;
                    if cx < area.x + area.width {
                        buf[(cx, row)].set_style(ctx_style).set_char(c);
                    }
                }
            }
        }
    }
}
