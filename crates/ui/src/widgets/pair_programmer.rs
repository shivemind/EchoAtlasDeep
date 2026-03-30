#![allow(dead_code, unused_imports, unused_variables)]
//! AI pair programmer side panel — Phase 12 Point 49.
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::Widget,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SuggestionKind {
    Completion,
    BugWarning,
    Refactor,
    Architecture,
}

impl SuggestionKind {
    pub fn icon(&self) -> char {
        match self {
            SuggestionKind::Completion => '✎',
            SuggestionKind::BugWarning => '⚠',
            SuggestionKind::Refactor => '⟳',
            SuggestionKind::Architecture => '⬡',
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            SuggestionKind::Completion => "Complete",
            SuggestionKind::BugWarning => "Bug",
            SuggestionKind::Refactor => "Refactor",
            SuggestionKind::Architecture => "Arch",
        }
    }

    pub fn color(&self) -> Color {
        match self {
            SuggestionKind::Completion => Color::Cyan,
            SuggestionKind::BugWarning => Color::Red,
            SuggestionKind::Refactor => Color::Yellow,
            SuggestionKind::Architecture => Color::Magenta,
        }
    }
}

#[derive(Clone, Debug)]
pub struct AiSuggestion {
    pub title: String,
    pub rationale: String,
    pub code: Option<String>,
    pub confidence: f32,
    pub kind: SuggestionKind,
}

pub struct PairProgrammerState {
    pub active: bool,
    pub suggestions: Vec<AiSuggestion>,
    pub selected: usize,
    pub last_keystrokes: Vec<char>,
    pub batch_pending: bool,
    pub thinking: bool,
}

impl PairProgrammerState {
    pub fn new() -> Self {
        Self {
            active: false,
            suggestions: Vec::new(),
            selected: 0,
            last_keystrokes: Vec::new(),
            batch_pending: false,
            thinking: false,
        }
    }

    pub fn push_key(&mut self, c: char) {
        self.last_keystrokes.push(c);
        // Keep only the last 200 keystrokes
        if self.last_keystrokes.len() > 200 {
            self.last_keystrokes.drain(..self.last_keystrokes.len() - 200);
        }
        self.batch_pending = true;
    }

    pub fn clear_suggestions(&mut self) {
        self.suggestions.clear();
        self.selected = 0;
    }

    pub fn add_suggestion(&mut self, s: AiSuggestion) {
        self.suggestions.push(s);
        // Keep sorted by confidence descending
        self.suggestions.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap_or(std::cmp::Ordering::Equal));
    }

    pub fn selected_suggestion(&self) -> Option<&AiSuggestion> {
        self.suggestions.get(self.selected)
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    pub fn move_down(&mut self) {
        if self.selected + 1 < self.suggestions.len() {
            self.selected += 1;
        }
    }
}

impl Default for PairProgrammerState {
    fn default() -> Self {
        Self::new()
    }
}

pub struct PairProgrammerWidget<'a> {
    pub state: &'a PairProgrammerState,
}

impl Widget for PairProgrammerWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 6 || area.height < 4 {
            return;
        }

        let bg = Style::default().bg(Color::Rgb(16, 14, 24));
        let border_style = Style::default().fg(Color::Magenta);
        let title_style = Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD);
        let label_style = Style::default().fg(Color::DarkGray);
        let selected_style = Style::default().bg(Color::Rgb(35, 20, 45)).fg(Color::White);
        let normal_style = Style::default().fg(Color::Gray);
        let spinner = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];

        // Fill background
        for row in area.y..area.y + area.height {
            for col in area.x..area.x + area.width {
                buf.get_mut(col, row).set_char(' ').set_style(bg);
            }
        }

        // Border
        for row in area.y..area.y + area.height {
            buf.get_mut(area.x, row).set_char('│').set_style(border_style);
            buf.get_mut(area.x + area.width - 1, row).set_char('│').set_style(border_style);
        }
        for col in area.x..area.x + area.width {
            buf.get_mut(col, area.y).set_char('─').set_style(border_style);
            buf.get_mut(col, area.y + area.height - 1).set_char('─').set_style(border_style);
        }
        buf.get_mut(area.x, area.y).set_char('┌').set_style(border_style);
        buf.get_mut(area.x + area.width - 1, area.y).set_char('┐').set_style(border_style);
        buf.get_mut(area.x, area.y + area.height - 1).set_char('└').set_style(border_style);
        buf.get_mut(area.x + area.width - 1, area.y + area.height - 1).set_char('┘').set_style(border_style);

        // Title
        let title = "AI Pair";
        for (i, ch) in title.chars().take((area.width as usize).saturating_sub(2)).enumerate() {
            buf.get_mut(area.x + 1 + i as u16, area.y).set_char(ch).set_style(title_style);
        }

        // Thinking/batch indicator
        if self.state.thinking || self.state.batch_pending {
            let spin_ch = spinner[0];
            buf.get_mut(area.x + area.width - 2, area.y).set_char(spin_ch).set_style(
                Style::default().fg(Color::Yellow)
            );
        }

        let content_y = area.y + 1;

        if self.state.suggestions.is_empty() {
            let empty_lines = [
                "Observing",
                "keystrokes...",
                "",
                "Suggestions",
                "appear here.",
            ];
            for (i, line) in empty_lines.iter().enumerate() {
                let ly = content_y + i as u16;
                if ly >= area.y + area.height - 1 { break; }
                for (j, ch) in line.chars().take((area.width as usize).saturating_sub(2)).enumerate() {
                    buf.get_mut(area.x + 1 + j as u16, ly).set_char(ch).set_style(label_style);
                }
            }
            return;
        }

        // Suggestions list (compact, one entry per row + detail for selected)
        let list_height = self.state.suggestions.len().min(5);
        let detail_start = content_y + list_height as u16 + 1;

        for (idx, suggestion) in self.state.suggestions.iter().enumerate().take(list_height) {
            let ry = content_y + idx as u16;
            if ry >= area.y + area.height - 1 { break; }
            let is_sel = idx == self.state.selected;
            let row_style = if is_sel { selected_style } else { normal_style };

            for col in area.x + 1..area.x + area.width - 1 {
                buf.get_mut(col, ry).set_char(' ').set_style(row_style);
            }

            // Kind icon
            let icon = suggestion.kind.icon();
            let icon_style = Style::default()
                .fg(suggestion.kind.color())
                .bg(if is_sel { Color::Rgb(35,20,45) } else { Color::Reset });
            buf.get_mut(area.x + 1, ry).set_char(icon).set_style(icon_style);

            // Confidence bar (5 chars)
            let conf_filled = (suggestion.confidence * 5.0) as usize;
            let conf_x = area.x + 3;
            for ci in 0..5usize {
                if conf_x + ci as u16 >= area.x + area.width - 1 { break; }
                let bar_ch = if ci < conf_filled { '▪' } else { '·' };
                let bar_color = if suggestion.confidence > 0.8 { Color::Green }
                    else if suggestion.confidence > 0.5 { Color::Yellow }
                    else { Color::DarkGray };
                buf.get_mut(conf_x + ci as u16, ry).set_char(bar_ch).set_style(
                    Style::default().fg(bar_color).bg(if is_sel { Color::Rgb(35,20,45) } else { Color::Reset })
                );
            }

            // Title (truncated)
            let title_x = area.x + 9;
            let title_w = (area.width as usize).saturating_sub(10);
            for (ti, ch) in suggestion.title.chars().take(title_w).enumerate() {
                let col = title_x + ti as u16;
                if col >= area.x + area.width - 1 { break; }
                buf.get_mut(col, ry).set_char(ch).set_style(row_style);
            }
        }

        // Separator before detail
        if detail_start < area.y + area.height - 3 {
            for col in area.x + 1..area.x + area.width - 1 {
                buf.get_mut(col, detail_start - 1).set_char('─').set_style(border_style);
            }

            // Selected suggestion detail
            if let Some(sugg) = self.state.selected_suggestion() {
                // Rationale (wrapped)
                let rat_chars: Vec<char> = sugg.rationale.chars().collect();
                let rat_w = (area.width as usize).saturating_sub(3);
                let rat_lines = (rat_chars.len() + rat_w - 1) / rat_w.max(1);
                let max_rat_lines = 3usize;
                for (li, chunk) in rat_chars.chunks(rat_w.max(1)).enumerate().take(max_rat_lines) {
                    let ly = detail_start + li as u16;
                    if ly >= area.y + area.height - 2 { break; }
                    for (ci, ch) in chunk.iter().enumerate() {
                        let col = area.x + 1 + ci as u16;
                        if col >= area.x + area.width - 1 { break; }
                        buf.get_mut(col, ly).set_char(*ch).set_style(
                            Style::default().fg(Color::White)
                        );
                    }
                }

                // Code snippet if present
                if let Some(ref code) = sugg.code {
                    let code_start = detail_start + max_rat_lines as u16 + 1;
                    if code_start < area.y + area.height - 2 {
                        for col in area.x + 1..area.x + area.width - 1 {
                            buf.get_mut(col, code_start - 1).set_char('·').set_style(label_style);
                        }
                        for (li, line) in code.lines().enumerate().take(3) {
                            let ly = code_start + li as u16;
                            if ly >= area.y + area.height - 2 { break; }
                            for col in area.x + 1..area.x + area.width - 1 {
                                buf.get_mut(col, ly).set_char(' ').set_style(
                                    Style::default().bg(Color::Rgb(20, 20, 32))
                                );
                            }
                            let display: String = line.chars().take((area.width as usize).saturating_sub(3)).collect();
                            for (ci, ch) in display.chars().enumerate() {
                                let col = area.x + 1 + ci as u16;
                                if col >= area.x + area.width - 1 { break; }
                                buf.get_mut(col, ly).set_char(ch).set_style(
                                    Style::default().fg(Color::LightCyan).bg(Color::Rgb(20,20,32))
                                );
                            }
                        }
                    }
                }

                // Confidence %
                let conf_pct = format!("{:.0}%", sugg.confidence * 100.0);
                let conf_y = area.y + area.height - 2;
                for (i, ch) in conf_pct.chars().enumerate() {
                    let col = area.x + 1 + i as u16;
                    if col >= area.x + area.width - 1 { break; }
                    buf.get_mut(col, conf_y).set_char(ch).set_style(
                        Style::default().fg(Color::DarkGray)
                    );
                }
            }
        }
    }
}
