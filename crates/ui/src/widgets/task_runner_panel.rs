#![allow(dead_code, unused_imports, unused_variables)]
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Widget},
};

pub struct TaskRunnerState {
    pub open: bool,
    pub selected: usize,
    pub watch_mode: bool,
    pub log_scroll: usize,
    pub log_filter: String,
}

impl TaskRunnerState {
    pub fn new() -> Self {
        Self {
            open: false,
            selected: 0,
            watch_mode: false,
            log_scroll: 0,
            log_filter: String::new(),
        }
    }
}

pub struct TaskRunnerWidget<'a> {
    pub state: &'a TaskRunnerState,
    pub records: &'a [runner::TaskRecord],
}

impl<'a> Widget for TaskRunnerWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        use runner::TaskStatus;

        // Background
        let bg_style = Style::default().bg(Color::Rgb(15, 15, 25));
        for y in area.y..area.y + area.height {
            for x in area.x..area.x + area.width {
                buf.get_mut(x, y).set_char(' ').set_style(bg_style);
            }
        }

        let block = Block::default()
            .title(" Task Runner ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan));
        block.render(area, buf);

        let inner = Rect {
            x: area.x + 1,
            y: area.y + 1,
            width: area.width.saturating_sub(2),
            height: area.height.saturating_sub(2),
        };

        if inner.width == 0 || inner.height == 0 {
            return;
        }

        // Split: top half for task list, bottom half for log
        let split_y = inner.height / 2;
        let list_area = Rect {
            height: split_y.max(3),
            ..inner
        };
        let log_area = Rect {
            y: inner.y + split_y.max(3),
            height: inner.height.saturating_sub(split_y.max(3)),
            ..inner
        };

        // Header row
        if list_area.height > 0 {
            let header = format!(
                "{:<20} {:<10} {:<12} {}",
                "Task", "Status", "Duration", "Last Log"
            );
            let style = Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD);
            let x = list_area.x;
            let y = list_area.y;
            for (i, ch) in header.chars().enumerate() {
                if x + i as u16 >= list_area.x + list_area.width {
                    break;
                }
                buf.get_mut(x + i as u16, y).set_char(ch).set_style(style);
            }
        }

        // Task rows
        let row_start = list_area.y + 1;
        for (i, record) in self.records.iter().enumerate() {
            let row_y = row_start + i as u16;
            if row_y >= list_area.y + list_area.height {
                break;
            }

            let is_selected = i == self.state.selected;
            let bg = if is_selected {
                Color::Rgb(30, 30, 50)
            } else {
                Color::Reset
            };

            let status_icon = record.status.icon();
            let duration = record
                .duration_ms
                .map(|d| format!("{:.1}s", d as f64 / 1000.0))
                .unwrap_or_else(|| "-".to_string());
            let last_log = record
                .last_log
                .as_deref()
                .unwrap_or("-")
                .chars()
                .take(30)
                .collect::<String>();

            let row = format!(
                "{:<20} {:<10} {:<12} {}",
                record.def.name.chars().take(20).collect::<String>(),
                status_icon,
                duration,
                last_log,
            );

            let style = Style::default().bg(bg);
            for (col, ch) in row.chars().enumerate() {
                let x = list_area.x + col as u16;
                if x >= list_area.x + list_area.width {
                    break;
                }
                buf.get_mut(x, row_y).set_char(ch).set_style(style);
            }
        }

        // Log panel
        if log_area.height > 1 {
            let log_border_style = Style::default().fg(Color::DarkGray);
            let log_title = if let Some(record) = self.records.get(self.state.selected) {
                format!(" Log: {} ", record.def.name)
            } else {
                " Log ".to_string()
            };
            // Draw simple border
            let border_y = log_area.y;
            let title_x = log_area.x + 1;
            buf.get_mut(log_area.x, border_y)
                .set_char('─')
                .set_style(log_border_style);
            for (i, ch) in log_title.chars().enumerate() {
                if title_x + i as u16 >= log_area.x + log_area.width {
                    break;
                }
                buf.get_mut(title_x + i as u16, border_y)
                    .set_char(ch)
                    .set_style(Style::default().fg(Color::Cyan));
            }

            // Show last_log content for selected task
            if let Some(record) = self.records.get(self.state.selected) {
                let log_text = record.last_log.as_deref().unwrap_or("No output yet");
                let content_y = log_area.y + 1;
                if content_y < log_area.y + log_area.height {
                    let style = Style::default().fg(Color::Gray);
                    for (i, ch) in log_text.chars().enumerate() {
                        let x = log_area.x + i as u16;
                        if x >= log_area.x + log_area.width {
                            break;
                        }
                        buf.get_mut(x, content_y).set_char(ch).set_style(style);
                    }
                }
            }
        }

        // Key hints at bottom
        if area.height > 2 {
            let hints = " [Enter]=Run  [c]=Cancel  [r]=Reload  [/]=Filter ";
            let hint_y = area.y + area.height - 1;
            let hint_style = Style::default().fg(Color::DarkGray);
            for (i, ch) in hints.chars().enumerate() {
                let x = area.x + i as u16;
                if x >= area.x + area.width {
                    break;
                }
                buf.get_mut(x, hint_y).set_char(ch).set_style(hint_style);
            }
        }
    }
}
