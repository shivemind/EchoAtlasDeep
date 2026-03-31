use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Widget,
};

pub struct StatusBar<'a> {
    pub mode: &'a str,
    pub file_name: Option<&'a str>,
    pub branch: Option<&'a str>,
    pub backend: &'a str,
    pub cursor_pos: (usize, usize),
    pub is_modified: bool,
}

impl<'a> Widget for StatusBar<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let mode_style = Style::default()
            .fg(Color::Black)
            .bg(Color::Cyan)
            .add_modifier(Modifier::BOLD);
        let info_style = Style::default().fg(Color::White).bg(Color::DarkGray);
        let right_style = Style::default().fg(Color::Gray).bg(Color::DarkGray);

        let mode_str = format!(" {} ", self.mode.to_uppercase());
        let file_str = format!(
            " {}{}",
            self.file_name.unwrap_or("[No Name]"),
            if self.is_modified { " [+]" } else { "" }
        );
        let branch_str = self.branch
            .map(|b| format!("  {b} "))
            .unwrap_or_default();
        let backend_str = format!(" AI:{} ", self.backend);
        let pos_str = format!(" {}:{} ", self.cursor_pos.0 + 1, self.cursor_pos.1 + 1);

        let left = format!("{mode_str}{file_str}{branch_str}");
        let right = format!("{backend_str}{pos_str}");

        // Build a string that fills exactly area.width chars: left flush, right flush.
        let w = area.width as usize;
        let right_len = right.len().min(w);
        let left_w = w.saturating_sub(right_len);
        let full = format!("{:<lw$}{}", &left[..left.len().min(left_w)], right, lw = left_w);

        // Render character by character so we can colour the mode badge differently.
        for (x_off, ch) in full.chars().enumerate() {
            let x = area.x + x_off as u16;
            if x_off >= w { break; }
            let style = if x_off < mode_str.len() {
                mode_style
            } else if x_off >= left_w {
                right_style
            } else {
                info_style
            };
            buf[(x, area.y)].set_char(ch).set_style(style);
        }
    }
}
