#![allow(dead_code, unused_imports, unused_variables)]
//! Workspace analytics panel — Phase 12 Point 44.
use std::path::Path;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::Widget,
};

#[derive(Clone, Debug)]
pub struct LangStat {
    pub language: String,
    pub loc: usize,
    pub files: usize,
}

#[derive(Clone, Debug)]
pub struct ChurnEntry {
    pub file: String,
    pub changes: usize,
    pub last_changed: u64,
}

#[derive(Clone, Debug)]
pub struct CoverageEntry {
    pub file: String,
    pub covered: usize,
    pub total: usize,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum AnalyticsTab {
    CodeHealth,
    Churn,
    Coverage,
    Velocity,
}

pub struct AnalyticsPanelState {
    pub open: bool,
    pub tab: AnalyticsTab,
    pub lang_stats: Vec<LangStat>,
    pub churn: Vec<ChurnEntry>,
    pub coverage: Vec<CoverageEntry>,
    pub commits_per_day: Vec<(String, u32)>,
    pub loading: bool,
}

impl AnalyticsPanelState {
    pub fn new() -> Self {
        Self {
            open: false,
            tab: AnalyticsTab::CodeHealth,
            lang_stats: Vec::new(),
            churn: Vec::new(),
            coverage: Vec::new(),
            commits_per_day: Vec::new(),
            loading: false,
        }
    }

    /// Count LOC by language by scanning workspace files.
    pub fn compute_lang_stats(workspace_root: &Path) -> Vec<LangStat> {
        use std::collections::HashMap;
        let mut map: HashMap<String, (usize, usize)> = HashMap::new();

        let walker = walkdir::WalkDir::new(workspace_root)
            .max_depth(6)
            .into_iter()
            .filter_entry(|e| {
                let name = e.file_name().to_string_lossy();
                !name.starts_with('.') && name != "target" && name != "node_modules"
            });

        for entry in walker.flatten() {
            if entry.file_type().is_file() {
                let ext = entry.path()
                    .extension()
                    .map(|e| e.to_string_lossy().to_lowercase())
                    .unwrap_or_default();
                let lang = match ext.as_str() {
                    "rs" => "Rust",
                    "py" => "Python",
                    "js" | "mjs" => "JavaScript",
                    "ts" | "tsx" => "TypeScript",
                    "go" => "Go",
                    "c" | "h" => "C",
                    "cpp" | "cc" | "cxx" | "hpp" => "C++",
                    "java" => "Java",
                    "lua" => "Lua",
                    "toml" => "TOML",
                    "json" => "JSON",
                    "md" => "Markdown",
                    "yaml" | "yml" => "YAML",
                    "sh" | "bash" => "Shell",
                    _ => continue,
                };
                if let Ok(content) = std::fs::read_to_string(entry.path()) {
                    let loc = content.lines().count();
                    let entry_stat = map.entry(lang.to_string()).or_insert((0, 0));
                    entry_stat.0 += loc;
                    entry_stat.1 += 1;
                }
            }
        }

        let mut stats: Vec<LangStat> = map.into_iter()
            .map(|(language, (loc, files))| LangStat { language, loc, files })
            .collect();
        stats.sort_by(|a, b| b.loc.cmp(&a.loc));
        stats
    }

    /// Parse lcov.info if present.
    pub fn parse_lcov(path: &Path) -> Vec<CoverageEntry> {
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => return Vec::new(),
        };

        let mut entries = Vec::new();
        let mut current_file: Option<String> = None;
        let mut covered = 0usize;
        let mut total = 0usize;

        for line in content.lines() {
            if let Some(sf) = line.strip_prefix("SF:") {
                if let Some(file) = current_file.take() {
                    if total > 0 {
                        entries.push(CoverageEntry { file, covered, total });
                    }
                }
                current_file = Some(sf.to_string());
                covered = 0;
                total = 0;
            } else if line.starts_with("DA:") {
                let parts: Vec<&str> = line[3..].split(',').collect();
                if parts.len() >= 2 {
                    total += 1;
                    if parts[1].trim() != "0" {
                        covered += 1;
                    }
                }
            } else if line == "end_of_record" {
                if let Some(file) = current_file.take() {
                    if total > 0 {
                        entries.push(CoverageEntry { file, covered, total });
                    }
                }
            }
        }

        entries.sort_by(|a, b| {
            let pa = if a.total > 0 { a.covered * 100 / a.total } else { 0 };
            let pb = if b.total > 0 { b.covered * 100 / b.total } else { 0 };
            pa.cmp(&pb)
        });
        entries
    }
}

impl Default for AnalyticsPanelState {
    fn default() -> Self {
        Self::new()
    }
}

pub struct AnalyticsPanelWidget<'a> {
    pub state: &'a AnalyticsPanelState,
}

fn render_bar(buf: &mut Buffer, x: u16, y: u16, max_width: u16, value: usize, max_value: usize, color: Color) {
    if max_value == 0 { return; }
    let bar_len = ((value as u64 * max_width as u64) / max_value as u64).min(max_width as u64) as u16;
    for i in 0..max_width {
        let ch = if i < bar_len { '█' } else { '░' };
        let style = if i < bar_len {
            Style::default().fg(color)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        buf.get_mut(x + i, y).set_char(ch).set_style(style);
    }
}

impl Widget for AnalyticsPanelWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 10 || area.height < 8 {
            return;
        }

        let w = (area.width * 9 / 10).max(80).min(area.width);
        let h = (area.height * 9 / 10).max(24).min(area.height);
        let x = area.x + (area.width.saturating_sub(w)) / 2;
        let y = area.y + (area.height.saturating_sub(h)) / 2;

        let bg = Style::default().bg(Color::Rgb(12, 14, 20));
        let border_style = Style::default().fg(Color::Blue);
        let title_style = Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD);
        let label_style = Style::default().fg(Color::DarkGray);

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
        let title = " Workspace Analytics ";
        let title_x = x + (w.saturating_sub(title.len() as u16)) / 2;
        for (i, ch) in title.chars().enumerate() {
            if title_x + i as u16 >= x + w - 1 { break; }
            buf.get_mut(title_x + i as u16, y).set_char(ch).set_style(title_style);
        }

        // Tab bar (row y+1)
        let tabs = [
            (AnalyticsTab::CodeHealth, " Code Health "),
            (AnalyticsTab::Churn, " Churn "),
            (AnalyticsTab::Coverage, " Coverage "),
            (AnalyticsTab::Velocity, " Velocity "),
        ];
        let mut tx = x + 2;
        for (tab, label) in &tabs {
            let is_active = *tab == self.state.tab;
            let tab_style = if is_active {
                Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            for ch in label.chars() {
                if tx >= x + w - 1 { break; }
                buf.get_mut(tx, y + 1).set_char(ch).set_style(tab_style);
                tx += 1;
            }
            tx += 1;
        }

        // Loading
        if self.state.loading {
            let loading = " ⠙ Loading analytics...";
            for (i, ch) in loading.chars().enumerate() {
                let col = x + 1 + i as u16;
                if col >= x + w - 1 { break; }
                buf.get_mut(col, y + 3).set_char(ch).set_style(Style::default().fg(Color::Yellow));
            }
            return;
        }

        // Separator
        for col in x + 1..x + w - 1 {
            buf.get_mut(col, y + 2).set_char('─').set_style(border_style);
        }
        buf.get_mut(x, y + 2).set_char('╟').set_style(border_style);
        buf.get_mut(x + w - 1, y + 2).set_char('╢').set_style(border_style);

        let content_y = y + 3;
        let content_h = (h as usize).saturating_sub(5);
        let bar_max_w = (w as usize / 2).min(40) as u16;

        match self.state.tab {
            AnalyticsTab::CodeHealth => {
                // Table + bar chart
                let header = format!(" {:<20} {:>10} {:>8}   {}", "LANGUAGE", "LOC", "FILES", "BAR");
                for (i, ch) in header.chars().take((w as usize).saturating_sub(2)).enumerate() {
                    let col = x + 1 + i as u16;
                    if col >= x + w - 1 { break; }
                    buf.get_mut(col, content_y).set_char(ch).set_style(label_style);
                }

                let max_loc = self.state.lang_stats.iter().map(|s| s.loc).max().unwrap_or(1);
                let colors = [Color::Cyan, Color::Green, Color::Yellow, Color::Magenta, Color::Blue,
                              Color::Red, Color::LightCyan, Color::LightGreen];

                for (idx, stat) in self.state.lang_stats.iter().enumerate().take(content_h.saturating_sub(2)) {
                    let ry = content_y + 1 + idx as u16;
                    if ry >= y + h - 2 { break; }

                    let color = colors[idx % colors.len()];
                    let row = format!(" {:<20} {:>10} {:>8}   ", stat.language, stat.loc, stat.files);
                    for (i, ch) in row.chars().take((w as usize / 2).saturating_sub(2)).enumerate() {
                        let col = x + 1 + i as u16;
                        if col >= x + w - 1 { break; }
                        buf.get_mut(col, ry).set_char(ch).set_style(Style::default().fg(Color::White));
                    }
                    let bar_x = x + 1 + 44;
                    if bar_x + bar_max_w < x + w - 1 {
                        render_bar(buf, bar_x, ry, bar_max_w.min(x + w - 2 - bar_x), stat.loc, max_loc, color);
                    }
                }

                // Total LOC summary
                let total_loc: usize = self.state.lang_stats.iter().map(|s| s.loc).sum();
                let total_files: usize = self.state.lang_stats.iter().map(|s| s.files).sum();
                let summary = format!(" Total: {} LOC across {} files", total_loc, total_files);
                let sy = content_y + content_h as u16 - 1;
                if sy < y + h - 1 {
                    for (i, ch) in summary.chars().enumerate() {
                        let col = x + 1 + i as u16;
                        if col >= x + w - 1 { break; }
                        buf.get_mut(col, sy).set_char(ch).set_style(
                            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
                        );
                    }
                }
            }
            AnalyticsTab::Churn => {
                let header = format!(" {:<40} {:>10}  CHANGES BAR", "FILE", "CHANGES");
                for (i, ch) in header.chars().take((w as usize).saturating_sub(2)).enumerate() {
                    let col = x + 1 + i as u16;
                    if col >= x + w - 1 { break; }
                    buf.get_mut(col, content_y).set_char(ch).set_style(label_style);
                }

                let max_changes = self.state.churn.iter().map(|c| c.changes).max().unwrap_or(1);

                for (idx, entry) in self.state.churn.iter().enumerate().take(content_h.saturating_sub(2)) {
                    let ry = content_y + 1 + idx as u16;
                    if ry >= y + h - 2 { break; }

                    let file_short: String = entry.file.chars().rev().take(39).collect::<String>().chars().rev().collect();
                    let row = format!(" {:<40} {:>10}  ", file_short, entry.changes);
                    for (i, ch) in row.chars().take(54).enumerate() {
                        let col = x + 1 + i as u16;
                        if col >= x + w - 1 { break; }
                        buf.get_mut(col, ry).set_char(ch).set_style(Style::default().fg(Color::White));
                    }

                    let bar_x = x + 55;
                    let bar_color = if entry.changes > max_changes * 3 / 4 {
                        Color::Red
                    } else if entry.changes > max_changes / 2 {
                        Color::Yellow
                    } else {
                        Color::Green
                    };
                    if bar_x + 20 < x + w - 1 {
                        render_bar(buf, bar_x, ry, (x + w - 2).saturating_sub(bar_x), entry.changes, max_changes, bar_color);
                    }
                }

                if self.state.churn.is_empty() {
                    let msg = " No churn data available. Run :Analytics to refresh.";
                    for (i, ch) in msg.chars().enumerate() {
                        let col = x + 1 + i as u16;
                        if col >= x + w - 1 { break; }
                        buf.get_mut(col, content_y + 1).set_char(ch).set_style(label_style);
                    }
                }
            }
            AnalyticsTab::Coverage => {
                let header = format!(" {:<45} {:>5}  COVERAGE BAR", "FILE", "%");
                for (i, ch) in header.chars().take((w as usize).saturating_sub(2)).enumerate() {
                    let col = x + 1 + i as u16;
                    if col >= x + w - 1 { break; }
                    buf.get_mut(col, content_y).set_char(ch).set_style(label_style);
                }

                for (idx, entry) in self.state.coverage.iter().enumerate().take(content_h.saturating_sub(2)) {
                    let ry = content_y + 1 + idx as u16;
                    if ry >= y + h - 2 { break; }

                    let pct = if entry.total > 0 { entry.covered * 100 / entry.total } else { 0 };
                    let file_short: String = entry.file.chars().rev().take(44).collect::<String>().chars().rev().collect();
                    let row = format!(" {:<45} {:>4}%  ", file_short, pct);
                    for (i, ch) in row.chars().take(54).enumerate() {
                        let col = x + 1 + i as u16;
                        if col >= x + w - 1 { break; }
                        buf.get_mut(col, ry).set_char(ch).set_style(Style::default().fg(Color::White));
                    }

                    let bar_color = if pct >= 80 { Color::Green } else if pct >= 50 { Color::Yellow } else { Color::Red };
                    let bar_x = x + 55;
                    if bar_x + 10 < x + w - 1 {
                        render_bar(buf, bar_x, ry, (x + w - 2).saturating_sub(bar_x), pct, 100, bar_color);
                    }
                }

                if self.state.coverage.is_empty() {
                    let msg = " No lcov.info found. Run tests with coverage to populate.";
                    for (i, ch) in msg.chars().enumerate() {
                        let col = x + 1 + i as u16;
                        if col >= x + w - 1 { break; }
                        buf.get_mut(col, content_y + 1).set_char(ch).set_style(label_style);
                    }
                }
            }
            AnalyticsTab::Velocity => {
                // Sparkline of commits per day
                let spark_label = " Commits per day (last 30 days):";
                for (i, ch) in spark_label.chars().enumerate() {
                    let col = x + 1 + i as u16;
                    if col >= x + w - 1 { break; }
                    buf.get_mut(col, content_y).set_char(ch).set_style(label_style);
                }

                let spark_blocks = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
                let max_commits = self.state.commits_per_day.iter().map(|(_, c)| *c).max().unwrap_or(1).max(1);
                let spark_y = content_y + 2;

                for (idx, (date, count)) in self.state.commits_per_day.iter().enumerate() {
                    let col = x + 2 + idx as u16;
                    if col >= x + w - 2 { break; }
                    let level = (*count as usize * 7 / max_commits as usize).min(7);
                    let spark_ch = spark_blocks[level];
                    let spark_color = if level >= 6 { Color::Green } else if level >= 4 { Color::Yellow } else { Color::DarkGray };
                    buf.get_mut(col, spark_y).set_char(spark_ch).set_style(Style::default().fg(spark_color));
                }

                // Date labels (every 7 days)
                for (idx, (date, _)) in self.state.commits_per_day.iter().enumerate() {
                    if idx % 7 == 0 {
                        let col = x + 2 + idx as u16;
                        let date_short: String = date.chars().skip(5).take(5).collect();
                        for (i, ch) in date_short.chars().enumerate() {
                            let dc = col + i as u16;
                            if dc >= x + w - 2 { break; }
                            buf.get_mut(dc, spark_y + 1).set_char(ch).set_style(label_style);
                        }
                    }
                }

                // Stats summary
                let total_commits: u32 = self.state.commits_per_day.iter().map(|(_, c)| c).sum();
                let avg = if self.state.commits_per_day.is_empty() {
                    0.0f32
                } else {
                    total_commits as f32 / self.state.commits_per_day.len() as f32
                };
                let summary = format!(" {} total commits  |  avg {:.1}/day  |  max {}/day",
                    total_commits, avg, max_commits);
                let sy = spark_y + 3;
                if sy < y + h - 1 {
                    for (i, ch) in summary.chars().enumerate() {
                        let col = x + 1 + i as u16;
                        if col >= x + w - 1 { break; }
                        buf.get_mut(col, sy).set_char(ch).set_style(
                            Style::default().fg(Color::Cyan)
                        );
                    }
                }

                if self.state.commits_per_day.is_empty() {
                    let msg = " No velocity data. Run :Analytics to refresh.";
                    for (i, ch) in msg.chars().enumerate() {
                        let col = x + 1 + i as u16;
                        if col >= x + w - 1 { break; }
                        buf.get_mut(col, content_y + 1).set_char(ch).set_style(label_style);
                    }
                }
            }
        }

        // Hint bar
        let hint = " [1-4] switch tab  [r] refresh  [Esc] close";
        for (i, ch) in hint.chars().enumerate() {
            let col = x + 1 + i as u16;
            if col >= x + w - 1 { break; }
            buf.get_mut(col, y + h - 2).set_char(ch).set_style(label_style);
        }
    }
}
