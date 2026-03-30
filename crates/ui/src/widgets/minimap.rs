#![allow(dead_code, unused_imports, unused_variables)]
//! Phase 10 — Point 27: Code minimap sidebar widget.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::Widget,
};

pub struct MinimapState {
    pub open: bool,
    pub width: u16, // default 12
    pub viewport_top: usize,
    pub viewport_height: usize,
    pub total_lines: usize,
    pub error_lines: Vec<usize>,
    pub warning_lines: Vec<usize>,
    pub search_lines: Vec<usize>,
    pub git_changed_lines: Vec<usize>,
}

impl MinimapState {
    pub fn new() -> Self {
        Self {
            open: false,
            width: 12,
            viewport_top: 0,
            viewport_height: 0,
            total_lines: 0,
            error_lines: Vec::new(),
            warning_lines: Vec::new(),
            search_lines: Vec::new(),
            git_changed_lines: Vec::new(),
        }
    }

    pub fn update(
        &mut self,
        lines: &[String],
        viewport_top: usize,
        viewport_height: usize,
        errors: &[usize],
        warnings: &[usize],
    ) {
        self.total_lines = lines.len();
        self.viewport_top = viewport_top;
        self.viewport_height = viewport_height;
        self.error_lines = errors.to_vec();
        self.warning_lines = warnings.to_vec();
    }
}

impl Default for MinimapState {
    fn default() -> Self {
        Self::new()
    }
}

pub struct MinimapWidget<'a> {
    pub state: &'a MinimapState,
    pub lines: &'a [String],
}

impl Widget for MinimapWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        let total = self.lines.len().max(1);
        let h = area.height as usize;
        let w = area.width as usize;

        // Background
        for y in area.y..area.y + area.height {
            for x in area.x..area.x + area.width {
                buf.get_mut(x, y)
                    .set_char(' ')
                    .set_style(Style::default().bg(Color::Rgb(20, 20, 25)));
            }
        }

        // Scale factor: how many file lines per display row
        let scale = total as f64 / h as f64;

        // Viewport highlight band
        let vp_start_row = (self.state.viewport_top as f64 / scale) as u16;
        let vp_end_row = ((self.state.viewport_top + self.state.viewport_height) as f64 / scale) as u16;

        for row in 0..h {
            let file_line_start = (row as f64 * scale) as usize;
            let file_line_end = ((row + 1) as f64 * scale) as usize;
            let y = area.y + row as u16;

            // Determine annotation color for this row
            let has_error = self.state.error_lines.iter()
                .any(|&l| l >= file_line_start && l < file_line_end);
            let has_warning = !has_error && self.state.warning_lines.iter()
                .any(|&l| l >= file_line_start && l < file_line_end);
            let has_search = self.state.search_lines.iter()
                .any(|&l| l >= file_line_start && l < file_line_end);
            let has_git = self.state.git_changed_lines.iter()
                .any(|&l| l >= file_line_start && l < file_line_end);

            let is_in_viewport = row as u16 >= vp_start_row && row as u16 < vp_end_row;

            let bg = if is_in_viewport {
                Color::Rgb(40, 40, 60)
            } else {
                Color::Rgb(20, 20, 25)
            };

            // Render minimap characters — use a compressed representation
            for col in 0..(w.saturating_sub(1)) {
                let char_fraction = col as f64 / (w.saturating_sub(1)) as f64;

                // Sample the file content
                let ch = if file_line_start < self.lines.len() {
                    let line = &self.lines[file_line_start];
                    let char_pos = (char_fraction * line.len() as f64) as usize;
                    let c = line.chars().nth(char_pos).unwrap_or(' ');
                    if c.is_whitespace() || c.is_control() { '·' } else { '█' }
                } else {
                    ' '
                };

                let style = Style::default().bg(bg).fg(Color::Rgb(60, 60, 70));
                let render_ch = if ch == '█' { '▌' } else { ' ' };
                buf.get_mut(area.x + col as u16, y)
                    .set_char(render_ch)
                    .set_style(style);
            }

            // Annotation column (rightmost)
            let ann_x = area.x + area.width.saturating_sub(1);
            let ann_style = if has_error {
                Style::default().fg(Color::Red).bg(bg)
            } else if has_warning {
                Style::default().fg(Color::Yellow).bg(bg)
            } else if has_search {
                Style::default().fg(Color::Cyan).bg(bg)
            } else if has_git {
                Style::default().fg(Color::Green).bg(bg)
            } else {
                Style::default().fg(Color::DarkGray).bg(bg)
            };

            let ann_ch = if has_error || has_warning || has_search || has_git {
                '▐'
            } else if is_in_viewport {
                '▌'
            } else {
                ' '
            };
            buf.get_mut(ann_x, y).set_char(ann_ch).set_style(ann_style);
        }
    }
}
