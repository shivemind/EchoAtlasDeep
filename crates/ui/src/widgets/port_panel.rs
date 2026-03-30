#![allow(dead_code, unused_imports, unused_variables)]
use std::time::SystemTime;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::Widget,
};

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[derive(Clone, Debug)]
pub struct PortEntry {
    pub port: u16,
    pub pid: Option<u32>,
    pub process_name: String,
    pub protocol: String,
}

pub struct PortPanelState {
    pub open: bool,
    pub ports: Vec<PortEntry>,
    pub selected: usize,
    pub last_refresh: u64,
}

impl PortPanelState {
    pub fn new() -> Self {
        Self {
            open: false,
            ports: Vec::new(),
            selected: 0,
            last_refresh: 0,
        }
    }

    /// Spawn `netstat` and parse output.
    pub fn refresh(&mut self) {
        self.last_refresh = now_secs();
        self.ports = discover_ports();
    }
}

fn discover_ports() -> Vec<PortEntry> {
    let mut ports = Vec::new();

    #[cfg(target_os = "windows")]
    {
        // Windows: netstat -ano
        let output = std::process::Command::new("netstat")
            .args(&["-ano"])
            .output();
        if let Ok(out) = output {
            let stdout = String::from_utf8_lossy(&out.stdout);
            for line in stdout.lines() {
                let line = line.trim();
                if !line.starts_with("TCP") && !line.starts_with("UDP") {
                    continue;
                }
                // Format: TCP    0.0.0.0:80    0.0.0.0:0    LISTENING    1234
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() < 4 {
                    continue;
                }
                let proto = parts[0];
                let local_addr = parts[1];
                let state = if proto == "TCP" { parts.get(3).copied().unwrap_or("") } else { "UDP" };
                let pid_str = parts.last().copied().unwrap_or("0");

                if state != "LISTENING" && state != "UDP" {
                    continue;
                }

                if let Some(port) = extract_port(local_addr) {
                    let pid: Option<u32> = pid_str.parse().ok();
                    ports.push(PortEntry {
                        port,
                        pid,
                        process_name: pid.map(|p| p.to_string()).unwrap_or_default(),
                        protocol: proto.to_string(),
                    });
                }
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        // Linux: netstat -tlnp (requires root or ss)
        let output = std::process::Command::new("ss")
            .args(&["-tlnp"])
            .output()
            .or_else(|_| {
                std::process::Command::new("netstat")
                    .args(&["-tlnp"])
                    .output()
            });
        if let Ok(out) = output {
            let stdout = String::from_utf8_lossy(&out.stdout);
            for line in stdout.lines().skip(1) {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() < 4 {
                    continue;
                }
                // ss output: Netid  State  Recv-Q  Send-Q  Local-Address:Port  ...
                let local = parts.get(4).copied().unwrap_or("");
                if let Some(port) = extract_port(local) {
                    ports.push(PortEntry {
                        port,
                        pid: None,
                        process_name: parts.last().copied().unwrap_or("").to_string(),
                        protocol: "TCP".to_string(),
                    });
                }
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        let output = std::process::Command::new("netstat")
            .args(&["-an", "-p", "tcp"])
            .output();
        if let Ok(out) = output {
            let stdout = String::from_utf8_lossy(&out.stdout);
            for line in stdout.lines() {
                if !line.starts_with("tcp") {
                    continue;
                }
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() < 6 {
                    continue;
                }
                let state = parts[5];
                if state != "LISTEN" {
                    continue;
                }
                let local = parts[3];
                if let Some(port) = extract_port(local) {
                    ports.push(PortEntry {
                        port,
                        pid: None,
                        process_name: String::new(),
                        protocol: "TCP".to_string(),
                    });
                }
            }
        }
    }

    // Deduplicate by port
    ports.sort_by_key(|p| p.port);
    ports.dedup_by_key(|p| p.port);
    ports
}

fn extract_port(addr: &str) -> Option<u16> {
    // Handle formats: 0.0.0.0:8080, :::8080, [::]:8080, *:8080
    if let Some(colon_pos) = addr.rfind(':') {
        addr[colon_pos + 1..].parse().ok()
    } else {
        None
    }
}

pub struct PortPanelWidget<'a> {
    pub state: &'a PortPanelState,
}

impl<'a> Widget for PortPanelWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Background
        let bg_style = Style::default().bg(Color::Rgb(15, 12, 20));
        for y in area.y..area.y + area.height {
            for x in area.x..area.x + area.width {
                buf.get_mut(x, y).set_char(' ').set_style(bg_style);
            }
        }

        if area.height < 3 {
            return;
        }

        // Border
        let border_style = Style::default().fg(Color::Magenta);
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
        let title = format!(" Port Manager — {} ports ", self.state.ports.len());
        for (i, ch) in title.chars().enumerate() {
            if area.x + 1 + i as u16 >= area.x + area.width - 1 {
                break;
            }
            buf.get_mut(area.x + 1 + i as u16, area.y)
                .set_char(ch)
                .set_style(Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD));
        }

        let inner_x = area.x + 1;
        let inner_w = area.width.saturating_sub(2);

        // Header
        let header_y = area.y + 1;
        if header_y < area.y + area.height - 1 {
            let header = format!("{:<8} {:<8} {:<8} {}", "Port", "Proto", "PID", "Process");
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
        for (i, entry) in self.state.ports.iter().enumerate() {
            let row_y = row_start + i as u16;
            if row_y >= area.y + area.height - 2 {
                break;
            }

            let is_selected = i == self.state.selected;
            let bg = if is_selected { Color::Rgb(25, 15, 35) } else { Color::Reset };

            let pid_str = entry.pid.map(|p| p.to_string()).unwrap_or_else(|| "-".to_string());
            let row = format!(
                "{:<8} {:<8} {:<8} {}",
                entry.port,
                entry.protocol,
                pid_str,
                entry.process_name.chars().take(30).collect::<String>(),
            );

            let style = Style::default().bg(bg).fg(Color::White);
            for (j, ch) in row.chars().enumerate() {
                let x = inner_x + j as u16;
                if x >= inner_x + inner_w {
                    break;
                }
                buf.get_mut(x, row_y).set_char(ch).set_style(style);
            }
        }

        if self.state.ports.is_empty() {
            let msg = " No listening ports found — press [r] to refresh ";
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
        let hints = " [r]=Refresh  [o]=Open in browser  [k]=Kill  [q]=Close ";
        let hints_y = area.y + area.height - 1;
        for (i, ch) in hints.chars().enumerate() {
            let x = inner_x + i as u16;
            if x >= inner_x + inner_w {
                break;
            }
            buf.get_mut(x, hints_y)
                .set_char(ch)
                .set_style(Style::default().fg(Color::DarkGray));
        }
    }
}
