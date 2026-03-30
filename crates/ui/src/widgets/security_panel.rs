#![allow(dead_code, unused_imports, unused_variables)]
//! Security scanner panel — Phase 12 Point 43.
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::Widget,
};
use regex::Regex;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Info,
    Low,
    Medium,
    High,
    Critical,
}

impl Severity {
    pub fn label(&self) -> &'static str {
        match self {
            Severity::Info => "INFO",
            Severity::Low => "LOW",
            Severity::Medium => "MEDIUM",
            Severity::High => "HIGH",
            Severity::Critical => "CRITICAL",
        }
    }

    pub fn color(&self) -> Color {
        match self {
            Severity::Info => Color::Gray,
            Severity::Low => Color::Blue,
            Severity::Medium => Color::Yellow,
            Severity::High => Color::Red,
            Severity::Critical => Color::LightRed,
        }
    }
}

#[derive(Clone, Debug)]
pub struct SecurityFinding {
    pub severity: Severity,
    pub kind: String,
    pub file: Option<String>,
    pub line: Option<usize>,
    pub description: String,
    pub fix_available: bool,
}

pub struct SecurityPanelState {
    pub open: bool,
    pub findings: Vec<SecurityFinding>,
    pub selected: usize,
    pub scanning: bool,
    pub filter_severity: Option<Severity>,
    pub scan_log: Vec<String>,
}

impl SecurityPanelState {
    pub fn new() -> Self {
        Self {
            open: false,
            findings: Vec::new(),
            selected: 0,
            scanning: false,
            filter_severity: None,
            scan_log: Vec::new(),
        }
    }

    pub fn filtered_findings(&self) -> Vec<&SecurityFinding> {
        self.findings
            .iter()
            .filter(|f| match self.filter_severity {
                Some(sev) => f.severity >= sev,
                None => true,
            })
            .collect()
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    pub fn move_down(&mut self) {
        let max = self.filtered_findings().len().saturating_sub(1);
        if self.selected < max {
            self.selected += 1;
        }
    }

    /// Scan files for secret patterns.
    pub fn scan_secrets(files: &[(&str, &str)]) -> Vec<SecurityFinding> {
        let patterns: &[(&str, &str, Severity)] = &[
            // AWS
            (r"AKIA[0-9A-Z]{16}", "AWS Access Key ID", Severity::Critical),
            (r"(?i)aws_secret_access_key\s*=\s*[A-Za-z0-9/+=]{40}", "AWS Secret Access Key", Severity::Critical),
            // GitHub
            (r"ghp_[A-Za-z0-9]{36}", "GitHub Personal Access Token", Severity::Critical),
            (r"ghs_[A-Za-z0-9]{36}", "GitHub App Secret", Severity::Critical),
            (r"github_pat_[A-Za-z0-9_]{82}", "GitHub Fine-Grained PAT", Severity::Critical),
            // Google
            (r"AIza[0-9A-Za-z\-_]{35}", "Google API Key", Severity::High),
            // Generic API key patterns
            (r#"(?i)api[_-]?key\s*[:=]\s*["']?[A-Za-z0-9_\-]{20,}["']?"#, "Generic API Key", Severity::High),
            (r#"(?i)secret[_-]?key\s*[:=]\s*["']?[A-Za-z0-9_\-]{20,}["']?"#, "Generic Secret Key", Severity::High),
            (r#"(?i)password\s*[:=]\s*["']?[^\s"']{8,}["']?"#, "Hardcoded Password", Severity::Medium),
            (r#"(?i)private[_-]?key\s*[:=]\s*["']?[A-Za-z0-9_/+=]{20,}["']?"#, "Private Key Material", Severity::Critical),
            // JWT
            (r"eyJ[A-Za-z0-9\-_=]+\.[A-Za-z0-9\-_=]+\.[A-Za-z0-9\-_=]+", "JWT Token", Severity::Medium),
            // Stripe
            (r"sk_live_[0-9a-zA-Z]{24}", "Stripe Live Secret Key", Severity::Critical),
            (r"pk_live_[0-9a-zA-Z]{24}", "Stripe Live Publishable Key", Severity::Low),
            // Generic bearer token
            (r#"(?i)bearer\s+[A-Za-z0-9\-_\.=]{20,}"#, "Bearer Token", Severity::Medium),
            // Slack
            (r"xox[baprs]-[0-9A-Za-z\-]{10,}", "Slack Token", Severity::High),
        ];

        let mut findings = Vec::new();

        for (path, content) in files {
            for (pattern_str, kind, severity) in patterns {
                if let Ok(re) = Regex::new(pattern_str) {
                    for (line_num, line) in content.lines().enumerate() {
                        if re.is_match(line) {
                            findings.push(SecurityFinding {
                                severity: *severity,
                                kind: format!("Secret Exposed: {}", kind),
                                file: Some(path.to_string()),
                                line: Some(line_num + 1),
                                description: format!(
                                    "{} detected in {}:{}",
                                    kind,
                                    path,
                                    line_num + 1
                                ),
                                fix_available: false,
                            });
                        }
                    }
                }
            }
        }

        // Sort by severity descending
        findings.sort_by(|a, b| b.severity.cmp(&a.severity));
        findings
    }
}

impl Default for SecurityPanelState {
    fn default() -> Self {
        Self::new()
    }
}

pub struct SecurityPanelWidget<'a> {
    pub state: &'a SecurityPanelState,
}

impl Widget for SecurityPanelWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 10 || area.height < 8 {
            return;
        }

        let w = (area.width * 4 / 5).max(70).min(area.width);
        let h = (area.height * 4 / 5).max(20).min(area.height);
        let x = area.x + (area.width.saturating_sub(w)) / 2;
        let y = area.y + (area.height.saturating_sub(h)) / 2;

        let bg = Style::default().bg(Color::Rgb(16, 14, 20));
        let border_style = Style::default().fg(Color::Red);
        let title_style = Style::default().fg(Color::LightRed).add_modifier(Modifier::BOLD);
        let label_style = Style::default().fg(Color::DarkGray);
        let selected_style = Style::default().bg(Color::Rgb(45, 20, 20)).fg(Color::White);
        let normal_style = Style::default().fg(Color::Gray);

        // Background fill
        for row in y..y + h {
            for col in x..x + w {
                buf.get_mut(col, row).set_char(' ').set_style(bg);
            }
        }

        // Border
        buf.get_mut(x, y).set_char('╔').set_style(border_style);
        buf.get_mut(x + w - 1, y).set_char('╗').set_style(border_style);
        for col in x + 1..x + w - 1 {
            buf.get_mut(col, y).set_char('═').set_style(border_style);
        }
        buf.get_mut(x, y + h - 1).set_char('╚').set_style(border_style);
        buf.get_mut(x + w - 1, y + h - 1).set_char('╝').set_style(border_style);
        for col in x + 1..x + w - 1 {
            buf.get_mut(col, y + h - 1).set_char('═').set_style(border_style);
        }
        for row in y + 1..y + h - 1 {
            buf.get_mut(x, row).set_char('║').set_style(border_style);
            buf.get_mut(x + w - 1, row).set_char('║').set_style(border_style);
        }

        // Title
        let title = " Security Scanner ";
        let title_x = x + (w.saturating_sub(title.len() as u16)) / 2;
        for (i, ch) in title.chars().enumerate() {
            if title_x + i as u16 >= x + w - 1 { break; }
            buf.get_mut(title_x + i as u16, y).set_char(ch).set_style(title_style);
        }

        // Summary counts by severity (row y+1)
        let findings = self.state.filtered_findings();
        let counts = [
            (Severity::Critical, "CRIT"),
            (Severity::High, "HIGH"),
            (Severity::Medium, "MED"),
            (Severity::Low, "LOW"),
            (Severity::Info, "INFO"),
        ];
        let mut sx = x + 2;
        for (sev, label) in &counts {
            let count = self.state.findings.iter().filter(|f| f.severity == *sev).count();
            let badge = format!(" {}: {} ", label, count);
            let badge_style = Style::default().fg(sev.color()).add_modifier(Modifier::BOLD);
            for ch in badge.chars() {
                if sx >= x + w - 1 { break; }
                buf.get_mut(sx, y + 1).set_char(ch).set_style(badge_style);
                sx += 1;
            }
            sx += 1;
        }

        // Scanning indicator
        if self.state.scanning {
            let scanning = " ⠙ Scanning...";
            for (i, ch) in scanning.chars().enumerate() {
                let col = x + 1 + i as u16;
                if col >= x + w - 1 { break; }
                buf.get_mut(col, y + 1)
                    .set_char(ch)
                    .set_style(Style::default().fg(Color::Yellow));
            }
        }

        // Separator
        for col in x + 1..x + w - 1 {
            buf.get_mut(col, y + 2).set_char('─').set_style(border_style);
        }
        buf.get_mut(x, y + 2).set_char('╟').set_style(border_style);
        buf.get_mut(x + w - 1, y + 2).set_char('╢').set_style(border_style);

        // Column headers
        let headers = format!(" {:<10} {:<14} {:<20} {:<5} {}", "SEVERITY", "KIND", "FILE", "LINE", "DESCRIPTION");
        for (i, ch) in headers.chars().take((w as usize).saturating_sub(2)).enumerate() {
            let col = x + 1 + i as u16;
            if col >= x + w - 1 { break; }
            buf.get_mut(col, y + 3).set_char(ch).set_style(label_style);
        }

        // Findings list
        let list_start = y + 4;
        let log_height = if !self.state.scan_log.is_empty() { 4u16 } else { 0 };
        let list_height = (h as usize).saturating_sub(7 + log_height as usize);
        let scroll = if self.state.selected >= list_height {
            self.state.selected - list_height + 1
        } else {
            0
        };

        for (idx, finding) in findings.iter().enumerate().skip(scroll).take(list_height) {
            let row_y = list_start + (idx - scroll) as u16;
            if row_y >= y + h - 1 - log_height { break; }
            let is_sel = idx == self.state.selected;
            let row_base = if is_sel { selected_style } else { normal_style };
            let row_bg = if is_sel { Color::Rgb(45,20,20) } else { Color::Reset };

            for col in x + 1..x + w - 1 {
                buf.get_mut(col, row_y).set_char(' ').set_style(row_base);
            }

            // Severity badge
            let sev_label = format!(" [{:<8}]", finding.severity.label());
            let sev_style = Style::default().fg(finding.severity.color()).bg(row_bg);
            for (i, ch) in sev_label.chars().enumerate() {
                let col = x + 1 + i as u16;
                if col >= x + w - 1 { break; }
                buf.get_mut(col, row_y).set_char(ch).set_style(sev_style);
            }

            // Kind
            let kind_short: String = finding.kind.chars().take(13).collect();
            for (i, ch) in kind_short.chars().enumerate() {
                let col = x + 11 + i as u16;
                if col >= x + w - 1 { break; }
                buf.get_mut(col, row_y).set_char(ch).set_style(
                    Style::default().fg(Color::LightMagenta).bg(row_bg)
                );
            }

            // File
            let file_str: String = finding.file.as_deref()
                .unwrap_or("-")
                .chars()
                .rev()
                .take(19)
                .collect::<String>()
                .chars()
                .rev()
                .collect();
            for (i, ch) in file_str.chars().enumerate() {
                let col = x + 25 + i as u16;
                if col >= x + w - 1 { break; }
                buf.get_mut(col, row_y).set_char(ch).set_style(
                    Style::default().fg(Color::Green).bg(row_bg)
                );
            }

            // Line
            let line_str = finding.line.map(|l| l.to_string()).unwrap_or_else(|| "-".to_string());
            for (i, ch) in line_str.chars().take(5).enumerate() {
                let col = x + 45 + i as u16;
                if col >= x + w - 1 { break; }
                buf.get_mut(col, row_y).set_char(ch).set_style(
                    Style::default().fg(Color::DarkGray).bg(row_bg)
                );
            }

            // Description
            let desc_start = 51u16;
            let desc: String = finding.description.chars().take((w as usize).saturating_sub(53)).collect();
            for (i, ch) in desc.chars().enumerate() {
                let col = x + desc_start + i as u16;
                if col >= x + w - 1 { break; }
                buf.get_mut(col, row_y).set_char(ch).set_style(
                    Style::default().fg(Color::White).bg(row_bg)
                );
            }

            // Fix badge
            if finding.fix_available {
                let fix = "[FIX]";
                for (i, ch) in fix.chars().enumerate() {
                    let col = x + w - 1 - 6 + i as u16;
                    if col >= x + w - 1 { break; }
                    buf.get_mut(col, row_y).set_char(ch).set_style(
                        Style::default().fg(Color::Green).bg(row_bg)
                    );
                }
            }
        }

        // Scan log at bottom
        if !self.state.scan_log.is_empty() {
            let log_start_y = y + h - 1 - log_height;
            for col in x + 1..x + w - 1 {
                buf.get_mut(col, log_start_y).set_char('─').set_style(border_style);
            }
            buf.get_mut(x, log_start_y).set_char('╟').set_style(border_style);
            buf.get_mut(x + w - 1, log_start_y).set_char('╢').set_style(border_style);

            let log_label = " Scan Log:";
            for (i, ch) in log_label.chars().enumerate() {
                let col = x + 1 + i as u16;
                if col >= x + w - 1 { break; }
                buf.get_mut(col, log_start_y).set_char(ch).set_style(label_style);
            }

            for (li, entry) in self.state.scan_log.iter().rev().take(log_height as usize - 1).enumerate() {
                let ly = log_start_y + 1 + li as u16;
                if ly >= y + h - 1 { break; }
                let display: String = entry.chars().take((w as usize).saturating_sub(4)).collect();
                for (i, ch) in display.chars().enumerate() {
                    let col = x + 2 + i as u16;
                    if col >= x + w - 1 { break; }
                    buf.get_mut(col, ly).set_char(ch).set_style(
                        Style::default().fg(Color::DarkGray)
                    );
                }
            }
        }

        // Hint bar
        let hint_y = y + h - 2;
        let no_findings = findings.is_empty();
        if no_findings && hint_y > y + 4 {
            let empty_msg = " No findings. Run :SecScan to start scanning.";
            for (i, ch) in empty_msg.chars().enumerate() {
                let col = x + 1 + i as u16;
                if col >= x + w - 1 { break; }
                buf.get_mut(col, y + 5).set_char(ch).set_style(label_style);
            }
        }
        let hint = " [↑↓] navigate  [f] filter severity  [Enter] goto  [s] scan  [Esc] close";
        for (i, ch) in hint.chars().enumerate() {
            let col = x + 1 + i as u16;
            if col >= x + w - 1 { break; }
            buf.get_mut(col, y + h - 2).set_char(ch).set_style(label_style);
        }
    }
}
