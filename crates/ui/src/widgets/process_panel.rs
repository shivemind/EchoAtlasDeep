#![allow(dead_code, unused_imports, unused_variables)]
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::Widget,
};

pub struct ProcessPanelState {
    pub open: bool,
    pub selected: usize,
}

impl ProcessPanelState {
    pub fn new() -> Self {
        Self {
            open: false,
            selected: 0,
        }
    }
}

pub struct ProcessPanelWidget<'a> {
    pub state: &'a ProcessPanelState,
    pub processes: &'a [runner::ManagedProcess],
}

impl<'a> Widget for ProcessPanelWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        use runner::ProcessStatus;

        // Background
        let bg_style = Style::default().bg(Color::Rgb(12, 15, 20));
        for y in area.y..area.y + area.height {
            for x in area.x..area.x + area.width {
                buf.get_mut(x, y).set_char(' ').set_style(bg_style);
            }
        }

        if area.height < 3 {
            return;
        }

        // Border
        let border_style = Style::default().fg(Color::Green);
        buf.get_mut(area.x, area.y).set_char('┌').set_style(border_style);
        buf.get_mut(area.x + area.width - 1, area.y).set_char('┐').set_style(border_style);
        buf.get_mut(area.x, area.y + area.height - 1).set_char('└').set_style(border_style);
        buf.get_mut(area.x + area.width - 1, area.y + area.height - 1).set_char('┘').set_style(border_style);
        for x in (area.x + 1)..(area.x + area.width - 1) {
            buf.get_mut(x, area.y).set_char('─').set_style(border_style);
            buf.get_mut(x, area.y + area.height - 1).set_char('─').set_style(border_style);
        }
        for y in (area.y + 1)..(area.y + area.height - 1) {
            buf.get_mut(area.x, y).set_char('│').set_style(border_style);
            buf.get_mut(area.x + area.width - 1, y).set_char('│').set_style(border_style);
        }

        // Title
        let title = " Process Manager ";
        for (i, ch) in title.chars().enumerate() {
            if area.x + 1 + i as u16 >= area.x + area.width - 1 {
                break;
            }
            buf.get_mut(area.x + 1 + i as u16, area.y)
                .set_char(ch)
                .set_style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD));
        }

        let inner_x = area.x + 1;
        let inner_w = area.width.saturating_sub(2);

        // Header
        let header_y = area.y + 1;
        if header_y < area.y + area.height - 1 {
            let header = format!(
                "{:<6} {:<20} {:<12} {:<6} {:<10} {}",
                "ID", "Name", "Status", "Port", "Uptime", "Command"
            );
            let h_style = Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD);
            for (i, ch) in header.chars().enumerate() {
                let x = inner_x + i as u16;
                if x >= inner_x + inner_w {
                    break;
                }
                buf.get_mut(x, header_y).set_char(ch).set_style(h_style);
            }
        }

        // Separator
        let sep_y = area.y + 2;
        if sep_y < area.y + area.height - 1 {
            for x in inner_x..(inner_x + inner_w) {
                buf.get_mut(x, sep_y)
                    .set_char('─')
                    .set_style(Style::default().fg(Color::DarkGray));
            }
        }

        // Rows
        let row_start = area.y + 3;
        for (i, proc) in self.processes.iter().enumerate() {
            let row_y = row_start + i as u16;
            if row_y >= area.y + area.height - 2 {
                break;
            }

            let is_selected = i == self.state.selected;
            let bg = if is_selected {
                Color::Rgb(20, 30, 20)
            } else {
                Color::Reset
            };

            let status_color = match proc.status {
                ProcessStatus::Running => Color::Green,
                ProcessStatus::Starting => Color::Yellow,
                ProcessStatus::Stopped => Color::DarkGray,
                ProcessStatus::Crashed => Color::Red,
                ProcessStatus::Restarting => Color::Magenta,
            };

            let port_str = proc.port.map(|p| p.to_string()).unwrap_or_else(|| "-".to_string());
            let pid_str = proc.pid.map(|p| p.to_string()).unwrap_or_else(|| "-".to_string());
            let cmd_short: String = proc.command.chars().take(30).collect();

            let row = format!(
                "{:<6} {:<20} {:<12} {:<6} {:<10} {}",
                pid_str,
                proc.name.chars().take(20).collect::<String>(),
                proc.status.label(),
                port_str,
                proc.uptime_str(),
                cmd_short,
            );

            let base_style = Style::default().bg(bg);
            for (j, ch) in row.chars().enumerate() {
                let x = inner_x + j as u16;
                if x >= inner_x + inner_w {
                    break;
                }
                // Color the status field
                let style = if j >= 27 && j < 39 {
                    base_style.fg(status_color)
                } else {
                    base_style.fg(Color::White)
                };
                buf.get_mut(x, row_y).set_char(ch).set_style(style);
            }
        }

        // Empty message
        if self.processes.is_empty() {
            let msg = " No processes registered ";
            let msg_y = row_start;
            if msg_y < area.y + area.height - 1 {
                for (i, ch) in msg.chars().enumerate() {
                    let x = inner_x + i as u16;
                    if x >= inner_x + inner_w {
                        break;
                    }
                    buf.get_mut(x, msg_y)
                        .set_char(ch)
                        .set_style(Style::default().fg(Color::DarkGray));
                }
            }
        }

        // Key hints
        let hints = " [k]=Kill  [r]=Restart  [d]=Details  [q]=Close ";
        let hints_y = area.y + area.height - 1;
        let hints_style = Style::default().fg(Color::DarkGray);
        for (i, ch) in hints.chars().enumerate() {
            let x = inner_x + i as u16;
            if x >= inner_x + inner_w {
                break;
            }
            buf.get_mut(x, hints_y).set_char(ch).set_style(hints_style);
        }
    }
}
