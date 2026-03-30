#![allow(dead_code)]
//! Fuzzy file picker widget.
use std::path::PathBuf;

use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::Widget,
};

pub struct FilePickerState {
    pub query: String,
    pub all_files: Vec<PathBuf>,
    /// (index into all_files, match score)
    pub filtered: Vec<(usize, i64)>,
    pub selected: usize,
    matcher: SkimMatcherV2,
}

impl FilePickerState {
    pub fn new(root: &std::path::Path) -> Self {
        let all_files: Vec<PathBuf> = walkdir::WalkDir::new(root)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .filter(|e| {
                !e.path().components().any(|c| {
                    let s = c.as_os_str();
                    s == ".git" || s == "target"
                })
            })
            .map(|e| e.path().to_path_buf())
            .collect();

        let mut s = Self {
            query: String::new(),
            all_files,
            filtered: Vec::new(),
            selected: 0,
            matcher: SkimMatcherV2::default(),
        };
        s.refilter();
        s
    }

    pub fn push_char(&mut self, c: char) {
        self.query.push(c);
        self.refilter();
    }

    pub fn pop_char(&mut self) {
        self.query.pop();
        self.refilter();
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

    pub fn selected_path(&self) -> Option<&PathBuf> {
        self.filtered.get(self.selected).map(|(i, _)| &self.all_files[*i])
    }

    fn refilter(&mut self) {
        self.selected = 0;
        if self.query.is_empty() {
            self.filtered = (0..self.all_files.len()).map(|i| (i, 0)).take(50).collect();
            return;
        }
        let mut scored: Vec<(usize, i64)> = self
            .all_files
            .iter()
            .enumerate()
            .filter_map(|(i, p)| {
                let s = p.to_string_lossy();
                self.matcher
                    .fuzzy_match(&s, &self.query)
                    .map(|score| (i, score))
            })
            .collect();
        scored.sort_by_key(|(_, s)| -*s);
        scored.truncate(50);
        self.filtered = scored;
    }
}

pub struct FilePickerWidget<'a> {
    pub state: &'a FilePickerState,
}

impl<'a> Widget for FilePickerWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 4 || area.height < 4 {
            return;
        }

        // Centered box: 70% width, 50% height
        let popup_width = (area.width as f32 * 0.7) as u16;
        let popup_height = (area.height as f32 * 0.5) as u16;
        let popup_x = area.x + (area.width.saturating_sub(popup_width)) / 2;
        let popup_y = area.y + (area.height.saturating_sub(popup_height)) / 2;

        let popup = Rect {
            x: popup_x,
            y: popup_y,
            width: popup_width,
            height: popup_height,
        };

        // Clear background
        for y in popup.y..popup.y + popup.height {
            for x in popup.x..popup.x + popup.width {
                buf.get_mut(x, y)
                    .set_char(' ')
                    .set_style(Style::default().bg(Color::DarkGray));
            }
        }

        // Draw border
        // top/bottom
        for x in popup.x..popup.x + popup.width {
            buf.get_mut(x, popup.y).set_char('─');
            buf.get_mut(x, popup.y + popup.height - 1).set_char('─');
        }
        for y in popup.y..popup.y + popup.height {
            buf.get_mut(popup.x, y).set_char('│');
            buf.get_mut(popup.x + popup.width - 1, y).set_char('│');
        }
        buf.get_mut(popup.x, popup.y).set_char('┌');
        buf.get_mut(popup.x + popup.width - 1, popup.y).set_char('┐');
        buf.get_mut(popup.x, popup.y + popup.height - 1).set_char('└');
        buf.get_mut(popup.x + popup.width - 1, popup.y + popup.height - 1).set_char('┘');

        // Title
        let title = " File Picker ";
        for (i, ch) in title.chars().enumerate() {
            let x = popup.x + 1 + i as u16;
            if x < popup.x + popup.width - 1 {
                buf.get_mut(x, popup.y).set_char(ch);
            }
        }

        // Inner area
        let inner_x = popup.x + 1;
        let inner_y = popup.y + 1;
        let inner_w = popup.width.saturating_sub(2) as usize;
        let inner_h = popup.height.saturating_sub(2);

        // Query prompt line
        let prompt = format!("> {}_", self.state.query);
        for (i, ch) in prompt.chars().enumerate() {
            if i >= inner_w {
                break;
            }
            buf.get_mut(inner_x + i as u16, inner_y)
                .set_char(ch)
                .set_style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD));
        }

        // File list
        let list_start_y = inner_y + 1;
        let list_height = inner_h.saturating_sub(1) as usize;

        for (row, (file_idx, _score)) in self.state.filtered.iter().enumerate().take(list_height) {
            let path = &self.state.all_files[*file_idx];
            let path_str = path.to_string_lossy();
            let selected = row == self.state.selected;

            let style = if selected {
                Style::default()
                    .bg(Color::Blue)
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Gray)
            };

            let screen_y = list_start_y + row as u16;
            // Clear line
            for x in inner_x..inner_x + inner_w as u16 {
                buf.get_mut(x, screen_y).set_char(' ').set_style(style);
            }
            // Draw path
            for (i, ch) in path_str.chars().enumerate() {
                if i >= inner_w {
                    break;
                }
                buf.get_mut(inner_x + i as u16, screen_y)
                    .set_char(ch)
                    .set_style(style);
            }
        }
    }
}
