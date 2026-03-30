#![allow(dead_code, unused_imports, unused_variables)]
//! Phase 10 — Point 25: DAP debugger panel widget.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::Widget,
};

pub struct DapPanelState {
    pub open: bool,
    pub selected_tab: DapTab,
    pub selected_frame: usize,
    pub selected_var: usize,
    pub console_scroll: usize,
}

#[derive(Clone, Copy, PartialEq)]
pub enum DapTab {
    Variables,
    CallStack,
    Breakpoints,
    Console,
}

impl DapTab {
    fn label(&self) -> &'static str {
        match self {
            DapTab::Variables   => "Variables",
            DapTab::CallStack   => "Call Stack",
            DapTab::Breakpoints => "Breakpoints",
            DapTab::Console     => "Console",
        }
    }
}

impl DapPanelState {
    pub fn new() -> Self {
        Self {
            open: false,
            selected_tab: DapTab::Variables,
            selected_frame: 0,
            selected_var: 0,
            console_scroll: 0,
        }
    }
}

impl Default for DapPanelState {
    fn default() -> Self {
        Self::new()
    }
}

pub struct DapPanelWidget<'a> {
    pub state: &'a DapPanelState,
    pub client: &'a dap::DapClient,
    pub breakpoints: &'a dap::BreakpointManager,
}

impl Widget for DapPanelWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        let bg_style = Style::default().bg(Color::Rgb(20, 20, 30));

        // Fill background
        for y in area.y..area.y + area.height {
            for x in area.x..area.x + area.width {
                buf.get_mut(x, y).set_char(' ').set_style(bg_style);
            }
        }

        // Border
        draw_box(buf, area, Style::default().fg(Color::Red));

        // Title with status
        let status_label = self.client.status.read().label().to_string();
        let title = format!(" DEBUG [{status_label}] ");
        let title_x = area.x + 1;
        for (i, ch) in title.chars().enumerate() {
            if title_x + i as u16 >= area.x + area.width.saturating_sub(1) {
                break;
            }
            buf.get_mut(title_x + i as u16, area.y)
                .set_char(ch)
                .set_style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD));
        }

        let inner = Rect {
            x: area.x + 1,
            y: area.y + 1,
            width: area.width.saturating_sub(2),
            height: area.height.saturating_sub(2),
        };

        if inner.height == 0 {
            return;
        }

        // Tab bar (row 0)
        let tabs = [DapTab::Variables, DapTab::CallStack, DapTab::Breakpoints, DapTab::Console];
        let mut tab_x = inner.x;
        for tab in &tabs {
            let label = format!(" {} ", tab.label());
            let is_active = *tab == self.state.selected_tab;
            let tab_style = if is_active {
                Style::default().fg(Color::Black).bg(Color::Red).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Gray).bg(Color::Rgb(35, 35, 45))
            };
            for (i, ch) in label.chars().enumerate() {
                if tab_x + i as u16 >= inner.x + inner.width {
                    break;
                }
                buf.get_mut(tab_x + i as u16, inner.y)
                    .set_char(ch)
                    .set_style(tab_style);
            }
            tab_x += label.len() as u16;
            if tab_x < inner.x + inner.width {
                buf.get_mut(tab_x, inner.y)
                    .set_char('│')
                    .set_style(Style::default().fg(Color::DarkGray));
                tab_x += 1;
            }
        }

        if inner.height < 2 {
            return;
        }

        // Separator after tabs
        let sep_y = inner.y + 1;
        for x in 0..inner.width {
            buf.get_mut(inner.x + x, sep_y)
                .set_char('─')
                .set_style(Style::default().fg(Color::DarkGray));
        }

        let content_y = sep_y + 1;
        let content_h = (inner.y + inner.height).saturating_sub(content_y) as usize;

        match self.state.selected_tab {
            DapTab::Variables => {
                let vars = self.client.variables.read();
                for (row, var) in vars.iter().enumerate().take(content_h) {
                    let y = content_y + row as u16;
                    let is_sel = row == self.state.selected_var;
                    let row_style = if is_sel {
                        Style::default().fg(Color::White).bg(Color::Rgb(40, 40, 80))
                    } else {
                        Style::default().fg(Color::Gray)
                    };
                    // Clear row
                    for x in 0..inner.width {
                        buf.get_mut(inner.x + x, y).set_char(' ').set_style(row_style);
                    }
                    let type_tag = var.var_type.as_deref().unwrap_or("?");
                    let line = format!("  {} : {} = {}", var.name, type_tag, var.value);
                    for (i, ch) in line.chars().enumerate() {
                        if i as u16 >= inner.width {
                            break;
                        }
                        buf.get_mut(inner.x + i as u16, y).set_char(ch).set_style(row_style);
                    }
                }
                if vars.is_empty() {
                    let msg = "  (no variables — not paused)";
                    for (i, ch) in msg.chars().enumerate() {
                        if i as u16 >= inner.width { break; }
                        buf.get_mut(inner.x + i as u16, content_y)
                            .set_char(ch)
                            .set_style(Style::default().fg(Color::DarkGray));
                    }
                }
            }
            DapTab::CallStack => {
                let frames = self.client.stack_frames.read();
                for (row, frame) in frames.iter().enumerate().take(content_h) {
                    let y = content_y + row as u16;
                    let is_sel = row == self.state.selected_frame;
                    let row_style = if is_sel {
                        Style::default().fg(Color::White).bg(Color::Rgb(40, 40, 80))
                    } else {
                        Style::default().fg(Color::Gray)
                    };
                    for x in 0..inner.width {
                        buf.get_mut(inner.x + x, y).set_char(' ').set_style(row_style);
                    }
                    let file = frame.source.as_ref()
                        .and_then(|s| s.path.as_deref())
                        .unwrap_or("?");
                    let line = format!("  #{} {} {}:{}", frame.id, frame.name, file, frame.line);
                    for (i, ch) in line.chars().enumerate() {
                        if i as u16 >= inner.width { break; }
                        buf.get_mut(inner.x + i as u16, y).set_char(ch).set_style(row_style);
                    }
                }
                if frames.is_empty() {
                    let msg = "  (no stack frames — not paused)";
                    for (i, ch) in msg.chars().enumerate() {
                        if i as u16 >= inner.width { break; }
                        buf.get_mut(inner.x + i as u16, content_y)
                            .set_char(ch)
                            .set_style(Style::default().fg(Color::DarkGray));
                    }
                }
            }
            DapTab::Breakpoints => {
                let bps = self.breakpoints.list();
                for (row, bp) in bps.iter().enumerate().take(content_h) {
                    let y = content_y + row as u16;
                    let row_style = Style::default().fg(if bp.enabled { Color::Red } else { Color::DarkGray });
                    for x in 0..inner.width {
                        buf.get_mut(inner.x + x, y).set_char(' ').set_style(Style::default());
                    }
                    let dot = if bp.enabled { "●" } else { "○" };
                    let cond = bp.condition.as_deref().map(|c| format!(" if {c}")).unwrap_or_default();
                    let line = format!("  {} {}:{}{}", dot, bp.file, bp.line, cond);
                    for (i, ch) in line.chars().enumerate() {
                        if i as u16 >= inner.width { break; }
                        buf.get_mut(inner.x + i as u16, y).set_char(ch).set_style(row_style);
                    }
                }
                if bps.is_empty() {
                    let msg = "  (no breakpoints set)";
                    for (i, ch) in msg.chars().enumerate() {
                        if i as u16 >= inner.width { break; }
                        buf.get_mut(inner.x + i as u16, content_y)
                            .set_char(ch)
                            .set_style(Style::default().fg(Color::DarkGray));
                    }
                }
            }
            DapTab::Console => {
                let log = self.client.output_log.read();
                let total = log.len();
                let scroll = self.state.console_scroll.min(total.saturating_sub(content_h));
                for row in 0..content_h {
                    let log_idx = scroll + row;
                    if log_idx >= total { break; }
                    let y = content_y + row as u16;
                    let line = format!("  {}", log[log_idx]);
                    for x in 0..inner.width {
                        buf.get_mut(inner.x + x, y).set_char(' ').set_style(Style::default());
                    }
                    for (i, ch) in line.chars().enumerate() {
                        if i as u16 >= inner.width { break; }
                        buf.get_mut(inner.x + i as u16, y)
                            .set_char(ch)
                            .set_style(Style::default().fg(Color::Green));
                    }
                }
            }
        }
    }
}

fn draw_box(buf: &mut Buffer, area: Rect, style: Style) {
    buf.get_mut(area.x, area.y).set_char('┌').set_style(style);
    buf.get_mut(area.x + area.width.saturating_sub(1), area.y)
        .set_char('┐')
        .set_style(style);
    let by = area.y + area.height.saturating_sub(1);
    buf.get_mut(area.x, by).set_char('└').set_style(style);
    buf.get_mut(area.x + area.width.saturating_sub(1), by)
        .set_char('┘')
        .set_style(style);
    for x in (area.x + 1)..(area.x + area.width.saturating_sub(1)) {
        buf.get_mut(x, area.y).set_char('─').set_style(style);
        buf.get_mut(x, by).set_char('─').set_style(style);
    }
    for y in (area.y + 1)..by {
        buf.get_mut(area.x, y).set_char('│').set_style(style);
        buf.get_mut(area.x + area.width.saturating_sub(1), y)
            .set_char('│')
            .set_style(style);
    }
}
