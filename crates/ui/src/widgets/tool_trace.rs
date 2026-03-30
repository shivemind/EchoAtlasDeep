#![allow(dead_code, unused_imports, unused_variables)]
//! Live tool call trace sidebar widget.
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Widget},
};

use ai::agent::ToolCall;

pub struct ToolTraceWidget<'a> {
    pub calls: &'a [ToolCall],
    pub scroll: usize,
    pub selected: usize,
    pub focused: bool,
}

impl<'a> Widget for ToolTraceWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let border_style = if self.focused {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let title = format!(" Tool Trace ({}) ", self.calls.len());
        let block = Block::default()
            .title(title.as_str())
            .borders(Borders::ALL)
            .border_style(border_style);
        let inner = block.inner(area);
        block.render(area, buf);

        if inner.height == 0 || inner.width == 0 || self.calls.is_empty() {
            if inner.height > 0 && inner.width > 0 && self.calls.is_empty() {
                render_text(buf, inner.x, inner.y, inner.width,
                    "No tool calls yet.",
                    Style::default().fg(Color::DarkGray));
            }
            return;
        }

        // Column widths
        let tool_w: usize = 14;
        let cost_w: usize = 10;
        let time_w: usize = 7;
        let remaining = (inner.width as usize).saturating_sub(tool_w + cost_w + time_w + 4);
        let args_w = remaining / 2;
        let result_w = remaining.saturating_sub(args_w);

        // Header
        let header = format!("{:<tool_w$} {:<args_w$} {:<result_w$} {:>time_w$} {:>cost_w$}",
            "Tool", "Args", "Result", "Time", "Cost($)");
        let header_style = Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD);
        render_text(buf, inner.x, inner.y, inner.width, &header, header_style);

        let mut y = inner.y + 1;
        // Separator under header
        if y < inner.bottom() {
            for i in 0..inner.width {
                buf[(inner.x + i, y)].set_char('─')
                    .set_style(Style::default().fg(Color::DarkGray));
            }
            y += 1;
        }

        let visible_rows = (inner.height as usize).saturating_sub(2);
        let scroll = self.scroll.min(self.calls.len().saturating_sub(1));

        for (row_idx, call) in self.calls.iter().enumerate().skip(scroll) {
            if y >= inner.bottom() {
                break;
            }
            let is_selected = row_idx == self.selected && self.focused;
            let base_style = if call.error {
                Style::default().fg(Color::Red)
            } else if is_selected {
                Style::default().fg(Color::Black).bg(Color::Cyan)
            } else {
                Style::default().fg(Color::White)
            };

            let args_preview = truncate(&call.args, args_w);
            let result_preview = match &call.result {
                Some(r) => truncate(r, result_w),
                None => "—".to_string(),
            };
            let time_str = format!("{}ms", call.duration_ms);
            let cost_str = format!("${:.5}", call.cost_usd);

            let row = format!("{:<tool_w$} {:<args_w$} {:<result_w$} {:>time_w$} {:>cost_w$}",
                truncate(&call.tool, tool_w),
                args_preview,
                result_preview,
                truncate(&time_str, time_w),
                truncate(&cost_str, cost_w),
            );

            render_text(buf, inner.x, y, inner.width, &row, base_style);
            y += 1;
        }

        // Scroll indicator if needed
        if self.calls.len() > visible_rows && y < inner.bottom() {
            let indicator = format!("  ↑↓ scroll ({}/{})", scroll + 1, self.calls.len());
            render_text(buf, inner.x, inner.bottom() - 1, inner.width, &indicator,
                Style::default().fg(Color::DarkGray));
        }
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn render_text(buf: &mut Buffer, x: u16, y: u16, width: u16, text: &str, style: Style) {
    let chars: Vec<char> = text.chars().collect();
    for (i, &ch) in chars.iter().enumerate() {
        let cx = x + i as u16;
        if cx >= x + width { break; }
        buf[(cx, y)].set_char(ch).set_style(style);
    }
}

fn truncate(s: &str, max: usize) -> String {
    if max == 0 { return String::new(); }
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= max {
        // Pad with spaces to fill column width
        let mut out = s.to_string();
        while out.chars().count() < max {
            out.push(' ');
        }
        out
    } else {
        let truncated: String = chars[..max.saturating_sub(3)].iter().collect();
        format!("{truncated}...")
    }
}
