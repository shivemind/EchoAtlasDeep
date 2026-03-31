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

        // Fill the entire status bar row.
        let full = format!("{:<width$}{:>rw$}",
            left,
            right,
            width = area.width as usize,
            rw = right.len(),
        );
        let line = Line::from(vec![
            Span::styled(full.clone(), info_style),
        ]);

        // Overwrite mode badge.
        if let Some(cell) = buf.cell_mut((area.x, area.y)) {
            // Just render the whole line then re-apply mode badge color.
        }
        let _ = ratatui::widgets::Paragraph::new(line)
            .style(info_style);

        // Render manually for simplicity.
        for (x_off, ch) in full.chars().enumerate() {
            let x = area.x + x_off as u16;
            if x >= area.x + area.width { break; }
            let style = if x_off < mode_str.len() { mode_style } else { info_style };
            buf[(x, area.y)].set_char(ch).set_style(style);
        }
    }
}
