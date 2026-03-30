#![allow(dead_code, unused_imports, unused_variables)]
use std::collections::HashMap;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::Widget,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DiffLineKind {
    Context,
    Added,
    Removed,
    Header,
}

#[derive(Clone, Debug)]
pub struct DiffLine {
    pub kind: DiffLineKind,
    pub content: String,
    pub old_line: Option<usize>,
    pub new_line: Option<usize>,
}

pub struct DiffHunk {
    pub header: String,
    pub lines: Vec<DiffLine>,
}

pub struct DiffReviewState {
    pub open: bool,
    pub hunks: Vec<DiffHunk>,
    pub selected_hunk: usize,
    pub file_path: String,
    pub ref_name: String, // "HEAD" by default
    pub ai_annotations: HashMap<usize, String>, // hunk_idx -> annotation
    pub scroll: usize,
}

impl DiffReviewState {
    pub fn new() -> Self {
        Self {
            open: false,
            hunks: Vec::new(),
            selected_hunk: 0,
            file_path: String::new(),
            ref_name: "HEAD".to_string(),
            ai_annotations: HashMap::new(),
            scroll: 0,
        }
    }

    pub fn load_diff(&mut self, file_path: &str, ref_name: &str, diff_text: &str) {
        self.file_path = file_path.to_string();
        self.ref_name = ref_name.to_string();
        self.hunks.clear();
        self.selected_hunk = 0;
        self.scroll = 0;

        let mut current_hunk: Option<DiffHunk> = None;
        let mut old_line = 0usize;
        let mut new_line = 0usize;

        for line in diff_text.lines() {
            if line.starts_with("@@") {
                // Save previous hunk
                if let Some(hunk) = current_hunk.take() {
                    self.hunks.push(hunk);
                }
                // Parse hunk header: @@ -old_start,old_count +new_start,new_count @@
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 3 {
                    let old_part = parts[1].trim_start_matches('-');
                    let new_part = parts[2].trim_start_matches('+');
                    old_line = old_part
                        .split(',')
                        .next()
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(1);
                    new_line = new_part
                        .split(',')
                        .next()
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(1);
                }
                current_hunk = Some(DiffHunk {
                    header: line.to_string(),
                    lines: vec![DiffLine {
                        kind: DiffLineKind::Header,
                        content: line.to_string(),
                        old_line: None,
                        new_line: None,
                    }],
                });
            } else if let Some(ref mut hunk) = current_hunk {
                let (kind, content) = if line.starts_with('+') {
                    let dl = DiffLine {
                        kind: DiffLineKind::Added,
                        content: line[1..].to_string(),
                        old_line: None,
                        new_line: Some(new_line),
                    };
                    new_line += 1;
                    hunk.lines.push(dl);
                    continue;
                } else if line.starts_with('-') {
                    let dl = DiffLine {
                        kind: DiffLineKind::Removed,
                        content: line[1..].to_string(),
                        old_line: Some(old_line),
                        new_line: None,
                    };
                    old_line += 1;
                    hunk.lines.push(dl);
                    continue;
                } else {
                    let dl = DiffLine {
                        kind: DiffLineKind::Context,
                        content: if line.starts_with(' ') {
                            line[1..].to_string()
                        } else {
                            line.to_string()
                        },
                        old_line: Some(old_line),
                        new_line: Some(new_line),
                    };
                    old_line += 1;
                    new_line += 1;
                    hunk.lines.push(dl);
                    continue;
                };
            }
        }

        if let Some(hunk) = current_hunk.take() {
            self.hunks.push(hunk);
        }
    }

    pub fn next_hunk(&mut self) {
        if self.selected_hunk + 1 < self.hunks.len() {
            self.selected_hunk += 1;
        }
    }

    pub fn prev_hunk(&mut self) {
        if self.selected_hunk > 0 {
            self.selected_hunk -= 1;
        }
    }
}

pub struct DiffReviewWidget<'a> {
    pub state: &'a DiffReviewState,
}

impl<'a> Widget for DiffReviewWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Background
        let bg_style = Style::default().bg(Color::Rgb(12, 12, 22));
        for y in area.y..area.y + area.height {
            for x in area.x..area.x + area.width {
                buf.get_mut(x, y).set_char(' ').set_style(bg_style);
            }
        }

        if area.height < 4 || area.width < 20 {
            return;
        }

        // Title bar
        let title = format!(
            " Diff Review: {} ({}) — Hunk {}/{} ",
            self.state.file_path,
            self.state.ref_name,
            self.state.selected_hunk + 1,
            self.state.hunks.len().max(1),
        );
        let title_style = Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD);
        for (i, ch) in title.chars().enumerate() {
            if area.x + i as u16 >= area.x + area.width {
                break;
            }
            buf.get_mut(area.x + i as u16, area.y)
                .set_char(ch)
                .set_style(title_style);
        }

        // Two-column layout
        let col_w = (area.width / 2).saturating_sub(1);
        let left_area = Rect {
            x: area.x,
            y: area.y + 1,
            width: col_w,
            height: area.height.saturating_sub(2),
        };
        let divider_x = area.x + col_w;
        let right_area = Rect {
            x: divider_x + 1,
            y: area.y + 1,
            width: area.width.saturating_sub(col_w + 1),
            height: area.height.saturating_sub(2),
        };

        // Draw divider
        for y in (area.y + 1)..(area.y + area.height - 1) {
            buf.get_mut(divider_x, y)
                .set_char('│')
                .set_style(Style::default().fg(Color::DarkGray));
        }

        // Column headers
        if left_area.height > 0 {
            let old_header = format!(" OLD ({}) ", self.state.ref_name);
            let new_header = " NEW (working) ";
            let header_style = Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD);
            for (i, ch) in old_header.chars().enumerate() {
                if left_area.x + i as u16 >= left_area.x + left_area.width {
                    break;
                }
                buf.get_mut(left_area.x + i as u16, left_area.y)
                    .set_char(ch)
                    .set_style(header_style);
            }
            for (i, ch) in new_header.chars().enumerate() {
                if right_area.x + i as u16 >= right_area.x + right_area.width {
                    break;
                }
                buf.get_mut(right_area.x + i as u16, right_area.y)
                    .set_char(ch)
                    .set_style(header_style);
            }
        }

        let content_y = area.y + 2;
        let content_h = area.height.saturating_sub(3) as usize;

        if let Some(hunk) = self.state.hunks.get(self.state.selected_hunk) {
            let scroll = self.state.scroll;
            let lines = &hunk.lines;

            for (i, dl) in lines.iter().skip(scroll).enumerate() {
                if i >= content_h {
                    break;
                }
                let row_y = content_y + i as u16;

                match dl.kind {
                    DiffLineKind::Header => {
                        let style = Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD);
                        for (j, ch) in dl.content.chars().enumerate() {
                            let x = area.x + j as u16;
                            if x >= area.x + area.width {
                                break;
                            }
                            buf.get_mut(x, row_y).set_char(ch).set_style(style);
                        }
                    }
                    DiffLineKind::Removed => {
                        // Show in left column (old) with red background
                        let line_no = dl
                            .old_line
                            .map(|n| format!("{:>4} ", n))
                            .unwrap_or_else(|| "     ".to_string());
                        let style = Style::default()
                            .fg(Color::Red)
                            .bg(Color::Rgb(40, 10, 10));
                        let text = format!("-{}{}", line_no, dl.content);
                        for (j, ch) in text.chars().enumerate() {
                            let x = left_area.x + j as u16;
                            if x >= left_area.x + left_area.width {
                                break;
                            }
                            buf.get_mut(x, row_y).set_char(ch).set_style(style);
                        }
                    }
                    DiffLineKind::Added => {
                        // Show in right column (new) with green background
                        let line_no = dl
                            .new_line
                            .map(|n| format!("{:>4} ", n))
                            .unwrap_or_else(|| "     ".to_string());
                        let style = Style::default()
                            .fg(Color::Green)
                            .bg(Color::Rgb(10, 40, 10));
                        let text = format!("+{}{}", line_no, dl.content);
                        for (j, ch) in text.chars().enumerate() {
                            let x = right_area.x + j as u16;
                            if x >= right_area.x + right_area.width {
                                break;
                            }
                            buf.get_mut(x, row_y).set_char(ch).set_style(style);
                        }
                    }
                    DiffLineKind::Context => {
                        // Show in both columns
                        let old_no = dl
                            .old_line
                            .map(|n| format!("{:>4} ", n))
                            .unwrap_or_else(|| "     ".to_string());
                        let new_no = dl
                            .new_line
                            .map(|n| format!("{:>4} ", n))
                            .unwrap_or_else(|| "     ".to_string());
                        let style = Style::default().fg(Color::Gray);
                        let left_text = format!(" {}{}", old_no, dl.content);
                        let right_text = format!(" {}{}", new_no, dl.content);
                        for (j, ch) in left_text.chars().enumerate() {
                            let x = left_area.x + j as u16;
                            if x >= left_area.x + left_area.width {
                                break;
                            }
                            buf.get_mut(x, row_y).set_char(ch).set_style(style);
                        }
                        for (j, ch) in right_text.chars().enumerate() {
                            let x = right_area.x + j as u16;
                            if x >= right_area.x + right_area.width {
                                break;
                            }
                            buf.get_mut(x, row_y).set_char(ch).set_style(style);
                        }
                    }
                }
            }

            // AI annotation if present
            if let Some(annotation) = self.state.ai_annotations.get(&self.state.selected_hunk) {
                let ann_y = area.y + area.height - 2;
                if ann_y > content_y {
                    let ann_text = format!(" AI: {} ", annotation);
                    let ann_style = Style::default()
                        .fg(Color::Magenta)
                        .bg(Color::Rgb(30, 10, 30));
                    for (i, ch) in ann_text.chars().enumerate() {
                        if area.x + i as u16 >= area.x + area.width {
                            break;
                        }
                        buf.get_mut(area.x + i as u16, ann_y)
                            .set_char(ch)
                            .set_style(ann_style);
                    }
                }
            }
        } else if self.state.hunks.is_empty() {
            let msg = " No diff available — file matches HEAD ";
            let msg_style = Style::default().fg(Color::DarkGray);
            for (i, ch) in msg.chars().enumerate() {
                if area.x + i as u16 >= area.x + area.width {
                    break;
                }
                buf.get_mut(area.x + i as u16, content_y)
                    .set_char(ch)
                    .set_style(msg_style);
            }
        }

        // Key hints
        let hints = " [n]=Next Hunk  [p]=Prev Hunk  [s]=Stage  [r]=Revert  [a]=AI Annotate  [q]=Close ";
        let hints_y = area.y + area.height - 1;
        let hints_style = Style::default().fg(Color::DarkGray);
        for (i, ch) in hints.chars().enumerate() {
            if area.x + i as u16 >= area.x + area.width {
                break;
            }
            buf.get_mut(area.x + i as u16, hints_y)
                .set_char(ch)
                .set_style(hints_style);
        }
    }
}
