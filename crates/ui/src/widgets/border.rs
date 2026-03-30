use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, Widget},
};

pub struct PaneBorder<'a> {
    pub title: Option<&'a str>,
    pub focused: bool,
}

impl<'a> Widget for PaneBorder<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let border_color = if self.focused { Color::Cyan } else { Color::DarkGray };
        let mut block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color));
        if let Some(title) = self.title {
            block = block.title(title);
        }
        block.render(area, buf);
    }
}
