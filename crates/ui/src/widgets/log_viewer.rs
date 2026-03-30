#![allow(dead_code, unused_imports, unused_variables)]
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::Widget,
};

pub struct LogViewerState {
    pub open: bool,
    pub entries: Vec<runner::LogEntry>,
    pub scroll: usize,
    pub follow: bool,
    pub filter_level: Option<runner::LogLevel>,
    pub filter_source: Option<String>,
    pub filter_regex: Option<String>,
    pub max_entries: usize, // 50_000
}

impl LogViewerState {
    pub fn new() -> Self {
        Self {
            open: false,
            entries: Vec::new(),
            scroll: 0,
            follow: true,
            filter_level: None,
            filter_source: None,
            filter_regex: None,
            max_entries: 50_000,
        }
    }

    pub fn push(&mut self, entry: runner::LogEntry) {
        self.entries.push(entry);
        if self.entries.len() > self.max_entries {
            let drain = self.entries.len() - self.max_entries;
            self.entries.drain(0..drain);
            if self.scroll >= drain {
                self.scroll -= drain;
            } else {
                self.scroll = 0;
            }
        }
        if self.follow {
            let visible = self.filtered().count();
            self.scroll = visible.saturating_sub(1);
        }
    }

    pub fn filtered<'a>(&'a self) -> impl Iterator<Item = &'a runner::LogEntry> {
        let level_filter = &self.filter_level;
        let source_filter = self.filter_source.as_deref();
        let regex_filter = self.filter_regex.as_deref();

        let compiled_regex: Option<regex::Regex> = regex_filter
            .and_then(|p| regex::Regex::new(p).ok());

        self.entries.iter().filter(move |entry| {
            // Level filter
            if let Some(ref fl) = level_filter {
                if !level_matches(&entry.level, fl) {
                    return false;
                }
            }

            // Source filter
            if let Some(src) = source_filter {
                if !entry.source.contains(src) {
                    return false;
                }
            }

            // Regex filter
            if let Some(ref rx) = compiled_regex {
                if !rx.is_match(&entry.message) {
                    return false;
                }
            }

            true
        })
    }

    pub fn scroll_to_end(&mut self) {
        let count = self.filtered().count();
        self.scroll = count.saturating_sub(1);
    }
}

fn level_matches(entry_level: &runner::LogLevel, filter: &runner::LogLevel) -> bool {
    use runner::LogLevel;
    // Filter means "show this level and above"
    let entry_prio = level_priority(entry_level);
    let filter_prio = level_priority(filter);
    entry_prio <= filter_prio
}

fn level_priority(level: &runner::LogLevel) -> u8 {
    use runner::LogLevel;
    match level {
        LogLevel::Error => 0,
        LogLevel::Warn => 1,
        LogLevel::Info => 2,
        LogLevel::Debug => 3,
        LogLevel::Trace => 4,
        LogLevel::Raw => 5,
    }
}

fn level_color(level: &runner::LogLevel) -> Color {
    use runner::LogLevel;
    match level {
        LogLevel::Error => Color::Red,
        LogLevel::Warn => Color::Yellow,
        LogLevel::Info => Color::Green,
        LogLevel::Debug => Color::Blue,
        LogLevel::Trace => Color::DarkGray,
        LogLevel::Raw => Color::Gray,
    }
}

fn level_label(level: &runner::LogLevel) -> &'static str {
    use runner::LogLevel;
    match level {
        LogLevel::Error => "ERROR",
        LogLevel::Warn => "WARN ",
        LogLevel::Info => "INFO ",
        LogLevel::Debug => "DEBUG",
        LogLevel::Trace => "TRACE",
        LogLevel::Raw => "RAW  ",
    }
}

pub struct LogViewerWidget<'a> {
    pub state: &'a LogViewerState,
}

impl<'a> Widget for LogViewerWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Background
        let bg_style = Style::default().bg(Color::Rgb(10, 10, 18));
        for y in area.y..area.y + area.height {
            for x in area.x..area.x + area.width {
                buf.get_mut(x, y).set_char(' ').set_style(bg_style);
            }
        }

        if area.height < 3 || area.width < 10 {
            return;
        }

        // Draw border
        let border_style = Style::default().fg(Color::Cyan);
        let title = " Log Viewer ";
        let title_y = area.y;
        buf.get_mut(area.x, title_y).set_char('┌').set_style(border_style);
        buf.get_mut(area.x + area.width - 1, title_y).set_char('┐').set_style(border_style);
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

        for (i, ch) in title.chars().enumerate() {
            if area.x + 1 + i as u16 >= area.x + area.width - 1 {
                break;
            }
            buf.get_mut(area.x + 1 + i as u16, title_y)
                .set_char(ch)
                .set_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));
        }

        // Filter bar (second row)
        let filter_y = area.y + 1;
        if filter_y < area.y + area.height - 1 {
            let filter_info = format!(
                " Filter: src={} level={} regex={} | {} entries | {} ",
                self.state.filter_source.as_deref().unwrap_or("*"),
                self.state.filter_level.as_ref().map(level_label).unwrap_or("ALL"),
                self.state.filter_regex.as_deref().unwrap_or("*"),
                self.state.entries.len(),
                if self.state.follow { "FOLLOW" } else { "SCROLL" },
            );
            let filter_style = Style::default().fg(Color::DarkGray);
            for (i, ch) in filter_info.chars().enumerate() {
                let x = area.x + 1 + i as u16;
                if x >= area.x + area.width - 1 {
                    break;
                }
                buf.get_mut(x, filter_y).set_char(ch).set_style(filter_style);
            }
        }

        // Content area
        let content_y_start = area.y + 2;
        let content_height = area.height.saturating_sub(3) as usize;
        if content_height == 0 {
            return;
        }

        let filtered_entries: Vec<&runner::LogEntry> = self.state.filtered().collect();
        let total = filtered_entries.len();

        let scroll = self.state.scroll.min(total.saturating_sub(1));
        let visible = &filtered_entries[scroll..total.min(scroll + content_height)];

        for (row_idx, entry) in visible.iter().enumerate() {
            let row_y = content_y_start + row_idx as u16;
            if row_y >= area.y + area.height - 1 {
                break;
            }

            let level_col = level_color(&entry.level);
            let level_lbl = level_label(&entry.level);

            // [LEVEL] source: message
            let prefix = format!("[{}] ", level_lbl);
            let x = area.x + 1;
            let mut col = 0usize;

            // Level label
            for ch in prefix.chars() {
                if x + col as u16 >= area.x + area.width - 1 {
                    break;
                }
                buf.get_mut(x + col as u16, row_y)
                    .set_char(ch)
                    .set_style(Style::default().fg(level_col));
                col += 1;
            }

            // Source
            let source_str = format!("{}: ", entry.source);
            for ch in source_str.chars() {
                if x + col as u16 >= area.x + area.width - 1 {
                    break;
                }
                buf.get_mut(x + col as u16, row_y)
                    .set_char(ch)
                    .set_style(Style::default().fg(Color::Cyan));
                col += 1;
            }

            // Message
            for ch in entry.message.chars() {
                if x + col as u16 >= area.x + area.width - 1 {
                    break;
                }
                buf.get_mut(x + col as u16, row_y)
                    .set_char(ch)
                    .set_style(Style::default().fg(Color::White));
                col += 1;
            }
        }

        // Scroll indicator
        if total > content_height && area.height > 3 {
            let scroll_info = format!(" {}/{} ", scroll + 1, total);
            let info_x = area.x + area.width.saturating_sub(scroll_info.len() as u16 + 1);
            let info_y = area.y + area.height - 1;
            for (i, ch) in scroll_info.chars().enumerate() {
                if info_x + i as u16 >= area.x + area.width - 1 {
                    break;
                }
                buf.get_mut(info_x + i as u16, info_y)
                    .set_char(ch)
                    .set_style(Style::default().fg(Color::DarkGray));
            }
        }
    }
}
