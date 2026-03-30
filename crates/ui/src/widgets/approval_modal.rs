#![allow(dead_code, unused_imports, unused_variables)]
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Widget},
};
use ai::approval::ApprovalModalState;

pub struct ApprovalModalWidget<'a> {
    pub state: &'a ApprovalModalState,
}

impl<'a> Widget for ApprovalModalWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let pending = match &self.state.pending {
            Some(p) => p,
            None => return,
        };

        // Centered overlay
        let w = area.width.min(72);
        let h = area.height.min(18);
        let x = area.x + (area.width.saturating_sub(w)) / 2;
        let y = area.y + (area.height.saturating_sub(h)) / 2;
        let popup_area = Rect { x, y, width: w, height: h };

        // Clear background
        Clear.render(popup_area, buf);

        let risk_color = match pending.risk.as_str() {
            "high"   => Color::Red,
            "medium" => Color::Yellow,
            _        => Color::Green,
        };

        let title = format!(" ⚠ Approval Required — {} [{}] ",
            pending.action_name, pending.risk.to_uppercase());
        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(risk_color).add_modifier(Modifier::BOLD));
        let inner = block.inner(popup_area);
        block.render(popup_area, buf);

        let mut iy = inner.y;

        let write_line = |buf: &mut Buffer, y: u16, text: &str, style: Style| {
            for (i, ch) in text.chars().take(inner.width as usize).enumerate() {
                let x = inner.x + i as u16;
                if x < inner.right() {
                    buf[(x, y)].set_char(ch).set_style(style);
                }
            }
        };

        write_line(buf, iy, &format!("Agent: {}", pending.agent),
            Style::default().fg(Color::Cyan));
        iy += 1;

        write_line(buf, iy, &"-".repeat(inner.width as usize),
            Style::default().fg(Color::DarkGray));
        iy += 1;

        for desc_line in pending.description.lines() {
            if iy >= inner.bottom().saturating_sub(3) { break; }
            write_line(buf, iy, desc_line, Style::default().fg(Color::White));
            iy += 1;
        }

        // Action buttons at the bottom
        let btn_y = popup_area.bottom().saturating_sub(2);
        let buttons = " [y] Approve   [n] Deny   [a] Approve All   [d] Deny All ";
        write_line(buf, btn_y, &"-".repeat(inner.width as usize),
            Style::default().fg(Color::DarkGray));
        write_line(buf, btn_y + 1, buttons,
            Style::default().fg(Color::White).add_modifier(Modifier::BOLD));
    }
}
