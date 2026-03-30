#![allow(dead_code, unused_imports, unused_variables)]
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Widget},
};
use ai::spend::ModelPricing;

pub struct ModelMatrixWidget<'a> {
    pub models: &'a [ModelPricing],
    pub selected: usize,
}

impl<'a> Widget for ModelMatrixWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block = Block::default()
            .title(" Model Capability Matrix ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan));
        let inner = block.inner(area);
        block.render(area, buf);

        // Header row
        if inner.height == 0 { return; }
        let header = format!("{:<28} {:>8} {:>8} {:>8} {:>6} {:>6} {:>8}",
            "Model", "Ctx Win", "In$/MTk", "Out$/MTk", "Vision", "Fn-Call", "Latency");
        let header_style = Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD);

        let mut y = inner.y;
        // Render header
        for (i, ch) in header.chars().enumerate() {
            let x = inner.x + i as u16;
            if x < inner.right() {
                buf[(x, y)].set_char(ch).set_style(header_style);
            }
        }
        y += 1;

        // Separator
        if y < inner.bottom() {
            let sep = "─".repeat(inner.width as usize);
            for (i, ch) in sep.chars().enumerate() {
                let x = inner.x + i as u16;
                if x < inner.right() {
                    buf[(x, y)].set_char(ch).set_style(Style::default().fg(Color::DarkGray));
                }
            }
            y += 1;
        }

        for (idx, model) in self.models.iter().enumerate() {
            if y >= inner.bottom() { break; }
            let is_sel = idx == self.selected;

            let ctx_k = if model.context_window >= 1_000_000 {
                format!("{}M", model.context_window / 1_000_000)
            } else {
                format!("{}K", model.context_window / 1_000)
            };

            let latency = match model.latency_tier {
                ai::spend::LatencyTier::Fast   => "fast",
                ai::spend::LatencyTier::Medium => "medium",
                ai::spend::LatencyTier::Slow   => "slow",
            };

            let row = format!("{:<28} {:>8} {:>8.3} {:>8.3} {:>6} {:>6} {:>8}",
                format!("{}/{}", model.provider, model.model_id),
                ctx_k,
                model.input_per_mtok,
                model.output_per_mtok,
                if model.supports_vision    { "✓" } else { "✗" },
                if model.supports_functions { "✓" } else { "✗" },
                latency,
            );

            let style = if is_sel {
                Style::default().bg(Color::DarkGray).fg(Color::White)
            } else if model.provider == "ollama" {
                Style::default().fg(Color::Green)
            } else {
                Style::default().fg(Color::White)
            };

            for (i, ch) in row.chars().enumerate() {
                let x = inner.x + i as u16;
                if x < inner.right() {
                    buf[(x, y)].set_char(ch).set_style(style);
                }
            }
            y += 1;
        }

        // Footer hint
        if y < inner.bottom() {
            let hint = " ↑↓ navigate  Enter switch model  q close";
            for (i, ch) in hint.chars().enumerate() {
                let x = inner.x + i as u16;
                if x < inner.right() {
                    buf[(x, y)].set_char(ch).set_style(Style::default().fg(Color::DarkGray));
                }
            }
        }
    }
}
