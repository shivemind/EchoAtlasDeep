#![allow(dead_code, unused_imports, unused_variables)]
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, Widget},
};

#[derive(Debug, Clone)]
pub struct SpendPanelState {
    pub breakdown: String,
    pub session_cost: f64,
    pub session_budget: f64,    // 0 = no budget
    pub budget_fraction: f64,   // 0.0–1.0+
    pub over_budget: bool,
    pub warning: bool,
    pub ai_status: String,      // "$0.0142"
}

pub struct SpendPanelWidget<'a> {
    pub state: &'a SpendPanelState,
}

impl<'a> Widget for SpendPanelWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block = Block::default()
            .title(" Spend Tracker ")
            .borders(Borders::ALL)
            .border_style(if self.state.over_budget {
                Style::default().fg(Color::Red)
            } else if self.state.warning {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::Cyan)
            });
        let inner = block.inner(area);
        block.render(area, buf);

        let mut y = inner.y;

        // Budget bar
        if self.state.session_budget > 0.0 && y < inner.bottom() {
            let pct = (self.state.budget_fraction * 100.0).min(100.0) as u16;
            let color = if self.state.over_budget { Color::Red }
                        else if self.state.warning { Color::Yellow }
                        else { Color::Green };
            let label = format!("Budget: ${:.4} / ${:.2}",
                self.state.session_cost, self.state.session_budget);
            // Simple bar
            let bar_width = inner.width as usize;
            let filled = (bar_width * pct as usize / 100).min(bar_width);
            for (i, ch) in label.chars().take(bar_width).enumerate() {
                let x = inner.x + i as u16;
                if x < inner.right() {
                    let style = if i < filled { Style::default().fg(Color::Black).bg(color) }
                                else { Style::default().fg(color) };
                    buf[(x, y)].set_char(ch).set_style(style);
                }
            }
            y += 1;
        }

        // Breakdown table
        for line in self.state.breakdown.lines() {
            if y >= inner.bottom() { break; }
            for (i, ch) in line.chars().enumerate() {
                let x = inner.x + i as u16;
                if x < inner.right() {
                    buf[(x, y)].set_char(ch).set_style(Style::default().fg(Color::White));
                }
            }
            y += 1;
        }
    }
}
