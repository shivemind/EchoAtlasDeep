#![allow(dead_code, unused_imports, unused_variables)]
//! Agent task panel widget — shows plan, status, tool trace, sub-agents.
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Widget},
};

use ai::agent::{AgentSession, AgentStatus, ToolCall};

pub struct AgentPanelState {
    pub session: Option<AgentSession>,
    pub selected_step: usize,
    pub show_trace: bool,
    pub trace_scroll: usize,
    pub plan_edit_mode: bool,
    pub plan_cursor: usize,
}

impl AgentPanelState {
    pub fn new() -> Self {
        Self {
            session: None,
            selected_step: 0,
            show_trace: false,
            trace_scroll: 0,
            plan_edit_mode: false,
            plan_cursor: 0,
        }
    }
}

impl Default for AgentPanelState {
    fn default() -> Self {
        Self::new()
    }
}

pub struct AgentPanelWidget<'a> {
    pub state: &'a AgentPanelState,
    pub focused: bool,
}

impl<'a> Widget for AgentPanelWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let border_style = if self.focused {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let block = Block::default()
            .title(" Agent ")
            .borders(Borders::ALL)
            .border_style(border_style);
        let inner = block.inner(area);
        block.render(area, buf);

        if inner.height == 0 || inner.width == 0 {
            return;
        }

        let session = match &self.state.session {
            Some(s) => s,
            None => {
                render_text(buf, inner.x, inner.y, inner.width,
                    "No active agent session. Use \\A to start.",
                    Style::default().fg(Color::DarkGray));
                return;
            }
        };

        let mut y = inner.y;
        let max_y = inner.y + inner.height;

        // ── Task line ────────────────────────────────────────────────────────
        if y < max_y {
            let task_label = format!("Task: {}", session.task);
            render_text(buf, inner.x, y, inner.width, &task_label,
                Style::default().fg(Color::White).add_modifier(Modifier::BOLD));
            y += 1;
        }

        // ── Status line ──────────────────────────────────────────────────────
        if y < max_y {
            let status_color = status_color(&session.status);
            let status_label = format!("Status: [{}]  Step: {}/{}",
                session.status.label(),
                session.current_step.min(session.plan.len()),
                session.plan.len());
            render_text(buf, inner.x, y, inner.width, &status_label,
                Style::default().fg(status_color));
            y += 1;
        }

        // ── Separator ────────────────────────────────────────────────────────
        if y < max_y {
            render_separator(buf, inner.x, y, inner.width);
            y += 1;
        }

        // ── Plan steps ──────────────────────────────────────────────────────
        if y < max_y {
            render_text(buf, inner.x, y, inner.width, "Plan:",
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));
            y += 1;
        }

        if session.plan.is_empty() && y < max_y {
            render_text(buf, inner.x, y, inner.width, "  (generating...)",
                Style::default().fg(Color::DarkGray));
            y += 1;
        }

        for (idx, step) in session.plan.iter().enumerate() {
            if y >= max_y {
                break;
            }
            let is_current = idx == session.current_step && session.status == AgentStatus::Acting;
            let is_done = idx < session.current_step;
            let is_selected = idx == self.state.selected_step;
            let is_editing = self.state.plan_edit_mode && idx == self.state.plan_cursor;

            let icon = if is_done {
                "✓"
            } else if is_current {
                "▶"
            } else {
                "○"
            };

            let style = if is_editing {
                Style::default().fg(Color::Black).bg(Color::Yellow)
            } else if is_selected && self.focused {
                Style::default().fg(Color::Black).bg(Color::Cyan)
            } else if is_done {
                Style::default().fg(Color::DarkGray)
            } else if is_current {
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            let line = format!("  {} {}. {}", icon, idx + 1, step);
            render_text(buf, inner.x, y, inner.width, &line, style);
            y += 1;
        }

        // ── Sub-agents ───────────────────────────────────────────────────────
        if !session.sub_agents.is_empty() {
            if y < max_y {
                render_separator(buf, inner.x, y, inner.width);
                y += 1;
            }
            if y < max_y {
                render_text(buf, inner.x, y, inner.width, "Sub-agents:",
                    Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD));
                y += 1;
            }
            for sa in &session.sub_agents {
                if y >= max_y { break; }
                let pct = (sa.progress * 100.0) as u32;
                let line = format!("  [{}] {} {}%", sa.status.label(), sa.role, pct);
                render_text(buf, inner.x, y, inner.width, &line,
                    Style::default().fg(Color::Cyan));
                y += 1;
            }
        }

        // ── Tool trace summary ───────────────────────────────────────────────
        if self.state.show_trace && !session.tool_trace.is_empty() {
            if y < max_y {
                render_separator(buf, inner.x, y, inner.width);
                y += 1;
            }
            if y < max_y {
                render_text(buf, inner.x, y, inner.width,
                    &format!("Tool Trace ({} calls):", session.tool_trace.len()),
                    Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD));
                y += 1;
            }
            let scroll = self.state.trace_scroll;
            for call in session.tool_trace.iter().skip(scroll) {
                if y >= max_y { break; }
                let result_preview = call.result.as_deref().unwrap_or("").chars().take(30)
                    .collect::<String>();
                let line = format!("  [{}] {} → {}  {}ms",
                    call.tool, truncate(&call.args, 20), truncate(&result_preview, 30),
                    call.duration_ms);
                let style = if call.error {
                    Style::default().fg(Color::Red)
                } else {
                    Style::default().fg(Color::DarkGray)
                };
                render_text(buf, inner.x, y, inner.width, &line, style);
                y += 1;
            }
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

fn render_separator(buf: &mut Buffer, x: u16, y: u16, width: u16) {
    for i in 0..width {
        buf[(x + i, y)].set_char('─').set_style(Style::default().fg(Color::DarkGray));
    }
}

fn status_color(status: &AgentStatus) -> Color {
    match status {
        AgentStatus::Idle             => Color::DarkGray,
        AgentStatus::Planning         => Color::Yellow,
        AgentStatus::Acting           => Color::Green,
        AgentStatus::Verifying        => Color::Blue,
        AgentStatus::WaitingApproval  => Color::Magenta,
        AgentStatus::Complete         => Color::Cyan,
        AgentStatus::Failed           => Color::Red,
        AgentStatus::Interrupted      => Color::Yellow,
    }
}

fn truncate(s: &str, max: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= max {
        s.to_string()
    } else {
        let truncated: String = chars[..max.saturating_sub(3)].iter().collect();
        format!("{truncated}...")
    }
}
